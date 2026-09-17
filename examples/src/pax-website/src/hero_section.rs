#![allow(unused_imports)]

use crate::SiteTheme;
use living_quilt::Example as LivingQuilt;
use pax_kit::*;

const COMPACT_BREAKPOINT_PX: f64 = 760.0;

#[pax]
#[file("hero_section.pax")]
pub struct HeroSection {
    pub compact: Property<bool>,
}

impl HeroSection {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
    }

    fn sync_layout(&mut self, ctx: &NodeContext) {
        self.compact
            .set_if_neq(ctx.bounds_self.get().0 < COMPACT_BREAKPOINT_PX);
    }
}
