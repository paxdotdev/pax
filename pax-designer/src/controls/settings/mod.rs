use pax_engine::api::*;
use pax_engine::*;
use pax_manifest::*;
use std::collections::HashMap;

use crate::{designer_node_type::DesignerNodeType, model};
use pax_std::*;

use convert_case::{Case, Casing};
pub mod color_picker;
pub mod control_flow_for_editor;
pub mod control_flow_if_editor;
pub mod property_editor;
use control_flow_for_editor::ControlFlowForEditor;
use control_flow_if_editor::ControlFlowIfEditor;
use property_editor::PropertyEditor;

use self::property_editor::{PropertyArea, PropertyAreas, WriteTarget};

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/settings/mod.pax")]
pub struct Settings {
    pub is_component_selected: Property<bool>,
    pub is_control_flow_if_selected: Property<bool>,
    pub is_control_flow_for_selected: Property<bool>,
    pub selected_component_name: Property<String>,
    pub custom_properties: Property<Vec<PropertyArea>>,
    pub custom_properties_total_height: Property<f64>,
    pub stid: Property<TypeId>,
    pub snid: Property<TemplateNodeId>,
    pub property_areas: Property<Vec<f64>>,
}

const SPACING: f64 = 10.0;

impl Settings {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.push_local_store(PropertyAreas(self.property_areas.clone()));
        model::read_app_state(|app_state| {
            self.bind_selected(&app_state, ctx);
            self.bind_snid(&app_state);
            self.bind_stid(&app_state);
            self.bind_custom_properties(ctx);
            self.bind_custom_properties_total_height();
        });
    }

    fn bind_selected(&mut self, app_state: &model::AppState, ctx: &NodeContext) {
        let stnids = app_state.selected_template_node_ids.clone();
        let comp_type_id = self.stid.clone();
        let deps = [stnids.untyped(), comp_type_id.untyped()];
        let ctx = ctx.clone();
        let node_type = Property::computed(
            move || {
                let Some(node_id) = stnids.read(|ids| (ids.len() == 1).then(|| ids[0].clone()))
                else {
                    return None;
                };
                let uni = UniqueTemplateNodeIdentifier::build(comp_type_id.get(), node_id);
                let mut dt = borrow_mut!(ctx.designtime);
                let orm = dt.get_orm_mut();
                let Some(node) = orm.get_node_builder(uni.clone(), false) else {
                    return Some(DesignerNodeType::Unregistered);
                };
                let node_type = DesignerNodeType::from_type_id(node.get_type_id());
                Some(node_type)
            },
            &deps,
        );

        let deps = [node_type.untyped()];
        let node_type_cp = node_type.clone();
        self.is_component_selected.replace_with(Property::computed(
            move || {
                let Some(node_type) = node_type_cp.get() else {
                    return false;
                };
                !matches!(
                    node_type,
                    DesignerNodeType::Conditional
                        | DesignerNodeType::Repeat
                        | DesignerNodeType::Slot
                )
            },
            &deps,
        ));
        let node_type_cp = node_type.clone();
        self.is_control_flow_if_selected
            .replace_with(Property::computed(
                move || {
                    let Some(node_type) = node_type_cp.get() else {
                        return false;
                    };
                    matches!(node_type, DesignerNodeType::Conditional)
                },
                &deps,
            ));
        let node_type_cp = node_type.clone();
        self.is_control_flow_for_selected
            .replace_with(Property::computed(
                move || {
                    let Some(node_type) = node_type_cp.get() else {
                        return false;
                    };
                    matches!(node_type, DesignerNodeType::Repeat)
                },
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
                            .to_owned(),
                    );
                } else {
                    selected_component_name.set("".to_owned())
                }

                let Some(mut node) = dt.get_orm_mut().get_node_builder(uni, false) else {
                    return vec![];
                };

                let props = node.get_all_properties();
                props
                    .into_iter()
                    .filter(|(prop_def, _)| {
                        !(matches!(
                            prop_def.name.as_str(),
                            "x" | "y"
                                | "width"
                                | "height"
                                | "rotate"
                                | "scale_x"
                                | "scale_y"
                                | "anchor_x"
                                | "anchor_y"
                                | "skew_x"
                                | "skew_y"
                                | "id"
                                | "transform"
                        ) || prop_def.name.starts_with('_'))
                    })
                    .enumerate()
                    .map(|(i, (propdef, _))| PropertyArea {
                        index: i + 1,
                        vertical_space: 10.0,
                        vertical_pos: Default::default(),
                        name_friendly: propdef.name.to_case(Case::Title),
                        name: String::from(propdef.name),
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
