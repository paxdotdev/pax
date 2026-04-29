#![allow(unused_imports)]

use crate::SidebarNav;
use pax_kit::*;

#[pax]
#[file("mobile_menu_drawer.pax")]
pub struct MobileMenuDrawer {
    pub mobile_menu_open: Property<bool>,
}
