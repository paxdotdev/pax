use crate::{SharedDefinitionDemo, SiteTheme};
use pax_kit::*;

#[pax]
#[file("authoring_section.pax")]
pub struct AuthoringSection {
    pub content_height: Property<f64>,
    pub _height_bound: Property<bool>,
    pub compact: Property<bool>,
}

impl AuthoringSection {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self._height_bound.set(false);
        crate::section_layout::bind_compact(ctx, &self.compact, 900.0);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        if !self._height_bound.get() {
            self._height_bound.set(crate::section_layout::bind_height(
                ctx,
                "authoring_content",
                &self.content_height,
            ));
        }
    }
}
