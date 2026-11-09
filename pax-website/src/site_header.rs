use crate::{NavLogo, SiteTheme};
use pax_kit::*;

/// Shared navigation for the full website and the static blog's embedded header.
/// The owning shell supplies positioning and the homepage's portal-logo twin.
#[pax]
#[file("site_header.pax")]
pub struct SiteHeader {
    pub compact: Property<bool>,
    pub show_logo: Property<bool>,
    pub controls_visible: Property<bool>,
    pub font_url: Property<String>,
}

impl SiteHeader {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        let bounds = ctx.bounds_self.clone();
        let compact = self.compact.local();
        let deps = [bounds.untyped()];
        let update = move || {
            compact.set_if_neq(bounds.get().0 < 760.0);
        };
        update();
        ctx.subscribe(&deps, update);
    }
}
