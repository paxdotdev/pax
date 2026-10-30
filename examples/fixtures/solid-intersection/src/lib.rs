use pax_kit::*;
pax_kit::runnable! { root: crate::Example }

#[pax]
#[file("lib.pax")]
pub struct Example {
    pub clock: Property<f64>,
    pub last_frame: Property<u64>,
    pub playing: Property<bool>,
    pub depth: Property<f64>,
    pub tilt: Property<f64>,
    pub note: Property<String>,
    pub presses: Property<u64>,
    pub solid_presses: Property<u64>,
    pub scroll_y: Property<f64>,
}
impl Example {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.note.set("This is a live, editable surface.".into());
        self.depth.set(-50.);
        self.tilt.set(-14.);
        self.last_frame.set(ctx.elapsed_millis.get());
    }
    pub fn tick(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_millis.get();
        let dt = now.saturating_sub(self.last_frame.get()).min(100);
        self.last_frame.set(now);
        if self.playing.get() {
            let t = self.clock.get() + dt as f64;
            self.clock.set(t);
            self.depth.set((t / 1800.).sin() * 230.);
        }
    }
    pub fn play(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.playing.set(!self.playing.get());
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
