#![allow(unused_imports)]

use super::StoreExample;
use pax_engine::api::*;
use pax_engine::*;
use pax_std::*;

#[pax]
#[main]
#[file("inner_comp.pax")]
pub struct InnerComp {}

impl InnerComp {
    pub fn clicked(&mut self, ctx: &NodeContext, event: Event<Click>) {
        ctx.dispatch_event("custom_event").unwrap();
    }

    pub fn on_unmount(&mut self, ctx: &NodeContext) {
        log::info!("unmounted!");
    }
}
