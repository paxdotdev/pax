use std::any::Any;

use crate::{
    math::{DecompositionConfiguration, IntoDecompositionConfiguration},
    model::{
        action::{
            orm::{group_ungroup, SetNodeProperties},
            Action, ActionContext, CanUndo,
        },
        tools::SelectNodes,
        GlassNode, GlassNodeSnapshot, SelectionStateSnapshot,
    },
    ROOT_PROJECT_ID,
};
use anyhow::{anyhow, Context, Result};
use pax_engine::{
    layout::{LayoutProperties, TransformAndBounds},
    log,
    math::{Point2, Transform2},
    NodeInterface,
};
use pax_manifest::{
    NodeLocation, TreeIndexPosition, TreeLocation, TypeId, UniqueTemplateNodeIdentifier,
};
use pax_runtime_api::borrow_mut;
use pax_std::primitives::Group;

use super::{MoveNode, ResizeMode, SetNodePropertiesFromTransform};

pub struct GroupSelected {}

impl Action for GroupSelected {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let selected: SelectionStateSnapshot = (&ctx.derived_state.selection_state.get()).into();

        // ------------ Figure out the location the group should be at ---------
        let Some(root) = selected.items.first() else {
            return Err(anyhow!("nothing selected to group"));
        };
        let root_parent = ctx
            .derived_state
            .open_containers
            .read(|v| v.first().cloned());
        let group_parent_location = if ctx
            .engine_context
            .get_nodes_by_id(ROOT_PROJECT_ID)
            .first()
            .unwrap()
            .global_id()
            == root_parent
        {
            NodeLocation::root(root.id.get_containing_component_type_id())
        } else {
            NodeLocation::parent(
                root.id.get_containing_component_type_id(),
                root_parent.as_ref().unwrap().get_template_node_id(),
            )
        };

        // -------- Create a group ------------
        let group_creation_save_data = {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let mut builder = dt.get_orm_mut().build_new_node(
                ctx.app_state.selected_component_id.get().clone(),
                TypeId::build_singleton(
                    &format!("pax_designer::pax_reexports::pax_std::primitives::Group"),
                    None,
                ),
            );
            builder.set_location(group_parent_location);
            builder
                .save()
                .map_err(|e| anyhow!("could not save: {}", e))?
        };

        // --------- Position the newly created group -------------------
        let group_parent_data = ctx
            .engine_context
            .get_nodes_by_global_id(root_parent.unwrap())
            .first()
            .unwrap()
            .clone();
        let group_parent_transform = group_parent_data.transform_and_bounds().get();
        let group_transform_and_bounds = selected.total_bounds.as_pure_size().cast_spaces();

        let group_parent_transform_and_bounds = TransformAndBounds {
            transform: ctx.glass_transform().get() * group_parent_transform.transform,
            bounds: group_parent_transform.bounds,
        };

        ctx.execute(SetNodePropertiesFromTransform {
            id: group_creation_save_data.unique_id.clone(),
            transform_and_bounds: group_transform_and_bounds,
            parent_transform_and_bounds: group_parent_transform_and_bounds,
            decomposition_config: DecompositionConfiguration::default(),
        })?;

        // ---------- Move nodes into newly created group ----------
        for node in &selected.items {
            ctx.execute(MoveNode {
                node_id: &node.id,
                node_transform_and_bounds: &node.transform_and_bounds,
                new_parent_uid: &group_creation_save_data.unique_id,
                new_parent_transform_and_bounds: &group_transform_and_bounds,
                index: TreeIndexPosition::Bottom,
                resize_mode: ResizeMode::KeepScreenBounds,
                node_inv_config: node.layout_properties.into_decomposition_config(),
            })?;
        }

        // ---------- Select the newly created group -----
        ctx.execute(SelectNodes {
            ids: &[group_creation_save_data.unique_id.get_template_node_id()],
            overwrite: true,
        })?;

        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}

pub struct UngroupSelected {}

impl Action for UngroupSelected {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let selected: SelectionStateSnapshot = (&ctx.derived_state.selection_state.get()).into();
        for group in selected.items {
            let parent = ctx
                .engine_context
                .get_nodes_by_global_id(group.id.clone())
                .first()
                .unwrap()
                .template_parent()
                .unwrap();

            let parent_id = parent.global_id();
            let group_parent_bounds = parent.transform_and_bounds().get();

            let group_children = borrow_mut!(ctx.engine_context.designtime)
                .get_orm_mut()
                .get_node_children(group.id.clone())
                .map_err(|e| anyhow!("group not found {:?}", e))?;

            // ---------- Move Nodes to group parent --------------
            for child in group_children.iter().rev() {
                let child_runtime_node = ctx
                    .engine_context
                    .get_nodes_by_global_id(child.clone())
                    .first()
                    .cloned()
                    .unwrap();
                let child_inv_config = child_runtime_node
                    .layout_properties()
                    .into_decomposition_config();
                let child_t_and_b = child_runtime_node.transform_and_bounds().get();
                ctx.execute(MoveNode {
                    node_id: &child,
                    node_transform_and_bounds: &child_t_and_b,
                    new_parent_uid: parent_id.as_ref().unwrap(),
                    new_parent_transform_and_bounds: &group_parent_bounds,
                    node_inv_config: child_inv_config,
                    index: TreeIndexPosition::Top,
                    resize_mode: ResizeMode::KeepScreenBounds,
                })?;
            }

            // ---------- Delete group --------------
            borrow_mut!(ctx.engine_context.designtime)
                .get_orm_mut()
                .remove_node(group.id)
                .map_err(|e| anyhow!("failed to remove group {:?}", e))?;
        }

        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}
