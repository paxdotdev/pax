use crate::PoolFloor;
use pax_kit::*;

#[pax]
#[custom(Default)]
#[file("lib.pax")]
pub struct Example {
    pub density: Property<f64>,
    pub amplitude: Property<f64>,
    pub speed: Property<f64>,
    pub floor_color: Property<Color>,
    pub hex: Property<String>,
    pub valid_color: Property<bool>,
    pub playing: Property<bool>,
    pub full_resolution: Property<bool>,
    pub mode: Property<usize>,
    pub point_light: Property<bool>,
    pub lamp_x: Property<f64>,
    pub lamp_y: Property<f64>,
}

impl Default for Example {
    fn default() -> Self {
        let water = PoolFloor::default();
        Self {
            density: water.density,
            amplitude: water.amplitude,
            speed: water.speed,
            floor_color: water.floor_color,
            hex: Property::default(),
            valid_color: Property::new(true),
            playing: Property::new(true),
            full_resolution: water.full_resolution,
            mode: Property::new(0),
            point_light: Property::new(true),
            lamp_x: Property::new(68.),
            lamp_y: Property::new(35.),
        }
    }
}

impl Example {
    pub fn mount(&mut self, _: &NodeContext) {
        let color = self.floor_color.local();
        let deps = [color.untyped()];
        self.hex.replace_with(LocalProperty::computed(
            move || {
                let [r, g, b, a] = color.get().to_rgba_0_1().map(|v| (v * 255.0).round() as u8);
                if a == 255 {
                    format!("#{r:02X}{g:02X}{b:02X}")
                } else {
                    format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
                }
            },
            &deps,
        ));

        let hex = self.hex.local();
        let deps = [hex.untyped()];
        self.valid_color.replace_with(LocalProperty::computed(
            move || hex_digits(&hex.get()).is_some(),
            &deps,
        ));
    }
    pub fn pause(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.playing.set(!self.playing.get());
    }
    pub fn light(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.point_light.set(!self.point_light.get());
    }
    fn position_light(&mut self, ctx: &NodeContext, x: f64, y: f64) {
        if self.point_light.get() {
            let p = ctx.local_point(pax_kit::math::Point2::new(x, y));
            self.lamp_x.set_if_neq(100. * p.x.clamp(0., 1.));
            self.lamp_y.set_if_neq(100. * p.y.clamp(0., 1.));
        }
    }
    pub fn pointer(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
        self.position_light(ctx, event.mouse.x, event.mouse.y);
    }
    pub fn touch(&mut self, ctx: &NodeContext, event: Event<TouchStart>) {
        if let Some(p) = event.touches.first() {
            self.position_light(ctx, p.x, p.y);
        }
    }
    pub fn color(&mut self, _: &NodeContext, event: Event<TextboxInput>) {
        if let Some(raw) = hex_digits(&event.text) {
            self.floor_color.set_if_neq(Color::from_hex(raw));
        }
    }
    pub fn quality(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.full_resolution.set(!self.full_resolution.get());
    }
    pub fn compare(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.mode.set((self.mode.get() + 1) % 3);
    }
}

fn hex_digits(text: &str) -> Option<&str> {
    let text = text.trim();
    let digits = text.strip_prefix('#').unwrap_or(text);
    (matches!(digits.len(), 6 | 8) && digits.bytes().all(|c| c.is_ascii_hexdigit()))
        .then_some(digits)
}
