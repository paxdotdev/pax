#![allow(unused_imports)]
use pax_engine::api::{ArgsClick, EasingCurve, NodeContext, Property};
use pax_engine::Pax;
use pax_std::primitives::{Ellipse, Frame, Group, Rectangle, Text};

#[pax]
#[main]
#[file("camera.pax")]
pub struct Camera {
    pub ticks: Property<usize>,
    pub zoom: Property<f64>,
    pub pan_x: Property<f64>,
    pub pan_y: Property<f64>,
    pub type_example: Property<TypeExample>,
}

#[pax]
pub struct TypeExample {
    pub foo: Property<usize>,
}

impl Camera {
    pub fn handle_mount(&mut self, _: NodeContext) {
        self.zoom.set(2.0);
        self.pan_x.set(0.0);
        self.pan_y.set(0.0);
    }

    pub fn handle_click(&mut self, _: NodeContext, args: ArgsClick) {
        let delta_pan = (
            args.mouse.x - self.pan_x.get(),
            args.mouse.y - self.pan_y.get(),
        );
        self.pan_x
            .ease_to(self.pan_x.get() + delta_pan.0, 200, EasingCurve::Linear);
        self.pan_y
            .ease_to(self.pan_y.get() + delta_pan.1, 200, EasingCurve::Linear);

        self.zoom.ease_to(0.5, 100, EasingCurve::OutQuad);
        self.zoom.ease_to_later(2.0, 100, EasingCurve::InQuad)
    }
}
