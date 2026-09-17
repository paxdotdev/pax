use crate::SiteTheme;
use pax_kit::*;

#[pax]
#[file("authoring_section.pax")]
pub struct AuthoringSection {
    pub compact: Property<bool>,
}

impl AuthoringSection {
    pub fn on_pre_render(&mut self, ctx: &NodeContext) {
        self.compact.set_if_neq(ctx.bounds_self.get().0 < 760.0);
    }
}
