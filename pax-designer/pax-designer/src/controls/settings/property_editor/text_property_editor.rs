use pax_engine::api::*;
use pax_engine::*;
use pax_manifest::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

use crate::controls::settings::AreaMsg;
use crate::controls::settings::REQUEST_PROPERTY_AREA_CHANNEL;

use super::PropertyEditorData;

#[pax]
#[file("controls/settings/property_editor/text_property_editor.pax")]
pub struct TextPropertyEditor {
    pub data: Property<PropertyEditorData>,

    // All the below props should be private: never set by user, used for internal state
    pub last_definition: Property<String>,
    pub textbox: Property<String>,
    pub error: Property<String>,
}

impl TextPropertyEditor {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        if let Some(index) = self.data.get().editor_index {
            let mut channel_guard = REQUEST_PROPERTY_AREA_CHANNEL.lock().unwrap();
            let channel = channel_guard.get_or_insert_with(Vec::new);
            channel.push(AreaMsg {
                index,
                vertical_space: 30.0,
            })
        }
    }

    pub fn on_render(&mut self, ctx: &NodeContext) {
        let value = self.data.get().get_value_as_str(ctx);
        if &value != self.last_definition.get() {
            self.last_definition.set(value.clone());
            self.textbox.set(value.clone());
            self.error.set("".to_owned());
        }
    }

    pub fn text_input(&mut self, _ctx: &NodeContext, args: Event<TextboxInput>) {
        self.textbox.set(args.text.to_owned());
    }

    pub fn text_change(&mut self, ctx: &NodeContext, args: Event<TextboxChange>) {
        self.textbox.set(args.text.to_owned());
        if let Err(_error) = self.data.get().set_value(ctx, &args.text) {
            self.error.set("error".to_owned());
        } else {
            self.error.set("".to_owned());
        }
    }
}
