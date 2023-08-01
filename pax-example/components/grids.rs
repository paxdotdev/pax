use pax_lang::api::*;
use pax_lang::*;
use pax_std::primitives::{Ellipse, Frame, Group, Path, Rectangle, Text};
use pax_std::components::{Stacker};
use pax_std::types::{Color, StackerDirection};


#[derive(Pax)]
#[file("grids.pax")]
pub struct Grids {
    pub ticks: Property<usize>,
    pub rects: Property<Vec<RectDef>>,
}

impl Grids {

    pub fn handle_did_mount(&mut self, ctx: RuntimeContext) {

        // self.rects.set(vec![
        //     pax_struct!(
        //         RectDef {
        //             x: 200,
        //             y: 150,
        //             width: 100,
        //             height: 100,
        //             fill: Color::rgb(1.0.into(), 0.0.into(), 0.0.into()),
        //         }
        //     ),
        //     pax_struct!(
        //         RectDef {
        //             x: 500,
        //             y: 300,
        //             width: 250,
        //             height: 350,
        //             fill: Color::rgb(0.0.into(), 1.0.into(), 0.0.into()),
        //         }
        //     ),
        // ]);
    }

}

#[derive(Pax)]
#[custom(Imports)]
pub struct RectDef {
    pub x: Property<usize>,
    pub y: Property<usize>,
    pub width: Property<usize>,
    pub height: Property<usize>,
    pub fill: Property<pax_std::types::Color>,
}
