use pax_kit::*;

#[pax]
#[file("mask_study.pax")]
pub struct MaskStudy {
    pub angle: Property<f64>,
    pub filtered: Property<bool>,
    pub feather: Property<f64>,
    pub fade: Property<f64>,
    pub mounted: Property<bool>,
    pub playing: Property<bool>,
    pub last_frame: Property<u64>,
}
impl MaskStudy {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.angle.set(28.);
        self.feather.set(6.);
        self.fade.set(0.8);
        self.mounted.set(true);
        self.last_frame.set(ctx.elapsed_millis.get());
    }
    pub fn tick(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_millis.get();
        if self.playing.get() {
            let dt = now.saturating_sub(self.last_frame.get()).min(100) as f64;
            self.angle.set((self.angle.get() + dt * 0.025) % 360.);
        }
        self.last_frame.set(now);
    }
    pub fn play(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.playing.set(!self.playing.get());
    }
    pub fn pause(&mut self, _: &NodeContext, _: Event<SliderChange>) {
        self.playing.set(false);
    }
    pub fn toggle_filter(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.filtered.set(!self.filtered.get());
    }
    pub fn toggle_source(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.mounted.set(!self.mounted.get());
    }
}
