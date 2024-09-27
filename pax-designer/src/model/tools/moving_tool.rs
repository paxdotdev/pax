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

pub struct MovingTool {
    hit: NodeInterface,
    pickup_point: Point2<Glass>,
    initial_selection: SelectionStateSnapshot,
    intent_snapper: IntentSnapper,
    vis: Property<ToolVisualizationState>,
    transaction: Option<Transaction>,
    node_hit_was_selected_before: bool,
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

        let intent_snapper = IntentSnapper::new_from_scene(&ctx, &[hit.global_id().unwrap()]);

        // set visualization outline to always be the bounds of the parent of the moving node
        let dt = borrow!(ctx.engine_context.designtime);
        let manifest_ver = dt.get_orm().get_manifest_version();
        let glass_transform = ctx.glass_transform();
        let slot_child_index = hit.global_id().unwrap().clone();
        let snap_lines = intent_snapper.get_snap_lines_prop();
        let deps = [
            glass_transform.untyped(),
            manifest_ver.untyped(),
            snap_lines.untyped(),
        ];
        let ctx_e = ctx.engine_context.clone();
        let vis = Property::computed(
            move || {
                if let Some(slot_child_parent) = ctx_e
                    .get_nodes_by_global_id(slot_child_index.clone())
                    .into_iter()
                    .next()
                    .and_then(|n| n.render_parent())
                {
                    let slot_child_parent = GlassNode::new(&slot_child_parent, &glass_transform);
                    let outline =
                        PathOutline::from_bounds(slot_child_parent.transform_and_bounds.get());
                    ToolVisualizationState {
                        outline,
                        snap_lines: snap_lines.get(),
                        ..Default::default()
                    }
                } else {
                    Default::default()
                }
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
                    let subtrees = orm::Copy {
                        ids: &self
                            .initial_selection
                            .items
                            .iter()
                            .map(|i| i.id.get_template_node_id())
                            .collect::<Vec<_>>(),
                    }
                    .perform(ctx)?;
                    let ids = orm::Paste {
                        subtrees: &subtrees,
                    }
                    .perform(ctx);
                    ids
                });

                // if copy succeeded, change the ids of the snapshot to those of the new ids
                // (want to move these instead of old)
                if let Ok(ids) = ids {
                    let comp_id = ctx.app_state.selected_component_id.get();
                    for (i, id) in ids.into_iter().enumerate() {
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

        ControlFlow::Continue(())
    }

    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        // move last little distance to pointer up position
        self.pointer_move(point, ctx);
        if self.transaction.is_none() && self.node_hit_was_selected_before {
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
