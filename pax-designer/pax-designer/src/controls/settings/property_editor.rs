use pax_lang::api::*;
use pax_lang::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[derive(Pax)]
#[file("controls/settings/property_editor.pax")]
pub struct PropertyEditor {
    pub name: Property<StringBox>,
    pub definition: Property<String>,
    pub def: Property<String>,
    pub init: Property<i32>,
}

impl PropertyEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        self.def.set(self.definition.get().clone());
    }

    pub fn on_render(&mut self, ctx: &NodeContext) {
        self.init.set(self.init.get() + 1);
        // Need to wait a bit before we initialize def, so that self.definition has been
        // populated with it's default value.
        if *self.init.get() == 2 {
            log("INIT!!");
            self.def.set(self.definition.get().clone());
        }
    }

    pub fn text_change(&mut self, ctx: &NodeContext, args: ArgsTextboxChange) {
        self.def.set(args.text);
    }
}
