
use pax_lang::*;
use pax_lang::api::*;
use pax_std::primitives::{Frame, Group, Rectangle, Text, Image};
use pax_std::types::{Color, Fill, LinearGradient, StackerDirection};
use pax_std::components::{Stacker, Sidebar};


const ROUTE_COUNT : usize = 5;


//noinspection RsMainFunctionNotFound
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
    pub sizes: Property<Vec<Option<Size>>>,
}

impl Example {
    pub fn handle_container_scroll(&mut self, ctx: RuntimeContext, args: ArgsScroll) {
        let mut scroll_position = *self.scroll_position.get();
        scroll_position = scroll_position - args.delta_y;
        scroll_position = scroll_position.min(0.0);
        scroll_position = scroll_position.max(-4000.0);
        self.scroll_position.set(scroll_position);
    }

    pub fn handle_container_key_down(&mut self, ctx: RuntimeContext, args: ArgsKeyDown) {
        let mut scroll_position = *self.scroll_position.get();
        if args.keyboard.key == "ArrowDown".to_string() || args.keyboard.key == "Down".to_string() {
            scroll_position = scroll_position - 20.0;
            scroll_position = scroll_position.min(0.0);
            scroll_position = scroll_position.max(-4000.0);
        }
        if args.keyboard.key == "ArrowUp".to_string() || args.keyboard.key == "Up".to_string() {
            scroll_position = scroll_position + 20.0;
            scroll_position = scroll_position.min(0.0);
            scroll_position = scroll_position.max(-4000.0);
        }
        if args.keyboard.key == "ArrowLeft".to_string() || args.keyboard.key == "Left".to_string() {
            scroll_position = scroll_position + 1000.0;
            scroll_position = scroll_position.min(0.0);
            scroll_position = scroll_position.max(-4000.0);
        }
        if args.keyboard.key == "ArrowRight".to_string() || args.keyboard.key == "Right".to_string() {
            scroll_position = scroll_position - 1000.0;
            scroll_position = scroll_position.min(0.0);
            scroll_position = scroll_position.max(-4000.0);
        }
        self.scroll_position.set(scroll_position);
    }

    pub fn handle_did_mount(&mut self, ctx: RuntimeContext) {
        self.sizes.set(vec![
            Some(Size::Percent(70.0.into())),
            None
        ]);
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