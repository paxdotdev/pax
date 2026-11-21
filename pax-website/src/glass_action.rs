use crate::SiteTheme;
use pax_kit::*;

/// Action surface lit by the shell lights and any direct light in its section.
#[pax]
#[file("glass_action.pax")]
pub struct GlassAction {
    pub label: Property<String>,
    pub url: Property<String>,
    pub opal: Property<bool>,
    pub github: Property<bool>,
}
