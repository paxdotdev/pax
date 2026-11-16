use pax_kit::*;
extern crate self as rain_pane;
pub mod wet_pane;
pub use wet_pane::WetPane;

pax_kit::runnable! { root: crate::Example }

#[pax]
#[custom(Default)]
#[file("lib.pax")]
pub struct Example {
    pub water_time: Property<f64>,
    pub light_phase: Property<f64>,
    pub light_x: Property<f64>,
    pub cyan_x: Property<f64>,
    pub cyan_y: Property<f64>,
    pub magenta_x: Property<f64>,
    pub magenta_y: Property<f64>,
    pub yellow_x: Property<f64>,
    pub yellow_y: Property<f64>,
    pub wetness: Property<f64>,
    pub drop_scale: Property<f64>,
    pub speed: Property<f64>,
    pub rain_frequency: Property<f64>,
    pub roughness: Property<f64>,
    pub strength: Property<f64>,
    pub seed: Property<u32>,
    pub playing: Property<bool>,
    pub mode: Property<usize>,
    pub motion: Property<usize>,
    pub light_kind: Property<usize>,
    pub half_resolution: Property<bool>,
    pub quiet: Property<bool>,
    pub visible: Property<bool>,
    pub last_time: Property<f64>,
    pub pending_time: Property<f64>,
    pub panel_width: Property<f64>,
    pub panel_height: Property<f64>,
    pub small: Property<bool>,
    pub clicks: Property<usize>,
}

impl Default for Example {
    fn default() -> Self {
        let water = WetPane::default();
        Self {
            water_time: water.time,
            light_phase: Property::new(0.75),
            light_x: Property::new(650.),
            cyan_x: Property::new(0.),
            cyan_y: Property::new(0.),
            magenta_x: Property::new(0.),
            magenta_y: Property::new(0.),
            yellow_x: Property::new(0.),
            yellow_y: Property::new(0.),
            wetness: water.wetness,
            drop_scale: water.drop_scale,
            speed: water.speed,
            rain_frequency: water.rain_frequency,
            roughness: water.roughness,
            strength: water.strength,
            seed: water.seed,
            playing: Property::new(true),
            mode: Property::new(0),
            motion: Property::new(0),
            light_kind: Property::new(3),
            half_resolution: water.half_resolution,
            quiet: water.quiet,
            visible: Property::new(true),
            last_time: Property::new(0.),
            pending_time: Property::new(0.),
            panel_width: Property::new(960.),
            panel_height: Property::new(480.),
            small: Property::new(false),
            clicks: Property::new(0),
        }
    }
}

impl Example {
    pub fn layout(&mut self, ctx: &NodeContext) {
        let width = ctx.viewport.get().width;
        self.small.set_if_neq(width < 700.);
        self.panel_width
            .set_if_neq((width - if width < 700. { 24. } else { 64. }).clamp(280., 1440.));
        self.panel_height.set_if_neq(if width < 700. {
            600.
        } else {
            (self.panel_width.get() * 0.5).clamp(480., 640.)
        });
    }
    pub fn reset(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.water_time.set(0.);
    }
    pub fn animate(&mut self, ctx: &NodeContext) {
        self.layout(ctx);
        let now = ctx.elapsed_time_millis() as f64;
        let elapsed = now - self.last_time.get();
        self.last_time.set(now);
        if !(0.0..100.0).contains(&elapsed)
            || !self.playing.get()
            || !self.visible.get()
            || self.mode.get() != 0
        {
            self.pending_time.set(0.);
            return;
        }
        let dt = self.pending_time.get() + elapsed / 1000.;
        let interval = if self.small.get() { 1. / 30. } else { 1. / 60. };
        if dt < interval {
            self.pending_time.set(dt);
            return;
        }
        self.pending_time.set(0.);
        if self.motion.get() != 2 {
            self.water_time.set(self.water_time.get() + dt);
        }
        if self.motion.get() != 1 {
            self.light_phase
                .set((self.light_phase.get() + dt / 36.).rem_euclid(1.));
        }
    }
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.layout(ctx);
        let phase = self.light_phase.local();
        let width = self.panel_width.local();
        let deps = [phase.untyped(), width.untyped()];
        self.light_x.replace_with(LocalProperty::computed(
            move || width.get() * (0.5 + 0.85 * (phase.get() * std::f64::consts::TAU).cos()),
            &deps,
        ));
        // Positions stay in the DAG, so resize and scrub update the same paths
        // without per-light tick work or coupling lighting to the water shader.
        for (target, vertical, offset, center, amplitude) in [
            (&self.cyan_x, false, 0., 0.24, 0.18),
            (&self.cyan_y, true, 0., 0.50, 0.28),
            (&self.magenta_x, false, 1. / 3., 0.50, 0.26),
            (&self.magenta_y, true, 1. / 3., 0.50, 0.28),
            (&self.yellow_x, false, 2. / 3., 0.76, 0.18),
            (&self.yellow_y, true, 2. / 3., 0.50, 0.28),
        ] {
            let phase = self.light_phase.local();
            let extent = if vertical {
                self.panel_height.local()
            } else {
                self.panel_width.local()
            };
            let deps = [phase.untyped(), extent.untyped()];
            target.replace_with(LocalProperty::computed(
                move || {
                    let angle = phase.get() * std::f64::consts::TAU;
                    let offset = offset * std::f64::consts::TAU;
                    let drift = if vertical {
                        (angle * 2. + offset).sin()
                    } else {
                        (angle + offset).cos()
                    };
                    extent.get() * (center + amplitude * drift)
                },
                &deps,
            ));
        }
    }
    pub fn visibility(&mut self, ctx: &NodeContext, event: Event<ViewportProximityChange>) {
        self.visible.set(event.current.is_in_viewport());
        self.last_time.set(ctx.elapsed_time_millis() as f64);
        self.pending_time.set(0.);
    }
    pub fn pause(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.playing.set(!self.playing.get());
    }
    pub fn comparison(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.mode.set((self.mode.get() + 1) % 3);
    }
    pub fn motion_mode(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.motion.set((self.motion.get() + 1) % 3);
    }
    pub fn light_mode(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.light_kind.set((self.light_kind.get() + 1) % 4);
    }
    pub fn quality(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.half_resolution.set(!self.half_resolution.get());
    }
    pub fn copy_space(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.quiet.set(!self.quiet.get());
    }
    pub fn click(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.clicks.set(self.clicks.get() + 1);
    }
    pub fn scrub(&mut self, _: &NodeContext, _: Event<SliderChange>) {
        self.playing.set(false);
    }
}
