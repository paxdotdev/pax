#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub easing_value: Property<f64>,
}

impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {}
    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {}

    pub fn left(&mut self, ctx: &NodeContext, args: Event<Click>) {
        self.easing_value
            .ease_to_later(0.0, 100, EasingCurve::Linear);
    }
    pub fn right(&mut self, ctx: &NodeContext, args: Event<Click>) {
        self.easing_value
            .ease_to_later(100.0, 100, EasingCurve::Linear);
    }
}
