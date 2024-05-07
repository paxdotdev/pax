use std::collections::HashMap;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering;
use std::sync::Mutex;

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
    // custom props
    pub custom_props: Property<Vec<PropertyArea>>,
    pub update_timer: Property<i32>,

    // selected template type id
    pub stid: Property<TypeId>,
    // selected template node id
    pub snid: Property<TemplateNodeId>,
}

#[pax]
#[custom(Imports)]
// #[derive(Debug)]
pub struct PropertyArea {
    pub vertical_space: f64,
    pub vertical_pos: f64,
    pub name: StringBox,
    pub name_friendly: StringBox,
}

// #[derive(Debug)]
pub struct AreaMsg {
    pub index: usize,
    pub vertical_space: f64,
}

const SPACING: f64 = 10.0;
pub static REQUEST_PROPERTY_AREA_CHANNEL: Mutex<Option<Vec<AreaMsg>>> = Mutex::new(None);

impl Settings {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {}

    pub fn pre_render(&mut self, ctx: &NodeContext) {
        model::read_app_state(|app_state| {
            if app_state.selected_template_node_ids.get().len() != 1 {
                self.is_component_selected.set(false);
                return;
            }
            self.is_component_selected.set(true);

            let temp_node_id = app_state.selected_template_node_ids.get()[0].clone();
            //TODOdag
            let type_id = app_state.selected_component_id.get().clone();

            let update = self.stid.get() != type_id || self.snid.get() != temp_node_id;

            self.stid.set(type_id.clone());
            self.snid.set(temp_node_id.clone());

            if !update {
                return;
            }

            let uni = UniqueTemplateNodeIdentifier::build(type_id.clone(), temp_node_id.clone());
            let mut dt = ctx.designtime.borrow_mut();
            if let Some(node) = dt
                .get_orm()
                .get_component(&type_id)
                .ok()
                .and_then(|c| c.template.as_ref())
                .and_then(|t| t.get_node(&temp_node_id))
            {
                self.selected_component_name.set(
                    node.type_id
                        .get_pascal_identifier()
                        .unwrap_or_else(|| String::new())
                        .to_uppercase()
                        .to_owned(),
                );
            } else {
                self.selected_component_name.set("".to_owned())
            }
            let Some(mut node) = dt.get_orm_mut().get_node(uni) else {
                return;
            };
            let props = node.get_all_properties();

            let mut custom_props = vec![];
            for (propdef, _) in props {
                //ignore common props
                match propdef.name.as_str() {
                    "x" | "y" | "width" | "height" | "rotate" | "scale_x" | "scale_y"
                    | "anchor_x" | "anchor_y" | "skew_x" | "skew_y" | "id" | "transform" => (),
                    custom => custom_props.push(PropertyArea {
                        //these will be overridden by messages being passed to this component
                        vertical_space: 10.0,
                        vertical_pos: Default::default(),
                        name_friendly: StringBox::from(Self::camel_to_title_case(custom)),
                        name: StringBox::from(custom),
                    }),
                };
            }
            self.custom_props.set(custom_props);
        });

        let timer = self.update_timer.get();
        if timer > 0 {
            let mut custom_props = self.custom_props.get();
            // HACK: pre-double-binding handle messages from children specifying their requested height
            {
                let mut msgs = REQUEST_PROPERTY_AREA_CHANNEL.lock().unwrap();

                if let Some(msgs) = msgs.as_mut() {
                    msgs.retain(|msg| {
                        if let Some(area) = custom_props.get_mut(msg.index) {
                            area.vertical_space = msg.vertical_space;
                            false
                        } else {
                            true
                        }
                    });
                }
            }
            let mut running_sum = 0.0;
            for area in &mut custom_props {
                area.vertical_pos = running_sum;
                running_sum += area.vertical_space + SPACING;
            }
            if timer == 1 {
                //trigger for loop refresh
                custom_props.pop();
            }
            self.update_timer.set(timer + 1);
            self.custom_props.set(custom_props);
        }
    }

    fn camel_to_title_case(s: &str) -> String {
        s.char_indices().fold(String::new(), |mut acc, (i, c)| {
            if i == 0 || s.chars().nth(i - 1).unwrap() == '_' {
                acc.push_str(&c.to_uppercase().to_string());
            } else if c == '_' {
                acc.push_str(" ");
            } else {
                acc.push(c);
            }
            acc
        })
    }
}
