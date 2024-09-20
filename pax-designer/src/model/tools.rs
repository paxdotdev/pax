use std::any::Any;
use std::ops::ControlFlow;
use std::rc::Rc;

use super::action::meta::Schedule;
use super::action::orm::space_movement::TranslateFromSnapshot;
use super::action::orm::{self, CreateComponent, SetNodeLayout};
use super::action::pointer::Pointer;
use super::action::world::ZoomToFit;
use super::action::{Action, ActionContext, RaycastMode, Transaction};
use super::input::{InputEvent, ModifierKey};
use super::{GlassNode, GlassNodeSnapshot, SelectionStateSnapshot, StageInfo};
use crate::designer_node_type::{self, DesignerNodeType};
use crate::glass::outline::PathOutline;
use crate::glass::wireframe_editor::editor_generation::slot_control::raycast_slot;
use crate::glass::wireframe_editor::editor_generation::stacker_control::sizes_to_string;
use crate::glass::{RectTool, SnapInfo, TextEdit, ToolVisualizationState};
use crate::math::coordinate_spaces::{Glass, World};
use crate::math::intent_snapper::{self, IntentSnapper, SnapSet};
use crate::math::{
    AxisAlignedBox, DecompositionConfiguration, GetUnit, IntoDecompositionConfiguration, SizeUnit,
};
use crate::model::action::orm::{tree_movement::MoveNode, NodeLayoutSettings};
use crate::model::Tool;
use crate::model::{AppState, ToolBehavior};
use crate::SetStage;
use anyhow::{anyhow, Result};
use pax_designtime::orm::template::builder::NodeBuilder;
use pax_designtime::DesigntimeManager;
use pax_engine::api::math::Transform2;
use pax_engine::api::Size;
use pax_engine::api::{borrow, borrow_mut, Axis, Window};
use pax_engine::api::{Color, NodeContext};
use pax_engine::math::Point2;
use pax_engine::math::Vector2;
use pax_engine::node_layout::{LayoutProperties, TransformAndBounds};
use pax_engine::pax_manifest::{
    PaxType, TemplateNodeId, TreeIndexPosition, TypeId, UniqueTemplateNodeIdentifier, Unit,
};
use pax_engine::{log, NodeInterface, NodeLocal, Property, Slot};
use pax_std::layout::stacker::Stacker;

pub struct PostCreationData<'a> {
    pub uid: &'a UniqueTemplateNodeIdentifier,
    pub bounds: &'a AxisAlignedBox<Glass>,
}

pub struct CreateComponentTool {
    designer_node_type: DesignerNodeType,
    origin: Point2<Glass>,
    bounds: Property<AxisAlignedBox>,
    builder_extra_commands: Option<Box<dyn Fn(&mut NodeBuilder) -> Result<()>>>,
    post_creation_hook: Option<Box<dyn Fn(&mut ActionContext, PostCreationData) -> Result<()>>>,
    intent_snapper: IntentSnapper,
}

impl CreateComponentTool {
    pub fn new(
        point: Point2<Glass>,
        designer_node_type: DesignerNodeType,
        ctx: &ActionContext,
    ) -> Self {
        Self {
            designer_node_type,
            origin: point,
            bounds: Property::new(AxisAlignedBox::new(Point2::default(), Point2::default())),
            intent_snapper: IntentSnapper::new_from_scene(ctx, &[]),
            builder_extra_commands: None,
            post_creation_hook: None,
        }
    }

    // WARNING: When this is called, the node exists in the manifest, but NOT in the engine
    // NOTE: This function is run inside the transaction performed while creating the component
    pub fn with_post_creation_hook(
        mut self,
        post_creation: impl Fn(&mut ActionContext, PostCreationData) -> Result<()> + 'static,
    ) -> Self {
        self.post_creation_hook = Some(Box::new(post_creation));
        self
    }

