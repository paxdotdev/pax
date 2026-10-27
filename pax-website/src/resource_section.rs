#![allow(unused_imports)]

use crate::{CalculatorPreview, SiteTheme};
use pax_kit::*;

const COMPACT_BREAKPOINT_PX: f64 = 1000.0;

#[pax]
#[file("resource_section.pax")]
pub struct ResourceSection {
    pub compact: Property<bool>,
    pub content_height: Property<f64>,
    pub _height_bound: Property<bool>,
    pub preview_height: Property<f64>,
}

impl ResourceSection {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self._height_bound.set(false);
        crate::section_layout::bind_compact(ctx, &self.compact, COMPACT_BREAKPOINT_PX);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        if !self._height_bound.get() {
            self._height_bound.set(crate::section_layout::bind_height(
                ctx,
                "footer_content",
                &self.content_height,
            ));
        }
    }
}
