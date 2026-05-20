#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub is_mobile: Property<bool>,
}

const MOBILE_BREAKPOINT_WIDTH: f64 = 720.0;

impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.sync_responsive_state(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        self.sync_responsive_state(ctx);
    }

    fn sync_responsive_state(&mut self, ctx: &NodeContext) {
        let (width, _) = ctx.bounds_self.get();
        self.is_mobile.set_if_neq(width < MOBILE_BREAKPOINT_WIDTH);
    }
}
