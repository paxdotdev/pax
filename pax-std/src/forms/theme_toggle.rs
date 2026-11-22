#[allow(unused_imports)]
use crate::*;
use pax_engine::api::{ButtonClick, Event, NodeContext};
use pax_engine::*;

/// A moon/sun switch with a native button for pointer and keyboard activation.
/// Bind `light_mode` to the preference the switch should edit. The 69×23px
/// artwork is centered inside the bounds supplied for the hit target.
#[pax]
#[engine_import_path("pax_engine")]
#[file("forms/theme_toggle.pax")]
pub struct ThemeToggle {
    /// Selected appearance: false for dark, true for light.
    pub light_mode: Property<bool>,
    /// Draw dark icons for a light surrounding surface, independently of selection.
    pub chrome_light_mode: Property<bool>,
    /// Optional accessible action label; by default it names the theme to switch to.
    pub label: Property<String>,
}

impl ThemeToggle {
    pub fn toggle(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.light_mode.set(!self.light_mode.get());
    }
}
