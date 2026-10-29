use pax_kit::*;

pax_kit::runnable! { root: crate::Example }

#[pax]
#[file("lib.pax")]
#[has_helpers]
pub struct Example {
    pub tilt_x: Property<f64>,
    pub tilt_y: Property<f64>,
    pub depth: Property<f64>,
    pub crossing_depth: Property<f64>,
    pub child_tilt: Property<f64>,
    pub alpha: Property<f64>,
    pub playing: Property<bool>,
    pub clipping: Property<bool>,
    pub crossing: Property<bool>,
    pub note: Property<String>,
    pub level: Property<f64>,
    pub clicks: Property<u64>,
    pub last_frame: Property<u64>,
    pub phase: Property<f64>,
}

impl Example {
    pub fn mount(&mut self, _ctx: &NodeContext) {
        self.reset_values();
        self.playing.set(true);
        self.note.set("Native text, in 3D.".into());
        self.level.set(42.0);
    }

    fn reset_values(&mut self) {
        self.tilt_x.set(12.0);
        self.tilt_y.set(-28.0);
        self.depth.set(40.0);
        self.crossing_depth.set(-25.0);
        self.child_tilt.set(0.0);
        self.alpha.set(1.0);
        self.crossing.set(true);
        self.clipping.set(false);
        self.phase.set(0.0);
    }

    pub fn reset(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.playing.set(false);
        self.reset_values();
    }

    pub fn flat(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.playing.set(false);
        self.tilt_x.set(0.0);
        self.tilt_y.set(0.0);
        self.depth.set(0.0);
        self.child_tilt.set(0.0);
    }

    pub fn back(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.playing.set(false);
        self.tilt_x.set(0.0);
        self.tilt_y.set(180.0);
    }

    pub fn toggle_play(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.playing.set(!self.playing.get());
    }
    pub fn toggle_clip(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.clipping.set(!self.clipping.get());
    }
    pub fn toggle_crossing(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.crossing.set(!self.crossing.get());
    }
    pub fn pause(&mut self, _ctx: &NodeContext, _event: Event<SliderChange>) {
        self.playing.set(false);
    }
    pub fn press(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.clicks.set(self.clicks.get() + 1);
    }
    pub fn pre_render(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_millis.get();
        let elapsed = now.saturating_sub(self.last_frame.get());
        if elapsed < 33 {
            return;
        }
        self.last_frame.set(now);
        if !self.playing.get() {
            return;
        }
        let phase = self.phase.get() + elapsed.min(100) as f64 / 2400.0;
        self.phase.set(phase);
        self.tilt_y.set(-28.0 + phase.sin() * 22.0);
        self.tilt_x.set(12.0 + (phase * 0.7).sin() * 8.0);
        self.crossing_depth.set(-25.0 + (phase * 0.8).sin() * 80.0);
    }
}

#[helpers]
impl Example {
    pub fn rounded(value: f64) -> String {
        format!("{value:.0}")
    }
}
