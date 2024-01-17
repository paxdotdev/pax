use pax_lang::api::*;
use pax_lang::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[derive(Pax)]
#[file("controls/settings/property_section.pax")]
pub struct PropertyEditor {
    pub name: Property<StringBox>,
}

impl PropertyEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {}
}
