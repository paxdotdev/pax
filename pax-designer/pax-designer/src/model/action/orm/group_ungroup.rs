use std::any::Any;

use crate::{
    math::InversionConfiguration,
    model::{
        action::{Action, ActionContext, CanUndo},
        RuntimeNodeInfo, SelectionStateSnapshot,
    },
    USERLAND_PROJECT_ID,
};
use anyhow::{anyhow, Context, Result};
use pax_engine::{layout::TransformAndBounds, log, math::Transform2};
use pax_manifest::{NodeLocation, TypeId};
use pax_runtime_api::borrow_mut;
use pax_std::primitives::Group;

use super::SetNodeTransformProperties;

pub struct GroupSelected {}

impl Action for GroupSelected {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let selected: SelectionStateSnapshot = (&ctx.selection_state()).into();
        let Some(root) = selected.items.first() else {
            return Err(anyhow!("nothing selected to group"));
        };
        let root_parent = ctx
            .derived_state
            .open_containers
            .read(|v| v.first().cloned());
        // this is the location the group will be created
        let group_location = if ctx
            .engine_context
            .get_nodes_by_id(USERLAND_PROJECT_ID)
            .first()
            .unwrap()
            .global_id()
            == root_parent
        {
            NodeLocation::root(root.id.get_containing_component_type_id())
        } else {
            NodeLocation::parent(
                root.id.get_containing_component_type_id(),
                root.id.get_template_node_id(),
            )
        };
        let group_parent_data = ctx
            .engine_context
            .get_nodes_by_global_id(root_parent.unwrap())
            .first()
            .unwrap()
            .clone();
        let group_parent_transform = group_parent_data.transform_and_bounds().get();
        let group_creation_save_data = {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let mut builder = dt.get_orm_mut().build_new_node(
                ctx.app_state.selected_component_id.get().clone(),
                TypeId::build_singleton(
                    &format!("pax_designer::pax_reexports::pax_std::primitives::Group"),
                    None,
                ),
            );
            builder.set_location(group_location);
            builder
                .save()
                .map_err(|e| anyhow!("could not save: {}", e))?
        };

        let selected_scale = selected.total_bounds.transform.get_scale();
        let new_selected_transform =
            selected.total_bounds.transform * Transform2::scale_sep(selected_scale).inverse();
        // Set the bounds of the group to selection bounds
        ctx.execute(SetNodeTransformProperties {
            id: group_creation_save_data.unique_id.clone(),
            transform_and_bounds: TransformAndBounds {
                transform: new_selected_transform.cast_spaces(),
                bounds: (selected_scale.x, selected_scale.y),
            },
            parent_transform_and_bounds: TransformAndBounds {
                transform: ctx.glass_transform().get() * group_parent_transform.transform,
                bounds: group_parent_transform.bounds,
            },
            inv_config: InversionConfiguration::default(),
        })?;

        let group_location = NodeLocation::parent(
            group_creation_save_data
                .unique_id
                .get_containing_component_type_id(),
            group_creation_save_data.unique_id.get_template_node_id(),
        );
        let _move_selected_into_group_command_ids = {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let mut command_ids = vec![];
            for node in selected.items {
                let cmd_id = dt.get_orm_mut().move_node(node.id, group_location.clone());
                command_ids.push(cmd_id);
            }
            command_ids
        };

        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}
