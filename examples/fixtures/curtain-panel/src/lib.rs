use pax_kit::*;
extern crate self as curtain_panel;
pub mod sateen;
pub use sateen::Sateen;

pax_kit::runnable! { root: crate::Example }

/// Playback speed used by the fixture and website for the passive side draft.
pub const DEFAULT_DRAFT_SPEED: f64 = 1.95;

/// Advance the cyclic cloth phase at the fixture’s desktop or compact cadence.
/// Initialize `last_ms` when the visible clock mounts to resume the held pose.
pub fn advance_draft(
    ctx: &NodeContext,
    last_ms: &Property<f64>,
    phase: &Property<f64>,
    speed: f64,
) {
    let now = ctx.elapsed_time_millis() as f64;
    let elapsed = now - last_ms.get();
    let hz = if ctx.viewport.get().width < 700. {
        30.
    } else {
        60.
    };
    if elapsed >= 100. || elapsed < 0. {
        last_ms.set(now);
    } else if elapsed >= 1000. / hz - 1. {
        last_ms.set(now);
        phase.set((phase.get() + speed * elapsed / 11000.).rem_euclid(1.));
    }
}

#[pax]
#[file("lib.pax")]
#[custom(Default)]
pub struct Example {
    pub phase: Property<f64>,
    pub playing: Property<bool>,
    pub show_title: Property<bool>,
    pub visible: Property<bool>,
    pub last_ms: Property<f64>,
    pub mode: Property<usize>,
    pub lighting: Property<usize>,
    pub light_position: Property<f64>,
    pub half_resolution: Property<bool>,
    pub spacing: Property<f64>,
    pub depth: Property<f64>,
    pub speed: Property<f64>,
    pub roughness: Property<f64>,
    pub sheen: Property<f64>,
}

impl Default for Example {
    fn default() -> Self {
        let cloth = Sateen::default();
        Self {
            phase: cloth.phase,
            playing: Property::new(false),
            show_title: Property::new(true),
            visible: Property::new(false),
            last_ms: Property::new(0.),
            mode: Property::new(0),
            lighting: Property::new(0),
            light_position: Property::new(0.646),
            half_resolution: cloth.half_resolution,
            spacing: cloth.spacing,
            depth: cloth.depth,
            speed: Property::new(DEFAULT_DRAFT_SPEED),
            roughness: cloth.roughness,
            sheen: cloth.sheen,
        }
    }
}

impl Example {
    pub fn visibility(&mut self, _: &NodeContext, event: Event<ViewportProximityChange>) {
        self.visible.set_if_neq(event.current.is_in_viewport());
    }

    pub fn reset_clock(&mut self, ctx: &NodeContext) {
        self.last_ms.set(ctx.elapsed_time_millis() as f64);
    }

    pub fn advance(&mut self, ctx: &NodeContext) {
        advance_draft(ctx, &self.last_ms, &self.phase, self.speed.get());
    }

    pub fn playback(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.playing.set(!self.playing.get());
    }

    pub fn toggle_title(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.show_title.set(!self.show_title.get());
    }

    pub fn compare(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.mode.set((self.mode.get() + 1) % 3);
    }

    pub fn change_light(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.lighting.set((self.lighting.get() + 1) % 3);
    }

    pub fn quality(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.half_resolution.set(!self.half_resolution.get());
    }
}
