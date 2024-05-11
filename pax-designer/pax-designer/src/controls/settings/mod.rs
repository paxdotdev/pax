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
    pub custom_properties: Property<Vec<PropertyArea>>,

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
    pub name: String,
    pub name_friendly: String,
}

const SPACING: f64 = 10.0;
thread_local! {
    pub static AREAS_PROP: Property<Vec<f64>> = Property::new(Vec::new());
}

impl Settings {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        model::read_app_state(|app_state| {
            let stnids = app_state.selected_template_node_ids.clone();
            let deps = [stnids.untyped()];
            self.is_component_selected.replace_with(Property::computed(
                move || stnids.read(|ids| ids.len()) == 1,
                &deps,
            ));

            let stnids = app_state.selected_template_node_ids.clone();
            self.snid.replace_with(Property::computed(
                move || stnids.read(|ids| ids.get(0).cloned().unwrap_or(TemplateNodeId::build(0))),
                &deps,
            ));

            let scid = app_state.selected_component_id.clone();
            let deps = [scid.untyped()];
            self.stid
                .replace_with(Property::computed(move || scid.get(), &deps));

            let stid = self.stid.clone();
            let snid = self.snid.clone();
            let ctx = ctx.clone();
            let comp_selected = self.is_component_selected.clone();
            let deps = [snid.untyped(), stid.untyped(), comp_selected.untyped()];
            let selected_component_name = self.selected_component_name.clone();
            let custom_props_default_position = Property::computed(
                move || {
                    let uni = UniqueTemplateNodeIdentifier::build(stid.get(), snid.get());
                    let mut dt = borrow_mut!(ctx.designtime);

                    if let Some(node) = dt
                        .get_orm()
                        .get_component(&stid.get())
                        .ok()
                        .and_then(|c| c.template.as_ref())
                        .and_then(|t| t.get_node(&snid.get()))
                    {
                        selected_component_name.set(
                            node.type_id
                                .get_pascal_identifier()
                                .unwrap_or_else(|| String::new())
                                .to_uppercase()
                                .to_owned(),
                        );
                    } else {
                        //does this work?
                        selected_component_name.set("".to_owned())
                    }
                    let Some(mut node) = dt.get_orm_mut().get_node(uni) else {
                        return vec![];
                    };
                    let props = node.get_all_properties();

                    let mut custom_props = vec![];
                    for (propdef, _) in props {
                        //ignore common props
                        match propdef.name.as_str() {
                            "x" | "y" | "width" | "height" | "rotate" | "scale_x" | "scale_y"
                            | "anchor_x" | "anchor_y" | "skew_x" | "skew_y" | "id"
                            | "transform" => (),
                            custom => custom_props.push(PropertyArea {
                                //these will be overridden by messages being passed to this component
                                vertical_space: 10.0,
                                vertical_pos: Default::default(),
                                name_friendly: String::from(Self::camel_to_title_case(custom)),
                                name: String::from(custom),
                            }),
                        };
                    }
                    custom_props
                },
                &deps,
            );
            let areas = AREAS_PROP.with(|p| p.clone());
            let deps = [custom_props_default_position.untyped(), areas.untyped()];
            self.custom_properties.replace_with(Property::computed(
                move || {
                    let mut custom_props = custom_props_default_position.get();
                    let mut running_sum = 0.0;
                    for (i, area) in areas.get().iter().take(custom_props.len()).enumerate() {
                        custom_props[i].vertical_space = *area;
                        custom_props[i].vertical_pos = running_sum;
                        running_sum += area + SPACING;
                    }
                    custom_props
                },
                &deps,
            ));
        });
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
