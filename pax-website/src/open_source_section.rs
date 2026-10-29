use crate::{Acrostic, SiteTheme};
use pax_kit::*;

#[pax]
#[file("open_source_section.pax")]
pub struct OpenSourceSection {
    pub content_height: Property<f64>,
    pub compact: Property<bool>,
    pub artwork_height: Property<f64>,
    pub _height_bound: Property<bool>,
}

impl OpenSourceSection {
    pub fn mount(&mut self, ctx: &NodeContext) {
        crate::section_layout::bind_compact(ctx, &self.compact, 1000.0);
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        let width = ctx.bounds_self.get().0;
        let artwork_width = if self.compact.get() {
            width - 48.0
        } else {
            width * 0.88 * 0.61
        };
        self.artwork_height.set_if_neq(Acrostic::height(
            artwork_width,
            if self.compact.get() { 6 } else { 7 },
        ));
        if !self._height_bound.get() {
            self._height_bound.set(crate::section_layout::bind_height(
                ctx,
                "open_source_content",
                &self.content_height,
            ));
        }
    }
}
