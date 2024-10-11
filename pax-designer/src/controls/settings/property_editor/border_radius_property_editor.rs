use pax_engine::*;
use pax_engine::{api::*, pax_manifest::utils::parse_value};
use pax_manifest::*;
use pax_std::*;

use super::{PropertyAreas, PropertyEditorData};

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/settings/property_editor/border_radius_property_editor.pax")]
pub struct BorderRadiusPropertyEditor {
    pub data: Property<PropertyEditorData>,

    // All the below props should be private: never set by user, used for internal state
    pub last_definition: Property<String>,
    pub textbox: Property<String>,
    pub error: Property<String>,
}

impl BorderRadiusPropertyEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let index = self.data.get().editor_index;
        if index != 0 {
            let _ = ctx.peek_local_store(|PropertyAreas(areas): &mut PropertyAreas| {
                areas.update(|areas| {
                    while areas.len() <= index {
                        areas.push(0.0)
                    }
                    areas[index - 1] = 40.0;
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
                // TODO assert is number?
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
        // TODO parse a int instead, and wrap in PaxValue directly?
        let value = match parse_value(&args.text) {
            Ok(value) => value,
            Err(e) => {
                log::warn!("failed to parse value in border radius editor: {e}");
                return;
            }
        };
        if let Err(_error) = self.data.get().set_value(ctx, value) {
            self.error.set("error".to_owned());
        } else {
            self.error.set("".to_owned());
        }
    }
}
