#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[file("landing_panel.pax")]
pub struct LandingPanel {
    pub is_mobile: Property<bool>,
}
