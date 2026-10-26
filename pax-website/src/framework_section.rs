#![allow(unused_imports)]

use crate::SiteTheme;
use led_oscilloscope::LedOscilloscope;
use pax_kit::*;

#[pax]
#[file("framework_section.pax")]
pub struct FrameworkSection {
    pub content_height: Property<f64>,
    pub _height_bound: Property<bool>,
    pub compact: Property<bool>,
    pub scope_height: Property<f64>,
}

impl FrameworkSection {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self._height_bound.set(false);
        crate::section_layout::bind_compact(ctx, &self.compact, 900.0);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        let width = ctx.bounds_self.get().0;
        let content_width = if self.compact.get() {
            width - 48.
        } else {
            width * 0.88
        };
        self.scope_height
            .set_if_neq(LedOscilloscope::height(content_width));
        if !self._height_bound.get() {
            self._height_bound.set(crate::section_layout::bind_height(
                ctx,
                "framework_content",
                &self.content_height,
            ));
        }
    }
}
