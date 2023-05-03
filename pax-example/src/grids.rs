use pax::api::{ArgsClick, ArgsRender, ArgsScroll, EasingCurve};
use pax::*;
use pax_std::primitives::{Ellipse, Frame, Group, Path, Rectangle, Text};

#[derive(Pax)]
#[file("grids.pax")]
pub struct Grids {
    pub ticks: Property<usize>,
    pub rects: Property<Vec<RectDef>>,
}


impl Grids {

    pub fn handle_did_mount(&mut self) {
        pax::log("Grids mounted!");
        self.rects.set(vec![]);
    }

    pub fn handle_will_render(&mut self, args: ArgsRender) {
        self.ticks.set(args.frames_elapsed);
    }

    pub fn handle_scroll(&mut self, args: ArgsScroll) {

    }

}

#[derive(Pax)]
pub struct RectDef {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl pax::api::Interpolatable for RectDef {}