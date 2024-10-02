use std::any::Any;

use crate::{
    designer_node_type::DesignerNodeType,
    math::{coordinate_spaces::Glass, DecompositionConfiguration, IntoDecompositionConfiguration},
    model::{
        action::{
            orm::group_ungroup,
            world::{SelectMode, SelectNodes},
            Action, ActionContext,
        },
        GlassNode, GlassNodeSnapshot, SelectionStateSnapshot,
    },
};
use anyhow::{anyhow, Context, Result};
use pax_designtime::orm::template::builder::NodeBuilder;
use pax_engine::pax_manifest::{
    NodeLocation, TreeIndexPosition, TreeLocation, TypeId, UniqueTemplateNodeIdentifier,
};
use pax_engine::{
    api::{borrow, borrow_mut},
    NodeLocal,
};
use pax_engine::{
    log,
    math::{Point2, Transform2},
    node_layout::{LayoutProperties, TransformAndBounds},
    NodeInterface,
};
use pax_std::core::group::Group;

use super::{tree_movement::MoveNode, CreateComponent, NodeLayoutSettings};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Copy)]
pub enum GroupType {
    Link,
    Group,
    Repeat,
    Conditional,
}

pub struct GroupSelected {
    pub group_type: GroupType,
}
impl Action for GroupSelected {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let selected: SelectionStateSnapshot = (&ctx.derived_state.selection_state.get()).into();
        // ------------ Figure out the location the group should be at ---------
        let Some(node_inside_group) = selected.items.first() else {
            return Err(anyhow!("nothing selected to group"));
        };
        let group_location = {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let orm = dt.get_orm_mut();
            orm.get_node_location(&node_inside_group.id)
        };

        let root_parent = ctx.derived_state.open_containers.get()[0].clone();

        GroupNodes {
            group_type: self.group_type,
            group_bounds: selected.total_bounds.cast_spaces(),
            nodes: &selected
                .items
                .iter()
                .map(|i| i.id.clone())
                .collect::<Vec<_>>(),
            group_parent: &root_parent,
            group_location_index: group_location
                .map(|l| l.index)
                .unwrap_or(TreeIndexPosition::Top),
        }
        .perform(ctx)?;
        Ok(())
    }
}

pub struct GroupNodes<'n> {
    pub group_type: GroupType,
    pub group_parent: &'n UniqueTemplateNodeIdentifier,
    pub group_location_index: TreeIndexPosition,
    pub group_bounds: TransformAndBounds<NodeLocal, Glass>,
    pub nodes: &'n [UniqueTemplateNodeIdentifier],
}

impl Action<UniqueTemplateNodeIdentifier> for GroupNodes<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<UniqueTemplateNodeIdentifier> {
        type BuilderExtraCommands<'a> = Option<&'a dyn Fn(&mut NodeBuilder) -> Result<()>>;
        let (group_type, builder_extra_commands, group_has_size): (_, BuilderExtraCommands, _) =
            match self.group_type {
                GroupType::Link => (DesignerNodeType::Link, None, true),
                GroupType::Group => (DesignerNodeType::Group, None, true),
                GroupType::Repeat => (
                    DesignerNodeType::Repeat,
                    Some(
                        &(|builder| {
                            // sane
                            builder.set_repeat_source("{0..5}")?;
                            builder.set_repeat_predicate("i")?;
                            Ok(())
                        }),
                    ),
                    false,
                ),
                GroupType::Conditional => (
                    DesignerNodeType::Conditional,
                    Some(
                        &(|builder| {
                            builder.set_conditional_source("true")?;
                            Ok(())
                        }),
                    ),
                    false,
                ),
            };
        let group_type_metadata =
            group_type.metadata(&borrow!(ctx.engine_context.designtime).get_orm());

        if !group_type_metadata.is_container {
            return Err(anyhow!(
                "can't group with component that is not a container"
            ));
        }

        let glass_nodes: Vec<_> = self
            .nodes
            .iter()
            .map(|id| ctx.get_glass_node_by_global_id(id))
            .collect::<Result<_, _>>()?;

        // -------- Create a group ------------
        let group_parent_data = ctx.get_glass_node_by_global_id(&self.group_parent).unwrap();

