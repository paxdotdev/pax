#![allow(unused_imports)]

use crate::{DeviceFrame, PathStudy, SiteTheme};
use pax_kit::*;

#[pax]
#[file("framework_section.pax")]
pub struct FrameworkSection {
    pub content_height: Property<f64>,
    pub compact: Property<bool>,
}

impl FrameworkSection {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
    }

    fn sync_layout(&mut self, ctx: &NodeContext) {
        crate::section_layout::publish_height(ctx, "framework_content", &self.content_height);
        self.compact.set_if_neq(ctx.bounds_self.get().0 < 900.0);
    }
}
