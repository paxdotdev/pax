use std::any::Any;
use std::ops::ControlFlow;
use std::rc::Rc;

use super::action::orm::{CreateComponent, SetNodePropertiesFromTransform};
use super::action::pointer::Pointer;
use super::action::{Action, ActionContext, CanUndo, RaycastMode};
use super::input::InputEvent;
use super::{GlassNode, GlassNodeSnapshot, SelectionStateSnapshot, StageInfo};
use crate::glass::{RectTool, ToolVisualizationState};
use crate::math::coordinate_spaces::{Glass, World};
use crate::math::{
    AxisAlignedBox, GetUnit, IntoInversionConfiguration, InversionConfiguration, SizeUnit,
};
use crate::model::action::orm::{MoveNode, ResizeNode};
use crate::model::Tool;
use crate::model::{AppState, ToolBehaviour};
use crate::{SetStage, ROOT_PROJECT_ID};
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::api::Color;
use pax_engine::api::Size;
use pax_engine::layout::{LayoutProperties, TransformAndBounds};
use pax_engine::math::Point2;
use pax_engine::math::Vector2;
use pax_engine::{log, NodeInterface, NodeLocal, Property, Slot};
use pax_manifest::{PaxType, TemplateNodeId, TypeId, UniqueTemplateNodeIdentifier};
use pax_runtime_api::math::Transform2;
use pax_runtime_api::{Axis, Window};
use pax_std::stacker::Stacker;

pub struct CreateComponentTool {
    type_id: TypeId,
    origin: Point2<Glass>,
    bounds: Property<AxisAlignedBox>,
}

impl CreateComponentTool {
    pub fn new(_ctx: &mut ActionContext, point: Point2<Glass>, type_id: &TypeId) -> Self {
        Self {
            type_id: type_id.clone(),
            origin: point,
            bounds: Property::new(AxisAlignedBox::new(Point2::default(), Point2::default())),
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
        let box_transform = ctx.world_transform() * self.bounds.get().as_transform();
        let (o, u, v) = box_transform.decompose();
        // TODO make CreateComponent take transform?
        let world_box = AxisAlignedBox::new(o, o + u + v);
        ctx.execute(CreateComponent {
            bounds: world_box,
            type_id: self.type_id.clone(),
            custom_props: vec![],
        })
        .unwrap();
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
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
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
        Ok(CanUndo::No)
    }
}

pub struct PointerTool {
    action: PointerToolAction,
    vis: Property<RectTool>,
}

pub enum PointerToolAction {
    Moving {
        has_moved: bool,
        hit: NodeInterface,
        pickup_point: Point2<Glass>,
        initial_selection: SelectionStateSnapshot,
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
                let _ = ctx.execute(SelectNodes {
                    ids: &[node_id.get_template_node_id()],
                    overwrite: false,
                });
            }
            let selection = ctx.derived_state.selection_state.get();
            Self {
                action: PointerToolAction::Moving {
                    hit,
                    has_moved: false,
                    pickup_point: point,
                    initial_selection: (&selection).into(),
                },
                vis: Default::default(),
            }
        } else {
            // resize stage if we are at edge
            let stage = ctx.app_state.stage.get();
            let world_point = ctx.world_transform() * point;
            if (world_point.y - stage.height as f64).abs() < 10.0 {
                Self {
                    action: PointerToolAction::ResizingStage(ResizeStageDim::Height),
                    vis: Default::default(),
                }
            } else if (world_point.x - stage.width as f64).abs() < 10.0 {
                Self {
                    action: PointerToolAction::ResizingStage(ResizeStageDim::Width),
                    vis: Default::default(),
                }
            } else {
                Self {
                    action: PointerToolAction::Selecting {
                        p1: point,
                        p2: point,
                    },
                    vis: Default::default(),
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
                ..
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
                    if let Err(e) = ctx.execute(SetNodePropertiesFromTransform {
                        id: item.id.clone(),
                        transform_and_bounds: move_translation * item.transform_and_bounds,
                        parent_transform_and_bounds: item.parent_transform_and_bounds,
                        inv_config: item.layout_properties.into_inv_config(),
                    }) {
                        pax_engine::log::error!("Error moving selected: {:?}", e);
                    }
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
                ctx.execute(SetStage(StageInfo {
                    width: new_width,
                    height: new_height,
                    color: Color::BLUE,
                }))
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
                let _ = ctx.execute(SelectNodes {
                    ids: &[],
                    overwrite: false,
                });
            }
            PointerToolAction::Moving {
                has_moved, ref hit, ..
            } => {
                if *has_moved {
                    // If we drop on another object, check if it's an object in a slot.
                    // If it is, add this object to the same parent
                    if let Some(drop_hit) =
                        ctx.raycast_glass(point, RaycastMode::RawNth(0), &[hit.clone()])
                    {
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
                        if let Some(drop_container) = drop_slot_topmost_container {
                            let mut slot_index = None;
                            let mut curr = drop_hit.clone();
                            let cc = drop_container.containing_component().unwrap();
                            while curr.is_descendant_of(&cc) {
                                if curr.is_of_type::<Slot>() {
                                    slot_index = Some(curr.with_properties(|props: &mut Slot| {
                                        props.index.get().to_int()
                                    }));
                                }
                                curr = curr.render_parent().unwrap();
                            }
                            if let Err(e) = ctx.execute(MoveNode {
                                node_id: &hit.global_id().unwrap(),
                                node_transform_and_bounds: &hit.transform_and_bounds().get(),
                                node_inv_config: InversionConfiguration::default(),
                                new_parent_transform_and_bounds: &cc.transform_and_bounds().get(),
                                new_parent_uid: &cc.global_id().unwrap(),
                                index: pax_manifest::TreeIndexPosition::At(
                                    slot_index.unwrap() as usize
                                ),
                                resize_mode: ResizeNode::Fill,
                            }) {
                                log::warn!("failed to swap nodes: {}", e);
                            };
                        }
                    }
                } else {
                    let _ = ctx.execute(SelectNodes {
                        ids: &[hit.global_id().unwrap().get_template_node_id()],
                        overwrite: false,
                    });
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
        let vis = self.vis.clone();
        let deps = [vis.untyped()];
        Property::computed(
            move || ToolVisualizationState {
                rect_tool: vis.get(),
                outline: Default::default(),
            },
            &deps,
        )
    }
}
