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

impl Example {
    pub fn custom_event_trigger(&mut self, ctx: &NodeContext) {
        log::debug!("custom event was triggered!");
    }
}
