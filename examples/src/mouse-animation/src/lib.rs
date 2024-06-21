#![allow(unused_imports)]

pub mod path_animation;
use path_animation::PathAnimation;

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

use pax_engine::math::Generic;
use pax_engine::math::Point2;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub scroll: Property<f64>,
}

impl Example {
    pub fn on_mouse_move(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
        let (_, h) = ctx.transform_and_bounds_self.get().bounds;
        let part = event.mouse.y / h;
        self.scroll.set(part);
    }
}
