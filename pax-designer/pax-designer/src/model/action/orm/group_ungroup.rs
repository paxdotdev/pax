use std::any::Any;

use crate::{
    math::{IntoInversionConfiguration, InversionConfiguration},
    model::{
        action::{orm::group_ungroup, Action, ActionContext, CanUndo},
        tools::SelectNodes,
        RuntimeNodeInfo, SelectionStateSnapshot,
    },
    ROOT_PROJECT_ID, USERLAND_EDIT_ID,
};
use anyhow::{anyhow, Context, Result};
use pax_engine::{layout::TransformAndBounds, log, math::Transform2};
use pax_manifest::{
    NodeLocation, TreeIndexPosition, TreeLocation, TypeId, UniqueTemplateNodeIdentifier,
};
use pax_runtime_api::borrow_mut;
use pax_std::primitives::Group;

use super::SetNodeTransformProperties;

pub struct GroupSelected {}

impl Action for GroupSelected {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let selected: SelectionStateSnapshot = (&ctx.selection_state()).into();

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
            .get_nodes_by_id(USERLAND_EDIT_ID)
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

        let group_location = NodeLocation::parent(
            group_creation_save_data
                .unique_id
                .get_containing_component_type_id(),
            group_creation_save_data.unique_id.get_template_node_id(),
        );

        // -------- Move the nodes to the newly created group ------------
        let _move_selected_into_group_command_ids = {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let mut command_ids = vec![];
            for node in &selected.items {
                let cmd_id = dt
                    .get_orm_mut()
                    .move_node(node.id.clone(), group_location.clone());
                command_ids.push(cmd_id);
            }
            command_ids
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
        ctx.execute(SetNodeTransformProperties {
            id: group_creation_save_data.unique_id.clone(),
            transform_and_bounds: group_transform_and_bounds,
            parent_transform_and_bounds: TransformAndBounds {
                transform: ctx.glass_transform().get() * group_parent_transform.transform,
                bounds: group_parent_transform.bounds,
            },
            inv_config: InversionConfiguration::default(),
        })?;

        // ---------- Reposition the children relative to the newly created group
        for node in selected.items {
            ctx.execute(SetNodeTransformProperties {
                id: node.id.clone(),
                transform_and_bounds: node.transform_and_bounds,
                parent_transform_and_bounds: group_transform_and_bounds,
                inv_config: node.layout_properties.into_inv_config(),
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
        let selected: SelectionStateSnapshot = (&ctx.selection_state()).into();
        let userland_proj_uid = ctx
            .engine_context
            .get_nodes_by_id(USERLAND_EDIT_ID)
            .first()
            .unwrap()
            .global_id();

        for group in selected.items {
            let parent = ctx
                .engine_context
                .get_nodes_by_global_id(group.id.clone())
                .first()
                .unwrap()
                .parent()
                .unwrap();

            let parent_id = parent.global_id();
            let group_parent_bounds = parent.transform_and_bounds().get();
            let new_location = if userland_proj_uid == parent_id {
                NodeLocation::root(group.id.get_containing_component_type_id())
            } else {
                NodeLocation::parent(
                    group.id.get_containing_component_type_id(),
                    parent_id.clone().unwrap().get_template_node_id(),
                )
            };
            let group_children = {
                let mut dt = borrow_mut!(ctx.engine_context.designtime);
                let group_children = dt
                    .get_orm_mut()
                    .get_node_children(group.id.clone())
                    .map_err(|e| anyhow!("group not found {:?}", e))?;

                for child in group_children.iter() {
                    dt.get_orm_mut()
                        .move_node(child.clone(), new_location.clone())
                        .map_err(|e| anyhow!("couldn't move child node {:?}", e))?;
                }
                dt.get_orm_mut()
                    .remove_node(group.id)
                    .map_err(|e| anyhow!("failed to remove group {:?}", e))?;
                group_children
            };
            for child in group_children {
                let child_runtime_node = ctx
                    .engine_context
                    .get_nodes_by_global_id(child.clone())
                    .first()
                    .cloned()
                    .unwrap();
                let child_t_and_b = child_runtime_node.transform_and_bounds().get();

                ctx.execute(SetNodeTransformProperties {
                    id: child,
                    transform_and_bounds: child_t_and_b,
                    parent_transform_and_bounds: group_parent_bounds,
                    inv_config: child_runtime_node.layout_properties().into_inv_config(),
                })?;
            }
        }

        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}

pub struct MoveNodeKeepScreenPos<'a> {
    node: &'a UniqueTemplateNodeIdentifier,
    new_parent: &'a UniqueTemplateNodeIdentifier,
    index: TreeIndexPosition,
}

impl Action for MoveNodeKeepScreenPos<'_> {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let parent_location = if ctx
            .engine_context
            .get_nodes_by_id(USERLAND_EDIT_ID)
            .first()
            .unwrap()
            .global_id()
            == Some(self.new_parent.clone())
        {
            NodeLocation::root(self.node.get_containing_component_type_id())
        } else {
            NodeLocation::parent(
                self.node.get_containing_component_type_id(),
                self.new_parent.get_template_node_id(),
            )
        };
        {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let _undo_id = dt
                .get_orm_mut()
                .move_node(self.node.clone(), parent_location)
                .map_err(|e| anyhow!("couldn't move child node {:?}", e))?;
        }
        let t_and_b_node = ctx
            .engine_context
            .get_nodes_by_global_id(self.node.clone())
            .first()
            .ok_or_else(|| anyhow!("child not in render tree"))?
            .transform_and_bounds()
            .get();
        let t_and_b_parent = ctx
            .engine_context
            .get_nodes_by_global_id(self.new_parent.clone())
            .first()
            .ok_or_else(|| anyhow!("parent not in render tree"))?
            .transform_and_bounds()
            .get();
        ctx.execute(SetNodeTransformProperties {
            id: self.node.clone(),
            transform_and_bounds: child_t_and_b,
            parent_transform_and_bounds: group_parent_bounds,
            inv_config: child_runtime_node.layout_properties().into_inv_config(),
        })?;
        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}
