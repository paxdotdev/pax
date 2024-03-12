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
#[derive(Debug)]
pub struct PropertyArea {
    pub vertical_space: f64,
    pub vertical_pos: f64,
    pub name: StringBox,
}

#[derive(Debug)]
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
            if app_state.selected_template_node_ids.len() != 1 {
                self.is_component_selected.set(false);
                return;
            }
            self.is_component_selected.set(true);

            let temp_node_id = app_state.selected_template_node_ids[0].clone();
            let type_id = app_state.selected_component_id.clone();

            let update = self.stid.get() != &type_id || self.snid.get() != &temp_node_id;

            self.stid.set(type_id.clone());
            self.snid.set(temp_node_id.clone());

            if !update {
                return;
            }

            let uni = UniqueTemplateNodeIdentifier::build(type_id.clone(), temp_node_id.clone());
            let mut dt = ctx.designtime.borrow_mut();
            let Some(mut node) = dt.get_orm_mut().get_node(uni) else {
                return;
            };
            let props = node.get_all_properties();

            let mut custom_props = vec![];
            for (propdef, _) in props {
                //ignore common props
                match propdef.name.as_str() {
                    "x" | "y" | "width" | "height" | "rotate" | "scale_x" | "scale_y"
                    | "anchor_x" | "anchor_y" | "skew_x" | "skew_y" => (),
                    custom => custom_props.push(PropertyArea {
                        //these will be overridden by messages being passed to this component
                        vertical_space: 10.0,
                        vertical_pos: Default::default(),
                        name: StringBox::from(custom),
                    }),
                };
            }
            self.custom_props.set(custom_props);
            self.selected_component_name.set(
                type_id
                    .get_pascal_identifier()
                    .unwrap_or_else(|| String::from("UNDEFINED"))
                    .to_uppercase()
                    .to_owned(),
            );

            // Setup for waiting for children to send updates about their size
            self.update_timer.set(2);
            self.custom_props.get_mut().push(PropertyArea {
                vertical_space: 0.0,
                vertical_pos: f64::MAX,
                name: StringBox::from("".to_owned()),
            });
        });

        let timer = self.update_timer.get_mut();
        if *timer > 0 {
            // HACK: pre-double-binding handle messages from children specifying their requested height
            {
                let mut msgs = REQUEST_PROPERTY_AREA_CHANNEL.lock().unwrap();

                if let Some(msgs) = msgs.as_mut() {
                    msgs.retain(|msg| {
                        if let Some(area) = self.custom_props.get_mut().get_mut(msg.index) {
                            area.vertical_space = msg.vertical_space;
                            false
                        } else {
                            true
                        }
                    })
                }
            }
            let mut running_sum = 0.0;
            for area in self.custom_props.get_mut() {
                area.vertical_pos = running_sum;
                running_sum += area.vertical_space + SPACING;
            }
            if *timer == 1 {
                //trigger for loop refresh
                self.custom_props.get_mut().pop();
            }
            *timer -= 1;
        }
    }
}
