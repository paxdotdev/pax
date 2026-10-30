use pax_kit::*;

pax_kit::runnable! { root: crate::Example }

#[pax]
#[file("lib.pax")]
pub struct Example {
    pub clock: Property<f64>,
    pub last_frame: Property<u64>,
    pub playing: Property<bool>,
    pub scroll_y: Property<f64>,
    pub note: Property<String>,
    pub presses: Property<u64>,
    pub fade: Property<f64>,
    pub power: Property<f64>,
}

impl Example {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.playing.set(true);
        self.note.set("A little light changes everything.".into());
        self.fade.set(1.0);
        self.power.set(1.8);
        self.last_frame.set(ctx.elapsed_millis.get());
    }
    pub fn tick(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_millis.get();
        let elapsed = now.saturating_sub(self.last_frame.get());
        self.last_frame.set(now);
        if self.playing.get() {
            self.clock.set(self.clock.get() + elapsed.min(100) as f64);
        }
    }
    pub fn play(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.playing.set(!self.playing.get());
    }
    pub fn press(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.presses.set(self.presses.get() + 1);
    }
    pub fn fade(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        let to = if self.fade.get() < 0.9 { 1.0 } else { 0.55 };
        self.fade.local().ease_to(
            to,
            Duration::Milliseconds(600.into()),
            EasingCurve::InOutQuad,
        );
    }
}
