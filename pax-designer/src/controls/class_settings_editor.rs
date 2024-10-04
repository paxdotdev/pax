use anyhow::anyhow;
use pax_engine::api::*;
use pax_engine::*;
use pax_manifest::*;
use std::collections::HashMap;

use crate::{designer_node_type::DesignerNodeType, model};
use convert_case::{Case, Casing};
use pax_std::*;

use crate::controls::settings::property_editor::PropertyEditor;

use super::settings::property_editor::{PropertyArea, PropertyAreas};

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/class_settings_editor.pax")]
pub struct ClassSettingsEditor {
    pub class_properties: Property<Vec<PropertyArea>>,
    pub class_properties_total_height: Property<f64>,
    pub stid: Property<TypeId>,
    pub class_name: Property<String>,
    pub all_available_properties: Property<Vec<String>>,
    pub selected_property_index: Property<Option<usize>>,
    pub new_property_value: Property<String>,
    pub property_areas: Property<Vec<f64>>,
}

const SPACING: f64 = 10.0;

impl ClassSettingsEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.push_local_store(PropertyAreas(self.property_areas.clone()));
        model::read_app_state(|app_state| {
            self.bind_stid(&app_state);
            self.bind_class_name(&app_state);
            self.bind_class_properties(ctx);
            self.bind_class_properties_total_height();
            self.bind_all_available_properties(ctx);
        });
    }

    fn bind_class_name(&mut self, app_state: &model::AppState) {
        let class_name = app_state.current_editor_class_name.clone();
        let deps = [class_name.untyped()];
        self.class_name.replace_with(Property::computed(
            move || class_name.get().unwrap_or_else(|| "error".to_string()),
            &deps,
        ));
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

        // Class prop definitions excluding size. Size is determined by the property editor
        // itself and send back to this component through the AREAS_PROP signal prop.
        let class_props_default_position = self.compute_class_props_default_position(
            ctx.clone(),
            stid.clone(),
            class_name.clone(),
        );
        // Here, AREAS_PROP and the above class prop definition is being combined to what
        // actually gets rendered
        let areas = self.property_areas.clone();
        let adjusted_class_props =
            self.adjust_class_props_positions(class_props_default_position, areas);

        self.class_properties.replace_with(adjusted_class_props);
    }

    fn bind_all_available_properties(&mut self, ctx: &NodeContext) {
        let dt = borrow!(ctx.designtime);
        let orm = dt.get_orm();
        let class_props = self.class_properties.clone();
        let deps = [orm.get_manifest_version().untyped()];
        let ctx = ctx.clone();
        self.all_available_properties
            .replace_with(Property::computed(
                move || {
                    let dt = borrow!(ctx.designtime);
                    let orm = dt.get_orm();
                    let mut properties: Vec<_> = orm
                        .get_all_property_definitions()
                        .iter()
                        .map(|pd| pd.name.clone())
                        .collect();
                    properties.sort();
                    properties.dedup();
                    class_props.read(|class_props| {
                        properties.retain(|v| {
                            !(v.starts_with("_") || class_props.iter().any(|c| &c.name == v))
                        });
                    });
                    properties
                },
                &deps,
            ));
    }

    fn bind_class_properties_total_height(&mut self) {
        let class_props = self.class_properties.clone();
        let deps = [class_props.untyped()];
        self.class_properties_total_height
            .replace_with(Property::computed(
                move || {
                    let cp = class_props.get();
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
        let manifest_ver = borrow!(ctx.designtime).get_manifest_version();
        let deps = [stid.untyped(), class_name.untyped(), manifest_ver.untyped()];
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

    fn adjust_class_props_positions(
        &self,
        class_props: Property<Vec<PropertyArea>>,
        areas: Property<Vec<f64>>,
    ) -> Property<Vec<PropertyArea>> {
        let deps = [class_props.untyped(), areas.untyped()];
        Property::computed(
            move || {
                let mut adjusted_props = class_props.get();
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

    pub fn close_class_editor(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        model::read_app_state(|app_state| {
            app_state.current_editor_class_name.set(None);
        });
    }
    pub fn add_class_property(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        let Some(selected) = self.selected_property_index.get() else {
            log::warn!("need to select property to add to class");
            return;
        };
        let key = self
            .all_available_properties
            .read(|all| all[selected].clone());
        let value = self.new_property_value.get();
        let t = model::with_action_context(ctx, |ac| ac.transaction("add class property"));
        let _ = t.run(|| {
            let mut dt = borrow_mut!(ctx.designtime);
            let orm = dt.get_orm_mut();
            let class_name = self.class_name.get();
            let mut builder = orm.get_class_builder(self.stid.get(), &class_name);
            builder.set_property(&key, &value)?;
            builder
                .save()
                .map_err(|e| anyhow!("couldn't add property to class: {e}"))?;
            self.new_property_value.set("".to_string());
            self.selected_property_index.set(None);
            Ok(())
        });
    }
}
