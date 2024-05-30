#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
pub mod inner_comp;
pub use inner_comp::InnerComp;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {}

pub struct StoreExample {
    pub i: i32,
}

impl Store for StoreExample {}

impl Example {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.push_local_store(StoreExample { i: 42 });
    }

    pub fn custom_event_trigger(&mut self, ctx: &NodeContext) {
        log::debug!("custom event was triggered!");
    }
}
