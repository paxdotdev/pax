#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::*;
pub mod inner_comp;
pub use inner_comp::InnerComp;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub visible: Property<bool>,
}

pub struct StoreExample {
    pub i: i32,
}

impl Store for StoreExample {}

impl Example {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        self.visible.set(true);
        ctx.push_local_store(StoreExample { i: 42 });
    }

    pub fn custom_event_trigger(&mut self, ctx: &NodeContext) {
        log::info!("custom event was triggered!");
        self.visible.set(false);
    }
}
