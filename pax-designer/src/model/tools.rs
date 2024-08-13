use std::any::Any;
use std::ops::ControlFlow;
use std::rc::Rc;

use super::action::orm::{CreateComponent, SetNodeLayout};
use super::action::pointer::Pointer;
use super::action::{Action, ActionContext, RaycastMode, Transaction};
use super::input::InputEvent;
use super::{GlassNode, GlassNodeSnapshot, SelectionStateSnapshot, StageInfo};
use crate::glass::outline::PathOutline;
use crate::glass::wireframe_editor::editor_generation::slot_control::{
    raycast_slot, wants_slot_behavior,
};
use crate::glass::wireframe_editor::editor_generation::stacker_control::sizes_to_string;
use crate::glass::{RectTool, SnapLines, ToolVisualizationState};
use crate::math::coordinate_spaces::{Glass, World};
use crate::math::intent_snapper::{self, IntentSnapper, SnapSet};
use crate::math::{
    AxisAlignedBox, DecompositionConfiguration, GetUnit, IntoDecompositionConfiguration, SizeUnit,
};
use crate::model::action::orm::{MoveNode, NodeLayoutSettings};
use crate::model::Tool;
use crate::model::{AppState, ToolBehavior};
use crate::{SetStage, ROOT_PROJECT_ID};
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::api::math::Transform2;
use pax_engine::api::Size;
use pax_engine::api::{borrow, borrow_mut, Axis, Window};
use pax_engine::api::{Color, NodeContext};
use pax_engine::layout::{LayoutProperties, TransformAndBounds};
use pax_engine::math::Point2;
use pax_engine::math::Vector2;
use pax_engine::pax_manifest::{
    PaxType, TemplateNodeId, TreeIndexPosition, TypeId, UniqueTemplateNodeIdentifier,
};
use pax_engine::{log, NodeInterface, NodeLocal, Property, Slot};
use pax_std::layout::stacker::Stacker;

pub struct CreateComponentTool {
    type_id: TypeId,
    origin: Point2<Glass>,
    bounds: Property<AxisAlignedBox>,
    mock_children: usize,
    custom_props: &'static [(&'static str, &'static str)],
    intent_snapper: IntentSnapper,
}

impl CreateComponentTool {
    pub fn new(
        point: Point2<Glass>,
        type_id: &TypeId,
        mock_children: usize,
        custom_props: &'static [(&'static str, &'static str)],
        ctx: &ActionContext,
    ) -> Self {
        Self {
            type_id: type_id.clone(),
            origin: point,
            mock_children,
            custom_props,
            bounds: Property::new(AxisAlignedBox::new(Point2::default(), Point2::default())),
            intent_snapper: IntentSnapper::new(ctx, &[]),
        }
    }
}

impl ToolBehavior for CreateComponentTool {
    fn pointer_down(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_move(
        &mut self,
        point: Point2<Glass>,
        ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        let is_shift_key_down = ctx
            .app_state
            .keys_pressed
            .get()
            .contains(&InputEvent::Shift);
        let is_alt_key_down = ctx.app_state.keys_pressed.get().contains(&InputEvent::Alt);
        let offset = self.intent_snapper.snap(&SnapSet::new(&[point]));
        self.bounds.set(
            AxisAlignedBox::new(self.origin, self.origin + Vector2::new(1.0, 1.0))
                .morph_constrained(
                    point + offset,
                    self.origin,
                    is_alt_key_down,
                    is_shift_key_down,
                ),
        );
        ControlFlow::Continue(())
    }

    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        self.pointer_move(point, ctx);
        let bounds = self.bounds.get();
        if bounds.width() < 3.0 || bounds.height() < 3.0 {
            // don't create if to small of a movement
            return ControlFlow::Break(());
        }
        let box_transform = bounds.as_transform();
        let parent = ctx.get_glass_node_by_global_id(&ctx.derived_state.open_container.get());
        let mut t = ctx.transaction("creating object");
        t.run(|| {
            CreateComponent {
                parent_id: &parent.id,
                parent_index: TreeIndexPosition::Top,
                node_layout: NodeLayoutSettings::KeepScreenBounds {
                    node_transform_and_bounds: &TransformAndBounds {
                        transform: box_transform,
                        bounds: (1.0, 1.0),
                    }
                    .as_pure_size(),
                    parent_transform_and_bounds: &parent.transform_and_bounds.get(),
                    node_decomposition_config: &Default::default(),
                },
                type_id: &self.type_id,
                custom_props: self.custom_props,
                mock_children: self.mock_children,
            }
            .perform(ctx)
            .map(|_| ())
        });
        if let Err(e) = t.finish(ctx) {
            log::warn!("failed to create component: {e}");
        }

        ControlFlow::Break(())
    }

