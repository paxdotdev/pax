use std::{rc::Rc, sync::Arc};

use super::{SelectedItem, SelectionState};
use crate::math::coordinate_spaces::World;
use crate::{math::AxisAlignedBox, model::AppState, DESIGNER_GLASS_ID, USERLAND_PROJECT_ID};
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::{
    api::{NodeContext, Window},
    math::{Point2, Space, Transform2},
    NodeInterface,
};
use pax_engine::{log, Property};
use pax_manifest::{TemplateNodeId, UniqueTemplateNodeIdentifier};

use crate::math::coordinate_spaces::Glass;

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
    pub engine_context: &'a NodeContext,
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
            .ok_or(anyhow!("undo stack empty"))?;
        undo_fn(self)
    }

    pub fn world_transform(&self) -> Transform2<Glass, World> {
        self.app_state.glass_to_world_transform.get()
    }

    pub fn glass_transform(&self) -> Transform2<Window, Glass> {
        self.transform_from_id::<Window, Glass>(DESIGNER_GLASS_ID)
    }

    pub fn transform_from_id<F: Space, T: Space>(&self, id: &str) -> Transform2<F, T> {
        let container = self.engine_context.get_nodes_by_id(id);
        if let Some(userland_proj) = container.first() {
            userland_proj
                .layout_properties()
                .transform
                .get()
                .inverse()
                .cast_spaces::<F, T>()
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

    pub fn selected_nodes(&mut self) -> Vec<(UniqueTemplateNodeIdentifier, NodeInterface)> {
        let type_id = self.app_state.selected_component_id.get().clone();
        // This is returning the FIRST expanded node matching a template, not all.
        // In the case of one to many relationships existing (for loops), this needs to be revamped.

        let mut nodes = vec![];
        let mut selected_ids = self.app_state.selected_template_node_ids.get();
        let mut discarded = false;
        selected_ids.retain(|id| {
            let unid = UniqueTemplateNodeIdentifier::build(type_id.clone(), id.clone());
            let Some(node) = self
                .engine_context
                .get_nodes_by_global_id(unid.clone())
                .into_iter()
                .next()
            else {
                discarded = true;
                return false;
            };
            nodes.push((unid, node));
            true
        });
        if discarded {
            self.app_state.selected_template_node_ids.set(selected_ids);
        }
        nodes
    }

    pub fn selection_state(&mut self) -> SelectionState {
        let to_glass_transform = self.glass_transform();
        let expanded_node = self.selected_nodes();
        let items: Vec<_> = expanded_node
            .into_iter()
            .flat_map(|(id, n)| {
                Some(SelectedItem {
                    bounds: {
                        let layout_props = n.layout_properties();
                        let deps = [
                            layout_props.transform.untyped(),
                            layout_props.bounds.untyped(),
                        ];
                        Property::computed(
                            move || {
                                AxisAlignedBox::bound_of_points(
                                    layout_props.corners().map(|c| to_glass_transform * c),
                                )
                            },
                            &deps,
                        )
                    },
                    props: n.common_properties(),
                    origin: to_glass_transform * n.origin()?,
                    id,
                })
            })
            .collect();

        let deps: Vec<_> = items.iter().map(|i| i.bounds.untyped()).collect();
        let bounds: Vec<_> = items.iter().map(|i| i.bounds.clone()).collect();
        let total_bounds = Property::computed(
            move || AxisAlignedBox::bound_of_boxes(bounds.iter().map(|v| v.get())),
            &deps,
        );
        SelectionState {
            items,
            total_bounds,
        }
    }
}
