use std::collections::HashMap;

use pax_engine::api::*;
use pax_engine::*;
use pax_manifest::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
use serde::Deserialize;

pub mod property_editor;
use property_editor::PropertyEditor;

use crate::model;

#[pax]
#[file("controls/settings/mod.pax")]
pub struct Settings {
    pub is_component_selected: Property<bool>,
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
    pub stid: Property<TypeId>,
    // selected template node id
    pub snid: Property<TemplateNodeId>,
}

#[pax]
#[custom(Imports)]
pub struct PropertyDef {
    pub name: StringBox,
    pub definition: StringBox,
}

impl Settings {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {}

    pub fn pre_render(&mut self, ctx: &NodeContext) {
        model::read_app_state(|app_state| {
            let Some(ref temp_node_id) = app_state.selected_template_node_id else {
                self.is_component_selected.set(false);
                return;
            };
            let type_id = app_state.selected_component_id.clone();

            self.is_component_selected.set(true);

            self.stid.set(type_id.clone());
            self.snid.set(temp_node_id.clone());

            let props = ctx
                .designtime
                .borrow_mut()
                .get_orm_mut()
                .get_node(UniqueTemplateNodeIdentifier::build(
                    type_id.clone(),
                    temp_node_id.clone(),
                ))
                .get_all_properties();

            let mut custom_props = vec![];
            for (propdef, value) in props {
                let str_value: String = value
                    .map(|v| match v {
                        ValueDefinition::LiteralValue(Token { raw_value, .. })
                        | ValueDefinition::Expression(Token { raw_value, .. }, _)
                        | ValueDefinition::Identifier(Token { raw_value, .. }, _) => raw_value,
                        _ => "ERROR: UNSUPPORTED BINDING TYPE".to_owned(),
                    })
                    .unwrap_or("".to_string());
                match propdef.name.as_str() {
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
            self.selected_component_name.set(
                type_id
                    .get_pascal_identifier()
                    .unwrap_or_else(|| String::from("UNDEFINED"))
                    .to_uppercase()
                    .to_owned(),
            );
        });
    }
}
