pub mod hello_rgb;
pub mod fireworks;
pub mod grids;
pub mod camera;
pub mod words;

use pax::*;
use pax::api::*;
use pax_std::primitives::{Frame};

use crate::grids::Grids;
use crate::hello_rgb::HelloRGB;
use crate::fireworks::Fireworks;
use crate::camera::Camera;
use crate::words::Words;

const ROUTE_COUNT : usize = 5;

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
            <Words />
        }
        if current_route == 3 {
            <Camera />
        }
        if current_route == 4 {
            <HelloRGB />
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
