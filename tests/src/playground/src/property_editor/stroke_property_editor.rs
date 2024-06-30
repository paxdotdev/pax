#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[pax]
#[main]
#[file("property_editor/stroke_property_editor.pax")]
pub struct StrokePropertyEditor {
    pub stid: Property<usize>,
    pub snid: Property<usize>,
}
