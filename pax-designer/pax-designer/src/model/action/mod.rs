use std::any::Any;
use std::{rc::Rc, sync::Arc};

use super::{DerivedAppState, GlassNode, SelectionState};
use crate::math::coordinate_spaces::World;
use crate::{math::AxisAlignedBox, model::AppState, DESIGNER_GLASS_ID, ROOT_PROJECT_ID};
use anyhow::{anyhow, Result};
use pax_designtime::orm::PaxManifestORM;
use pax_designtime::DesigntimeManager;
use pax_engine::api::{borrow, Axis, Size};
use pax_engine::layout::TransformAndBounds;
use pax_engine::math::Vector2;
use pax_engine::pax_manifest::{TemplateNodeId, UniqueTemplateNodeIdentifier};
use pax_engine::{
    api::{NodeContext, Window},
    math::{Point2, Space, Transform2},
    NodeInterface,
};
use pax_engine::{log, Property};
use pax_std::drawing::rectangle::Rectangle;

use crate::math::coordinate_spaces::Glass;

pub mod orm;
pub mod pointer;
pub mod world;

#[derive(Default)]
pub struct UndoRedoStack {
    undo_stack: Vec<usize>,
    redo_stack: Vec<usize>,
}

impl UndoRedoStack {
    pub fn push(&mut self, undo_id: usize) {
        self.undo_stack.push(undo_id);
        self.redo_stack.clear();
    }

    fn undo(&mut self, orm: &mut PaxManifestORM) -> Option<()> {
        let curr_id = orm.get_last_undo_id()?;
        let undo_id = self.undo_stack.pop()?;
        orm.undo_until(undo_id).ok()?;
        self.redo_stack.push(curr_id);
        Some(())
    }

    fn redo(&mut self, orm: &mut PaxManifestORM) -> Option<()> {
        let curr_id = orm.get_last_undo_id()?;
        let redo_id = self.redo_stack.pop()?;
        orm.redo_including(redo_id).ok()?;
        self.undo_stack.push(curr_id);
        Some(())
    }
}

pub trait Action<R = ()> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<R>;
}

pub struct ActionContext<'a> {
    pub engine_context: &'a NodeContext,
    pub app_state: &'a mut AppState,
    pub derived_state: &'a DerivedAppState,
    pub undo_stack: &'a mut UndoRedoStack,
}

impl ActionContext<'_> {
    pub fn world_transform(&self) -> Transform2<Glass, World> {
        self.app_state.glass_to_world_transform.get()
    }

    pub fn glass_transform(&self) -> Property<Transform2<Window, Glass>> {
        self.derived_state.to_glass_transform.get()
    }

    pub fn selected_nodes(&self) -> Vec<(UniqueTemplateNodeIdentifier, NodeInterface)> {
        self.derived_state.selected_nodes.get()
    }

    pub fn raycast_glass(
        &self,
        point: Point2<Glass>,
        mode: RaycastMode,
        skip: &[NodeInterface],
    ) -> Option<NodeInterface> {
        let window_point = self.glass_transform().get().inverse() * point;
        let all_elements_beneath_ray = self.engine_context.raycast(window_point, false);

        let userland = self.engine_context.get_nodes_by_id(ROOT_PROJECT_ID).pop()?;
        let userland_id = userland.global_id();

        let mut potential_targets = all_elements_beneath_ray
            .into_iter()
            .filter(|elem| !skip.iter().any(|v| elem == v || elem.is_descendant_of(v)))
            .filter(|elem| elem.is_descendant_of(&userland));

        if let RaycastMode::RawNth(index) = mode {
            return potential_targets.nth(index);
        }

        let mut target = potential_targets.next()?;

        // Find the ancestor that is a direct root element inside container
        // or one that's in the current edit root
        loop {
            let Some(parent) = target.template_parent() else {
                break;
            };

            // check one step ahead if we are drilling into a group or similar

            if parent.global_id() == userland_id {
                break;
            };
            if parent
                .global_id()
                .is_some_and(|v| self.derived_state.open_containers.get().contains(&v))
            {
                break;
            }
            if matches!(mode, RaycastMode::DrillOne) {
                let Some(next_parent) = parent.template_parent() else {
                    break;
                };
                if next_parent.global_id() == userland_id {
                    break;
                };
                if next_parent
                    .global_id()
                    .is_some_and(|v| self.derived_state.open_containers.get().contains(&v))
                {
                    break;
                }
            }
            target = target.template_parent().unwrap();
        }
        Some(target)
    }

    pub fn undo_save(&mut self) {
        let before_undo_id = borrow!(self.engine_context.designtime)
            .get_orm()
            .get_last_undo_id()
            .unwrap_or(0);
        self.undo_stack.push(before_undo_id);
    }
}

pub enum RaycastMode {
    // Only hit elements that are either directly bellow the userland project
    // root, or ones that are at the same level as an already selected node
    Top,
    // Hit the children of the "Top" elements
    DrillOne,
    // Hit all elements, and choose the nth hit
    RawNth(usize),
}
