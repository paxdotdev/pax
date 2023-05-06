use pax::api::{ArgsClick, EasingCurve, NodeContext, Property, PropertyLiteral};
use pax::Pax;
use pax_std::primitives::{Rectangle, Group, Frame, Text, Ellipse};

#[derive(Pax)]
#[file("camera.pax")]
pub struct Camera {
    pub ticks: Property<usize>,
    pub zoom: Property<f64>,
    pub pan_x: Property<f64>,
    pub pan_y: Property<f64>,
}


const LOOP_DURATION_FRAMES : usize = 600;

impl Camera {
    pub fn handle_did_mount(&mut self, ctx: NodeContext) {
        self.zoom.set(1.0);
        self.pan_x.set(0.0);
        self.pan_y.set(0.0);
    }

    pub fn handle_click(&mut self, ctx: NodeContext, args: ArgsClick) {
        // let delta_pan = (args.x, args.y) - (1.0, 1.0);
        let delta_pan = (args.x - self.pan_x.get(), args.y - self.pan_y.get());
        self.pan_x.ease_to(self.pan_x.get() + delta_pan.0, 200, EasingCurve::InQuad);
        self.pan_y.ease_to(self.pan_y.get() + delta_pan.1, 200, EasingCurve::InQuad);

        self.zoom.ease_to(0.5, 100, EasingCurve::OutQuad);
        self.zoom.ease_to_later(1.0, 100, EasingCurve::InQuad)
    }

    pub fn handle_will_render(&mut self, ctx: NodeContext) {
        // self.ticks.set((self.ticks.get() + 1) % LOOP_DURATION_FRAMES);
        // let new_tick = *self.ticks.get();
        //
        // if new_tick == 1 {
        //     self.zoom.ease_to(0.5, 300, EasingCurve::InQuad);
        // } else if new_tick == 301 {
        //     self.zoom.ease_to(1.0, 300, EasingCurve::OutQuad);
        // }

    }
}
