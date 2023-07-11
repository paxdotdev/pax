
use pax_lang::*;
use pax_lang::api::*;
use pax_std::primitives::{Frame, Group, Rectangle, Text, Image};
use pax_std::types::{Color, Fill, LinearGradient, StackerDirection};
use pax_std::components::{Stacker};


const ROUTE_COUNT : usize = 5;

#[derive(Pax)]
#[main]
// #[inlined(
//     <Frame width=100% height=100% @key_press=modulate  >
//         if current_route == 2 {
//             <Grids />
//         }
//         if current_route == 1 {
//             <Fireworks />
//         }
//         if current_route == 0 {
//             <Words />
//         }
//         if current_route == 3 {
//             <Camera />
//         }
//         if current_route == 4 {
//             <HelloRGB />
//         }
//     </Frame>
// )]
#[file("lib.pax")]
pub struct Example {
    pub scroll_position: Property<f64>,
    pub panels: Property<Vec<Panel>>,
}

impl Example {
    pub fn handle_container_scroll(&mut self, ctx: RuntimeContext, args: ArgsScroll) {
        let mut scroll_position = *self.scroll_position.get();
        scroll_position = scroll_position - args.delta_y;
        scroll_position = scroll_position.min(0.0);
        scroll_position = scroll_position.max(-4000.0);
        self.scroll_position.set(scroll_position);
    }

    pub fn handle_did_mount(&mut self, ctx: RuntimeContext) {
    //     self.panels.set(vec![
    //         Panel{
    //             fill: Fill::LinearGradient(
    //                 LinearGradient {
    //                     start: (Size::Percent(0.0.into()),Size::Percent(0.0.into())),
    //                     end: (Size::Percent(100.0.into()),Size::Percent(100.0.into())),
    //                     stops: (
    //                         Color::rgb(Numeric::from(1.0), Numeric::from(1.0), Numeric::from(0.0)),
    //                         Color::rgb(Numeric::from(1.0), Numeric::from(0.0), Numeric::from(1.0))
    //                     ),
    //                 }
    //             )
    //         },
    //         Panel{
    //             fill: Fill::LinearGradient(
    //                 LinearGradient {
    //                     start: (Size::Percent(0.0.into()),Size::Percent(0.0.into())),
    //                     end: (Size::Percent(100.0.into()),Size::Percent(100.0.into())),
    //                     stops: (
    //                         Color::rgb(Numeric::from(0.0), Numeric::from(1.0), Numeric::from(1.0)),
    //                         Color::rgb(Numeric::from(0.0), Numeric::from(1.0), Numeric::from(0.0))
    //                     ),
    //                 }
    //             )
    //         },
    //         Panel{
    //             fill: Fill::LinearGradient(
    //                 LinearGradient {
    //                     start: (Size::Percent(0.0.into()),Size::Percent(0.0.into())),
    //                     end: (Size::Percent(100.0.into()),Size::Percent(100.0.into())),
    //                     stops: (
    //                         Color::rgb(Numeric::from(1.0), Numeric::from(0.0), Numeric::from(0.0)),
    //                         Color::rgb(Numeric::from(0.0), Numeric::from(1.0), Numeric::from(1.0))
    //                     ),
    //                 }
    //             )
    //         },
    //         Panel{
    //             fill: Fill::LinearGradient(
    //                 LinearGradient {
    //                     start: (Size::Percent(0.0.into()),Size::Percent(0.0.into())),
    //                     end: (Size::Percent(100.0.into()),Size::Percent(100.0.into())),
    //                     stops: (
    //                         Color::rgb(Numeric::from(1.0), Numeric::from(0.0), Numeric::from(1.0)),
    //                         Color::rgb(Numeric::from(0.0), Numeric::from(1.0), Numeric::from(0.0))
    //                     ),
    //                 }
    //             )
    //         },
    //         Panel{
    //             fill: Fill::LinearGradient(
    //                 LinearGradient {
    //                     start: (Size::Percent(0.0.into()),Size::Percent(0.0.into())),
    //                     end: (Size::Percent(100.0.into()),Size::Percent(100.0.into())),
    //                     stops: (
    //                         Color::rgb(Numeric::from(0.0), Numeric::from(0.0), Numeric::from(1.0)),
    //                         Color::rgb(Numeric::from(1.0), Numeric::from(1.0), Numeric::from(0.0))
    //                     ),
    //                 }
    //             )
    //         },
    //     ])
     }
}

#[derive(Pax)]
#[custom(Imports)]
pub struct Panel {
    pub fill: Fill,
}