use pax_kit::*;
mod scroll_study;
use scroll_study::ScrollStudy;
pax_kit::runnable! { root: crate::Example }

#[pax]
#[file("lib.pax")]
pub struct Example {
    pub study: Property<bool>,
    pub native: Property<bool>,
    pub solids: Property<bool>,
    pub fading: Property<bool>,
    pub fade_clock: Property<f64>,
    pub clock: Property<f64>,
    pub last_frame: Property<u64>,
    pub playing: Property<bool>,
    pub depth: Property<f64>,
    pub tilt: Property<f64>,
    pub note: Property<String>,
    pub presses: Property<u64>,
    pub solid_presses: Property<u64>,
    pub scroll_y: Property<f64>,
    pub fade: Property<f64>,
    pub outside_depth: Property<f64>,
}
impl Example {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.native.set(true);
        self.solids.set(true);
        self.note.set("This is a live, editable surface.".into());
        self.depth.set(-50.);
        self.tilt.set(-14.);
        self.fade.set(1.);
        self.outside_depth.set(-25.);
        self.last_frame.set(ctx.elapsed_millis.get());
    }
    pub fn tick(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_millis.get();
        let dt = now.saturating_sub(self.last_frame.get()).min(100);
        self.last_frame.set(now);
        if self.fading.get() {
            let t = self.fade_clock.get() + dt as f64;
            self.fade_clock.set(t);
            self.fade.set(0.65 + 0.3 * (t / 900.).cos());
        }
        if self.playing.get() {
            let t = self.clock.get() + dt as f64;
            self.clock.set(t);
            self.depth.set((t / 1800.).sin() * 230.);
        }
    }
    pub fn switch_study(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.playing.set(false);
        self.fading.set(false);
        self.study.set(!self.study.get());
    }
    pub fn toggle_native(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.native.set(!self.native.get());
    }
    pub fn toggle_solids(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.solids.set(!self.solids.get());
    }
    pub fn toggle_fading(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.fading.set(!self.fading.get());
    }
    pub fn play(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.playing.set(!self.playing.get());
    }
    pub fn fade_assembly(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.fading.set(false);
        let to = if self.fade.get() < 0.9 { 1.0 } else { 0.45 };
        self.fade.local().ease_to(
            to,
            Duration::Milliseconds(900.into()),
            EasingCurve::InOutQuad,
        );
    }
    pub fn front(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.playing.set(false);
        self.depth.set(230.);
    }
    pub fn through(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.playing.set(false);
        self.depth.set(-50.);
    }
    pub fn behind(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.playing.set(false);
        self.depth.set(-230.);
    }
    pub fn press(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.presses.set(self.presses.get() + 1);
    }
    pub fn solid_press(&mut self, _: &NodeContext, _: Event<Click>) {
        self.solid_presses.set(self.solid_presses.get() + 1);
    }
}
