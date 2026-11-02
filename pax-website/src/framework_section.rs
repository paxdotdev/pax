#![allow(unused_imports)]

use crate::{ScreenshotStudy, SiteTheme};
use pax_kit::*;

#[pax]
#[file("framework_section.pax")]
pub struct FrameworkSection {
    pub content_height: Property<f64>,
    pub _height_bound: Property<bool>,
    pub compact: Property<bool>,
    pub scope_height: Property<f64>,
    pub game_height: Property<f64>,
    pub scene_progress: Property<f64>,
}

impl FrameworkSection {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self._height_bound.set(false);
        crate::section_layout::bind_compact(ctx, &self.compact, 900.0);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        if !self._height_bound.get() {
            self._height_bound.set(crate::section_layout::bind_height(
                ctx,
                "framework_content",
                &self.content_height,
            ));
        }
    }
}
