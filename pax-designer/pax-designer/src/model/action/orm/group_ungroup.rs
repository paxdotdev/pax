use std::any::Any;

use crate::{
    math::{DecompositionConfiguration, IntoDecompositionConfiguration},
    model::{
        action::{
            orm::{group_ungroup, SetNodeProperties},
            Action, ActionContext,
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

use super::{CreateComponent, MoveNode, ResizeMode, SetNodePropertiesFromTransform};

pub struct GroupSelected {}

impl Action for GroupSelected {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let selected: SelectionStateSnapshot = (&ctx.derived_state.selection_state.get()).into();

        // ------------ Figure out the location the group should be at ---------
        let Some(_) = selected.items.first() else {
            return Err(anyhow!("nothing selected to group"));
        };
        let root_parent = ctx
            .derived_state
            .open_containers
            .read(|v| v.first().cloned())
            .unwrap();

        // -------- Create a group ------------
        let group_parent_data = ctx
            .engine_context
            .get_nodes_by_global_id(root_parent)
            .first()
            .unwrap()
            .clone();

        let group_parent_data = GlassNode::new(&group_parent_data, &ctx.glass_transform());
        let group_transform_and_bounds = selected.total_bounds.as_pure_size().cast_spaces();

        let group_uid = CreateComponent {
            parent: &(&group_parent_data).into(),
            parent_index: TreeIndexPosition::Top,
            bounds: group_transform_and_bounds,
            type_id: TypeId::build_singleton(
                "pax_designer::pax_reexports::pax_std::primitives::Group",
                None,
            ),
            custom_props: vec![],
            mock_child: true,
        }
        .perform(ctx)?;

        // ---------- Move nodes into newly created group ----------
        for node in &selected.items {
            MoveNode {
                node_id: &node.id,
                index: TreeIndexPosition::Bottom,
                new_parent_uid: &group_uid,
                resize_mode: ResizeMode::KeepScreenBounds {
                    node_transform_and_bounds: &node.transform_and_bounds,
                    new_parent_transform_and_bounds: &group_transform_and_bounds,
                    node_inv_config: node.layout_properties.into_decomposition_config(),
                },
            }
            .perform(ctx)?;
        }

        // ---------- Select the newly created group -----
        SelectNodes {
            ids: &[group_uid.get_template_node_id()],
            overwrite: true,
        }
        .perform(ctx)?;

        Ok(())
    }
}

pub struct UngroupSelected {}

impl Action for UngroupSelected {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
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
                MoveNode {
                    node_id: &child,
                    new_parent_uid: parent_id.as_ref().unwrap(),
                    index: TreeIndexPosition::Top,
                    resize_mode: ResizeMode::KeepScreenBounds {
                        new_parent_transform_and_bounds: &group_parent_bounds,
                        node_transform_and_bounds: &child_t_and_b,
                        node_inv_config: child_inv_config,
                    },
                }
                .perform(ctx)?;
            }

            // ---------- Delete group --------------
            borrow_mut!(ctx.engine_context.designtime)
                .get_orm_mut()
                .remove_node(group.id)
                .map_err(|e| anyhow!("failed to remove group {:?}", e))?;
        }

        Ok(())
    }
}
