use pax_kit::*;
mod lettering;

const DEFAULT_SPACING: f64 = 8.0;
const DEFAULT_LIGHT_INTENSITY: f64 = 4.0;
const DEFAULT_SHEEN: f64 = 1.0;
const DEFAULT_FLARE: f64 = 4.0;
const DEFAULT_SEED: u32 = 17;

pax_kit::runnable! { root: crate::Example }

#[pax]
#[custom(Default)]
#[file("lib.pax")]
pub struct Example {
    pub phase: Property<f64>,
    pub light_x: Property<f64>,
    pub sheen: Property<f64>,
    pub spacing: Property<f64>,
    pub brightness: Property<f64>,
    pub flare: Property<f64>,
    pub seed: Property<u32>,
    pub playing: Property<bool>,
    pub treated: Property<bool>,
    pub dark: Property<bool>,
    pub outline: Property<bool>,
    pub visible: Property<bool>,
    pub last_time: Property<f64>,
    pub panel_width: Property<f64>,
    pub small: Property<bool>,
    pub letters: Property<Vec<PathElement>>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            phase: Property::new(0.25),
            light_x: Property::new(460.),
            sheen: Property::new(DEFAULT_SHEEN),
            spacing: Property::new(DEFAULT_SPACING),
            brightness: Property::new(DEFAULT_LIGHT_INTENSITY),
            flare: Property::new(DEFAULT_FLARE),
            seed: Property::new(DEFAULT_SEED),
            playing: Property::new(true),
            treated: Property::new(true),
            dark: Property::new(true),
            outline: Property::new(false),
            visible: Property::new(true),
            last_time: Property::new(0.),
            panel_width: Property::new(920.),
            small: Property::new(false),
            letters: Property::new(lettering::lettering()),
        }
    }
}

impl Example {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.layout(ctx);
        let phase = self.phase.local();
        let deps = [phase.untyped()];
        self.light_x.replace_with(LocalProperty::computed(
            move || 460. - 720. * (phase.get() * std::f64::consts::TAU).cos(),
            &deps,
        ));
    }

    pub fn layout(&mut self, ctx: &NodeContext) {
        let width = ctx.viewport.get().width;
        self.panel_width.set_if_neq((width - 40.).clamp(280., 920.));
        self.small.set_if_neq(width < 650.);
    }
    pub fn animate(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        let dt = ((now - self.last_time.get()) / 1000.).clamp(0., 0.05);
        self.last_time.set(now);
        if self.playing.get() && self.treated.get() && self.visible.get() {
            self.phase.set((self.phase.get() + dt / 14.).rem_euclid(1.));
        }
    }
    pub fn visibility(&mut self, ctx: &NodeContext, event: Event<ViewportProximityChange>) {
        self.visible.set(event.current.is_in_viewport());
        self.last_time.set(ctx.elapsed_time_millis() as f64);
    }
    pub fn pause(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        self.playing.set(!self.playing.get());
        self.last_time.set(ctx.elapsed_time_millis() as f64);
    }
    pub fn scrub(&mut self, _: &NodeContext, event: Event<SliderChange>) {
        self.playing.set(false);
        self.phase.set(event.value);
    }
    pub fn treatment(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.treated.set(!self.treated.get());
    }
    pub fn theme(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.dark.set(!self.dark.get());
    }
    pub fn outline(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.outline.set(!self.outline.get());
    }
    pub fn reseed(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.seed.set(self.seed.get().wrapping_add(1));
    }
    pub fn reset(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        self.phase.set(0.25);
        self.sheen.set(DEFAULT_SHEEN);
        self.spacing.set(DEFAULT_SPACING);
        self.brightness.set(DEFAULT_LIGHT_INTENSITY);
        self.flare.set(DEFAULT_FLARE);
        self.seed.set(DEFAULT_SEED);
        self.treated.set(true);
        self.playing.set(true);
        self.last_time.set(ctx.elapsed_time_millis() as f64);
    }
}
