use pax::api::{ArgsClick, EasingCurve, RuntimeContext, Property, PropertyLiteral};
use pax::Pax;
use pax_std::primitives::{Rectangle, Group, Frame, Text, Ellipse};

#[derive(Pax)]
#[file("camera.pax")]
pub struct Camera {
    pub ticks: Property<usize>,
    pub zoom: Property<f64>,
    pub pan_x: Property<f64>,
    pub pan_y: Property<f64>,
    pub type_example: Property<TypeExample>,
}

#[derive(Pax)]
#[custom(Imports)]
pub struct TypeExample {
    pub foo: Property<usize>,
}

const LOOP_DURATION_FRAMES : usize = 600;

impl Camera {
    pub fn handle_did_mount(&mut self, ctx: RuntimeContext) {
        self.zoom.set(2.0);
        self.pan_x.set(0.0);
        self.pan_y.set(0.0);
    }

    pub fn handle_click(&mut self, ctx: RuntimeContext, args: ArgsClick) {
        // let delta_pan = (args.x, args.y) - (1.0, 1.0);
        let delta_pan = (args.x - self.pan_x.get(), args.y - self.pan_y.get());
        self.pan_x.ease_to(self.pan_x.get() + delta_pan.0, 200, EasingCurve::Linear);
        self.pan_y.ease_to(self.pan_y.get() + delta_pan.1, 200, EasingCurve::Linear);

        self.zoom.ease_to(0.5, 100, EasingCurve::OutQuad);
        self.zoom.ease_to_later(2.0, 100, EasingCurve::InQuad)
    }

}
