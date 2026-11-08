use pax_kit::*;
pub use pax_website::SiteHeader;

// The header's source stays in the website. This package only selects and
// configures that same component as an independently mounted runnable root.
pax_kit::runnable! { root: crate::SiteHeader, init: init_header }

pub fn init_header(_: &InitContext) -> AppResult<SiteHeader> {
    Ok(SiteHeader {
        show_logo: Property::new(true),
        controls_visible: Property::new(true),
        font_url: Property::new("/blog/_pax/header/assets/fonts/Manrope.ttf".into()),
        ..Default::default()
    })
}
