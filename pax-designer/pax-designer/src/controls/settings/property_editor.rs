use pax_lang::api::*;
use pax_lang::*;
use pax_manifest::*;
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
    pub stid: Property<StringBox>,
    pub snid: Property<Numeric>,
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
        //this can be removed once we send changes to orm property? (might need both)
        //hope it updates in time.
        self.textbox.set(args.text.to_owned());
        let name = &self.name.get().string;
        let variable = name.strip_suffix(':').unwrap_or(&name);
        // log(&format!(
        //     "will set {}:{} prop <{}> to {} through orm",
        //     self.stid.get().string,
        //     self.snid.get().get_as_int(),
        //     variable,
        //     args.text
        // ));
        ctx.designtime.borrow_mut().set_template_node_setting(
            &self.stid.get().string,
            self.snid.get().get_as_int() as usize,
            variable,
            ValueDefinition::LiteralValue(Token::new_from_raw_value(
                args.text.clone(),
                TokenType::LiteralValue,
            )),
        );
    }
}
