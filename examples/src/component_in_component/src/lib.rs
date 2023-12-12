#![allow(unused_imports)]

mod inner;
use inner::Inner;
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
    pub x: Property<Size>,
    pub y: Property<Size>,
}

impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.x.set(Size::Percent(12.5.into()));
        self.y.set(Size::Percent(12.5.into()));
    }
}
