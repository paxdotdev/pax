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
    pub num: Property<Vec<usize>>,
}

impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.num.set(vec![0, 1]);
    }

    pub fn click0(&mut self, ctx: &NodeContext, args: Event<Click>) {
        self.num.set(vec![0, 1]);
    }

    pub fn click1(&mut self, ctx: &NodeContext, args: Event<Click>) {
        self.num.set(vec![0, 1, 2]);
    }

    pub fn click2(&mut self, ctx: &NodeContext, args: Event<Click>) {
        self.num.set(vec![0, 1, 2, 3, 4]);
    }
    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {}
}
