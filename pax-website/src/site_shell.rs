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
        let bounds = ctx.bounds_self.clone();
        let left = self.safe_left.local();
        let right = self.safe_right.local();
        let deps = [bounds.untyped(), left.untyped(), right.untyped()];
        let compact = self.compact.local();
        let update = move || {
            compact.set_if_neq(bounds.get().0 - left.get() - right.get() < COMPACT_BREAKPOINT_PX);
        };
        update();
        ctx.subscribe(&deps, update);
    }
}
