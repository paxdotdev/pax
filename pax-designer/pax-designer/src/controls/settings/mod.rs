use std::collections::HashMap;

use pax_lang::api::*;
use pax_lang::*;
use pax_manifest::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
use serde::Deserialize;

pub mod property_editor;
use property_editor::PropertyEditor;

#[pax]
#[file("controls/settings/settings.pax")]
pub struct Settings {
    pub component_selected: Property<bool>,
    pub selected_component_name: Property<String>,

    // common props
    pub pos_x: Property<StringBox>,
    pub pos_y: Property<StringBox>,
    pub size_width: Property<StringBox>,
    pub size_height: Property<StringBox>,
    pub rotation_z: Property<StringBox>,
    pub scale_x: Property<StringBox>,
    pub scale_y: Property<StringBox>,
    pub anchor_x: Property<StringBox>,
    pub anchor_y: Property<StringBox>,
    pub skew_x: Property<StringBox>,
    pub skew_y: Property<StringBox>,

    // custom props
    pub custom_props: Property<Vec<PropertyDef>>,

    // selected template type id
    pub stid: Property<StringBox>,
    // selected template node id
    pub snid: Property<Numeric>,
}

#[pax]
#[custom(Imports)]
pub struct PropertyDef {
    pub name: StringBox,
    pub definition: StringBox,
}

impl Settings {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {}

    pub fn set_object1(&mut self, _ctx: &NodeContext, _args: ArgsButtonClick) {
        self.component_selected.set(true);
        self.stid.set(StringBox::from(
            "crate::controls::settings::Settings".to_owned(),
        ));
        self.snid.set(5.into());
    }

    pub fn set_object2(&mut self, _ctx: &NodeContext, _args: ArgsButtonClick) {
        self.component_selected.set(true);
        self.stid.set(StringBox::from(
            "crate::controls::settings::property_editor::PropertyEditor".to_owned(),
        ));
        self.snid.set(1.into());
    }

    pub fn set_object3(&mut self, _ctx: &NodeContext, _args: ArgsButtonClick) {
        self.component_selected.set(true);
        self.stid
            .set(StringBox::from("crate::controls::tree::Tree".to_owned()));
        self.snid.set(1.into());
    }

    pub fn pre_render(&mut self, ctx: &NodeContext) {
        if *self.component_selected.get() {
            self.set_to_selected(ctx);
        }
    }

    pub fn set_to_selected(&mut self, ctx: &NodeContext) {
        let type_id = &self.stid.get().string;
        let temp_node_id = self.snid.get().get_as_int() as usize;
        let (properties, type_name) = ctx
            .designtime
            .borrow_mut()
            .get_orm_mut()
            .get_node(type_id, temp_node_id)
            .get_property_definitions()
            .expect("selected node has properties");

        // log(&format!("{:#?}", properties));
        let mut custom_props = vec![];
        for (value, name, _type_id) in properties {
            let str_value: String = value
                .map(|v| match v {
                    ValueDefinition::LiteralValue(Token { raw_value, .. })
                    | ValueDefinition::Expression(Token { raw_value, .. }, _)
                    | ValueDefinition::Identifier(Token { raw_value, .. }, _) => raw_value,
                    _ => "ERROR: UNSUPPORTED BINDING TYPE".to_owned(),
                })
                .unwrap_or("".to_string());
            match name.as_str() {
                "x" => self.pos_x.set(StringBox::from(str_value)),
                "y" => self.pos_y.set(StringBox::from(str_value)),
                "width" => self.size_width.set(StringBox::from(str_value)),
                "height" => self.size_height.set(StringBox::from(str_value)),
                "rotate" => self.rotation_z.set(StringBox::from(str_value)),
                "scale_x" => self.scale_x.set(StringBox::from(str_value)),
                "scale_y" => self.scale_y.set(StringBox::from(str_value)),
                "anchor_x" => self.anchor_x.set(StringBox::from(str_value)),
                "anchor_y" => self.anchor_y.set(StringBox::from(str_value)),
                "skew_x" => self.skew_x.set(StringBox::from(str_value)),
                "skew_y" => self.skew_y.set(StringBox::from(str_value)),
                custom => custom_props.push(PropertyDef {
                    name: StringBox::from(custom),
                    definition: StringBox::from(str_value),
                }),
            }
        }
        self.custom_props.set(custom_props);
        let (_, name) = type_name.rsplit_once("::").unwrap_or(("", &type_name));
        self.selected_component_name.set(name.to_owned());
    }
}
