use pax_kit::*;

mod tile_lab;
pub use tile_lab::*;

pax_kit::runnable! { root: crate::Example }

#[pax]
#[file("lib.pax")]
#[has_helpers]
pub struct Example {
    pub tile_lab: Property<bool>,
    pub solid: Property<bool>,
    pub camera_pan: Property<f64>,
    pub camera_zoom: Property<f64>,
    pub camera_panned: Property<bool>,
    pub camera_zoomed_out: Property<bool>,
    pub perspective: Property<bool>,
    pub camera_fov: Property<f64>,
    pub camera_dolly: Property<f64>,
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
    pub fn toggle_solid(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.solid.set(!self.solid.get());
    }

    pub fn toggle_lab(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.tile_lab.set(!self.tile_lab.get());
        self.playing.set(false);
        self.camera_pan.local().cancel_transitions();
        self.camera_zoom.local().cancel_transitions();
        self.camera_pan.set(0.0);
        self.camera_zoom.set(1.0);
        self.camera_panned.set(false);
        self.camera_zoomed_out.set(false);
        self.camera_fov.set(60.0);
        self.camera_dolly.set(0.0);
        self.perspective.set(true);
    }

    pub fn mount(&mut self, _ctx: &NodeContext) {
        self.reset_values();
        self.playing.set(true);
        self.note.set("Native text, in 3D.".into());
        self.level.set(42.0);
    }

    fn reset_values(&mut self) {
        self.camera_pan.local().cancel_transitions();
        self.camera_zoom.local().cancel_transitions();
        self.camera_pan.set(0.0);
        self.camera_zoom.set(1.0);
        self.camera_panned.set(false);
        self.camera_zoomed_out.set(false);
        self.perspective.set(false);
        self.camera_fov.set(60.0);
        self.camera_dolly.set(0.0);
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

    pub fn move_camera(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        let panned = !self.camera_panned.get();
        self.camera_panned.set(panned);
        self.camera_pan.local().ease_to(
            if panned { 80.0 } else { 0.0 },
            Duration::Milliseconds(450.into()),
            EasingCurve::InOutQuad,
        );
    }

    pub fn zoom_camera(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        let zoomed_out = !self.camera_zoomed_out.get();
        self.camera_zoomed_out.set(zoomed_out);
        self.camera_zoom.local().ease_to(
            if zoomed_out { 0.8 } else { 1.0 },
            Duration::Milliseconds(450.into()),
            EasingCurve::InOutQuad,
        );
    }

    pub fn toggle_perspective(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.perspective.set(!self.perspective.get());
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
