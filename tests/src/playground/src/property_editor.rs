#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
pub mod fill_property_editor;
pub mod stroke_property_editor;
pub mod text_property_editor;
use fill_property_editor::FillPropertyEditor;
use stroke_property_editor::StrokePropertyEditor;
use text_property_editor::TextPropertyEditor;

#[pax]
#[main]
#[file("property_editor.pax")]
pub struct PropertyEditor {
    pub stid: Property<usize>,
    pub snid: Property<usize>,
    pub name: Property<String>,
    pub ind: Property<usize>,
    pub prop_type_ident_id: Property<usize>,
}

impl PropertyEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        self.prop_type_ident_id.replace_with(self.ind);
    }
}
