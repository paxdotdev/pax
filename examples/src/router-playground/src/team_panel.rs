#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[file("team_panel.pax")]
pub struct TeamPanel {
    pub is_mobile: Property<bool>,
}
