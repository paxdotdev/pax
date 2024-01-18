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
    pub definition: Property<StringBox>,
    pub last_definition: Property<String>,
    pub textbox: Property<String>,
}

impl PropertyEditor {
    pub fn on_render(&mut self, ctx: &NodeContext) {
        if &self.definition.get().string != self.last_definition.get() {
            self.last_definition
                .set(self.definition.get().string.clone());
            self.textbox.set(self.definition.get().string.clone());
        }
    }

    pub fn text_change(&mut self, ctx: &NodeContext, args: ArgsTextboxChange) {
        self.textbox.set(args.text);
    }
}