        let parent_is_slot_container = (|| {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let orm = dt.get_orm_mut();
            let parent_type_id = orm
                .get_node(group_parent_data.id.clone(), false)?
                .get_type_id();
            let node_type = DesignerNodeType::from_type_id(parent_type_id);
            Some(node_type.metadata(&dt.get_orm()).is_slot_container)
        })()
        .unwrap_or(false);

        let group_transform_and_bounds = self.group_bounds.as_pure_size();
        let t = ctx.transaction(&format!(
            "grouping selected objects into {}",
            group_type_metadata
                .type_id
                .get_pascal_identifier()
                .unwrap_or_else(|| "<no ident>".to_string()),
        ));

        t.run(|| {
            let group_parent_t_and_b = group_parent_data.transform_and_bounds.get().as_pure_size();
            let decomp_config = Default::default();
            let node_layout = group_has_size.then_some(if parent_is_slot_container {
                NodeLayoutSettings::Fill
            } else {
                NodeLayoutSettings::KeepScreenBounds {
                    node_transform_and_bounds: &group_transform_and_bounds,
                    parent_transform_and_bounds: &group_parent_t_and_b,
                    node_decomposition_config: &decomp_config,
                }
            });

            let group_uid = CreateComponent {
                parent_id: &group_parent_data.id,
                node_layout,
                parent_index: self.group_location_index.clone(),
                designer_node_type: group_type,
                builder_extra_commands,
            }
            .perform(ctx)?;

            // ---------- Move nodes into newly created group ----------
            for node in &glass_nodes {
                MoveNode {
                    node_id: &node.id,
                    index: TreeIndexPosition::Bottom,
                    new_parent_uid: &group_uid,
                    new_node_layout: group_has_size.then_some(
                        NodeLayoutSettings::KeepScreenBounds {
                            node_transform_and_bounds: &node
                                .transform_and_bounds
                                .get()
                                .as_pure_size(),
                            parent_transform_and_bounds: &group_transform_and_bounds,
                            node_decomposition_config: &node
                                .layout_properties
                                .into_decomposition_config(),
                        },
                    ),
                }
                .perform(ctx)?;
            }

            // ---------- Select the newly created group -----
            SelectNodes {
                ids: &[group_uid.get_template_node_id()],
                mode: SelectMode::DiscardOthers,
            }
            .perform(ctx)?;

            Ok(group_uid)
        })
    }
}

pub struct UngroupSelected {}

impl Action for UngroupSelected {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let selected: SelectionStateSnapshot = (&ctx.derived_state.selection_state.get()).into();
        let t = ctx.transaction("ungrouping");
        t.run(|| {
            let mut select_toggle = vec![];
            for group in selected.items {
                {
                    let mut dt = borrow_mut!(ctx.engine_context.designtime);
                    let node = dt
                        .get_orm_mut()
                        .get_node(group.id.clone(), false)
                        .ok_or_else(|| anyhow!("no thing"))?;
                    let node_type = DesignerNodeType::from_type_id(node.get_type_id());
                    if !node_type.metadata(&dt.get_orm()).is_container {
                        continue;
                    }
                }

                select_toggle.push(group.id.get_template_node_id());

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

                let group_location = {
                    let mut dt = borrow_mut!(ctx.engine_context.designtime);
                    let orm = dt.get_orm_mut();
                    orm.get_node_location(&group.id)
                };
                let new_node_index = group_location
                    .map(|l| l.index)
                    .unwrap_or(TreeIndexPosition::Top);

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
                        index: new_node_index.clone(),
                        new_node_layout: Some(NodeLayoutSettings::KeepScreenBounds {
                            parent_transform_and_bounds: &group_parent_bounds,
                            node_transform_and_bounds: &child_t_and_b,
                            node_decomposition_config: &child_inv_config,
                        }),
                    }
                    .perform(ctx)?;
                }

                select_toggle.extend(group_children.iter().map(|v| v.get_template_node_id()));

                // ---------- Delete group --------------
                borrow_mut!(ctx.engine_context.designtime)
                    .get_orm_mut()
                    .remove_node(group.id)
                    .map_err(|e| anyhow!("failed to remove group {:?}", e))
                    .map(|_| ())?;
            }

            SelectNodes {
                ids: &select_toggle,
                mode: SelectMode::KeepOthers,
            }
            .perform(ctx)?;

            Ok(())
        })
    }
}
