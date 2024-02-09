#![allow(unused_imports)]

mod slotted;

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
use slotted::Slotted;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub num: Property<usize>,
}

impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.num.set(1);
    }

    pub fn click0(&mut self, ctx: &NodeContext, args: ArgsClick) {
        self.num.set(1);
    }

    pub fn click1(&mut self, ctx: &NodeContext, args: ArgsClick) {
        self.num.set(3);
    }

    pub fn click2(&mut self, ctx: &NodeContext, args: ArgsClick) {
        self.num.set(5);
    }
    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {}
}
