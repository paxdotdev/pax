use std::any::Any;
use std::f64::consts::PI;

pub use self::space_movement_primitives::{NodeLayoutSettings, SetNodeLayout};

use super::{Action, ActionContext};
use crate::designer_node_type::DesignerNodeType;
use crate::glass::wireframe_editor::editor_generation::stacker_control::sizes_to_string;
use crate::math::approx::ApproxEq;
use crate::math::coordinate_spaces::{Glass, SelectionSpace, World};
use crate::math::{
    self, AxisAlignedBox, DecompositionConfiguration, GetUnit, IntoDecompositionConfiguration,
    RotationUnit, SizeUnit,
};
use crate::model::input::{InputEvent, ModifierKey};
use crate::model::tools::{SelectMode, SelectNodes};
use crate::model::{GlassNode, GlassNodeSnapshot, SelectionStateSnapshot};
use crate::{math::BoxPoint, model, model::AppState};
use anyhow::{anyhow, Context, Result};
use pax_designtime::orm::template::builder::{self, NodeBuilder};
use pax_designtime::orm::{MoveToComponentEntry, SubTrees};
use pax_designtime::{DesigntimeManager, Serializer};
use pax_engine::api::{borrow, borrow_mut, Rotation};
use pax_engine::api::{Axis, Percent};
use pax_engine::math::{Generic, Transform2, TransformParts};
use pax_engine::node_layout::{LayoutProperties, TransformAndBounds};
use pax_engine::pax_manifest::{
    NodeLocation, PaxType, TemplateNodeId, TreeIndexPosition, TreeLocation, TypeId,
    UniqueTemplateNodeIdentifier,
};
use pax_engine::pax_runtime::RepeatInstance;
use pax_engine::serde::Serialize;
use pax_engine::{
    api::Size,
    math::{Point2, Space, Vector2},
    serde,
};
use pax_engine::{log, NodeInterface, NodeLocal, Slot};
use pax_std::layout::stacker::Stacker;
pub mod group_ungroup;
pub mod other;
pub mod space_movement;
pub mod space_movement_primitives;
pub mod tree_movement;

pub struct CreateComponent<'a> {
    pub parent_id: &'a UniqueTemplateNodeIdentifier,
    pub parent_index: TreeIndexPosition,
    pub designer_node_type: DesignerNodeType,
    pub builder_extra_commands: Option<&'a dyn Fn(&mut NodeBuilder) -> Result<()>>,
    pub node_layout: NodeLayoutSettings<'a, Glass>,
}

impl Action<UniqueTemplateNodeIdentifier> for CreateComponent<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<UniqueTemplateNodeIdentifier> {
        let parent_location = ctx.location(self.parent_id, &self.parent_index);

        let node_type_metadata = self
            .designer_node_type
            .metadata(&borrow!(ctx.engine_context.designtime).get_orm());
        // probably move transactions to happen here? (and remove from callers)
        // WARNING: if making this change, make sure mock children are in same transaction
        let save_data = {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let mut builder = dt.get_orm_mut().build_new_node(
                ctx.app_state.selected_component_id.get().clone(),
                node_type_metadata.type_id,
            );

            builder.set_location(parent_location);

            if let Some(extra_build_commands) = self.builder_extra_commands {
                extra_build_commands(&mut builder)?;
            }

            builder
                .save()
                .map_err(|e| anyhow!("could not save: {}", e))?
        };

        // HACK: if the parent of this is a scroller, modify parent transform
        // and bounds to reflect the size of the inner pane instead of the
        // outer. Eventually, create a general framework for figuring out "true"
        // parent bounds for all containers. (more complicated for stacker where
        // it depends on number of children)
        if let NodeLayoutSettings::KeepScreenBounds {
            node_transform_and_bounds,
            node_decomposition_config,
            ..
        } = self.node_layout
        {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let orm = dt.get_orm_mut();
            if let Some(parent) = orm.get_node(self.parent_id.clone(), false) {
                if DesignerNodeType::from_type_id(parent.get_type_id())
                    == DesignerNodeType::Scroller
                {
                    drop(dt);
                    let node = ctx
                        .engine_context
                        .get_nodes_by_id("_scroller_inner_container")
                        .into_iter()
                        .filter_map(|n| {
                            let mut steps = 0;
                            let mut node = n.clone();
                            while let Some(parent) = node.template_parent() {
                                if &parent.global_id().unwrap() == self.parent_id {
                                    break;
                                }
                                node = parent;
                                steps += 1;
                            }
                            node.template_parent().is_some().then_some((n, steps))
                        })
                        .min_by_key(|(_, v)| *v);
                    if let Some((node, _)) = node {
                        let glass_node = GlassNode::new(&node, &ctx.glass_transform());
                        SetNodeLayout {
                            id: &save_data.unique_id,
                            node_layout: &NodeLayoutSettings::KeepScreenBounds {
                                node_transform_and_bounds,
                                node_decomposition_config,
                                parent_transform_and_bounds: &glass_node.transform_and_bounds.get(),
                            },
                        }
                        .perform(ctx)?;
                        return Ok(save_data.unique_id);
                    }
                }
            }
        }

