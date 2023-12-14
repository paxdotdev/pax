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
    pub activated: Property<bool>,
    pub not_activated: Property<bool>,
    pub message: Property<String>,
}

impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.message.set("Click me".to_string());
    }
    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        let old_ticks = self.ticks.get();
        self.ticks.set(old_ticks + 1);
    }

    pub fn increment(&mut self, ctx: &NodeContext, args: ArgsClick) {
        self.activated.set(!self.activated.get());
        self.not_activated.set(!self.activated.get());
        self.message.set(format!(
            "{} - {}",
            self.activated.get(),
            self.not_activated.get()
        ));
    }
}
