#![allow(unused_imports)]

use pax_lang::*;
use pax_lang::api::*;
use pax_std::primitives::*;
use pax_std::types::*;
use pax_std::types::text::*;
use pax_std::components::Stacker;

#[derive(Pax)]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub num_clicks: Property<usize>,
}

impl Example {
    pub fn handle_mount(&mut self, ctx: RuntimeContext) {
        pax_lang::log("Mounted!");
    }

}