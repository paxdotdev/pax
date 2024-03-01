use std::rc::Rc;

use super::math::coordinate_spaces::World;
use crate::{math::AxisAlignedBox, model::AppState, DESIGNER_GLASS_ID, USERLAND_PROJECT_ID};
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::{
    api::{NodeContext, Window},
    math::{Point2, Space, Transform2},
    NodeInterface,
};
use pax_manifest::UniqueTemplateNodeIdentifier;

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
            if let Some(target) = all_elements_beneath_ray
                .into_iter()
                .find(|elem| elem.is_descendant_of(&container))
            {
                return Some(target);
            }
        }
        None
    }

    pub fn selected_node(&self) -> Option<NodeInterface> {
        let type_id = self.app_state.selected_component_id.clone();
        let temp_node_id = self.app_state.selected_template_node_id.as_ref()?.clone();
        self.engine_context
            .get_nodes_by_global_id(UniqueTemplateNodeIdentifier::build(type_id, temp_node_id))
            .into_iter()
            .next()
    }

    pub fn selected_bounds(&self) -> Option<(AxisAlignedBox, Point2<Glass>)> {
        let to_glass_transform = self.glass_transform();
        let expanded_node = self.selected_node()?;
        let bounds = expanded_node
            .bounding_points()?
            .map(|p| to_glass_transform * p);
        Some((
            axis_aligned(bounds),
            to_glass_transform * expanded_node.origin()?,
        ))
    }
}

fn axis_aligned(bound: [Point2<Glass>; 4]) -> AxisAlignedBox {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    for p in bound {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }

    AxisAlignedBox::new(Point2::new(min_x, min_y), Point2::new(max_x, max_y))
}
