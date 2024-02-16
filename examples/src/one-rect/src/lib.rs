#![allow(unused_imports)]

use pax_engine::*;
use pax_engine::api::*;
use pax_std::primitives::*;
use pax_std::types::*;
use pax_std::types::text::*;
use pax_std::components::*;
use pax_std::components::Stacker;

#[pax]
#[main]
#[file("lib.pax")]
pub struct OneRect {
    pub ticks: Property<usize>,
    pub num_clicks: Property<usize>,
    pub message: Property<String>,
}

impl OneRect {

    pub fn handle_pre_render(&mut self, ctx: &EngineContext) {
        let old_ticks = self.ticks.get();
        self.ticks.set(old_ticks + 1);
    }

}