//! Focused geometry-service measurements; this is not a frame-rate benchmark.
//! Run with `cargo run -p pax-runtime --example scene_geometry_bench --release`.

#[path = "../src/scene_geometry.rs"]
#[allow(dead_code)]
mod scene_geometry;

use kurbo::{Affine, Rect};
use scene_geometry::{CanvasGeometry, PreparedCanvasGeometry, SceneGeometry};
#[path = "../tests/support/counting_allocator.rs"]
mod counting_allocator;
use counting_allocator::{ALLOCATIONS, LIVE_BYTES};
use std::sync::atomic::Ordering::Relaxed;
use std::time::Instant;

fn geometry(id: u32, spacing: f64) -> PreparedCanvasGeometry {
    PreparedCanvasGeometry::new(
        0,
        (300.0, 100.0),
        Affine::translate((0.0, id as f64 * spacing)),
        CanvasGeometry::for_layout((300.0, 100.0)),
    )
}

fn queries(scene: &mut SceneGeometry, name: &str, positions: impl Iterator<Item = f64>) {
    let allocations = ALLOCATIONS.load(Relaxed);
    let stats = scene.stats;
    let start = Instant::now();
    let mut count = 0;
    let mut fallback = 0;
    for y in positions {
        fallback += scene
            .query(0, &[Rect::new(0.0, y, 320.0, y + 720.0)])
            .is_none() as u64;
        count += 1;
    }
    let elapsed = start.elapsed();
    let allocated = ALLOCATIONS.load(Relaxed) - allocations;
    println!("  {name}: queries={count} us/query={:.2} allocations/query={:.2} candidates/query={:.2} full_replay_fallbacks={fallback}",
        elapsed.as_secs_f64() * 1e6 / count as f64,
        allocated as f64 / count as f64,
        (scene.stats.candidate_tests - stats.candidate_tests) as f64 / count as f64);
}

fn main() {
    println!("Geometry service only; allocator requested bytes exclude allocator overhead/RSS.");
    for count in [1_000, 10_000] {
        let baseline = LIVE_BYTES.load(Relaxed);
        let allocations = ALLOCATIONS.load(Relaxed);
        let start = Instant::now();
        let mut scene = SceneGeometry::default();
        for id in 0..count {
            scene.insert(id, geometry(id, 200.0));
        }
        let elapsed = start.elapsed();
        let live = LIVE_BYTES.load(Relaxed) - baseline;
        let allocated = ALLOCATIONS.load(Relaxed) - allocations;
        println!("nodes={count} initial_ms={:.2} live_bytes={live} bytes/node={:.1} allocations={allocated}", elapsed.as_secs_f64()*1e3, live as f64/count as f64);
        queries(
            &mut scene,
            "steady scroll",
            (0..2_000).map(|i| 80_000.0 + (i % 500) as f64 * 5.0),
        );
        queries(
            &mut scene,
            "large jumps",
            (0..200).map(|i| (i * 7919 % count) as f64 * 200.0),
        );
        queries(&mut scene, "empty region", (0..200).map(|_| -100_000.0));
        let start = Instant::now();
        for id in 0..count {
            scene.insert(id, geometry(id, 220.0));
        }
        println!(
            "  full relayout_ms={:.2}",
            start.elapsed().as_secs_f64() * 1e3
        );
        let stats = scene.stats;
        for _ in 0..2_000 {
            assert!(scene.take_dirty().is_empty());
        }
        assert_eq!(
            scene.stats, stats,
            "idle must perform no preparation/query work"
        );
    }
}
