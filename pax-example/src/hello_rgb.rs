use pax::api::{ArgsClick, ArgsRender, ArgsScroll, EasingCurve};
use pax::*;
use pax_std::components::Stacker;
use pax_std::primitives::{Ellipse, Frame, Group, Path, Rectangle, Text};


#[derive(Pax)]
#[file("hello_rgb.pax")]
pub struct HelloRGB {
    pub rotation: Property<f64>,
}

impl HelloRGB {
    pub fn handle_click(&mut self, args: ArgsClick) {
        log("click-ellipse");
    }
    pub fn handle_scroll(&mut self, args: ArgsScroll) {
        const ROTATION_COEFFICIENT: f64 = 0.005;
        let old_t = self.rotation.get();
        let new_t = old_t + args.delta_y * ROTATION_COEFFICIENT;
        self.rotation.set(new_t);
    }
    pub fn handle_global_click(&mut self, args: ArgsClick) {
        log("click-anywhere");
    }
    pub fn handle_global_scroll(&mut self, args: ArgsScroll) {
        log("scroll-anywhere");
    }
}

#[derive(Pax)]
pub struct RectDef {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

