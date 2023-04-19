use pax::api::{ArgsClick, ArgsRender, ArgsScroll, EasingCurve};
use pax::*;
use pax_std::components::Stacker;
use pax_std::primitives::{Ellipse, Frame, Group, Path, Rectangle, Text};

#[pax_file("pax-example.pax")]
pub struct PaxExample {
    pub rotation: Property<f64>,
    pub ticks: Property<usize>,
    pub heartbeat: Property<f64>,
    pub squares: Property<Vec<f64>>,
}

const ROTATION_COEFFICIENT: f64 = 0.00010;
const HEARTBEAT_AMPLITUDE: f64 = 1.15;

impl PaxExample {

    pub fn handle_did_mount(&mut self) {
        pax::log("Mounted!");
        self.squares.set(vec![0.5, 1.5, 2.5, 3.5, 4.5]);
    }

    pub fn handle_will_render(&mut self, args: ArgsRender) {
        self.ticks.set(args.frames_elapsed);
        if args.frames_elapsed % 260 == 0 {
            pax::log("heartbeat");
            // self.heartbeat.ease_to(HEARTBEAT_AMPLITUDE, 40, EasingCurve::OutBack);
            // self.heartbeat.ease_to_later(-HEARTBEAT_AMPLITUDE / 2.0, 50, EasingCurve::OutBack);
            // self.heartbeat.ease_to_later(0.0, 70, EasingCurve::OutBack);
        }
    }

    pub fn handle_scroll(&mut self, args: ArgsScroll) {
        let old_t = self.rotation.get();
        let new_t = old_t - args.delta_y * ROTATION_COEFFICIENT;
        self.rotation.set(f64::max(0.0,new_t));
    }
}

#[pax_type]
#[derive(Default)]
pub struct RectDef {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

