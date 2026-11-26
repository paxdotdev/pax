pub use crate::surface::WaterSurface;
use pax_kit::*;

/// A top-down pool floor whose procedural daylight and scoped scene reflections share a wave field.
#[pax]
#[custom(Default)]
#[file("pool.pax")]
pub struct PoolFloor {
    /// Spatial frequency multiplier, 0.25–3. One unit gives roughly 108 logical pixels per main wave.
    pub density: Property<f64>,
    /// Wave strength, 0–1. Zero shows the unchanged floor paint and stops the water clock.
    pub amplitude: Property<f64>,
    /// Time multiplier, 0–3. Zero holds the current pose while scene lights remain responsive.
    pub speed: Property<f64>,
    /// Floor paint sets the hue of the wave troughs and supplies the solid CPU fallback.
    pub floor_color: Property<Color>,
    /// Render the effect at full physical resolution. The default is half on each axis.
    pub full_resolution: Property<bool>,
    /// Use the timing-matched identity pass to measure capture and composition overhead.
    pub identity: Property<bool>,
    pub phase: Property<f64>,
    pub visible: Property<bool>,
    #[pax(rust)]
    last_ms: f64,
}

impl Default for PoolFloor {
    fn default() -> Self {
        Self {
            density: Property::new(1.),
            amplitude: Property::new(0.65),
            speed: Property::new(1.),
            floor_color: Property::new(Color::from_hex("00C5D3")),
            full_resolution: Property::new(false),
            identity: Property::new(false),
            phase: Property::new(2.9),
            visible: Property::new(false),
            last_ms: 0.,
        }
    }
}

impl PoolFloor {
    pub fn visibility(&mut self, _: &NodeContext, event: Event<ViewportProximityChange>) {
        self.visible.set_if_neq(event.current.is_in_viewport());
    }

    pub fn reset_clock(&mut self, ctx: &NodeContext) {
        self.last_ms = ctx.elapsed_time_millis() as f64;
    }

    pub fn advance(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        let dt = now - self.last_ms;
        if !(0.0..100.0).contains(&dt) {
            self.last_ms = now;
            return;
        }
        // Coalesce high-refresh display ticks into at most 60 water updates per second.
        // The phase uses actual elapsed time, including the ticks we skipped.
        if (now * 0.06).floor() == (self.last_ms * 0.06).floor() {
            return;
        }
        self.last_ms = now;
        // Every time coefficient is an integer multiple of 0.01; the wrap preserves all waves.
        self.phase.set(
            (self.phase.get() + dt * 0.001 * self.speed.get().clamp(0., 3.))
                .rem_euclid(std::f64::consts::TAU * 100.),
        );
    }
}
