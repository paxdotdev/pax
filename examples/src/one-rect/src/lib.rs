#![allow(unused_imports)]

use pax_lang::*;
use pax_lang::api::*;
use pax_std::primitives::*;
use pax_std::types::*;
use pax_std::types::text::*;
use pax_std::components::*;
use pax_std::components::Stacker;

#[derive(Pax)]
#[main]
#[file("lib.pax")]
pub struct OneRect {
    pub ticks: Property<usize>,
    pub num_clicks: Property<usize>,
    pub message: Property<String>,
}

impl OneRect {

    pub fn handle_pre_render(&mut self, ctx: RuntimeContext) {
        let old_ticks = self.ticks.get();
        self.ticks.set(old_ticks + 1);
    }

}