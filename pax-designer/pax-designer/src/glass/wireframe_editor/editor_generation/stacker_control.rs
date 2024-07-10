use std::{cell::RefCell, ops::ControlFlow, rc::Rc};

use pax_engine::{
    api::NodeContext,
    layout::TransformAndBounds,
    log,
    math::{Point2, Transform2},
    NodeInterface, Property, Slot,
};
use pax_runtime_api::{borrow, borrow_mut, Color};
use pax_std::stacker::Stacker;

use crate::{
    glass::{
        control_point::{ControlPointBehaviourFactory, ControlPointStyling},
        outline::PathOutline,
        wireframe_editor::editor_generation::CPoint,
        ToolVisualizationState,
    },
    math::{coordinate_spaces::Glass, DecompositionConfiguration, IntoDecompositionConfiguration},
    model::{
        self,
        action::{
            orm::{MoveNode, NodeLayoutSettings, SetNodePropertiesFromTransform},
            Action, ActionContext, RaycastMode,
        },
        input::InputEvent,
        GlassNode, GlassNodeSnapshot, ToolBehaviour,
    },
    ROOT_PROJECT_ID,
};

use super::ControlPointSet;

pub fn stacker_control_set(ctx: NodeContext, item: GlassNode) -> Vec<Property<ControlPointSet>> {
    struct StackerBehaviour {
        initial_node: GlassNodeSnapshot,
        container: GlassNodeSnapshot,
        pickup_point: Point2<Glass>,
        before_move_undo_id: usize,
        vis: Property<ToolVisualizationState>,
    }

    let to_glass_transform =
        model::read_app_state_with_derived(|_, derived| derived.to_glass_transform.get());

    // re-do this whenever slots change
    let stacker_node = ctx
        .get_nodes_by_global_id(item.id)
        .into_iter()
        .next()
        .unwrap();
    let slot_count = stacker_node.flattened_slot_children_count();

    impl ToolBehaviour for StackerBehaviour {
        fn pointer_down(
            &mut self,
            _point: Point2<Glass>,
            _ctx: &mut ActionContext,
        ) -> ControlFlow<()> {
            ControlFlow::Break(())
        }

        fn pointer_move(
            &mut self,
            point: Point2<Glass>,
            ctx: &mut ActionContext,
        ) -> ControlFlow<()> {
            let translation = point - self.pickup_point;

            let move_translation = TransformAndBounds {
                transform: Transform2::translate(translation),
                bounds: (1.0, 1.0),
            };

            if let Err(e) = (SetNodePropertiesFromTransform {
                id: &self.initial_node.id,
                transform_and_bounds: &(move_translation * self.initial_node.transform_and_bounds),
                parent_transform_and_bounds: &self.initial_node.parent_transform_and_bounds,
                decomposition_config: &self
                    .initial_node
                    .layout_properties
                    .into_decomposition_config(),
            }
            .perform(ctx))
            {
                pax_engine::log::error!("Error moving stacker object: {:?}", e);
            }
            let raycast_hit = raycast_slot(
                ctx,
                point,
                self.initial_node.raw_node_interface.clone(),
                false,
            );
            if let Some((_container, slot)) = raycast_hit {
                let t_and_b = TransformAndBounds {
                    transform: ctx.glass_transform().get(),
                    bounds: (1.0, 1.0),
                } * slot.transform_and_bounds().get();
                let outline = PathOutline::from_bounds(t_and_b);
                self.vis.set(ToolVisualizationState {
                    rect_tool: Default::default(),
                    outline,
                });
            } else {
                self.vis.set(ToolVisualizationState::default());
            }
            ControlFlow::Continue(())
        }

        fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
            if let Some((container, slot)) = raycast_slot(
                ctx,
                point,
                self.initial_node.raw_node_interface.clone(),
                false,
            ) {
                if let Err(e) = (MoveNode {
                    node_id: &self.initial_node.id,
                    new_parent_uid: &container.global_id().unwrap(),
                    index: pax_manifest::TreeIndexPosition::At(
                        slot.with_properties(|f: &mut Slot| f.index.get().to_int()) as usize,
                    ),
                    node_layout: NodeLayoutSettings::Fill::<Glass>,
                }
                .perform(ctx))
                {
                    log::warn!("failed to swap nodes: {}", e);
                };
            } else if !self.container.transform_and_bounds.contains_point(point) {
                let stacker_parent = GlassNode::new(
                    &&self.container.raw_node_interface.template_parent().unwrap(),
                    &ctx.glass_transform(),
                );
                let curr_node = ctx
                    .engine_context
                    .get_nodes_by_global_id(self.initial_node.id.clone())
                    .into_iter()
                    .next()
                    .unwrap();
                let curr_node = GlassNode::new(&curr_node, &ctx.glass_transform());
                if let Err(e) = (MoveNode {
                    node_id: &curr_node.id,
                    new_parent_uid: &stacker_parent.id,
                    index: pax_manifest::TreeIndexPosition::Top,
                    node_layout: NodeLayoutSettings::KeepScreenBounds {
                        node_transform_and_bounds: &curr_node.transform_and_bounds.get(),
                        new_parent_transform_and_bounds: &stacker_parent.transform_and_bounds.get(),
                        node_inv_config: self
                            .initial_node
                            .layout_properties
                            .into_decomposition_config(),
                    },
                }
                .perform(ctx))
                {
                    log::warn!("failed to move node outside of stacker: {e}");
                };
            } else {
                let mut dt = borrow_mut!(ctx.engine_context.designtime);
                if let Err(e) = dt.get_orm_mut().undo_until(self.before_move_undo_id) {
                    log::warn!("failed to undo stacker object move: {e}");
                }
            }
            ControlFlow::Break(())
        }

        fn keyboard(
            &mut self,
            _event: InputEvent,
            _dir: crate::model::input::Dir,
            _ctx: &mut ActionContext,
        ) -> ControlFlow<()> {
            ControlFlow::Break(())
        }

        fn get_visual(&self) -> Property<crate::glass::ToolVisualizationState> {
            let vis = self.vis.clone();
            let deps = [vis.untyped()];
            Property::computed(move || vis.get(), &deps)
        }
    }

    fn stacker_control_point_factory(
        container: GlassNode,
        slot_child: GlassNode,
    ) -> ControlPointBehaviourFactory {
        Rc::new(move |ac, p| {
            let container = container.clone();
            let dt = borrow!(ac.engine_context.designtime);
            let before_move_undo_id = dt.get_orm().get_last_undo_id().unwrap_or(0);
            Rc::new(RefCell::new(StackerBehaviour {
                container: (&container).into(),
                initial_node: (&slot_child).into(),
                pickup_point: p,
                before_move_undo_id,
                vis: Property::new(ToolVisualizationState::default()),
            }))
        })
    }

    let control_point_styling = ControlPointStyling {
        round: true,
        stroke: Color::RED,
        fill: Color::rgba(255.into(), 255.into(), 255.into(), 150.into()),
        stroke_width_pixels: 2.0,
        size_pixels: 15.0,
    };

    let t_and_b = stacker_node.transform_and_bounds();
    let deps = [t_and_b.untyped(), slot_count.untyped()];
    vec![Property::computed(
        move || {
            let mut slots = vec![];
            let mut search_set: Vec<NodeInterface> = stacker_node.children();
            while let Some(node) = search_set.pop() {
                for n in node.children() {
                    if n.is_of_type::<Slot>() {
                        slots.push(n)
                    } else {
                        search_set.push(n)
                    }
                }
            }
            let to_glass = to_glass_transform.clone();
            let stacker_node = GlassNode::new(&stacker_node, &to_glass);
            let stacker_control_points = slots
                .into_iter()
                .map(|s| {
                    let slot_child = s.children().into_iter().next().unwrap();
                    let slot_child = GlassNode::new(&slot_child, &to_glass);
                    let t_and_b = slot_child.transform_and_bounds.get();
                    CPoint::new(
                        t_and_b.center(),
                        stacker_control_point_factory(stacker_node.clone(), slot_child),
                    )
                })
                .collect();
            ControlPointSet {
                points: stacker_control_points,
                styling: control_point_styling.clone(),
            }
        },
        &deps,
    )]
}

