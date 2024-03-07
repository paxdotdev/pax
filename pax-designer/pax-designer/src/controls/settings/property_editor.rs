use pax_engine::api::*;
use pax_engine::*;
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
    pub stid: Property<TypeId>,
    pub snid: Property<TemplateNodeId>,
    pub error: Property<String>,
}

impl PropertyEditor {
    pub fn on_render(&mut self, _ctx: &NodeContext) {
        if &self.definition.get().string != self.last_definition.get() {
            self.last_definition
                .set(self.definition.get().string.clone());
            self.textbox.set(self.definition.get().string.clone());
            self.error.set("".to_owned());
        }
    }

    pub fn text_input(&mut self, _ctx: &NodeContext, args: Event<TextboxInput>) {
        self.textbox.set(args.text.to_owned());
    }

    pub fn text_change(&mut self, ctx: &NodeContext, args: Event<TextboxChange>) {
        self.textbox.set(args.text.to_owned());
        let name = &self.name.get().string;
        let mut dt = ctx.designtime.borrow_mut();
        let Some(mut node_definition) =
            dt.get_orm_mut()
                .get_node(UniqueTemplateNodeIdentifier::build(
                    self.stid.get().clone(),
                    self.snid.get().clone(),
                ))
        else {
            return;
        };

        let variable = name.strip_suffix(':').unwrap_or(&name);
        if let Err(_error) = node_definition.set_property(variable, &args.text) {
            self.error.set("error".to_owned());
        } else {
            node_definition.save().expect("failed to save");
            self.error.set("".to_owned());
        }
    }
}
