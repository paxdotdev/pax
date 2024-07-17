#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
use rand::Rng;

#[pax]
#[main]
#[file("lib.pax")]
pub struct BouncingRectangles {
    pub rectangles: Property<Vec<RectangleData>>,
    pub scale: Property<f64>,
}

#[pax]
pub struct RectangleData {
    pub x: Property<f64>,
    pub y: Property<f64>,
    pub dx: Property<f64>,
    pub dy: Property<f64>,
    pub color: Property<Color>,
}

impl BouncingRectangles {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        let mut rng = rand::thread_rng();
        let (width, height) = ctx.bounds_self.get();
        let mut rectangles = Vec::new();

        for _ in 0..20 {
            rectangles.push(RectangleData {
                x: Property::new(rng.gen_range(0.0..width)),
                y: Property::new(rng.gen_range(0.0..height)),
                dx: Property::new(rng.gen_range(-2.0..2.0)),
                dy: Property::new(rng.gen_range(-2.0..2.0)),
                color: Property::new(Color::rgb(
                    ColorChannel::Integer(rng.gen_range(0..=255).into()),
                    ColorChannel::Integer(rng.gen_range(0..=255).into()),
                    ColorChannel::Integer(rng.gen_range(0..=255).into()),
                )),
            });
        }

        self.rectangles.set(rectangles);
        self.scale.set(1.0);
    }

    pub fn handle_tick(&mut self, ctx: &NodeContext) {
        let (width, height) = ctx.bounds_self.get();
        let rect_size = 50.0 * self.scale.get();

        self.rectangles.update(|rectangles| {
            for rect in rectangles.iter_mut() {
                let mut x = rect.x.get();
                let mut y = rect.y.get();
                let mut dx = rect.dx.get();
                let mut dy = rect.dy.get();

                x += dx;
                y += dy;

                if x <= 0.0 || x + rect_size >= width {
                    dx = -dx;
                    x = x.clamp(0.0, width - rect_size);
                }
                if y <= 0.0 || y + rect_size >= height {
                    dy = -dy;
                    y = y.clamp(0.0, height - rect_size);
                }

                rect.x.set(x);
                rect.y.set(y);
                rect.dx.set(dx);
                rect.dy.set(dy);
            }
        });
    }

    pub fn handle_wheel(&mut self, _ctx: &NodeContext, args: Event<Wheel>) {
        let current_scale = self.scale.get();
        let new_scale = (current_scale - args.delta_y * 0.001).clamp(0.5, 2.0);
        self.scale.set(new_scale);
    }
}