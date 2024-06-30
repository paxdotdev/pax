#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
pub mod property_editor;

use property_editor::PropertyEditor;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub showing: Property<bool>,
    pub stid: Property<usize>,
    pub snid: Property<usize>,
}

impl Example {
    pub fn on_mount(&mut self, ctx: &NodeContext) {}

    pub fn toggle(&mut self, ctx: &NodeContext, args: Event<Click>) {
        self.showing.set(!self.showing.get());
    }

    pub fn fake_toggle(&mut self, ctx: &NodeContext, args: Event<Click>) {
        self.showing.set(self.showing.get());
    }
}
