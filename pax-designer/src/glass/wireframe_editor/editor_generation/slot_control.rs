use std::{any::Any, cell::RefCell, ops::ControlFlow, rc::Rc};

use model::tools::tool_plugins::drop_intent_handler::DropIntentHandler;
use pax_engine::api::{borrow, borrow_mut, Color};
use pax_engine::math::Vector2;
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
use crate::model::tools::ToolBehavior;
use crate::utils::designer_cursor::DesignerCursorType;
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
        GlassNode, GlassNodeSnapshot,
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
        drop_intent_handler: DropIntentHandler,
        vis: Property<ToolVisualizationState>,
        transaction: model::action::Transaction,
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
            let _ = self.transaction.run(|| {
                SetNodeLayout {
                    id: &self.initial_node.id,
                    node_layout: &NodeLayoutSettings::KeepScreenBounds {
                        node_transform_and_bounds: &(move_translation
                            * self.initial_node.transform_and_bounds),
                        parent_transform_and_bounds: &glass_curr_node
                            .parent_transform_and_bounds
                            .get(),
                        node_decomposition_config: &self
                            .initial_node
                            .layout_properties
                            .into_decomposition_config(),
                    },
                }
                .perform(ctx)
            });

            self.drop_intent_handler.update(ctx, point);
            ControlFlow::Continue(())
        }

        fn pointer_up(
            &mut self,
            _point: Point2<Glass>,
            ctx: &mut ActionContext,
        ) -> ControlFlow<()> {
            if let Err(e) = self.transaction.run(|| {
                self.drop_intent_handler.handle_drop(ctx);
                Ok(())
            }) {
                log::warn!("failed slot dot movement: {e}");
            };
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
                let drop_intent_handler =
                    DropIntentHandler::new(&[slot_child.raw_node_interface.clone()]);
                let intent_areas = drop_intent_handler.get_intent_areas_prop();
                let deps = [intent_areas.untyped()];
                let transaction = ctx.transaction("slot dot move");

                let vis = Property::computed(
                    move || ToolVisualizationState {
                        intent_areas: intent_areas.get(),
                        ..Default::default()
                    },
                    &deps,
                );
                Rc::new(RefCell::new(SlotBehavior {
                    initial_node: (&slot_child).into(),
                    pickup_point: p,
                    vis,
                    drop_intent_handler,
                    transaction,
                }))
            }),
            double_click_behavior: Rc::new(|_| ()),
        }
    }

    let slot_dot_point_styling = ControlPointStyling {
        round: true,
        stroke_color: Color::RED,
        fill_color: Color::rgba(255.into(), 255.into(), 255.into(), 150.into()),
        stroke_width_pixels: 1.0,
        affected_by_transform: false,
        width: 14.0,
        height: 14.0,
        cursor_type: DesignerCursorType::Move,
        hit_padding: 10.0,
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
                    CPoint {
                        point: t_and_b.center(),
                        behavior: slot_dot_control_factory(slot_child),
                        rotation: 0.0,
                        ..Default::default()
                    }
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
