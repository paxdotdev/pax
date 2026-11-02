//! Shared-property publication/import cost; this is not a frame-rate benchmark.
//! Run with `cargo run -p pax-runtime-api --example property_import_bench --release`.

use pax_runtime_api::properties::{Property, PropertyGraph};
use std::{hint::black_box, time::Instant};

fn main() {
    const TURNS: usize = 500;
    println!("Active bindings only; no dormant views, rendering, or effect evaluation.");
    println!("Five 500-turn averages; changed=1 includes publication and import.");
    for count in [100, 10_000, 50_000] {
        let graph = PropertyGraph::new(|| {});
        let _entered = graph.enter();
        let properties: Vec<_> = (0..count).map(|_| Property::new(0_u32)).collect();
        for value in &properties {
            black_box(value.local());
        }
        for changed in [false, true] {
            let mut samples = Vec::new();
            for _ in 0..5 {
                let start = Instant::now();
                for value in 1..=TURNS {
                    if changed {
                        properties[0].set(value as u32);
                    }
                    assert_eq!(black_box(graph.import()), usize::from(changed));
                }
                samples.push(start.elapsed().as_secs_f64() * 1e6 / TURNS as f64);
            }
            samples.sort_by(f64::total_cmp);
            println!(
                "bindings={count} changed={} median_us/turn={:.3} min={:.3} max={:.3}",
                usize::from(changed),
                samples[2],
                samples[0],
                samples[4]
            );
        }
        graph.close();
    }
}
