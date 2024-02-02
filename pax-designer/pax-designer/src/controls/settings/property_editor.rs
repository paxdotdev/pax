use pax_lang::api::*;
use pax_lang::*;
use pax_manifest::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[pax]
#[file("controls/settings/property_editor.pax")]
pub struct PropertyEditor {
    pub name: Property<StringBox>,
    pub definition: Property<StringBox>,
    pub last_definition: Property<String>,
    pub textbox: Property<String>,
    pub stid: Property<StringBox>,
    pub snid: Property<Numeric>,
    pub error: Property<String>,
}

impl PropertyEditor {
    pub fn on_render(&mut self, ctx: &NodeContext) {
        if &self.definition.get().string != self.last_definition.get() {
            self.last_definition
                .set(self.definition.get().string.clone());
            self.textbox.set(self.definition.get().string.clone());
            self.text_change(
                ctx,
                ArgsTextboxChange {
                    text: self.definition.get().string.to_owned(),
                },
            );
        }
    }

    pub fn text_change(&mut self, ctx: &NodeContext, args: ArgsTextboxChange) {
        self.textbox.set(args.text.to_owned());
        let name = &self.name.get().string;
        let mut dt = ctx.designtime.borrow_mut();
        let mut node_definition = dt.get_orm_mut().get_node(
            &self.stid.get().string,
            self.snid.get().get_as_int() as usize,
        );

        let variable = name.strip_suffix(':').unwrap_or(&name);
        if let Err(error) = node_definition.set_property(variable, &args.text) {
            self.error.set("error".to_owned());
        } else {
            node_definition.save().expect("failed to save");
            self.error.set("".to_owned());
        }
    }
}
