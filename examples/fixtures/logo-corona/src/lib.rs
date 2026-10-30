use pax_kit::*;
extern crate self as logo_corona;

pax_kit::runnable! { root: crate::Example }

#[pax]
#[custom(Default)]
#[file("lib.pax")]
pub struct Example {
    pub dark: Property<bool>,
    pub compare: Property<bool>,
    pub treated: Property<bool>,
    pub playing: Property<bool>,
    pub tinted: Property<bool>,
    pub visible: Property<bool>,
    pub phase: Property<f64>,
    pub progress: Property<f64>,
    pub emission: Property<f64>,
    pub reach: Property<f64>,
    pub amplitude: Property<f64>,
    pub falloff: Property<f64>,
    pub surface: Property<f64>,
    pub speed: Property<f64>,
    pub last_time: Property<f64>,
    pub logo_width: Property<f64>,
    pub panel_width: Property<f64>,
    pub stage_height: Property<f64>,
    pub small: Property<bool>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            dark: Property::new(true),
            compare: Property::new(true),
            treated: Property::new(true),
            playing: Property::new(true),
            tinted: Property::new(false),
            visible: Property::new(true),
            phase: Property::new(0.),
            progress: Property::new(1.),
            emission: Property::new(1.),
            reach: Property::new(20.),
            amplitude: Property::new(0.8),
            falloff: Property::new(2.7),
            surface: Property::new(0.04),
            speed: Property::new(2.),
            last_time: Property::new(0.),
            logo_width: Property::new(360.),
            panel_width: Property::new(900.),
            stage_height: Property::new(380.),
            small: Property::new(false),
        }
    }
}

impl Example {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.last_time.set(ctx.elapsed_time_millis() as f64);
        self.layout(ctx);
    }
    fn layout(&self, ctx: &NodeContext) {
        let width = ctx.viewport.get().width;
        let small = width < 760.;
        self.small.set_if_neq(small);
        self.panel_width
            .set_if_neq((width - 40.).clamp(260., 1000.));
        let available = if self.compare.get() && !small {
            (width - 100.) * 0.5
        } else {
            width - 64.
        };
        self.logo_width
            .set_if_neq((available * 0.78).clamp(80., 720.));
        self.stage_height
            .set_if_neq((self.logo_width.get() * 0.504 + 140.).max(280.));
    }
    pub fn tick(&mut self, ctx: &NodeContext) {
        self.layout(ctx);
    }
    pub fn animate(&mut self, ctx: &NodeContext) {
        if !self.playing.get()
            || !self.visible.get()
            || !self.treated.get()
            || self.speed.get() == 0.
        {
            return;
        }
        let now = ctx.elapsed_time_millis() as f64;
        let dt = ((now - self.last_time.get()) / 1000.).clamp(0., 0.05);
        self.last_time.set(now);
        self.phase
            .set((self.phase.get() + dt * self.speed.get() * 0.04).rem_euclid(1.));
    }
    pub fn visibility(&mut self, ctx: &NodeContext, event: Event<ViewportProximityChange>) {
        self.visible.set(event.current.is_in_viewport());
        self.last_time.set(ctx.elapsed_time_millis() as f64);
    }
    pub fn theme(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.dark.set(!self.dark.get());
    }
    pub fn tint(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.tinted.set(!self.tinted.get());
    }
    pub fn comparison(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.compare.set(!self.compare.get());
    }
    pub fn treatment(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.treated.set(!self.treated.get());
    }
    pub fn pause(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        self.playing.set(!self.playing.get());
        self.last_time.set(ctx.elapsed_time_millis() as f64);
    }
    pub fn replay(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        let p = self.progress.local();
        p.cancel_transitions();
        p.set(0.);
        p.ease_to(1., Duration::Milliseconds(1440.into()), EasingCurve::Linear);
    }
    pub fn scrub(&mut self, _: &NodeContext, event: Event<SliderChange>) {
        let p = self.progress.local();
        p.cancel_transitions();
        p.set(event.value);
    }
    pub fn bold(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.emission.set(0.9);
        self.reach.set(105.);
        self.amplitude.set(0.65);
        self.falloff.set(1.2);
        self.surface.set(0.12);
        self.speed.set(1.);
    }
    pub fn quiet(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.tinted.set(false);
        self.emission.set(1.);
        self.reach.set(20.);
        self.amplitude.set(0.8);
        self.falloff.set(2.7);
        self.surface.set(0.04);
        self.speed.set(2.);
    }
}
