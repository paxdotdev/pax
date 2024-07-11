use std::any::Any;
use std::ops::ControlFlow;
use std::rc::Rc;

use super::action::orm::{CreateComponent, SetNodePropertiesFromTransform};
use super::action::pointer::Pointer;
use super::action::{Action, ActionContext, RaycastMode};
use super::input::InputEvent;
use super::{GlassNode, GlassNodeSnapshot, SelectionStateSnapshot, StageInfo};
use crate::glass::outline::PathOutline;
use crate::glass::wireframe_editor::editor_generation::stacker_control::raycast_slot;
use crate::glass::{RectTool, ToolVisualizationState};
use crate::math::coordinate_spaces::{Glass, World};
use crate::math::{
    AxisAlignedBox, DecompositionConfiguration, GetUnit, IntoDecompositionConfiguration, SizeUnit,
};
use crate::model::action::orm::{MoveNode, NodeLayoutSettings};
use crate::model::Tool;
use crate::model::{AppState, ToolBehaviour};
use crate::{SetStage, ROOT_PROJECT_ID};
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::api::Size;
use pax_engine::api::{Color, NodeContext};
use pax_engine::layout::{LayoutProperties, TransformAndBounds};
use pax_engine::math::Point2;
use pax_engine::math::Vector2;
use pax_engine::{log, NodeInterface, NodeLocal, Property, Slot};
use pax_manifest::{
    PaxType, TemplateNodeId, TreeIndexPosition, TypeId, UniqueTemplateNodeIdentifier,
};
use pax_runtime_api::math::Transform2;
use pax_runtime_api::{Axis, Window};
use pax_std::stacker::Stacker;

pub struct CreateComponentTool {
    type_id: TypeId,
    origin: Point2<Glass>,
    bounds: Property<AxisAlignedBox>,
    mock_children: usize,
}

impl CreateComponentTool {
    pub fn new(
        _ctx: &mut ActionContext,
        point: Point2<Glass>,
        type_id: &TypeId,
        mock_children: usize,
    ) -> Self {
        Self {
            type_id: type_id.clone(),
            origin: point,
            bounds: Property::new(AxisAlignedBox::new(Point2::default(), Point2::default())),
            mock_children,
        }
    }
}

impl ToolBehaviour for CreateComponentTool {
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
        self.bounds.set(
            AxisAlignedBox::new(self.origin, self.origin + Vector2::new(1.0, 1.0))
                .morph_constrained(point, self.origin, is_alt_key_down, is_shift_key_down),
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
        let parent = ctx
            .engine_context
            .get_nodes_by_id(ROOT_PROJECT_ID)
            .into_iter()
            .next()
            .unwrap();
        let parent = GlassNode::new(&parent, &ctx.glass_transform());

        if let Err(e) = (CreateComponent {
            parent_id: &parent.id,
            parent_index: TreeIndexPosition::Top,
            node_layout: NodeLayoutSettings::KeepScreenBounds {
                node_transform_and_bounds: &TransformAndBounds {
                    transform: box_transform,
                    bounds: (1.0, 1.0),
                }
                .as_pure_size(),
                new_parent_transform_and_bounds: &parent.transform_and_bounds.get(),
                node_decompositon_config: Default::default(),
            },
            type_id: &self.type_id,
            custom_props: &[],
            mock_children: self.mock_children,
        }
        .perform(ctx))
        {
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
                        stroke: Color::rgba(0.into(), 0.into(), 255.into(), 200.into()),
                        fill: Color::rgba(0.into(), 0.into(), 255.into(), 30.into()),
                    },
                    outline: Default::default(),
                }
            },
            &deps,
        )
    }
}

pub struct SelectNodes<'a> {
    pub ids: &'a [TemplateNodeId],
    //if true, deselects all other objects first
    pub overwrite: bool,
}

impl Action for SelectNodes<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let mut ids = ctx.app_state.selected_template_node_ids.get();
        // TODO this is not it, should instead not trigger selectnodes if
        // clicking on group of nodes that is already selected and was moved
        if self.overwrite
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
        vis: Property<ToolVisualizationState>,
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
                    overwrite: false,
                }
                .perform(ctx);
            }
            let selection = ctx.derived_state.selection_state.get();
            Self {
                action: PointerToolAction::Moving {
                    hit,
                    has_moved: false,
                    pickup_point: point,
                    initial_selection: (&selection).into(),
                    vis: Property::new(ToolVisualizationState::default()),
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

impl ToolBehaviour for PointerTool {
    fn pointer_down(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_move(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        match &mut self.action {
            &mut PointerToolAction::Moving {
                pickup_point,
                ref initial_selection,
                ref mut has_moved,
                ref hit,
                ref mut vis,
            } => {
                if (pickup_point - point).length_squared() < 3.0 {
                    // don't commit any movement for very small pixel changes,
                    // this creates designtime changes that
                    // make double click behavior for for example
                    // text editing not work
                    return ControlFlow::Continue(());
                }
                *has_moved = true;
                let translation = point - pickup_point;

                let move_translation = TransformAndBounds {
                    transform: Transform2::translate(translation),
                    bounds: (1.0, 1.0),
                };

                for item in &initial_selection.items {
                    if let Err(e) = (SetNodePropertiesFromTransform {
                        id: &item.id,
                        transform_and_bounds: &(move_translation * item.transform_and_bounds),
                        parent_transform_and_bounds: &item.parent_transform_and_bounds,
                        decomposition_config: &item.layout_properties.into_decomposition_config(),
                    }
                    .perform(ctx))
                    {
                        pax_engine::log::error!("Error moving selected: {:?}", e);
                    }
                }

                let raycast_hit = raycast_slot(ctx, point, hit.clone(), true);
                if let Some((_container, slot)) = raycast_hit {
                    let t_and_b = TransformAndBounds {
                        transform: ctx.glass_transform().get(),
                        bounds: (1.0, 1.0),
                    } * slot.transform_and_bounds().get();
                    let outline = PathOutline::from_bounds(t_and_b);
                    vis.set(ToolVisualizationState {
                        rect_tool: Default::default(),
                        outline,
                    });
                } else {
                    vis.set(ToolVisualizationState::default());
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

        match &self.action {
            PointerToolAction::Selecting { .. } => {
                // TODO select multiple objects
                let _ = SelectNodes {
                    ids: &[],
                    overwrite: false,
                }
                .perform(ctx);
            }
            PointerToolAction::Moving {
                has_moved, ref hit, ..
            } => {
                // TODO add check here that we're not moving something from within the same stacker
                if *has_moved {
                    if let Some((container, slot)) = raycast_slot(ctx, point, hit.clone(), true) {
                        if let Err(e) = (MoveNode::<Glass> {
                            node_id: &hit.global_id().unwrap(),
                            new_parent_uid: &container.global_id().unwrap(),
                            index: pax_manifest::TreeIndexPosition::At(
                                slot.with_properties(|f: &mut Slot| {
                                    f.index.get().to_int() as usize
                                })
                                .unwrap(),
                            ),
                            node_layout: NodeLayoutSettings::Fill,
                        }
                        .perform(ctx))
                        {
                            log::warn!("failed to swap nodes: {}", e);
                        };
                        let _ = SelectNodes {
                            ids: &[container.global_id().unwrap().get_template_node_id()],
                            overwrite: true,
                        }
                        .perform(ctx);
                    };
                } else {
                    let _ = SelectNodes {
                        ids: &[hit.global_id().unwrap().get_template_node_id()],
                        overwrite: false,
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