        SetNodeLayout {
            id: &save_data.unique_id,
            node_layout: &self.node_layout,
        }
        .perform(ctx)?;

        Ok(save_data.unique_id)
    }
}

// TODO move this into group_ungroup and make work again
pub struct SelectedIntoNewComponent {}

impl Action for SelectedIntoNewComponent {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let selection = ctx.derived_state.selection_state.get();
        if selection.items.len() == 0 {
            return Err(anyhow!("can't create new embty component"));
        };
        let mut dt = borrow_mut!(ctx.engine_context.designtime);

        let world_transform = ctx.world_transform();
        let entries: Vec<_> = selection
            .items
            .iter()
            .map(|e| {
                let b = TransformAndBounds {
                    transform: world_transform,
                    bounds: (1.0, 1.0),
                } * e.transform_and_bounds.get();
                let parts: TransformParts = b.transform.into();
                MoveToComponentEntry {
                    x: parts.origin.x,
                    y: parts.origin.y,
                    width: parts.scale.x * b.bounds.0,
                    height: parts.scale.y * b.bounds.1,
                    id: e.id.clone(),
                }
            })
            .collect();

        let tb = TransformAndBounds {
            transform: world_transform,
            bounds: (1.0, 1.0),
        } * selection.total_bounds.get();
        let (o, u, v) = tb.transform.decompose();
        let u = u * tb.bounds.0;
        let v = v * tb.bounds.1;
        let t = ctx.transaction("moving selected into new component");
        t.run(|| {
            dt.get_orm_mut()
                .move_to_new_component(&entries, o.x, o.y, u.length(), v.length())
                .map_err(|e| anyhow!("couldn't move to component: {}", e))
        })
    }
}

pub struct SerializeRequested;

impl Action for SerializeRequested {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        if let Err(e) = dt.send_component_update(&ctx.app_state.selected_component_id.get()) {
            pax_engine::log::error!("failed to save component to file: {:?}", e);
        }
        Ok(())
    }
}

pub struct UndoRequested;

impl Action for UndoRequested {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            ctx.undo_stack.undo(dt.get_orm_mut());
        }
        SerializeRequested.perform(ctx)?;
        Ok(())
    }
}

pub struct RedoRequested;

impl Action for RedoRequested {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            ctx.undo_stack.redo(dt.get_orm_mut());
        }
        SerializeRequested.perform(ctx)?;
        Ok(())
    }
}

pub struct DeleteSelected;

impl Action for DeleteSelected {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let selected = ctx.app_state.selected_template_node_ids.get();
        let t = ctx.transaction("delete node");

        t.run(|| {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            for s in selected {
                let uid = UniqueTemplateNodeIdentifier::build(
                    ctx.app_state.selected_component_id.get(),
                    s.clone(),
                );
                dt.get_orm_mut()
                    .remove_node(uid)
                    .map_err(|_| anyhow!("couldn't delete node"))?;
            }
            ctx.app_state
                .selected_template_node_ids
                .update(|ids| ids.clear());
            Ok(())
        })
    }
}

pub struct Copy<'a> {
    pub ids: &'a [TemplateNodeId],
}

impl Action<SubTrees> for Copy<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<SubTrees> {
        let comp_id = ctx.app_state.selected_component_id.get();
        let dt = borrow!(ctx.engine_context.designtime);
        let subtree = dt
            .get_orm()
            .copy_subtrees(&comp_id, &self.ids)
            .ok_or_else(|| anyhow!("couldn't copy"))?;
        Ok(subtree)
    }
}

pub struct Paste<'a> {
    pub subtrees: &'a SubTrees,
}

impl Action<Vec<TemplateNodeId>> for Paste<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<Vec<TemplateNodeId>> {
        let parent = ctx.derived_state.open_container.get();
        let loc = ctx.location(&parent, &TreeIndexPosition::Top);
        let t = ctx.transaction("pasting object");
        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        let ids = t.run(|| {
            let ids = dt
                .get_orm_mut()
                .paste_subtrees(loc, self.subtrees.clone())
                .map_err(|e| anyhow!("failed to paste: {e}"))?;
            SelectNodes {
                ids: &ids,
                mode: SelectMode::DiscardOthers,
            }
            .perform(ctx)?;
            Ok(ids)
        });
        ids
    }
}
