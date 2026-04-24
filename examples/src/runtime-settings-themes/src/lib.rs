#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub color_mode: Property<u32>,
    pub typography_mode: Property<u32>,
    pub corner_mode: Property<u32>,
}

impl Example {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
        self.color_mode.set(0);
        self.typography_mode.set(0);
        self.corner_mode.set(0);
    }
}

#[pax]
#[file("light_mode_theme.pax")]
pub struct LightModeTheme {}

#[pax]
#[file("dark_mode_theme.pax")]
pub struct DarkModeTheme {}

#[pax]
#[file("sans_typography_theme.pax")]
pub struct SansTypographyTheme {
    pub is_dark: Property<bool>,
}

#[pax]
#[file("serif_typography_theme.pax")]
pub struct SerifTypographyTheme {
    pub is_dark: Property<bool>,
}

#[pax]
#[file("rounded_corner_theme.pax")]
pub struct RoundedCornerTheme {}

#[pax]
#[file("squared_corner_theme.pax")]
pub struct SquaredCornerTheme {}
