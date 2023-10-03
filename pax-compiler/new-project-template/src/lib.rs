#![allow(unused_imports)]

use pax_lang::*;
use pax_lang::api::*;
use pax_std::primitives::{Group, Text};
use pax_std::types::{Color};
use pax_std::components::{Stacker};

#[derive(Pax)]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub num_clicks: Property<usize>,
}

impl Example {
    pub fn handle_did_mount(&mut self, ctx: RuntimeContext) {
        pax_lang::log("Mounted!");
    }

}