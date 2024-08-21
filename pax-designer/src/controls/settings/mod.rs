use pax_engine::api::*;
use pax_engine::*;
use pax_manifest::*;
use std::collections::HashMap;

use crate::model;
use pax_std::*;

pub mod color_picker;
pub mod property_editor;
use property_editor::PropertyEditor;

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/settings/mod.pax")]
pub struct Settings {
    pub is_component_selected: Property<bool>,
    pub selected_component_name: Property<String>,
    pub custom_properties: Property<Vec<PropertyArea>>,
    pub custom_properties_total_height: Property<f64>,
    pub stid: Property<TypeId>,
    pub snid: Property<TemplateNodeId>,
}

#[pax]
#[engine_import_path("pax_engine")]
#[custom(Imports)]
pub struct PropertyArea {
    pub vertical_space: f64,
    pub vertical_pos: f64,
    pub name: String,
    pub name_friendly: String,
    pub index: usize,
}

const SPACING: f64 = 10.0;
thread_local! {
    pub static AREAS_PROP: Property<Vec<f64>> = Property::new(Vec::new());
}

impl Settings {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        model::read_app_state(|app_state| {
            self.bind_is_component_selected(&app_state);
            self.bind_snid(&app_state);
            self.bind_stid(&app_state);
            self.bind_custom_properties(ctx);
            self.bind_custom_properties_total_height();
        });
    }

    fn bind_is_component_selected(&mut self, app_state: &model::AppState) {
        let stnids = app_state.selected_template_node_ids.clone();
        let deps = [stnids.untyped()];
        self.is_component_selected.replace_with(Property::computed(
            move || stnids.read(|ids| ids.len()) == 1,
            &deps,
        ));
    }

    fn bind_snid(&mut self, app_state: &model::AppState) {
        let stnids = app_state.selected_template_node_ids.clone();
        let deps = [stnids.untyped()];
        self.snid.replace_with(Property::computed(
            move || stnids.read(|ids| ids.get(0).cloned().unwrap_or(TemplateNodeId::build(0))),
            &deps,
        ));
    }

    fn bind_stid(&mut self, app_state: &model::AppState) {
        let scid = app_state.selected_component_id.clone();

        let deps = [scid.untyped()];
        self.stid
            .replace_with(Property::computed(move || scid.get(), &deps));
    }

    fn bind_custom_properties(&mut self, ctx: &NodeContext) {
        let stid = self.stid.clone();
        let snid = self.snid.clone();
        let selected_component_name = self.selected_component_name.clone();

        // Custom prop definitions excluding size. Size is determined by the property editor
        // itself and send back to this component through the AREAS_PROP signal prop.
        let custom_props_default_position = self.compute_custom_props_default_position(
            ctx.clone(),
            stid.clone(),
            snid.clone(),
            selected_component_name,
        );
        // Here, AREAS_PROP and the above custom prop definition is being combined to what
        // actually gets rendered
        let areas = AREAS_PROP.with(|p| p.clone());
        let adjusted_custom_props =
            self.adjust_custom_props_positions(custom_props_default_position, areas);

        self.custom_properties.replace_with(adjusted_custom_props);
    }

    fn bind_custom_properties_total_height(&mut self) {
        let custom_props = self.custom_properties.clone();
        let deps = [custom_props.untyped()];
        self.custom_properties_total_height
            .replace_with(Property::computed(
                move || {
                    let cp = custom_props.get();
                    let l = cp.into_iter().next().unwrap_or(PropertyArea::default());
                    l.vertical_pos + l.vertical_space
                },
                &deps,
            ));
    }

    fn compute_custom_props_default_position(
        &self,
        ctx: NodeContext,
        stid: Property<TypeId>,
        snid: Property<TemplateNodeId>,
        selected_component_name: Property<String>,
    ) -> Property<Vec<PropertyArea>> {
        let deps = [stid.untyped(), snid.untyped()];
        Property::computed(
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
                    selected_component_name.set("".to_owned())
                }

                let Some(mut node) = dt.get_orm_mut().get_node(uni, false) else {
                    return vec![];
                };

                let props = node.get_all_properties();
                props
                    .into_iter()
                    .enumerate()
                    .filter_map(|(i, (propdef, _))| match propdef.name.as_str() {
                        "x" | "y" | "width" | "height" | "rotate" | "scale_x" | "scale_y"
                        | "anchor_x" | "anchor_y" | "skew_x" | "skew_y" | "id" | "transform" => {
                            None
                        }
                        custom => (!custom.starts_with('_')).then_some(PropertyArea {
                            index: i + 1,
                            vertical_space: 10.0,
                            vertical_pos: Default::default(),
                            name_friendly: Self::camel_to_title_case(custom),
                            name: String::from(custom),
                        }),
                    })
                    .collect()
            },
            &deps,
        )
    }

    fn adjust_custom_props_positions(
        &self,
        custom_props: Property<Vec<PropertyArea>>,
        areas: Property<Vec<f64>>,
    ) -> Property<Vec<PropertyArea>> {
        let deps = [custom_props.untyped(), areas.untyped()];
        Property::computed(
            move || {
                let mut adjusted_props = custom_props.get();
                let areas = areas.get();
                let mut running_sum = 0.0;
                for prop in &mut adjusted_props {
                    let area = areas.get(prop.index - 1).unwrap_or(&10.0);
                    prop.vertical_space = *area;
                    prop.vertical_pos = running_sum;
                    running_sum += area + SPACING;
                }
                adjusted_props.into_iter().rev().collect()
            },
            &deps,
        )
    }

    fn camel_to_title_case(s: &str) -> String {
        s.char_indices().fold(String::new(), |mut acc, (i, c)| {
            if i == 0 || s.chars().nth(i - 1).unwrap() == '_' {
                acc.push_str(&c.to_uppercase().to_string());
            } else if c == '_' {
                acc.push(' ');
            } else {
                acc.push(c);
            }
            acc
        })
    }
}
