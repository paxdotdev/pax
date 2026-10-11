#![allow(unused_imports)]

use crate::SiteTheme;
use pax_kit::*;

const COMPACT_BREAKPOINT_PX: f64 = 760.0;

#[pax]
#[file("site_shell.pax")]
pub struct SiteShell {
    pub compact: Property<bool>,
    pub safe_top: Property<f64>,
    pub safe_bottom: Property<f64>,
    pub safe_left: Property<f64>,
    pub safe_right: Property<f64>,
}

impl SiteShell {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
    }

    fn sync_layout(&mut self, ctx: &NodeContext) {
        self.compact.set_if_neq(
            ctx.bounds_self.get().0 - self.safe_left.get() - self.safe_right.get()
                < COMPACT_BREAKPOINT_PX,
        );
    }
}
