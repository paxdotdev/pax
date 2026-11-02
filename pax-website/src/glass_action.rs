use crate::SiteTheme;
use pax_kit::*;

/// Shared action surface, lit by the shell's stationary strip and pointer light.
#[pax]
#[file("glass_action.pax")]
pub struct GlassAction {
    pub label: Property<String>,
    pub url: Property<String>,
    pub opal: Property<bool>,
    pub github: Property<bool>,
}
