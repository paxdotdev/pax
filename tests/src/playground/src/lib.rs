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
    pub ticks: Property<usize>,
    pub num_clicks: Property<usize>,
}

struct SomeStore {
    i: i32,
}

impl Store for SomeStore {}

impl Example {
    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        ctx.push_environment(SomeStore { i: 45 });
        let old_ticks = self.ticks.get();
        self.ticks.set((old_ticks + 1) % 255);
    }

    pub fn increment(&mut self, ctx: &NodeContext, args: Event<Click>) {
        let old_num_clicks = self.num_clicks.get();
        self.num_clicks.set(old_num_clicks + 1);
    }
}
