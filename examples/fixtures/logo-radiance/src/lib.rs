use pax_kit::*;

pax_kit::runnable! { root: crate::Example }

#[pax]
#[custom(Default)]
#[file("lib.pax")]
pub struct Example {
    pub playing: Property<bool>,
    pub visible: Property<bool>,
    pub treated: Property<bool>,
    pub marker: Property<bool>,
    pub phase: Property<f64>,
    pub progress: Property<f64>,
    pub light_x: Property<f64>,
    pub light_y: Property<f64>,
    pub depth: Property<f64>,
    pub brightness: Property<f64>,
    pub reach: Property<f64>,
    pub falloff: Property<f64>,
    pub transmission: Property<f64>,
    pub polish: Property<f64>,
    pub last_time: Property<f64>,
    pub logo_width: Property<f64>,
    pub panel_width: Property<f64>,
    pub stage_height: Property<f64>,
    pub small: Property<bool>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            playing: Property::new(true),
            visible: Property::new(true),
            treated: Property::new(true),
            marker: Property::new(false),
            phase: Property::new(0.),
            progress: Property::new(1.),
            light_x: Property::new(820.),
            light_y: Property::new(185.),
            depth: Property::new(340.),
            brightness: Property::new(5.),
            reach: Property::new(320.),
            falloff: Property::new(1.),
            transmission: Property::new(0.16),
            polish: Property::new(0.55),
            last_time: Property::new(0.),
            logo_width: Property::new(560.),
            panel_width: Property::new(900.),
            stage_height: Property::new(480.),
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
        self.small.set_if_neq(width < 650.);
        self.panel_width.set_if_neq((width - 40.).clamp(260., 940.));
        // Leave room for the projected volume, including on narrow screens.
        self.logo_width
            .set_if_neq(((width - 32.) / 1.8).clamp(100., 560.));
        self.stage_height
            .set_if_neq((self.logo_width.get() * 0.82).max(270.));
    }
    pub fn tick(&mut self, ctx: &NodeContext) {
        self.layout(ctx);
    }
    fn place_light(&self) {
        let angle = self.phase.get() * std::f64::consts::TAU;
        self.light_x.set(490. + 330. * angle.cos());
        self.light_y.set(205. - 20. * angle.cos());
    }
    pub fn animate(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        let dt = ((now - self.last_time.get()) / 1000.).clamp(0., 0.05);
        self.last_time.set(now);
        if !self.playing.get() || !self.visible.get() || !self.treated.get() {
            return;
        }
        self.phase.set((self.phase.get() + dt / 18.).rem_euclid(1.));
        self.place_light();
    }
    pub fn visibility(&mut self, ctx: &NodeContext, event: Event<ViewportProximityChange>) {
        self.visible.set(event.current.is_in_viewport());
        self.last_time.set(ctx.elapsed_time_millis() as f64);
    }
    pub fn pause(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        self.playing.set(!self.playing.get());
        self.last_time.set(ctx.elapsed_time_millis() as f64);
    }
    pub fn treatment(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.treated.set(!self.treated.get());
    }
    pub fn marker(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.marker.set(!self.marker.get());
    }
    pub fn scrub_light(&mut self, _: &NodeContext, event: Event<SliderChange>) {
        self.playing.set(false);
        self.phase.set(event.value);
        self.place_light();
    }
    pub fn move_light(&mut self, _: &NodeContext, _: Event<SliderChange>) {
        self.playing.set(false);
    }
    pub fn replay(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        let p = self.progress.local();
        p.cancel_transitions();
        p.set(0.);
        p.ease_to(1., Duration::Milliseconds(1440.into()), EasingCurve::Linear);
    }
    pub fn scrub_logo(&mut self, _: &NodeContext, event: Event<SliderChange>) {
        let p = self.progress.local();
        p.cancel_transitions();
        p.set(event.value);
    }
    pub fn reset(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        self.phase.set(0.);
        self.place_light();
        self.depth.set(340.);
        self.brightness.set(5.);
        self.reach.set(320.);
        self.falloff.set(1.);
        self.transmission.set(0.16);
        self.polish.set(0.55);
        self.treated.set(true);
        self.playing.set(true);
        self.last_time.set(ctx.elapsed_time_millis() as f64);
    }
}
