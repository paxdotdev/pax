use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub phase: Property<f64>,
    pub last_time: Property<f64>,
    pub playing: Property<bool>,
    pub moving: Property<bool>,
    pub source_phase: Property<f64>,
    pub radius: Property<f64>,
    pub reference: Property<bool>,
    pub panel_width: Property<f64>,
}
impl Example {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.playing.set(true);
        self.radius.set(8.);
        self.last_time.set(ctx.elapsed_time_millis() as f64);
    }
    pub fn tick(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        let dt = (now - self.last_time.get()).clamp(0., 50.) / 1000.;
        self.last_time.set(now);
        if self.playing.get() {
            self.phase.set(self.phase.get() + dt);
        }
        if self.moving.get() {
            self.source_phase.set(self.source_phase.get() + dt);
        }
        self.panel_width
            .set_if_neq((ctx.viewport.get().width - 48.).clamp(240., 1040.));
    }
    pub fn toggle_play(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.playing.set(!self.playing.get());
    }
    pub fn toggle_source(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.moving.set(!self.moving.get());
    }
    pub fn toggle_reference(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.reference.set(!self.reference.get());
    }
    pub fn blur_more(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.radius.set((self.radius.get() + 8.).rem_euclid(56.));
    }
}

#[pax]
#[file("landscape.pax")]
#[has_helpers]
pub struct Landscape {
    pub phase: Property<f64>,
}

#[helpers]
impl Landscape {
    pub fn sun_x(phase: f64) -> f64 {
        50.0 + 14.0 * phase.sin()
    }
}
