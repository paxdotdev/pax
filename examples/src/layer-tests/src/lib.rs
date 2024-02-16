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
    pub activated: Property<bool>,
    pub message: Property<String>,
}

impl Example {
    pub fn handle_mount(&mut self, ctx: &EngineContext) {
        self.message.set("false".to_string());
    }

    pub fn handle_pre_render(&mut self, ctx: &EngineContext) {
        let old_ticks = self.ticks.get();
        self.ticks.set(old_ticks + 1);
    }

    pub fn toggle(&mut self, ctx: &EngineContext, args: ArgsClick) {
        self.activated.set(!self.activated.get());
        self.message.set(format!("{}", self.activated.get()));
    }
}
