use std::{rc::Rc, sync::Arc};

use super::{SelectedItem, SelectionState};
use crate::math::coordinate_spaces::World;
use crate::{math::AxisAlignedBox, model::AppState, DESIGNER_GLASS_ID, USERLAND_PROJECT_ID};
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::layout::TransformAndBounds;
use pax_engine::math::Vector2;
use pax_engine::{
    api::{NodeContext, Window},
    math::{Point2, Space, Transform2},
    NodeInterface,
};
use pax_engine::{log, Property};
use pax_manifest::{TemplateNodeId, UniqueTemplateNodeIdentifier};
use pax_runtime_api::{Axis, Size};

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

    pub fn glass_transform(&self) -> Property<Transform2<Window, Glass>> {
        self.transform_from_id::<Window, Glass>(DESIGNER_GLASS_ID)
    }

    pub fn transform_from_id<F: Space, T: Space>(&self, id: &str) -> Property<Transform2<F, T>> {
        let container = self.engine_context.get_nodes_by_id(id);
        if let Some(userland_proj) = container.first() {
            let t_and_b = userland_proj.transform_and_bounds();
            let deps = [t_and_b.untyped()];
            Property::computed(
                move || t_and_b.get().transform.inverse().cast_spaces::<F, T>(),
                &deps,
            )
        } else {
            panic!("no userland project")
        }
    }

    pub fn raycast_glass(&self, point: Point2<Glass>) -> Option<NodeInterface> {
        let window_point = self.glass_transform().get().inverse() * point;
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
                    transform_and_bounds: {
                        let t_and_b = n.transform_and_bounds();
                        let deps = [t_and_b.untyped(), to_glass_transform.untyped()];
                        let to_glass = to_glass_transform.clone();
                        Property::computed(
                            move || {
                                TransformAndBounds {
                                    transform: to_glass.get(),
                                    bounds: (1.0, 1.0),
                                } * t_and_b.get()
                            },
                            &deps,
                        )
                    },
                    origin: {
                        let parent_t_and_b = n
                            .parent()
                            .map(|p| p.transform_and_bounds())
                            .unwrap_or_default();
                        let properties = n.layout_properties();
                        let deps = [parent_t_and_b.untyped(), to_glass_transform.untyped()];
                        let to_glass = to_glass_transform.clone();
                        Property::computed(
                            move || {
                                let parent_t_and_b = parent_t_and_b.get();
                                let parent_bounds = parent_t_and_b.bounds;
                                let local_x = properties
                                    .x
                                    .unwrap_or(Size::ZERO())
                                    .evaluate(parent_bounds, Axis::X);
                                let local_y = properties
                                    .y
                                    .unwrap_or(Size::ZERO())
                                    .evaluate(parent_bounds, Axis::Y);
                                to_glass.get()
                                    * parent_t_and_b.transform
                                    * Point2::new(local_x, local_y)
                            },
                            &deps,
                        )
                    },
                    layout_properties: n.layout_properties(),
                    id,
                })
            })
            .collect();

        let deps: Vec<_> = items
            .iter()
            .map(|i| i.transform_and_bounds.untyped())
            .collect();
        let bounds: Vec<_> = items
            .iter()
            .map(|i| i.transform_and_bounds.clone())
            .collect();
        let total_bounds = Property::computed(
            move || {
                if bounds.len() == 1 {
                    let t_and_b = bounds[0].get();
                    let transform = t_and_b.transform.cast_spaces();
                    TransformAndBounds {
                        transform,
                        bounds: t_and_b.bounds,
                    }
                } else {
                    let axis_box =
                        AxisAlignedBox::bound_of_points(bounds.iter().flat_map(|t_and_b| {
                            let t_and_b = t_and_b.get();
                            let (o, u, v) = t_and_b.transform.decompose();
                            let u = u * t_and_b.bounds.0;
                            let v = v * t_and_b.bounds.1;
                            [o, o + v, o + u, o + v + u]
                        }));
                    let transform = Transform2::compose(
                        axis_box.top_left(),
                        Vector2::new(axis_box.width(), 0.0),
                        Vector2::new(0.0, axis_box.height()),
                    );
                    TransformAndBounds {
                        transform,
                        bounds: (1.0, 1.0),
                    }
                }
            },
            &deps,
        );
        let origin: Vec<_> = items.iter().map(|i| i.origin.clone()).collect();
        let total_origin = if origin.len() == 1 {
            origin[0].clone()
        } else {
            let deps = [total_bounds.untyped()];
            let t_b = total_bounds.clone();
            Property::computed(
                move || {
                    let t_b = t_b.get();
                    let (o, u, v) = t_b.transform.decompose();
                    let u = u * t_b.bounds.0;
                    let v = v * t_b.bounds.1;
                    let center = o + u / 2.0 + v / 2.0;
                    center
                },
                &deps,
            )
        };

        SelectionState {
            total_origin,
            items,
            total_bounds,
        }
    }
}