    pub fn with_extra_builder_commands(
        mut self,
        cmds: impl Fn(&mut NodeBuilder) -> Result<()> + 'static,
    ) -> Self {
        self.builder_extra_commands = Some(Box::new(cmds));
        self
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
        let modifiers = ctx.app_state.modifiers.get();
        let is_shift_key_down = modifiers.contains(&ModifierKey::Shift);
        let is_alt_key_down = modifiers.contains(&ModifierKey::Alt);
        let offset = self.intent_snapper.snap(&[point], false, false);
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
        let Ok(parent) =
            ctx.get_glass_node_by_global_id(&ctx.derived_state.open_container.get()[0].clone())
        else {
            return ControlFlow::Continue(());
        };
        let unit = ctx.app_state.unit_mode.get();
        let t = ctx.transaction("creating object");
        let _ = t.run(|| {
            let uid = CreateComponent {
                parent_id: &parent.id,
                parent_index: TreeIndexPosition::Top,
                node_layout: NodeLayoutSettings::KeepScreenBounds {
                    node_transform_and_bounds: &TransformAndBounds {
                        transform: box_transform,
                        bounds: (1.0, 1.0),
                    }
                    .as_pure_size(),
                    parent_transform_and_bounds: &parent.transform_and_bounds.get().as_pure_size(),
                    node_decomposition_config: &DecompositionConfiguration {
                        unit_x_pos: unit,
                        unit_y_pos: unit,
                        unit_width: unit,
                        unit_height: unit,
                        ..Default::default()
                    },
                },
                designer_node_type: self.designer_node_type.clone(),
                builder_extra_commands: self.builder_extra_commands.as_ref().map(|v| v.as_ref()),
            }
            .perform(ctx)?;

            if let Some(post_creation) = &self.post_creation_hook {
                post_creation(
                    ctx,
                    PostCreationData {
                        uid: &uid,
                        bounds: &bounds,
                    },
                )?;
            }
            SelectNodes {
                ids: &[uid.get_template_node_id()],
                mode: SelectMode::DiscardOthers,
            }
            .perform(ctx)?;

            Ok(())
        });
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
                    event_blocker_active: true,
                }
            },
            &deps,
        )
    }

    fn finish(&mut self, _ctx: &mut ActionContext) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct SelectNodes<'a> {
    pub ids: &'a [TemplateNodeId],
    //if true, deselects all other objects first
    pub mode: SelectMode,
}

// TODO use this
pub enum SelectMode {
    KeepOthers,
    DiscardOthers,
    Dynamic,
}

impl Action for SelectNodes<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let mut ids = ctx.app_state.selected_template_node_ids.get();
        let deselect_others = match self.mode {
            SelectMode::KeepOthers => false,
            SelectMode::DiscardOthers => true,
            SelectMode::Dynamic => !ctx.app_state.modifiers.get().contains(&ModifierKey::Shift),
        };
        if deselect_others {
            ids.clear();
        }
        // not efficient but should never be large sets
        for id in self.ids {
            if ids.contains(id) {
                ids.retain(|e| e != id);
            } else {
                ids.push(id.clone());
            }
        }
        // Only set if changed, otherwise re-triggers when same object gets re-selected
        if ids != ctx.app_state.selected_template_node_ids.get() {
            ctx.app_state.selected_template_node_ids.set(ids);
        }
        Ok(())
    }
}

pub struct MovingTool {
    has_moved: bool,
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
                        rect_tool: Default::default(),
                        outline,
                        snap_lines: snap_lines.get(),
                        event_blocker_active: true,
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
            has_moved: false,
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

        if !self.has_moved {
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
            self.transaction = Some(t);
        }
        let Some(transaction) = &self.transaction else {
            unreachable!("assigned above, now or in last iteration")
        };
        self.has_moved = true;
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

        if self.initial_selection.items.len() != 1 {
            return ControlFlow::Continue(());
        }

        let item = self.initial_selection.items[0].clone();

        let Ok(curr_node) = ctx.get_glass_node_by_global_id(&item.id) else {
            return ControlFlow::Continue(());
        };
        let curr_slot = curr_node.raw_node_interface.render_parent().unwrap();
        let curr_render_container_glass = GlassNode::new(&curr_slot, &ctx.glass_transform());

        let move_translation = TransformAndBounds {
            transform: Transform2::translate(total_translation),
            bounds: (1.0, 1.0),
        };

