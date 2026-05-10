#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[file("guide_panel.pax")]
pub struct GuidePanel {
    pub is_mobile: Property<bool>,
}
