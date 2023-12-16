#![allow(unused_imports)]

use pax_lang::api::*;
use pax_lang::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[derive(Pax)]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub ticks: Property<usize>,
    pub num_clicks: Property<usize>,
    pub message: Property<String>,
    pub conditional: Property<bool>,
}

impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.message.set("Click me".to_string());
    }
    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        let old_ticks = self.ticks.get();
        self.ticks.set(old_ticks + 1);
    }

    pub fn toggle(&mut self, ctx: &NodeContext, args: ArgsClick) {
        let old_num_clicks = self.num_clicks.get();
        self.num_clicks.set(old_num_clicks + 1);
        self.message
            .set(format!("{} clicks", self.num_clicks.get()));
        self.conditional.set(!self.conditional.get());
    }
}
