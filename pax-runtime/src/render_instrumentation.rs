//! Opt-in, native-only wall-clock measurements. These are not GPU timestamps.

use std::cell::RefCell;
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub(crate) enum Phase {
    Render,
    Lighting,
    LightingUpload,
    Prepare,
    Flush,
    Encode,
    Submit,
    Cleanup,
    Present,
    LayerInit,
}

const NAMES: [&str; 10] = [
    "render",
    "lighting",
    "lighting_upload",
    "prepare",
    "flush",
    "encode",
    "submit",
    "cleanup",
    "present",
    "layer_init",
];

#[derive(Clone, Copy)]
pub(crate) enum Counter {
    TileCreated,
    TileSetChanged,
    MatchingTileDiscarded,
    OriginRetargeted,
    TileResized,
    SceneReset,
    TileReused,
    ImageTextureUploaded,
    ImageTextureUploadBytes,
    ImageTextureReused,
}

const COUNTER_NAMES: [&str; 10] = [
    "tile_created",
    "tile_set_changed",
    "matching_tile_discarded",
    "origin_retargeted",
    "tile_resized",
    "scene_reset",
    "tile_reused",
    "image_texture_uploaded",
    "image_texture_upload_bytes",
    "image_texture_reused",
];

pub(crate) fn count(counter: Counter, count: usize) {
    if enabled() {
        WINDOW.with(|window| window.borrow_mut().counts[counter as usize] += count as u64);
    }
}

#[derive(Clone, Copy, Default)]
struct Samples {
    count: u64,
    total: Duration,
    max: Duration,
}

#[derive(Default)]
struct Window {
    started: Option<Instant>,
    samples: [Samples; 10],
    counts: [u64; 10],
}

thread_local! {
    static WINDOW: RefCell<Window> = RefCell::new(Window::default());
}

pub(crate) fn enabled() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        false
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *ENABLED.get_or_init(|| {
            std::env::var("PAX_RENDER_TIMINGS").is_ok_and(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "on"
                )
            })
        })
    }
}

pub(crate) struct Span {
    phase: Phase,
    started: Option<Instant>,
}

impl Span {
    pub(crate) fn new(phase: Phase) -> Self {
        Self {
            phase,
            started: enabled().then(Instant::now),
        }
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        let Some(started) = self.started else { return };
        let elapsed = started.elapsed();
        WINDOW.with(|window| {
            let mut window = window.borrow_mut();
            window.started.get_or_insert(started);
            let sample = &mut window.samples[self.phase as usize];
            sample.count += 1;
            sample.total += elapsed;
            sample.max = sample.max.max(elapsed);
            // Emit only at the end of a render, so nested spans stay in the same window.
            if matches!(self.phase, Phase::Render)
                && window
                    .started
                    .is_some_and(|start| start.elapsed().as_secs_f64() >= 1.0)
            {
                let fields = NAMES
                    .iter()
                    .zip(window.samples)
                    .map(|(name, sample)| {
                        format!(
                            "{name}={}/{:.3}/{:.3}/{:.3}",
                            sample.count,
                            sample.total.as_secs_f64() * 1000.0,
                            sample.total.as_secs_f64() * 1000.0 / sample.count.max(1) as f64,
                            sample.max.as_secs_f64() * 1000.0
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("[PaxRender] count/total_ms/avg_ms/max_ms {fields}");
                let counts = COUNTER_NAMES
                    .iter()
                    .zip(window.counts)
                    .map(|(name, count)| format!("{name}={count}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("[PaxRenderTiles] {counts}");
                *window = Window::default();
            }
        });
    }
}