    fn keyboard(
        &mut self,
        _event: crate::model::input::InputEvent,
        _dir: crate::model::input::Dir,
        _ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn get_visual(&self) -> Property<ToolVisualizationState> {
        let bounds = self.bounds.clone();
        let snap_lines = self.intent_snapper.get_snap_lines_prop();
        let deps = [bounds.untyped(), snap_lines.untyped()];
        Property::computed(
            move || {
                let bounds = bounds.get();
                ToolVisualizationState {
                    rect_tool: RectTool {
                        x: Size::Pixels(bounds.top_left().x.into()),
                        y: Size::Pixels(bounds.top_left().y.into()),
                        width: Size::Pixels(bounds.width().into()),
                        height: Size::Pixels(bounds.height().into()),
                        stroke: Color::rgba(0.into(), 0.into(), 255.into(), 200.into()),
                        fill: Color::rgba(0.into(), 0.into(), 255.into(), 30.into()),
                    },
                    outline: Default::default(),
                    snap_lines: snap_lines.get(),
                }
            },
            &deps,
        )
    }
}

pub struct SelectNodes<'a> {
    pub ids: &'a [TemplateNodeId],
    //if true, deselects all other objects first
    pub force_deselection_of_others: bool,
}

impl Action for SelectNodes<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let mut ids = ctx.app_state.selected_template_node_ids.get();
        if self.force_deselection_of_others
            || !ctx
                .app_state
                .keys_pressed
                .get()
                .contains(&InputEvent::Shift)
        {
            ids.clear();
        }
        ids.extend_from_slice(self.ids);
        // Only set if changed, otherwise re-triggers when same object gets re-selected
        if ids != ctx.app_state.selected_template_node_ids.get() {
            ctx.app_state.selected_template_node_ids.set(ids);
        }
        Ok(())
    }
}

pub struct PointerTool {
    action: PointerToolAction,
}

pub enum PointerToolAction {
    Moving {
        has_moved: bool,
        hit: NodeInterface,
        pickup_point: Point2<Glass>,
        initial_selection: SelectionStateSnapshot,
        intent_snapper: IntentSnapper,
        vis: Property<ToolVisualizationState>,
        transaction: Option<Transaction>,
    },
    Selecting {
        p1: Point2<Glass>,
        p2: Point2<Glass>,
    },
    ResizingStage(ResizeStageDim),
}

pub enum ResizeStageDim {
    Height,
    Width,
}

impl PointerTool {
    pub fn new(ctx: &mut ActionContext, point: Point2<Glass>) -> Self {
        if let Some(hit) = ctx.raycast_glass(point, RaycastMode::Top, &[]) {
            let node_id = hit.global_id().unwrap();
            let selected = ctx.derived_state.selection_state.get();
            if !selected.items.iter().any(|s| s.id == node_id) {
                let _ = SelectNodes {
                    ids: &[node_id.get_template_node_id()],
                    force_deselection_of_others: false,
                }
                .perform(ctx);
            }

            let intent_snapper = IntentSnapper::new(&ctx, &[hit.global_id().unwrap()]);

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
                        let slot_child_parent =
                            GlassNode::new(&slot_child_parent, &glass_transform);
                        let outline =
                            PathOutline::from_bounds(slot_child_parent.transform_and_bounds.get());
                        ToolVisualizationState {
                            rect_tool: Default::default(),
                            outline,
                            snap_lines: snap_lines.get(), // TODO snapline impl
                        }
                    } else {
                        Default::default()
                    }
                },
                &deps,
            );

            let selection = ctx.derived_state.selection_state.get();
            Self {
                action: PointerToolAction::Moving {
                    hit,
                    has_moved: false,
                    pickup_point: point,
                    initial_selection: (&selection).into(),
                    intent_snapper,
                    vis,
                    transaction: None,
                },
            }
        } else {
            // resize stage if we are at edge
            let stage = ctx.app_state.stage.get();
            let world_point = ctx.world_transform() * point;
            if (world_point.y - stage.height as f64).abs() < 10.0 {
                Self {
                    action: PointerToolAction::ResizingStage(ResizeStageDim::Height),
                }
            } else if (world_point.x - stage.width as f64).abs() < 10.0 {
                Self {
                    action: PointerToolAction::ResizingStage(ResizeStageDim::Width),
                }
            } else {
                Self {
                    action: PointerToolAction::Selecting {
                        p1: point,
                        p2: point,
                    },
                }
            }
        }
    }
}

