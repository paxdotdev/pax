use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[pax]
#[file("controls/settings/property_section.pax")]
pub struct PropertyEditor {
    pub name: Property<StringBox>,
}

impl PropertyEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {}
}
