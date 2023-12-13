#![allow(unused_imports)]

mod slotted;

use pax_lang::api::*;
use pax_lang::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
use slotted::Slotted;

#[derive(Pax)]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub vec: Property<Vec<Numeric>>,
    pub val: Property<Numeric>,
}

impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {}

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        let (w, h) = ctx.bounds_parent;
        self.vec.set(vec![Numeric::from(w / 2.0)]);
    }
}