pub fn raycast_slot(
    ctx: &ActionContext,
    point: Point2<Glass>,
    original_hit: NodeInterface,
    prevent_move_self: bool,
) -> Option<(NodeInterface, NodeInterface)> {
    // If we drop on another object, check if it's an object in a slot.
    // If it is, add this object to the same parent
    let drop_hit = ctx.raycast_glass(point, RaycastMode::RawNth(0), &[original_hit.clone()])?;
    let mut drop_slot_container = drop_hit.clone();
    let drop_slot_topmost_container = loop {
        if drop_slot_container
            .containing_component()
            .is_some_and(|v| v.is_of_type::<Stacker>())
        {
            break Some(drop_slot_container);
        }
        if let Some(par) = drop_slot_container.render_parent() {
            drop_slot_container = par;
        } else {
            break None;
        };
    };
    let drop_container = drop_slot_topmost_container?;
    let mut slot = None;
    let mut curr = drop_hit.clone();
    let cc = drop_container.containing_component().unwrap();
    while curr.is_descendant_of(&cc) {
        if curr.is_of_type::<Slot>() {
            slot = Some(curr.clone());
        }
        curr = curr.render_parent().unwrap();
    }
    if prevent_move_self && original_hit.is_descendant_of(&cc) {
        return None;
    };
    Some((cc, slot.unwrap()))
}