impl ToolBehavior for PointerTool {
    fn pointer_down(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_move(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        match &mut self.action {
            &mut PointerToolAction::Moving {
                pickup_point,
                ref initial_selection,
                ref mut has_moved,
                ref intent_snapper,
                ref mut transaction,
                ..
            } => {
                if (pickup_point - point).length_squared() < 3.0 {
                    // don't commit any movement for very small pixel changes,
                    // this creates designtime changes that
                    // make double click behavior for for example
                    // text editing not work
                    return ControlFlow::Continue(());
                }

                if !*has_moved {
                    *transaction = Some(ctx.transaction("moving object"));
                }
                let Some(transaction) = transaction else {
                    unreachable!("assigned above, now or in last iteration")
                };
                *has_moved = true;
                let translation = point - pickup_point;

                let potential_move_translation = TransformAndBounds {
                    transform: Transform2::translate(translation),
                    bounds: (1.0, 1.0),
                };

                let potential_new_bounds =
                    potential_move_translation * initial_selection.total_bounds;
                let snap_set = SnapSet::from_transform_and_bounds(potential_new_bounds);
                let offset = intent_snapper.snap(&snap_set);
                let move_translation = TransformAndBounds {
                    transform: Transform2::translate(translation + offset),
                    bounds: (1.0, 1.0),
                };

                let unit = match ctx.app_state.keys_pressed.get().contains(&InputEvent::Meta) {
                    true => SizeUnit::Pixels,
                    false => SizeUnit::Percent,
                };
                transaction.run(|| {
                    for item in &initial_selection.items {
                        let curr_node = ctx.get_glass_node_by_global_id(&item.id);

                        SetNodeLayout {
                            id: &item.id,
                            node_layout: &NodeLayoutSettings::KeepScreenBounds {
                                node_transform_and_bounds: &(move_translation
                                    * item.transform_and_bounds),
                                parent_transform_and_bounds: &curr_node
                                    .parent_transform_and_bounds
                                    .get(),
                                node_decomposition_config: &DecompositionConfiguration {
                                    unit_x_pos: unit,
                                    unit_y_pos: unit,
                                    ..item.layout_properties.into_decomposition_config()
                                },
                            },
                        }
                        .perform(ctx)?
                    }
                    Ok(())
                });

                if initial_selection.items.len() != 1 {
                    return ControlFlow::Continue(());
                }

                let item = initial_selection.items[0].clone();

                let curr_node = ctx.get_glass_node_by_global_id(&item.id);
                let curr_slot = curr_node.raw_node_interface.render_parent().unwrap();
                let curr_render_container_glass =
                    GlassNode::new(&curr_slot, &ctx.glass_transform());

                let move_translation = TransformAndBounds {
                    transform: Transform2::translate(translation),
                    bounds: (1.0, 1.0),
                };

                if let Some((container, slot_hit)) =
                    raycast_slot(ctx, &curr_node.raw_node_interface, point)
                {
                    let new_index =
                        slot_hit.with_properties(|f: &mut Slot| f.index.get().to_int() as usize);
                    let old_index =
                        curr_slot.with_properties(|f: &mut Slot| f.index.get().to_int() as usize);

                    if curr_slot == slot_hit && new_index == old_index {
                        return ControlFlow::Continue(());
                    }

                    let glass_slot_hit = GlassNode::new(&slot_hit, &ctx.glass_transform());
                    transaction.run(|| {
                        MoveNode {
                            node_id: &item.id,
                            new_parent_uid: &container.global_id().unwrap(),
                            index: pax_engine::pax_manifest::TreeIndexPosition::At(
                                new_index.unwrap(),
                            ),
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
                                    * item.transform_and_bounds),
                                parent_transform_and_bounds: &glass_slot_hit
                                    .transform_and_bounds
                                    .get(),
                                node_decomposition_config: &item
                                    .layout_properties
                                    .into_decomposition_config(),
                            },
                        }
                        .perform(ctx)
                    });
                } else if !curr_render_container_glass
                    .transform_and_bounds
                    .get()
                    .contains_point(point)
                    && ctx
                        .engine_context
                        .get_nodes_by_id(ROOT_PROJECT_ID)
                        .into_iter()
                        .next()
                        .unwrap()
                        != curr_slot
                {
                    let container_parent = curr_node
                        .raw_node_interface
                        .template_parent()
                        .unwrap()
                        .template_parent()
                        .unwrap();
                    let container_parent =
                        GlassNode::new(&container_parent, &ctx.glass_transform());
                    transaction.run(|| {
                        MoveNode {
                            node_id: &item.id,
                            new_parent_uid: &container_parent.id,
                            index: pax_engine::pax_manifest::TreeIndexPosition::Top,
                            node_layout: NodeLayoutSettings::KeepScreenBounds {
                                node_transform_and_bounds: &(move_translation
                                    * item.transform_and_bounds),
                                parent_transform_and_bounds: &container_parent
                                    .transform_and_bounds
                                    .get(),
                                node_decomposition_config: &item
                                    .layout_properties
                                    .into_decomposition_config(),
                            },
                        }
                        .perform(ctx)
                    });
                }
            }
            PointerToolAction::Selecting { ref mut p2, .. } => *p2 = point,
            PointerToolAction::ResizingStage(dir) => {
                let world_point = ctx.world_transform() * point;
                let size_before = ctx.app_state.stage.get();
                let (new_width, new_height) = match dir {
                    ResizeStageDim::Height => (size_before.width, world_point.y as u32),
                    ResizeStageDim::Width => (world_point.x as u32, size_before.height),
                };
                SetStage(StageInfo {
                    width: new_width,
                    height: new_height,
                    color: Color::WHITE,
                })
                .perform(ctx)
                .unwrap();
            }
        }
        ControlFlow::Continue(())
    }

    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        // move last little distance to pointer up position
        self.pointer_move(point, ctx);

        match &mut self.action {
            PointerToolAction::Selecting { .. } => {
                // TODO select multiple objects
                let _ = SelectNodes {
                    ids: &[],
                    force_deselection_of_others: false,
                }
                .perform(ctx);
            }
            PointerToolAction::Moving {
                has_moved,
                ref hit,
                ref mut transaction,
                ..
            } => {
                if *has_moved {
                    let transaction = transaction.as_mut().unwrap();
                    let hit_id = hit.global_id().unwrap();
                    let curr_node = ctx.get_glass_node_by_global_id(&hit_id);
                    if curr_node
                        .raw_node_interface
                        .render_parent()
                        .unwrap()
                        .is_of_type::<Slot>()
                        && wants_slot_behavior(
                            &&curr_node.raw_node_interface.template_parent().unwrap(),
                        )
                    {
                        transaction.run(|| {
                            SetNodeLayout {
                                id: &hit_id,
                                node_layout: &NodeLayoutSettings::Fill::<Glass>,
                            }
                            .perform(ctx)
                        });
                    }
                    if let Err(e) = transaction.finish(ctx) {
                        log::warn!("move failed: {e}");
                    }
                } else {
                    let _ = SelectNodes {
                        ids: &[hit.global_id().unwrap().get_template_node_id()],
                        force_deselection_of_others: false,
                    }
                    .perform(ctx);
                }
            }
            PointerToolAction::ResizingStage(_dir) => {}
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
        match &self.action {
            PointerToolAction::Moving { vis, .. } => {
                let vis = vis.clone();
                let deps = [vis.untyped()];
                Property::computed(move || vis.get(), &deps)
            }
            _ => Property::new(ToolVisualizationState::default()),
        }
    }
}
