#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub jump: Property<bool>,
}

impl Example {
    pub fn toggle(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        log::info!("toggle skip");
        self.jump.set(!self.jump.get());
    }
}
