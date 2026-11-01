use crate::SiteTheme;
use pax_kit::*;

#[pax]
#[file("site_logo.pax")]
pub struct SiteLogo {
    pub radiance_enabled: Property<bool>,
    pub light_x: Property<f64>,
    pub light_y: Property<f64>,
    pub brightness: Property<f64>,
    pub progress: Property<f64>,
    pub click_to_replay: Property<bool>,
    pub logo_scale: Property<f64>,
}

impl SiteLogo {
    pub fn hover(&mut self, ctx: &NodeContext, _: Event<MouseOver>) {
        ctx.set_cursor(cursor::CursorStyle::Pointer);
    }

    pub fn leave(&mut self, ctx: &NodeContext, _: Event<MouseOut>) {
        ctx.set_cursor(cursor::CursorStyle::Auto);
    }

    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let bounds = ctx.bounds_self.clone();
        let deps = [bounds.untyped()];
        self.logo_scale.replace_with(LocalProperty::computed(
            move || {
                let (width, height) = bounds.get();
                (width / 920.72).min(height / 463.95).max(0.0) * 100.0
            },
            &deps,
        ));
    }
}
