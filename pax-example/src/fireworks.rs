use pax::api::{ArgsClick, ArgsRender, ArgsScroll, EasingCurve};
use pax::*;
use pax_std::primitives::{Ellipse, Frame, Group, Path, Rectangle, Text};

#[pax_file("fireworks.pax")]
pub struct Fireworks {
    pub rotation: Property<f64>,
    pub ticks: Property<usize>,
    pub heartbeat: Property<f64>,
}

const ROTATION_COEFFICIENT: f64 = 0.00010;
const HEARTBEAT_AMPLITUDE: f64 = 1.25;

impl Fireworks {

    pub fn handle_will_render(&mut self, args: ArgsRender) {
        self.ticks.set(args.frames_elapsed);
        if args.frames_elapsed % 360 == 0 {
            pax::log("heartbeat");
            self.heartbeat.ease_to(HEARTBEAT_AMPLITUDE, 120, EasingCurve::OutBack);
            self.heartbeat.ease_to_later(-HEARTBEAT_AMPLITUDE / 2.0, 120, EasingCurve::OutBack);
            self.heartbeat.ease_to_later(1.0, 120, EasingCurve::OutBack);
        }
    }

    pub fn handle_scroll(&mut self, args: ArgsScroll) {
        let old_t = self.rotation.get();
        let new_t = old_t - args.delta_y * ROTATION_COEFFICIENT;
        self.rotation.set(f64::max(0.0,new_t));
    }
}
