use pax_kit::math::Point2;
use pax_kit::*;
use rand::Rng;

extern crate self as glass_bricks;
pub mod glass;
pub use glass::GlassBricks;

pax_kit::runnable! { root: crate::Example }

/// The center is in wall widths, independent of the light's editable radius.
#[pax]
pub struct RoamingLight {
    pub x_fraction: f64,
    pub y_fraction: f64,
    pub speed: f64,
}

#[pax]
pub struct LampSettings {
    pub id: usize,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub intensity: f64,
    pub radius_percent: f64,
}

impl RoamingLight {
    fn advance(&mut self, mut dt: f64, width: f64, radius_fraction: f64, rng: &mut impl Rng) {
        loop {
            let remaining = (self.x_fraction + radius_fraction).max(0.0) * width / self.speed;
            if dt < remaining {
                self.x_fraction -= self.speed * dt / width;
                return;
            }
            // Both ends are a full attenuation radius beyond the wall. Carry the
            // leftover frame time into the newly sampled pass, even on a tiny wall.
            dt -= remaining;
            self.x_fraction = 1.0 + radius_fraction;
            self.speed = rng.gen_range(28.0..67.0);
            self.y_fraction = rng.gen_range(0.08..0.92);
        }
    }
}

#[pax]
#[file("lib.pax")]
pub struct Example {
    pub lights: Property<Vec<RoamingLight>>,
    pub lamp_settings: Property<Vec<LampSettings>>,
    pub selected_lamp: Property<u32>,
    pub panel_size: Property<f64>,
    pub controls_space: Property<f64>,
    pub playing: Property<bool>,
    pub visible: Property<bool>,
    pub previous_ms: Property<f64>,
    pub pointer_x: Property<f64>,
    pub pointer_y: Property<f64>,
    pub depth: Property<f64>,
    pub brick_size_percent: Property<f64>,
    pub rear_speed: Property<f64>,
    pub thickness: Property<f64>,
    pub roughness: Property<f64>,
    pub inner_bezel: Property<f64>,
    pub outer_bezel: Property<f64>,
    pub light_mode: Property<bool>,
    pub strip: Property<bool>,
    pub rear: Property<bool>,
    pub front: Property<bool>,
    pub mode: Property<usize>,
}

