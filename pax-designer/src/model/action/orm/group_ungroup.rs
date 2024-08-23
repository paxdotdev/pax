use std::any::Any;

use crate::{
    math::{DecompositionConfiguration, IntoDecompositionConfiguration},
    model::{
        action::{
            orm::{group_ungroup, SetNodeLayoutProperties},
            Action, ActionContext,
        },
        tools::{SelectMode, SelectNodes},
        GlassNode, GlassNodeSnapshot, SelectionStateSnapshot,
    },
};
use anyhow::{anyhow, Context, Result};
use pax_engine::api::borrow_mut;
use pax_engine::pax_manifest::{
    NodeLocation, TreeIndexPosition, TreeLocation, TypeId, UniqueTemplateNodeIdentifier,
};
use pax_engine::{
    layout::{LayoutProperties, TransformAndBounds},
    log,
    math::{Point2, Transform2},
    NodeInterface,
};
use pax_std::core::group::Group;

use super::{CreateComponent, MoveNode, NodeLayoutSettings, SetNodeLayoutPropertiesFromTransform};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Copy)]
pub enum GroupType {
    Link,
    Group,
}

impl From<GroupType> for TypeId {
    fn from(value: GroupType) -> Self {
        match value {
            GroupType::Group => TypeId::build_singleton("pax_std::core::group::Group", None),
            GroupType::Link => TypeId::build_singleton("pax_std::core::link::Link", None),
        }
    }
}

pub struct GroupSelected {
    pub group_type: GroupType,
}

impl Action for GroupSelected {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let selected: SelectionStateSnapshot = (&ctx.derived_state.selection_state.get()).into();

        // ------------ Figure out the location the group should be at ---------
        let Some(_) = selected.items.first() else {
            return Err(anyhow!("nothing selected to group"));
        };
        let root_parent = ctx.derived_state.open_container.get();

        let new_parent_type_id: TypeId = self.group_type.into();
        // -------- Create a group ------------
        let group_parent_data = ctx.get_glass_node_by_global_id(&root_parent).unwrap();
        let group_transform_and_bounds = selected.total_bounds.as_pure_size().cast_spaces();
        let t = ctx.transaction(&format!(
            "grouping selected objects into {}",
            new_parent_type_id
                .get_pascal_identifier()
                .unwrap_or_else(|| "<no ident>".to_string()),
        ));

        t.run(|| {
            let group_uid = CreateComponent {
                parent_id: &group_parent_data.id,
                node_layout: NodeLayoutSettings::KeepScreenBounds {
                    node_transform_and_bounds: &group_transform_and_bounds,
                    parent_transform_and_bounds: &group_parent_data.transform_and_bounds.get(),
                    node_decomposition_config: &Default::default(),
                },
                parent_index: TreeIndexPosition::Top,
                type_id: &new_parent_type_id,
                custom_props: &[],
                mock_children: 0,
            }
            .perform(ctx)?;

            // ---------- Move nodes into newly created group ----------
            for node in &selected.items {
                MoveNode {
                    node_id: &node.id,
                    index: TreeIndexPosition::Bottom,
                    new_parent_uid: &group_uid,
                    node_layout: NodeLayoutSettings::KeepScreenBounds {
                        node_transform_and_bounds: &node.transform_and_bounds,
                        parent_transform_and_bounds: &group_transform_and_bounds,
                        node_decomposition_config: &node
                            .layout_properties
                            .into_decomposition_config(),
                    },
                }
                .perform(ctx)?;
            }

            // ---------- Select the newly created group -----
            SelectNodes {
                ids: &[group_uid.get_template_node_id()],
                mode: SelectMode::DiscardOthers,
            }
            .perform(ctx)?;

            Ok(())
        })
    }
}

pub struct UngroupSelected {}

impl Action for UngroupSelected {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let selected: SelectionStateSnapshot = (&ctx.derived_state.selection_state.get()).into();
        let t = ctx.transaction("ungrouping");
        t.run(|| {
            for group in selected.items {
                let parent = ctx
                    .get_glass_node_by_global_id(
                        &group
                            .raw_node_interface
                            .template_parent()
                            .unwrap()
                            .global_id()
                            .unwrap(),
                    )
                    .unwrap();
                let group_parent_bounds = parent.transform_and_bounds.get();

                let group_children = borrow_mut!(ctx.engine_context.designtime)
                    .get_orm_mut()
                    .get_node_children(group.id.clone())
                    .map_err(|e| anyhow!("group not found {:?}", e))?;

                // ---------- Move Nodes to group parent --------------
                for child in group_children.iter().rev() {
                    let child_runtime_node = ctx.get_glass_node_by_global_id(&child).unwrap();
                    let child_inv_config = child_runtime_node
                        .layout_properties
                        .into_decomposition_config();
                    let child_t_and_b = child_runtime_node.transform_and_bounds.get();
                    MoveNode {
                        node_id: &child,
                        new_parent_uid: &parent.id,
                        index: TreeIndexPosition::Top,
                        node_layout: NodeLayoutSettings::KeepScreenBounds {
                            parent_transform_and_bounds: &group_parent_bounds,
                            node_transform_and_bounds: &child_t_and_b,
                            node_decomposition_config: &child_inv_config,
                        },
                    }
                    .perform(ctx)?;
                }

                // ---------- Delete group --------------
                borrow_mut!(ctx.engine_context.designtime)
                    .get_orm_mut()
                    .remove_node(group.id)
                    .map_err(|e| anyhow!("failed to remove group {:?}", e))
                    .map(|_| ())?;
            }

            Ok(())
        })
    }
}
