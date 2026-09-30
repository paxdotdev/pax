#![allow(unused_imports)]

use crate::{CalculatorPreview, SiteTheme};
use pax_kit::*;

const COMPACT_BREAKPOINT_PX: f64 = 1000.0;

#[pax]
#[file("resource_section.pax")]
pub struct ResourceSection {
    pub compact: Property<bool>,
    pub content_height: Property<f64>,
    pub phone_height: Property<f64>,
}

impl ResourceSection {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
    }

    fn sync_layout(&mut self, ctx: &NodeContext) {
        self.compact
            .set_if_neq(ctx.bounds_self.get().0 < COMPACT_BREAKPOINT_PX);
        crate::section_layout::publish_height(ctx, "footer_content", &self.content_height);
    }
}
