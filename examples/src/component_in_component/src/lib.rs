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

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub message_outer: Property<String>,
    pub outer_active: Property<bool>,
    pub x: Property<Size>,
}

impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.message_outer.set("testing 12049".to_string());
        self.x.set(Size::Percent(30.into()));
    }

    pub fn outer_clicked(&mut self, ctx: &NodeContext, args: ArgsClick) {
        self.outer_active.set(!self.outer_active.get());
    }
}
