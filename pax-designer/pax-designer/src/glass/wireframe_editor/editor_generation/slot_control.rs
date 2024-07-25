use std::{any::Any, cell::RefCell, ops::ControlFlow, rc::Rc};

use pax_engine::{
    api::NodeContext,
    layout::TransformAndBounds,
    log,
    math::{Point2, Transform2},
    NodeInterface, NodeLocal, Property, Slot,
};
use pax_engine::pax_manifest::UniqueTemplateNodeIdentifier;
use pax_engine::api::{borrow, borrow_mut, Color};
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
            orm::{MoveNode, NodeLayoutSettings, SetNodeLayout},
            Action, ActionContext, RaycastMode,
        },
        input::InputEvent,
        GlassNode, GlassNodeSnapshot, ToolBehaviour,
    },
    utils::filter_with_last::FilterWithLastExt,
    ROOT_PROJECT_ID,
};

use super::ControlPointSet;

// NOTE: a lot of this logic is very similar to that of movement performed with PointerTool.
// TODO: decide if this should stay as separate logic (if we want different behaviour at times)
// or if slot_dot dragging should always behave as normal movement. If so, make this use the
// PointerTool with  a pre-defined target.
pub fn slot_dot_control_set(ctx: NodeContext, item: GlassNode) -> Property<ControlPointSet> {
    struct SlotBehaviour {
        initial_node: GlassNodeSnapshot,
        pickup_point: Point2<Glass>,
        _before_move_undo_id: usize,
        vis: Property<ToolVisualizationState>,
    }

    let to_glass_transform =
        model::read_app_state_with_derived(|_, derived| derived.to_glass_transform.get());

    // re-do this whenever slots change
    let slot_parent_node = ctx
        .get_nodes_by_global_id(item.id)
        .into_iter()
        .next()
        .unwrap();
    let slot_count = slot_parent_node.flattened_slot_children_count();

    impl ToolBehaviour for SlotBehaviour {
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

            let curr_node = ctx
                .engine_context
                .get_nodes_by_global_id(self.initial_node.id.clone())
                .into_iter()
                .next()
                .unwrap();

            let curr_container = curr_node.template_parent().unwrap();
            let glass_curr_container = GlassNode::new(&curr_container, &ctx.glass_transform());

            let move_translation = TransformAndBounds {
                transform: Transform2::translate(translation),
                bounds: (1.0, 1.0),
            };

            let glass_curr_node = GlassNode::new(&curr_node, &ctx.glass_transform());
            if let Err(e) = (SetNodeLayout {
                id: &self.initial_node.id,
                node_layout: &NodeLayoutSettings::KeepScreenBounds {
                    node_transform_and_bounds: &(move_translation
                        * self.initial_node.transform_and_bounds),
                    parent_transform_and_bounds: &glass_curr_node.parent_transform_and_bounds.get(),
                    node_decomposition_config: &self
                        .initial_node
                        .layout_properties
                        .into_decomposition_config(),
                },
            }
            .perform(ctx))
            {
                pax_engine::log::error!("Error moving slot object: {:?}", e);
            }

            if let Some((container, slot_hit)) = raycast_slot(ctx, &curr_node, point) {
                let curr_slot = curr_node.render_parent().unwrap();

                let new_index =
                    slot_hit.with_properties(|f: &mut Slot| f.index.get().to_int() as usize);
                let old_index =
                    curr_slot.with_properties(|f: &mut Slot| f.index.get().to_int() as usize);

                if curr_slot == slot_hit && new_index == old_index {
                    return ControlFlow::Continue(());
                }

                let glass_slot_hit = GlassNode::new(&slot_hit, &ctx.glass_transform());
                if let Err(e) = (MoveNode {
                    node_id: &self.initial_node.id,
                    new_parent_uid: &container.global_id().unwrap(),
                    index: pax_engine::pax_manifest::TreeIndexPosition::At(new_index.unwrap()),
                    // TODO try to make this the "future calculated position
                    // after ORM updates" instead. might be possible to
                    // subscribe to manifest changes, and update the bounds
                    // whenever that happens, but still takes 2 ticks (manifest
                    // update -> recalc bounds -> manifest update). Leave as
                    // this for now as the problem is only visible from first
                    // movement over a new slot to the next mouse-move op WHEN
                    // the slot bounds will change as a consequence of movement.
                    node_layout: NodeLayoutSettings::KeepScreenBounds {
                        node_transform_and_bounds: &(move_translation
                            * self.initial_node.transform_and_bounds),
                        parent_transform_and_bounds: &glass_slot_hit.transform_and_bounds.get(),
                        node_decomposition_config: &self
                            .initial_node
                            .layout_properties
                            .into_decomposition_config(),
                    },
                }
                .perform(ctx))
                {
                    log::warn!("failed to swap nodes: {}", e);
                };
            } else if !glass_curr_container
                .transform_and_bounds
                .get()
                .contains_point(point)
                && ctx
                    .engine_context
                    .get_nodes_by_id(ROOT_PROJECT_ID)
                    .into_iter()
                    .next()
                    .unwrap()
                    != curr_container
            {
                let container_parent = curr_container.template_parent().unwrap();
                let container_parent = GlassNode::new(&container_parent, &ctx.glass_transform());
                if let Err(e) = (MoveNode {
                    node_id: &self.initial_node.id,
                    new_parent_uid: &container_parent.id,
                    index: pax_engine::pax_manifest::TreeIndexPosition::Top,
                    node_layout: NodeLayoutSettings::KeepScreenBounds {
                        node_transform_and_bounds: &(move_translation
                            * self.initial_node.transform_and_bounds),
                        parent_transform_and_bounds: &container_parent.transform_and_bounds.get(),
                        node_decomposition_config: &self
                            .initial_node
                            .layout_properties
                            .into_decomposition_config(),
                    },
                }
                .perform(ctx))
                {
                    log::warn!("failed to swap nodes: {}", e);
                };
            }
            ControlFlow::Continue(())
        }

        fn pointer_up(
            &mut self,
            _point: Point2<Glass>,
            ctx: &mut ActionContext,
        ) -> ControlFlow<()> {
            let curr_node = ctx
                .engine_context
                .get_nodes_by_global_id(self.initial_node.id.clone())
                .into_iter()
                .next()
                .unwrap();
            if curr_node.render_parent().unwrap().is_of_type::<Slot>() {
                if let Err(e) = (SetNodeLayout {
                    id: &self.initial_node.id,
                    node_layout: &NodeLayoutSettings::Fill::<Glass>,
                }
                .perform(ctx))
                {
                    log::warn!("failed: {e}")
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

    fn slot_dot_control_factory(slot_child: GlassNode) -> ControlPointBehaviourFactory {
        ControlPointBehaviourFactory {
            tool_behaviour: Rc::new(move |ctx, p| {
                let dt = borrow!(ctx.engine_context.designtime);
                let before_move_undo_id = dt.get_orm().get_last_undo_id().unwrap_or(0);

                // set visualization outline to always be the bounds of the parent of the moving node
                let manifest_ver = dt.get_orm().get_manifest_version();
                let glass_transform = ctx.glass_transform();
                let slot_child_index = slot_child.id.clone();
                let deps = [glass_transform.untyped(), manifest_ver.untyped()];
                let ctx = ctx.engine_context.clone();
                let vis = Property::computed(
                    move || {
                        let slot_child_parent = ctx
                            .get_nodes_by_global_id(slot_child_index.clone())
                            .into_iter()
                            .next()
                            .unwrap()
                            .render_parent()
                            .unwrap();
                        let slot_child_parent =
                            GlassNode::new(&slot_child_parent, &glass_transform);
                        let outline =
                            PathOutline::from_bounds(slot_child_parent.transform_and_bounds.get());
                        ToolVisualizationState {
                            rect_tool: Default::default(),
                            outline,
                        }
                    },
                    &deps,
                );
                Rc::new(RefCell::new(SlotBehaviour {
                    initial_node: (&slot_child).into(),
                    pickup_point: p,
                    _before_move_undo_id: before_move_undo_id,
                    vis,
                }))
            }),
            double_click_behaviour: Rc::new(|_| ()),
        }
    }

    let control_point_styling = ControlPointStyling {
        round: true,
        stroke: Color::RED,
        fill: Color::rgba(255.into(), 255.into(), 255.into(), 150.into()),
        stroke_width_pixels: 2.0,
        affected_by_transform: false,
        width: 15.0,
        height: 15.0,
    };

    let t_and_b = slot_parent_node.transform_and_bounds();
    let deps = [t_and_b.untyped(), slot_count.untyped()];
    Property::computed(
        move || {
            let mut slots = vec![];
            let mut search_set: Vec<NodeInterface> = slot_parent_node.children();
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
            let slot_dot_control_points = slots
                .into_iter()
                .map(|s| {
                    let slot_child = s.children().into_iter().next().unwrap();
                    let slot_child = GlassNode::new(&slot_child, &to_glass);
                    let t_and_b = slot_child.transform_and_bounds.get();
                    CPoint::new(t_and_b.center(), slot_dot_control_factory(slot_child))
                })
                .collect();
            ControlPointSet {
                points: slot_dot_control_points,
                styling: control_point_styling.clone(),
            }
        },
        &deps,
    )
}

pub fn raycast_slot(
    ctx: &ActionContext,
    moving: &NodeInterface,
    point: Point2<Glass>,
) -> Option<(NodeInterface, NodeInterface)> {
    let all_elements_beneath_ray = ctx
        .engine_context
        .raycast(ctx.glass_transform().get().inverse() * point, true);
    let root = ctx
        .engine_context
        .get_nodes_by_id(ROOT_PROJECT_ID)
        .into_iter()
        .next()
        .unwrap();

    let open_containers = ctx.derived_state.open_containers.get();
    let slot_hit = all_elements_beneath_ray
        .into_iter()
        .filter(|n| n.is_descendant_of(&root))
        .filter(|n| !n.is_descendant_of(moving))
        .filter(|n| n.is_of_type::<Slot>())
        .filter(|n| {
            // is either directly in an open container, or one level deep
            open_containers.contains(
                &n.containing_component()
                    .unwrap()
                    .template_parent()
                    .unwrap()
                    .global_id()
                    .unwrap(),
            ) || open_containers.contains(&n.containing_component().unwrap().global_id().unwrap())
        })
        .rev()
        .next()?;

    let container = slot_hit
        .children()
        .first()
        .as_ref()
        .unwrap()
        .template_parent()
        .unwrap();
    Some((container, slot_hit))
}
