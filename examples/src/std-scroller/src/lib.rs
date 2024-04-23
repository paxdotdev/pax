#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Scroller;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {}

impl Example {
    pub fn on_click(&mut self, ctx: &NodeContext, event: Event<Click>) {
        log::debug!("click registered!!");
    }
}
