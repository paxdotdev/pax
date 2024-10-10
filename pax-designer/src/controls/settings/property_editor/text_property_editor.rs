use pax_engine::pax_manifest::utils::parse_value;
use pax_engine::*;
use pax_engine::{api::*, pax_manifest::parsing::parse_value_definition};
use pax_manifest::*;
use pax_std::*;

use super::{PropertyAreas, PropertyEditorData};

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/settings/property_editor/text_property_editor.pax")]
pub struct TextPropertyEditor {
    pub data: Property<PropertyEditorData>,

    // All the below props should be private: never set by user, used for internal state
    pub last_definition: Property<String>,
    pub textbox: Property<String>,
    pub error: Property<String>,
}

impl TextPropertyEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let index = self.data.get().editor_index;
        if index != 0 {
            let _ = ctx.peek_local_store(|PropertyAreas(areas): &mut PropertyAreas| {
                areas.update(|areas| {
                    while areas.len() <= index {
                        areas.push(0.0)
                    }
                    areas[index - 1] = 75.0;
                });
            });
        }
        let data = self.data.clone();
        let deps = [data.untyped()];
        let ctx = ctx.clone();
        let err = self.error.clone();
        self.textbox.replace_with(Property::computed(
            move || {
                err.set("".to_string());
                data.get()
                    .get_value(&ctx)
                    .map(|v| v.to_string())
                    .unwrap_or_default()
            },
            &deps,
        ));
    }

    pub fn text_input(&mut self, _ctx: &NodeContext, args: Event<TextboxInput>) {
        self.textbox.set(args.text.to_owned());
    }

    pub fn text_change(&mut self, ctx: &NodeContext, args: Event<TextboxChange>) {
        self.textbox.set(args.text.to_owned());
        let value_definition = match parse_value(&args.text) {
            Ok(value) => value,
            Err(e) => {
                log::warn!("failed to parse textbox value: {e}");
                return;
            }
        };
        if let Err(_error) = self.data.get().set_value(ctx, Some(value_definition)) {
            self.error.set("error".to_owned());
        } else {
            self.error.set("".to_owned());
        }
    }
}
