#![allow(unused_imports)]

use pax_lang::api::*;
use pax_lang::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[derive(Pax)]
#[file("inner.pax")]
pub struct Inner {
    pub message_inner: Property<String>,
    pub x_pos: Property<Size>,
}

impl Inner {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {}
}
