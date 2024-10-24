use std::ops::ControlFlow;

use pax_engine::{
    api::borrow,
    log,
    math::{Point2, Transform2},
    pax_manifest::UniqueTemplateNodeIdentifier,
    pax_runtime::TransformAndBounds,
    NodeInterface, Property, Slot,
};

use crate::{
    glass::{outline::PathOutline, ToolVisualizationState},
    math::{
        coordinate_spaces::Glass, intent_snapper::IntentSnapper, IntoDecompositionConfiguration,
    },
    model::{
        action::{
            orm::{
                self, space_movement::TranslateFromSnapshot, tree_movement::MoveNode,
                NodeLayoutSettings, SetNodeLayout,
            },
            world::{SelectMode, SelectNodes},
            Action, ActionContext, Transaction,
        },
        input::ModifierKey,
        GlassNode, SelectionStateSnapshot, ToolBehavior,
    },
};

use super::tool_plugins::drop_intent_handler::DropIntentHandler;

pub struct MovingTool {
    hit: NodeInterface,
    pickup_point: Point2<Glass>,
    initial_selection: SelectionStateSnapshot,
    intent_snapper: IntentSnapper,
    vis: Property<ToolVisualizationState>,
    transaction: Option<Transaction>,
    node_hit_was_selected_before: bool,
    drop_intent_handler: DropIntentHandler,
}

impl MovingTool {
    pub fn new(ctx: &mut ActionContext, point: Point2<Glass>, hit: NodeInterface) -> Self {
        let node_id = hit.global_id().unwrap();
        let selected = ctx.derived_state.selection_state.get();
        let node_hit_was_selected = selected.items.iter().any(|s| s.id == node_id);
        if !node_hit_was_selected {
            let _ = SelectNodes {
                ids: &[node_id.get_template_node_id()],
                mode: SelectMode::Dynamic,
            }
            .perform(ctx);
        }
        let selected = ctx.derived_state.selection_state.get();

        let intent_snapper = IntentSnapper::new_from_scene(&ctx, &[hit.global_id().unwrap()]);
        let drop_intent_handler = DropIntentHandler::new(
            &selected
                .items
                .iter()
                .map(|n| n.raw_node_interface.clone())
                .collect::<Vec<_>>(),
        );

        // set visualization outline to always be the bounds of the parent of the moving node
        let snap_lines = intent_snapper.get_snap_lines_prop();
        let intent_areas = drop_intent_handler.get_intent_areas_prop();
        let deps = [snap_lines.untyped(), intent_areas.untyped()];
        let vis = Property::computed(
            move || ToolVisualizationState {
                snap_lines: snap_lines.get(),
                intent_areas: intent_areas.get(),
                ..Default::default()
            },
            &deps,
        );

        let selection = ctx.derived_state.selection_state.get();
        Self {
            hit,
            pickup_point: point,
            initial_selection: (&selection).into(),
            intent_snapper,
            vis,
            transaction: None,
            node_hit_was_selected_before: node_hit_was_selected,
            drop_intent_handler,
        }
    }
}

impl ToolBehavior for MovingTool {
    fn pointer_down(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_move(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        if (self.pickup_point - point).length_squared() < 3.0 {
            // don't commit any movement for very small pixel changes,
            // this creates designtime changes that
            // make double click behavior for for example
            // text editing not work
            return ControlFlow::Continue(());
        }

        let transaction = self.transaction.get_or_insert_with(|| {
            let t = ctx.transaction("moving object");
            if ctx.app_state.modifiers.get().contains(&ModifierKey::Alt) {
                //copy paste object and leave newly created object behind
                let ids = t.run(|| {
                    let subtrees = orm::copy_paste::Copy {
                        ids: &self
                            .initial_selection
                            .items
                            .iter()
                            .map(|i| i.id.get_template_node_id())
                            .collect::<Vec<_>>(),
                    }
                    .perform(ctx)?;
                    let ids = orm::copy_paste::Paste {
                        subtrees: &subtrees,
                    }
                    .perform(ctx);
                    ids
                });

                // if copy succeeded, change the ids of the snapshot to those of the new ids
                // (want to move these instead of old)
                if let Ok(ids) = ids {
                    let comp_id = ctx.app_state.selected_component_id.get();
                    for (i, id) in ids.into_iter().rev().enumerate() {
                        self.initial_selection.items[i].id =
                            UniqueTemplateNodeIdentifier::build(comp_id.clone(), id);
                    }
                }
            }
            t
        });

        let translation = point - self.pickup_point;

        let potential_move_translation = TransformAndBounds {
            transform: Transform2::translate(translation),
            bounds: (1.0, 1.0),
        };

        let potential_new_bounds = potential_move_translation * self.initial_selection.total_bounds;
        let mut points_to_snap = Vec::new();
        points_to_snap.extend(potential_new_bounds.corners());
        points_to_snap.push(potential_new_bounds.center());
        let mut lock_x = false;
        let mut lock_y = false;
        if ctx.app_state.modifiers.get().contains(&ModifierKey::Shift) {
            if translation.x.abs() > translation.y.abs() {
                lock_y = true;
            } else {
                lock_x = true;
            }
        }
        let offset = self.intent_snapper.snap(&points_to_snap, lock_x, lock_y);
        let mut total_translation = translation + offset;
        if lock_x {
            total_translation.x = 0.0;
        }
        if lock_y {
            total_translation.y = 0.0;
        }

        if let Err(e) = transaction.run(|| {
            TranslateFromSnapshot {
                translation: total_translation,
                initial_selection: &self.initial_selection,
            }
            .perform(ctx)
        }) {
            log::warn!("failed to move: {e}");
            return ControlFlow::Break(());
        };

        self.drop_intent_handler.update(ctx, point);
        ControlFlow::Continue(())
    }

    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        // move last little distance to pointer up position
        self.pointer_move(point, ctx);

        if let Some(transaction) = &self.transaction {
            let _ = transaction.run(|| {
                self.drop_intent_handler.handle_drop(ctx);
                Ok(())
            });
        } else if self.node_hit_was_selected_before {
            let _ = SelectNodes {
                ids: &[self.hit.global_id().unwrap().get_template_node_id()],
                mode: SelectMode::Dynamic,
            }
            .perform(ctx);
        }

        ControlFlow::Break(())
    }

    fn keyboard(
        &mut self,
        _event: crate::model::input::InputEvent,
        _dir: crate::model::input::Dir,
        _ctx: &mut ActionContext,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn get_visual(&self) -> Property<ToolVisualizationState> {
        let vis = self.vis.clone();
        let deps = [vis.untyped()];
        Property::computed(move || vis.get(), &deps)
    }

    fn finish(&mut self, _ctx: &mut ActionContext) -> anyhow::Result<()> {
        Ok(())
    }
}
