use pax_lang::api::*;
use pax_lang::*;
use pax_std::primitives::{Ellipse, Frame, Group, Path, Rectangle, Text};

#[derive(Pax)]
#[main]
#[file("hello-rgb.pax")]
pub struct HelloRGB {
    pub rotation: Property<f64>,
}

const ROTATION_COEFFICIENT: f64 = 0.005;
impl HelloRGB {
    pub fn handle_scroll(&mut self, ctx: &NodeContext, args: ArgsWheel) {
        let old_t = self.rotation.get();
        let new_t = old_t + args.delta_y * ROTATION_COEFFICIENT;
        self.rotation.set(new_t);
    }
}