impl Example {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let viewport = ctx.viewport.clone();
        let viewport_width = viewport.clone();
        self.controls_space.replace_with(LocalProperty::computed(
            move || {
                if viewport_width.get().width >= 860. {
                    340.
                } else {
                    0.
                }
            },
            &[viewport.untyped()],
        ));
        let controls_space = self.controls_space.local();
        let deps = [viewport.untyped(), controls_space.untyped()];
        self.panel_size.replace_with(LocalProperty::computed(
            move || {
                let v = viewport.get();
                (v.width - 32. - controls_space.get())
                    .min((v.height - 148.).max(380.))
                    .clamp(1., 784.)
            },
            &deps,
        ));
        self.lamp_settings.set(
            [(0, 225, 255), (255, 7, 116), (255, 162, 6)]
                .into_iter()
                .enumerate()
                .map(|(id, (red, green, blue))| LampSettings {
                    id,
                    red,
                    green,
                    blue,
                    intensity: 4.,
                    radius_percent: 57.,
                })
                .collect(),
        );
        let mut rng = rand::thread_rng();
        self.lights.set(
            [(0.19, 0.28), (0.81, 0.38), (0.46, 0.79)]
                .into_iter()
                .map(|(x, y)| RoamingLight {
                    x_fraction: x,
                    y_fraction: y,
                    speed: rng.gen_range(28.0..67.0),
                })
                .collect(),
        );
        self.pointer_x.set(0.65);
        self.pointer_y.set(0.25);
        self.depth.set(95.);
        self.brick_size_percent.set(100.);
        self.rear_speed.set(1.);
        let glass = GlassBricks::default();
        self.thickness.set(glass.thickness.get());
        self.roughness.set(glass.roughness.get());
        self.inner_bezel.set(glass.inner_bezel.get());
        self.outer_bezel.set(glass.outer_bezel.get());
        self.front.set(true);
        self.rear.set(true);
        self.playing.set(true);
        self.visible.set(true);
    }

    pub fn animate(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        let previous = self.previous_ms.get();
        self.previous_ms.set(now);
        if previous == 0. {
            return;
        }
        let dt = ((now - previous) * 0.001).clamp(0.0, 0.1) * self.rear_speed.get();
        let width = self.panel_size.get();
        if width <= 0. {
            return;
        }
        let mut lights = self.lights.get();
        let mut rng = rand::thread_rng();
        self.lamp_settings.read(|settings| {
            for (light, lamp) in lights.iter_mut().zip(settings) {
                light.advance(dt, width, lamp.radius_percent / 100., &mut rng);
            }
        });
        self.lights.set(lights);
    }

    pub fn start_clock(&mut self, ctx: &NodeContext) {
        self.previous_ms.set(ctx.elapsed_time_millis() as f64);
    }
    pub fn visibility(&mut self, _: &NodeContext, event: Event<ViewportProximityChange>) {
        self.visible.set_if_neq(event.current.is_in_viewport());
    }
    pub fn pause(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.playing.set(!self.playing.get());
    }
    pub fn toggle_rear(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.rear.set(!self.rear.get());
    }
    pub fn toggle_front(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.front.set(!self.front.get());
    }
    pub fn toggle_theme(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.light_mode.set(!self.light_mode.get());
    }

    fn edit_lamp(&self, edit: impl FnOnce(&mut LampSettings)) {
        let index = self.selected_lamp.get() as usize;
        self.lamp_settings.update(|lamps| edit(&mut lamps[index]));
    }
    pub fn set_red(&mut self, _: &NodeContext, event: Event<SliderChange>) {
        self.edit_lamp(|lamp| lamp.red = event.value as u8);
    }
    pub fn set_green(&mut self, _: &NodeContext, event: Event<SliderChange>) {
        self.edit_lamp(|lamp| lamp.green = event.value as u8);
    }
    pub fn set_blue(&mut self, _: &NodeContext, event: Event<SliderChange>) {
        self.edit_lamp(|lamp| lamp.blue = event.value as u8);
    }
    pub fn set_intensity(&mut self, _: &NodeContext, event: Event<SliderChange>) {
        self.edit_lamp(|lamp| lamp.intensity = event.value);
    }
    pub fn set_radius(&mut self, _: &NodeContext, event: Event<SliderChange>) {
        self.edit_lamp(|lamp| lamp.radius_percent = event.value);
    }
    pub fn toggle_strip(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.strip.set(!self.strip.get());
    }
    pub fn compare(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.mode.set((self.mode.get() + 1) % 3);
    }

    fn place_light(&mut self, ctx: &NodeContext, x: f64, y: f64) {
        let point = ctx.local_point(Point2::new(x, y));
        self.pointer_x.set_if_neq(point.x);
        self.pointer_y.set_if_neq(point.y);
    }
    pub fn pointer(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
        self.place_light(ctx, event.mouse.x, event.mouse.y);
    }
    pub fn touch_start(&mut self, ctx: &NodeContext, event: Event<TouchStart>) {
        if let Some(touch) = event.touches.first() {
            self.place_light(ctx, touch.x, touch.y);
        }
    }
    pub fn touch_move(&mut self, ctx: &NodeContext, event: Event<TouchMove>) {
        if let Some(touch) = event.touches.first() {
            self.place_light(ctx, touch.x, touch.y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn next_pass_starts_only_after_the_entire_old_glow_clears() {
        let mut rng = StdRng::seed_from_u64(7);
        let (width, radius) = (600., 300.);
        let mut lamp = RoamingLight {
            x_fraction: (-radius + 12.) / width,
            y_fraction: 0.5,
            speed: 60.,
            ..Default::default()
        };
        lamp.advance(0.1, width, radius / width, &mut rng);
        assert!(lamp.x_fraction < 0. && lamp.x_fraction * width + radius > 0.);
        assert_eq!((lamp.y_fraction, lamp.speed), (0.5, 60.));
        lamp.advance(0.101, width, radius / width, &mut rng);
        assert!((lamp.x_fraction * width - (width + radius)).abs() < 0.1);
        assert_ne!(lamp.y_fraction, 0.5);
        assert_ne!(lamp.speed, 60.);
        let start = lamp.x_fraction;
        lamp.advance(0.1, width, radius / width, &mut rng);
        assert!(lamp.x_fraction < start);
    }

    #[test]
    fn radius_changes_keep_the_center_and_use_the_new_wrap_boundary() {
        let mut rng = StdRng::seed_from_u64(11);
        let mut lamp = RoamingLight {
            x_fraction: -0.48,
            y_fraction: 0.5,
            speed: 60.,
        };
        lamp.advance(0.1, 600., 0.8, &mut rng);
        assert!((lamp.x_fraction + 0.49).abs() < 0.00001);
        assert_eq!(lamp.speed, 60.);
        // Shrinking below the current distance puts the entire glow offscreen.
        // Only the new pass consumes this frame's time.
        lamp.advance(0.1, 600., 0.2, &mut rng);
        assert!((lamp.x_fraction - (1.2 - 0.1 * lamp.speed / 600.)).abs() < 0.00001);
    }
}