        if let Some((container, slot_hit)) = raycast_slot(ctx, &curr_node.raw_node_interface, point)
        {
            let new_index =
                slot_hit.with_properties(|f: &mut Slot| f.index.get().to_int() as usize);
            let old_index =
                curr_slot.with_properties(|f: &mut Slot| f.index.get().to_int() as usize);

            if curr_slot == slot_hit && new_index == old_index {
                return ControlFlow::Continue(());
            }

            let glass_slot_hit = GlassNode::new(&slot_hit, &ctx.glass_transform());
            let _ = transaction.run(|| {
                MoveNode {
                    node_id: &item.id,
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
                        node_transform_and_bounds: &(move_translation * item.transform_and_bounds),
                        parent_transform_and_bounds: &glass_slot_hit.transform_and_bounds.get(),
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
                .get_userland_root_expanded_node()
                .is_some_and(|n| n != curr_slot)
        {
            let container_parent = curr_node
                .raw_node_interface
                .template_parent()
                .unwrap()
                .template_parent()
                .unwrap();
            let container_parent = GlassNode::new(&container_parent, &ctx.glass_transform());
            if let Err(_) = transaction.run(|| {
                MoveNode {
                    node_id: &item.id,
                    new_parent_uid: &container_parent.id,
                    index: pax_engine::pax_manifest::TreeIndexPosition::Top,
                    node_layout: NodeLayoutSettings::KeepScreenBounds {
                        node_transform_and_bounds: &(move_translation * item.transform_and_bounds),
                        parent_transform_and_bounds: &container_parent.transform_and_bounds.get(),
                        node_decomposition_config: &item
                            .layout_properties
                            .into_decomposition_config(),
                    },
                }
                .perform(ctx)
            }) {
                return ControlFlow::Break(());
            };
        }

        ControlFlow::Continue(())
    }

    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        // move last little distance to pointer up position
        self.pointer_move(point, ctx);

        if self.has_moved {
            let transaction = self.transaction.as_mut().unwrap();
            let hit_id = self.hit.global_id().unwrap();
            let Ok(curr_node) = ctx.get_glass_node_by_global_id(&hit_id) else {
                return ControlFlow::Continue(());
            };
            let curr_node_parent_id = curr_node
                .raw_node_interface
                .template_parent()
                .and_then(|p| p.global_id())
                .unwrap();
            let node_type = ctx.designer_node_type(&curr_node_parent_id);
            let is_slot_container = {
                let dt = borrow!(ctx.engine_context.designtime);
                let orm = dt.get_orm();
                node_type.metadata(&orm).is_slot_container
            };
            if curr_node
                .raw_node_interface
                .render_parent()
                .unwrap()
                .is_of_type::<Slot>()
                // replace want's slot behaviour with DesignerNodeType:: .metadata().is_slot_container?
                && is_slot_container
            {
                let _ = transaction.run(|| {
                    SetNodeLayout {
                        id: &hit_id,
                        node_layout: &NodeLayoutSettings::Fill::<Glass>,
                    }
                    .perform(ctx)
                });
            }
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

pub struct MultiSelectTool {
    p1: Point2<Glass>,
    bounds: Property<AxisAlignedBox>,
    last_set: Vec<TemplateNodeId>,
}
impl MultiSelectTool {
    pub fn new(ctx: &mut ActionContext, point: Point2<Glass>) -> Self {
        if let Err(e) = (SelectNodes {
            ids: &[],
            mode: SelectMode::Dynamic,
        }
        .perform(ctx))
        {
            log::warn!("failed multi-select pointer up: {e}");
        };
        Self {
            p1: point,
            bounds: Property::new(AxisAlignedBox::new(point, point)),
            last_set: Default::default(),
        }
    }
}

impl ToolBehavior for MultiSelectTool {
    fn pointer_down(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_move(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        self.bounds.set(AxisAlignedBox::new(self.p1, point));
        let Some(project_root) = ctx.engine_context.get_userland_root_expanded_node() else {
            log::warn!("coudln't find userland root expanded node");
            return ControlFlow::Break(());
        };
        let selection_box = TransformAndBounds {
            transform: self.bounds.get().as_transform(),
            bounds: (1.0, 1.0),
        };
        let glass_transform = ctx.glass_transform();
        let open_container = ctx.derived_state.open_container.get()[0].clone();
        let mut to_process = project_root.children();
        let mut hits = vec![];
        while let Some(node) = to_process.pop() {
            if node.global_id().unwrap() == open_container {
                to_process.extend(node.children());
                continue;
            }
            let t_and_b = TransformAndBounds {
                transform: glass_transform.get(),
                bounds: (1.0, 1.0),
            } * node.transform_and_bounds().get();
            if t_and_b.intersects(&selection_box) {
                hits.push(node.global_id().unwrap().get_template_node_id());
            }
        }

        let mut newly_selected_nodes = hits.clone();
        newly_selected_nodes.retain(|e| !self.last_set.contains(e));
        let mut newly_deselected_nodes = self.last_set.clone();
        newly_deselected_nodes.retain(|e| !hits.contains(e));
        let to_toggle: Vec<_> = newly_selected_nodes
            .into_iter()
            .chain(newly_deselected_nodes.into_iter())
            .collect();
        if !to_toggle.is_empty() {
            if let Err(e) = (SelectNodes {
                ids: &to_toggle,
                mode: SelectMode::KeepOthers,
            }
            .perform(ctx))
            {
                log::warn!("failed to multi-select nodes: {e}");
            };
            self.last_set = hits;
        }
        ControlFlow::Continue(())
    }

    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        self.pointer_move(point, ctx);
        ControlFlow::Break(())
    }

    fn keyboard(
        &mut self,
        _event: InputEvent,
        _dir: super::input::Dir,
        _ctx: &mut ActionContext,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn get_visual(&self) -> Property<ToolVisualizationState> {
        let bounds = self.bounds.clone();
        let deps = [bounds.untyped()];
        Property::computed(
            move || {
                let bounds = bounds.get();
                ToolVisualizationState {
                    rect_tool: RectTool {
                        x: Size::Pixels(bounds.top_left().x.into()),
                        y: Size::Pixels(bounds.top_left().y.into()),
                        width: Size::Pixels(bounds.width().into()),
                        height: Size::Pixels(bounds.height().into()),
                        stroke: Color::rgba(50.into(), 50.into(), 100.into(), 200.into()),
                        fill: Color::rgba(100.into(), 100.into(), 255.into(), 30.into()),
                    },
                    outline: Default::default(),
                    snap_lines: Default::default(),
                    event_blocker_active: true,
                }
            },
            &deps,
        )
    }

    fn finish(&mut self, _ctx: &mut ActionContext) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct ZoomToFitTool {
    origin: Point2<Glass>,
    bounds: Property<AxisAlignedBox>,
}

impl ZoomToFitTool {
    pub fn new(point: Point2<Glass>) -> Self {
        Self {
            origin: point,
            bounds: Property::new(AxisAlignedBox::new(Point2::default(), Point2::default())),
        }
    }
}

impl ToolBehavior for ZoomToFitTool {
    fn pointer_down(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_move(&mut self, point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        self.bounds.set(AxisAlignedBox::new(self.origin, point));
        ControlFlow::Continue(())
    }

    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        self.pointer_move(point, ctx);
        let bounds = self.bounds.get();
        if bounds.width() < 50.0 || bounds.height() < 50.0 {
            return ControlFlow::Break(());
        }

        let glass_to_world = ctx.world_transform();
        if let Err(e) = (ZoomToFit {
            top_left: glass_to_world * bounds.top_left(),
            bottom_right: glass_to_world * bounds.bottom_right(),
        }
        .perform(ctx))
        {
            log::warn!("failed to zoom: {e}");
        };

        ControlFlow::Break(())
    }

    fn keyboard(
        &mut self,
        _event: InputEvent,
        _dir: super::input::Dir,
        _ctx: &mut ActionContext,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn get_visual(&self) -> Property<ToolVisualizationState> {
        let bounds = self.bounds.clone();
        let deps = [bounds.untyped()];
        Property::computed(
            move || {
                let bounds = bounds.get();
                ToolVisualizationState {
                    rect_tool: RectTool {
                        x: Size::Pixels(bounds.top_left().x.into()),
                        y: Size::Pixels(bounds.top_left().y.into()),
                        width: Size::Pixels(bounds.width().into()),
                        height: Size::Pixels(bounds.height().into()),
                        stroke: Color::rgba(0.into(), 20.into(), 200.into(), 200.into()),
                        fill: Color::rgba(0.into(), 20.into(), 200.into(), 30.into()),
                    },
                    ..Default::default()
                }
            },
            &deps,
        )
    }

    fn finish(&mut self, _ctx: &mut ActionContext) -> anyhow::Result<()> {
        Ok(())
    }
}
