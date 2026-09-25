use crate::{DeviceFrame, PathStudy, SiteTheme};
use pax_kit::*;

#[pax]
#[file("authoring_section.pax")]
pub struct AuthoringSection {
    pub content_height: Property<f64>,
    pub compact: Property<bool>,
}

impl AuthoringSection {
    pub fn on_pre_render(&mut self, ctx: &NodeContext) {
        crate::section_layout::publish_height(ctx, "authoring_content", &self.content_height);
        self.compact.set_if_neq(ctx.bounds_self.get().0 < 900.0);
    }
}
