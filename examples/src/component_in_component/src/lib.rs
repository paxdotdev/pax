#![allow(unused_imports)]

mod inner;
use inner::Inner;
use pax_engine::api::*;
use pax_engine::*;
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
    pub some_num_outer: Property<i32>,
    pub some_str_outer: Property<String>,
}

impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.message_outer.set("testing 12049".to_string());
        self.x.set(Size::Percent(30.into()));
        self.some_num_outer.set(3490);
        self.some_str_outer.set("testing!!".to_string());
    }

    pub fn outer_clicked(&mut self, ctx: &NodeContext, args: Event<Click>) {
        self.outer_active.set(!self.outer_active.get());
    }
}
