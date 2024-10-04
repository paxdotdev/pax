use pax_engine::api::*;
use pax_engine::*;
use pax_manifest::*;
use std::collections::HashMap;

use crate::{designer_node_type::DesignerNodeType, model};
use convert_case::{Case, Casing};
use pax_std::*;

use crate::controls::settings::property_editor::PropertyEditor;

use super::settings::property_editor::PropertyAreas;

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/class_settings_editor.pax")]
pub struct ClassSettingsEditor {
    pub custom_properties: Property<Vec<PropertyArea>>,
    pub custom_properties_total_height: Property<f64>,
    pub stid: Property<TypeId>,
    pub class_name: Property<String>,
    pub property_areas: Property<Vec<f64>>,
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

impl ClassSettingsEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.push_local_store(PropertyAreas(self.property_areas.clone()));
        model::read_app_state(|app_state| {
            self.bind_stid(&app_state);
            self.bind_class_name(&app_state);
            self.bind_class_properties(ctx);
            self.bind_custom_properties_total_height();
        });
    }

    fn bind_class_name(&mut self, app_state: &model::AppState) {
        let class_name = app_state.current_editor_class_name.clone();
        let deps = [class_name.untyped()];
        self.class_name
            .replace_with(Property::computed(move || class_name.get(), &deps));
    }

    fn bind_stid(&mut self, app_state: &model::AppState) {
        let scid = app_state.selected_component_id.clone();

        let deps = [scid.untyped()];
        self.stid
            .replace_with(Property::computed(move || scid.get(), &deps));
    }

    fn bind_class_properties(&mut self, ctx: &NodeContext) {
        let stid = self.stid.clone();
        let class_name = self.class_name.clone();

        // Custom prop definitions excluding size. Size is determined by the property editor
        // itself and send back to this component through the AREAS_PROP signal prop.
        let custom_props_default_position = self.compute_class_props_default_position(
            ctx.clone(),
            stid.clone(),
            class_name.clone(),
        );
        // Here, AREAS_PROP and the above custom prop definition is being combined to what
        // actually gets rendered
        let areas = self.property_areas.clone();
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

    fn compute_class_props_default_position(
        &self,
        ctx: NodeContext,
        stid: Property<TypeId>,
        class_name: Property<String>,
    ) -> Property<Vec<PropertyArea>> {
        let deps = [stid.untyped(), class_name.untyped()];
        Property::computed(
            move || {
                let dt = borrow_mut!(ctx.designtime);
                let props = match dt.get_orm().get_class(&stid.get(), &class_name.get()) {
                    Err(e) => {
                        log::warn!("failed to fetch class: {e}");
                        return vec![];
                    }
                    Ok(class_props) => class_props,
                };
                props
                    .into_iter()
                    .enumerate()
                    .map(|(i, (propdef, _, _))| PropertyArea {
                        index: i + 1,
                        vertical_space: 10.0,
                        vertical_pos: Default::default(),
                        name_friendly: propdef.clone(),
                        name: propdef,
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
                    prop.vertical_space = *area - 40.0;
                    prop.vertical_pos = running_sum;
                    running_sum += area + SPACING;
                }
                let res = adjusted_props.into_iter().rev().collect();
                res
            },
            &deps,
        )
    }
}
