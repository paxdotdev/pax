use pax_lang::api::*;
use pax_lang::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

pub mod property_editor;
use property_editor::PropertyEditor;

#[derive(Pax)]
#[file("controls/settings/settings.pax")]
pub struct Settings {
    pub selected_component_name: Property<String>,
    pub custom_props: Property<Vec<PropertyDef>>,
}

#[derive(Pax)]
#[custom(Imports)]
pub struct PropertyDef {
    pub name: StringBox,
}

impl Settings {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.selected_component_name.set("ELLIPSE 1".to_owned());
        self.custom_props.set(vec![
            PropertyDef {
                name: StringBox::from("Stroke".to_owned()),
            },
            PropertyDef {
                name: StringBox::from("Fill".to_owned()),
            },
        ]);
    }
}
