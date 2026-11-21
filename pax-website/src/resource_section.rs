#![allow(unused_imports)]

use crate::{FrameworkSection, GetStarted, SiteTheme};
use pax_kit::*;

const COMPACT_BREAKPOINT_PX: f64 = 1000.0;

#[pax]
#[file("resource_section.pax")]
pub struct ResourceSection {
    pub light_mode: Property<bool>,
    pub compact: Property<bool>,
    pub content_height: Property<f64>,
    pub bottom_gap: Property<f64>,
    pub _vertical_padding: Property<f64>,
    pub get_started_height: Property<f64>,
    pub oscilloscope_height: Property<f64>,
    pub _height_bound: Property<bool>,
}

impl ResourceSection {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self._height_bound.set(false);
        crate::section_layout::bind_compact(ctx, &self.compact, COMPACT_BREAKPOINT_PX);
        let bottom_gap = self.bottom_gap.local();
        // The stack starts at 80px; the last 14px text line has 4px below its ink.
        self._vertical_padding.replace_with(LocalProperty::computed(
            move || 80.0 + (bottom_gap.get().round() - 4.0).max(0.0),
            &[self.bottom_gap.untyped()],
        ));
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        if !self._height_bound.get() {
            self._height_bound
                .set(crate::section_layout::bind_height_with_padding(
                    ctx,
                    "footer_content",
                    &self.content_height,
                    &self._vertical_padding,
                ));
        }
    }
}
