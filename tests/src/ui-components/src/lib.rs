#![allow(unused_imports)]

pub use pax_component_library::ConfirmationDialog;
pub use pax_component_library::PaxDropdown;
pub use pax_component_library::PaxRadioSet;
pub use pax_component_library::PaxSlider;
pub use pax_component_library::Resizable;
pub use pax_component_library::Tabs;

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub selected: Property<u32>,
    pub dialog_open: Property<bool>,
    pub signal: Property<bool>,
}

impl Example {
    pub fn on_click(&mut self, ctx: &NodeContext, event: Event<Click>) {
        self.selected.set(2);
        self.dialog_open.set(true);
    }
}
