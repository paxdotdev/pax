#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[pax]
#[file("inner.pax")]
pub struct Inner {
    pub message_inner: Property<String>,
    pub inner_active: Property<bool>,
    pub x_pos: Property<Size>,
    pub some_num: Property<f64>,
    pub some_str: Property<String>,
}

impl Inner {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {}

    pub fn inner_clicked(&mut self, ctx: &NodeContext, args: Event<Click>) {
        self.inner_active.set(!self.inner_active.get());
        log::debug!("some num outer passed in: {:?}", self.some_num.get());
        log::debug!("some str outer passed in: {:?}", self.some_str.get());
    }
}
