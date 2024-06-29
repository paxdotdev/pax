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
    pub num: Property<usize>,
    pub showing: Property<bool>,
}

impl Example {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        self.num.set(20);
    }
    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {}

    pub fn click(&mut self, ctx: &NodeContext, args: Event<Click>) {
        self.num.set(self.num.get() - 1);
    }

    pub fn toggle(&mut self, ctx: &NodeContext, args: Event<Click>) {
        self.showing.set(!self.showing.get());
    }
}
