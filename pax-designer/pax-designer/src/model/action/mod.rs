use std::{rc::Rc, sync::Arc};

use super::{math::coordinate_spaces::World, SelectedItem, SelectionState};
use crate::{math::AxisAlignedBox, model::AppState, DESIGNER_GLASS_ID, USERLAND_PROJECT_ID};
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::{
    api::{NodeContext, Window},
    math::{Point2, Space, Transform2},
    NodeInterface,
};
use pax_manifest::{TemplateNodeId, UniqueTemplateNodeIdentifier};

use super::math::coordinate_spaces::Glass;

pub mod meta;
pub mod orm;
pub mod pointer;
pub mod world;

type UndoFunc = dyn FnOnce(&mut ActionContext) -> Result<()>;

#[derive(Default)]
pub struct UndoStack {
    stack: Vec<Box<UndoFunc>>,
}

pub trait Action {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo>;
}

impl Action for Box<dyn Action> {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        (*self).perform(ctx)
    }
}

pub enum CanUndo {
    Yes(Box<UndoFunc>),
    No,
}

pub struct ActionContext<'a> {
    pub engine_context: &'a NodeContext<'a>,
    pub app_state: &'a mut AppState,
    pub undo_stack: &'a mut UndoStack,
}

impl ActionContext<'_> {
    pub fn execute(&mut self, action: impl Action) -> Result<()> {
        if let CanUndo::Yes(undo_fn) = Box::new(action).perform(self)? {
            self.undo_stack.stack.push(undo_fn);
        }
        Ok(())
    }

    pub fn undo_last(&mut self) -> Result<()> {
        let undo_fn = self
            .undo_stack
            .stack
            .pop()
            .ok_or(anyhow!("undo stack embty"))?;
        undo_fn(self)
    }

    pub fn world_transform(&self) -> Transform2<Glass, World> {
        self.app_state.glass_to_world_transform
    }

    pub fn glass_transform(&self) -> Transform2<Window, Glass> {
        self.transform_from_id::<Window, Glass>(DESIGNER_GLASS_ID)
    }

    pub fn transform_from_id<F: Space, T: Space>(&self, id: &str) -> Transform2<F, T> {
        let container = self.engine_context.get_nodes_by_id(id);
        if let Some(userland_proj) = container.first() {
            if let Some(transform) = userland_proj.transform() {
                transform.cast_spaces::<F, T>()
            } else {
                Transform2::identity()
            }
        } else {
            panic!("no userland project")
        }
    }

    pub fn raycast_glass(&self, point: Point2<Glass>) -> Option<NodeInterface> {
        let window_point = self.glass_transform().inverse() * point;
        let all_elements_beneath_ray = self.engine_context.raycast(window_point);

        if let Some(container) = self
            .engine_context
            .get_nodes_by_id(USERLAND_PROJECT_ID)
            .first()
        {
            let mut target = all_elements_beneath_ray
                .into_iter()
                .find(|elem| elem.is_descendant_of(&container))?;

            // Find the ancestor that is a direct root element inside container
            while target
                .parent()
                .is_some_and(|p| p.global_id() != container.global_id())
            {
                target = target.parent().unwrap();
            }
            return Some(target);
        }
        None
    }

    pub fn selected_nodes(&mut self) -> Vec<NodeInterface> {
        let type_id = self.app_state.selected_component_id.clone();
        // This is returning the FIRST expanded node matching a template, not all.
        // In the case of one to many relationships existing (for loops), this needs to be revamped.

        let mut nodes = vec![];
        self.app_state.selected_template_node_ids.retain(|id| {
            let Some(node) = self
                .engine_context
                .get_nodes_by_global_id(UniqueTemplateNodeIdentifier::build(
                    type_id.clone(),
                    id.clone(),
                ))
                .into_iter()
                .next()
            else {
                return false;
            };
            nodes.push(node);
            true
        });
        nodes
    }

    pub fn selection_state(&mut self) -> SelectionState {
        let to_glass_transform = self.glass_transform();
        let expanded_node = self.selected_nodes();
        let items: Vec<_> = expanded_node
            .into_iter()
            .flat_map(|n| {
                Some(SelectedItem {
                    bounds: AxisAlignedBox::bound_of_points(
                        n.bounding_points()?.map(|p| to_glass_transform * p),
                    ),
                    origin: to_glass_transform * n.origin()?,
                })
            })
            .collect();

        let total_bounds = AxisAlignedBox::bound_of_boxes(items.iter().map(|i| i.bounds.clone()));
        SelectionState {
            items,
            total_bounds,
        }
    }
}
