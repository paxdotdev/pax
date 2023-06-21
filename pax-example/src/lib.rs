
pub mod hello_rgb;
pub mod fireworks;
pub mod grids;
pub mod camera;

use pax::*;
use pax::api::*;
use pax_std::primitives::{Frame};

use crate::grids::Grids;
use crate::hello_rgb::HelloRGB;
use crate::fireworks::Fireworks;
use crate::camera::Camera;

const ROUTE_COUNT : usize = 4;

// pub mod pax_reexports {
//     pub use crate::Example;
//     pub mod camera {
//         pub use crate::camera::Camera;
//         pub use crate::camera::TypeExample;
//     }
//     pub mod fireworks {
//         pub use crate::fireworks::Fireworks;
//     }
//     pub mod grids {
//         pub use crate::grids::Grids;
//         pub use crate::grids::RectDef;
//     }
//     pub mod hello_rgb {
//         pub use crate::hello_rgb::HelloRGB;
//     }
//     pub use f64;
//     pub mod pax_std{
//         pub mod primitives{
//             pub use pax_std::primitives::Ellipse;
//             pub use pax_std::primitives::Frame;
//             pub use pax_std::primitives::Group;
//             pub use pax_std::primitives::Rectangle;
//         }
//         pub mod types{
//             pub use pax_std::types::Color;
//             pub use pax_std::types::Stroke;
//         }
//     }
//     pub mod std{
//         pub mod vec{
//             pub use std::vec::Vec;
//         }
//     }
//     pub use usize;
//
// }


#[derive(Pax)]
#[main]
#[inlined(
    <Frame width=100% height=100% @click=modulate  >
        if current_route == 0 {
            <Grids />
        }
        if current_route == 1 {
            <Fireworks />
        }
        if current_route == 2 {
            <HelloRGB />
        }
        if current_route == 3 {
            <Camera />
        }
    </Frame>
)]
pub struct Example {
    pub current_route: Property<usize>,
}

impl Example {
    pub fn modulate(&mut self, ctx: RuntimeContext, args: ArgsClick) {
        let old_route = self.current_route.get();
        self.current_route.set((old_route + 1) % ROUTE_COUNT);
    }
}
