#![allow(unused_imports)]

use pax_component_library::table::Col;
use pax_component_library::table::Row;
use pax_component_library::table::Span;
pub use pax_component_library::ConfirmationDialog;
pub use pax_component_library::PaxDropdown;
pub use pax_component_library::PaxRadioSet;
pub use pax_component_library::PaxSlider;
pub use pax_component_library::Resizable;
pub use pax_component_library::Table;
pub use pax_component_library::Tabs;
pub use pax_component_library::Toast;
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
#[custom(Default)]
pub struct Example {
    pub selected: Property<u32>,
    pub dialog_open: Property<bool>,
    pub message: Property<String>,
    pub signal: Property<bool>,
    pub num: Property<usize>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            message: Property::default(),
            selected: Property::new(1),
            dialog_open: Property::new(false),
            signal: Property::new(false),
            num: Default::default(),
        }
    }
}

impl Example {
    pub fn on_click(&mut self, ctx: &NodeContext, event: Event<Click>) {
        self.selected.set(2);
        self.dialog_open.set(true);
    }
    pub fn on_left_side_click(&mut self, ctx: &NodeContext, event: Event<Click>) {
        self.message
            .set(format!("this is a message! mouse x-pos: {}", event.mouse.x));
    }
}
