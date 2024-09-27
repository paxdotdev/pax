use std::{any::Any, cell::RefCell, ops::ControlFlow, rc::Rc};

use pax_engine::api::{borrow, borrow_mut, Color};
use pax_engine::pax_manifest::UniqueTemplateNodeIdentifier;
use pax_engine::{
    api::NodeContext,
    log,
    math::{Point2, Transform2},
    node_layout::TransformAndBounds,
    NodeInterface, NodeLocal, Property, Slot,
};
use pax_std::stacker::Stacker;

use crate::designer_node_type::designer_behavior_extensions::IntentState;
use crate::designer_node_type::DesignerNodeType;
use crate::glass::intent::IntentDef;
use crate::{
    glass::{
        control_point::{ControlPointStyling, ControlPointToolFactory},
        outline::PathOutline,
        wireframe_editor::editor_generation::CPoint,
        ToolVisualizationState,
    },
    math::{coordinate_spaces::Glass, DecompositionConfiguration, IntoDecompositionConfiguration},
    model::{
        self,
        action::{
            orm::{tree_movement::MoveNode, NodeLayoutSettings, SetNodeLayout},
            Action, ActionContext, RaycastMode,
        },
        input::InputEvent,
        GlassNode, GlassNodeSnapshot, ToolBehavior,
    },
    utils::filter_with_last::FilterWithLastExt,
};

use super::ControlPointSet;

// NOTE: a lot of this logic is very similar to that of movement performed with PointerTool.
// TODO: decide if this should stay as separate logic (if we want different behavior at times)
// or if slot_dot dragging should always behave as normal movement. If so, make this use the
// MovingTool with  a pre-defined target.
pub fn slot_dot_control_set(ctx: NodeContext, item: GlassNode) -> Property<ControlPointSet> {
    struct SlotBehavior {
        initial_node: GlassNodeSnapshot,
        pickup_point: Point2<Glass>,
        _before_move_undo_id: usize,
        drop_intent_state: Vec<(
            Transform2<NodeLocal, Glass>,
            Box<dyn Fn(&[NodeInterface]) -> Box<dyn Action>>,
        )>,
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

    impl ToolBehavior for SlotBehavior {
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

            if let Some(node) = ctx.raycast_glass(
                point,
                // TODO hit things like stackers even with no items in them?
                RaycastMode::Top,
                &[self.initial_node.raw_node_interface.clone()],
            ) {
                // TODO PERF if hit same object as before, skip re-running get_intents, and just do the rest
                let node_type = ctx.designer_node_type(&node.global_id().unwrap());
                let intents = node_type
                    .designer_behavior_extensions()
                    .get_intents(ctx, &node);
                let to_glass_transform = ctx.glass_transform();
                let node_transform = GlassNode::new(&node, &to_glass_transform)
                    .transform_and_bounds
                    .get();
                self.vis.update(|tool_visual| {
                    tool_visual.intent_areas = intents
                        .intent_areas
                        .iter()
                        .find_map(|intent| {
                            let transform = node_transform.transform * intent.draw_area;
                            let hit_transform = node_transform.transform * intent.hit_area;
                            hit_transform.contains_point(point).then(|| {
                                IntentDef::new(
                                    transform,
                                    intent.fill.clone(),
                                    intent.stroke.clone(),
                                )
                            })
                        })
                        .as_slice()
                        .to_vec();
                });
                let drop_intent_state: Vec<_> = intents
                    .intent_areas
                    .into_iter()
                    .map(|intent| {
                        let transform = node_transform.transform * intent.hit_area;
                        (transform, intent.intent_drop_behavior_factory)
                    })
                    .collect();
                self.drop_intent_state = drop_intent_state;
            }
            ControlFlow::Continue(())
        }

        fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
            log::debug!("hit pointer up");
            for (area, action_factory) in &self.drop_intent_state {
                if area.contains_point(point) {
                    // TODO transaction?
                    let action = action_factory(&[self.initial_node.raw_node_interface.clone()]);
                    if let Err(e) = action.perform(ctx) {
                        log::warn!("failed to perform intent movement: {e}");
                    };
                    break;
                }
            }
            ControlFlow::Break(())
        }

        fn finish(&mut self, _ctx: &mut ActionContext) -> anyhow::Result<()> {
            Ok(())
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

    fn slot_dot_control_factory(slot_child: GlassNode) -> ControlPointToolFactory {
        ControlPointToolFactory {
            tool_factory: Rc::new(move |ctx, p| {
                let dt = borrow!(ctx.engine_context.designtime);
                let before_move_undo_id = dt.get_orm().get_last_undo_id().unwrap_or(0);

                Rc::new(RefCell::new(SlotBehavior {
                    initial_node: (&slot_child).into(),
                    pickup_point: p,
                    _before_move_undo_id: before_move_undo_id,
                    drop_intent_state: Default::default(),
                    vis: Property::new(ToolVisualizationState::default()),
                }))
            }),
            double_click_behavior: Rc::new(|_| ()),
        }
    }

    let slot_dot_point_styling = ControlPointStyling {
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
                styling: slot_dot_point_styling.clone(),
            }
        },
        &deps,
    )
}
