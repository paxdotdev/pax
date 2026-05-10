#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[file("top_level_fallback_panel.pax")]
pub struct TopLevelFallbackPanel {
    pub is_mobile: Property<bool>,
}
