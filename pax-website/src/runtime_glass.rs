use glass_bricks::{GlassBricks, RoamingLight};
use pax_kit::*;

#[pax]
#[file("runtime_glass.pax")]
pub struct RuntimeGlass {
    pub light_mode: Property<bool>,
    pub visible: Property<bool>,
    pub wall_width: Property<f64>,
    pub radius_fraction: Property<f64>,
    pub colors: Property<Vec<Color>>,
    pub lights: Property<Vec<RoamingLight>>,
    #[pax(rust)]
    pub last_ms: f64,
}

impl RuntimeGlass {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let bounds = ctx.bounds_self.clone();
        self.wall_width.replace_with(LocalProperty::computed(
            move || bounds.get().0,
            &[ctx.bounds_self.untyped()],
        ));
        self.radius_fraction.set(0.89);
        self.colors.set(vec![
            Color::rgb(0.into(), 225.into(), 255.into()),
            Color::rgb(255.into(), 7.into(), 116.into()),
            Color::rgb(255.into(), 162.into(), 6.into()),
        ]);
        self.lights
            .set(RoamingLight::initial_trio(&mut rand::thread_rng()));
    }

    pub fn visibility(&mut self, _: &NodeContext, event: Event<ViewportProximityChange>) {
        self.visible.set_if_neq(event.current.is_in_viewport());
    }

    pub fn reset_clock(&mut self, ctx: &NodeContext) {
        self.last_ms = ctx.elapsed_time_millis() as f64;
    }

    pub fn animate(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        let dt = (now - self.last_ms) / 1000.;
        self.last_ms = now;
        let width = self.wall_width.get();
        if width > 0. && (0.0..0.1).contains(&dt) {
            let radius = self.radius_fraction.get();
            let mut rng = rand::thread_rng();
            self.lights.update(|lights| {
                for light in lights {
                    light.advance(dt * 3.5, width, radius, &mut rng);
                }
            });
        }
    }
}
