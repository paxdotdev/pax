use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use super::wireframe_editor::GlassPoint;
use super::ToolVisualizationState;
use crate::glass;
use crate::math::intent_snapper::{self, IntentSnapper, SnapSet};
use crate::model::action::tool::SetToolBehaviour;
use crate::model::tools::ToolBehavior;
use crate::utils::designer_cursor::{DesignerCursor, DesignerCursorType};
use anyhow::Result;
use pax_engine::api::cursor::CursorStyle;
use pax_engine::api::Fill;
use pax_engine::api::*;
use pax_engine::math::{Point2, Transform2, Vector2};
use pax_engine::pax_manifest::UniqueTemplateNodeIdentifier;
use pax_engine::*;
use pax_std::*;
use serde::Deserialize;

use crate::math::AxisAlignedBox;
use crate::model::app_state::AppState;
use crate::model::{self, action};

use crate::math;
use crate::math::coordinate_spaces::{self, Glass, World};
use crate::model::action::pointer::{Pointer, SetCursor};
use crate::model::action::{Action, ActionContext, Transaction};
use crate::model::input::Dir;

#[pax]
#[engine_import_path("pax_engine")]
#[file("glass/control_point.pax")]
pub struct ControlPoint {
    pub data: Property<ControlPointDef>,
    pub ind: Property<Numeric>,
    pub styles: Property<Vec<ControlPointStyling>>,
    // the transform of the currently selected object
    pub object_rotation: Property<Rotation>,

    // private
    // the transform to be applied to this control point
    pub applied_rotation: Property<Rotation>,
    // derived from styling lookup ind in controlpointdef on mount.
    pub style: Property<ControlPointStyling>,
}

#[derive(Clone)]
pub struct ControlPointToolFactory {
    pub tool_factory:
        Rc<dyn Fn(&mut ActionContext, Point2<Glass>) -> Rc<RefCell<dyn ToolBehavior>>>,
    pub double_click_behavior: Rc<dyn Fn(&mut ActionContext)>,
}

pub struct ControlPointTool {
    transaction: Transaction,
    snapper: Option<IntentSnapper>,
    behaviour: Box<dyn ControlPointBehavior>,
    cursor_override: Property<DesignerCursor>,
}

pub enum Snap<'a> {
    No,
    Yes(&'a [UniqueTemplateNodeIdentifier]),
}

impl ControlPointTool {
    pub fn new(
        ctx: &mut ActionContext,
        name: &str,
        snapper: Option<IntentSnapper>,
        behaviour: impl ControlPointBehavior + 'static,
    ) -> Self {
        let transaction = ctx.transaction(name);
        let cursor_override = Property::new(ctx.app_state.cursor.get());
        Self {
            transaction,
            behaviour: Box::new(behaviour),
            snapper,
            cursor_override,
        }
    }
}

pub trait ControlPointBehavior {
    fn step(&self, ctx: &mut ActionContext, point: Point2<Glass>) -> Result<()>;
}

impl ToolBehavior for ControlPointTool {
    fn pointer_down(
        &mut self,
        _point: Point2<Glass>,
        _ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        std::ops::ControlFlow::Continue(())
    }

    fn pointer_move(
        &mut self,
        point: Point2<Glass>,
        ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        let point = match &self.snapper {
            Some(intent_snapper) => {
                let offset = intent_snapper.snap(&[point], false, false);
                point + offset
            }
            None => point,
        };
        match self.transaction.run(|| self.behaviour.step(ctx, point)) {
            Ok(()) => std::ops::ControlFlow::Continue(()),
            Err(_) => std::ops::ControlFlow::Break(()),
        }
    }

    fn pointer_up(
        &mut self,
        _point: Point2<Glass>,
        _ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        std::ops::ControlFlow::Break(())
    }

    fn keyboard(
        &mut self,
        _event: model::input::InputEvent,
        _dir: Dir,
        _ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        std::ops::ControlFlow::Continue(())
    }

    fn get_visual(&self) -> Property<crate::glass::ToolVisualizationState> {
        let snap_lines = self
            .snapper
            .as_ref()
            .map(|snapper| snapper.get_snap_lines_prop())
            .unwrap_or_default();
        let cursor_override = self.cursor_override.clone();
        let deps = [snap_lines.untyped(), cursor_override.untyped()];
        Property::computed(
            move || ToolVisualizationState {
                snap_lines: snap_lines.get(),
                cursor_override: cursor_override.get(),
                ..Default::default()
            },
            &deps,
        )
    }

    fn finish(&mut self, _ctx: &mut ActionContext) -> Result<()> {
        Ok(())
    }
}

impl ControlPoint {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        // NOTE: styling only applied once at mount, not reactive
        let style = self
            .styles
            .read(|styles| styles[self.data.read(|data| data.styling_lookup_ind)].clone());
        let affected_by_transform = style.affected_by_transform.clone();
        self.style.replace_with(Property::new(style));

        let object_transform = self.object_rotation.clone();
        let data = self.data.clone();
        let deps = [data.untyped(), object_transform.untyped()];
        self.applied_rotation.replace_with(Property::computed(
            move || {
                if affected_by_transform {
                    Rotation::Degrees(
                        (object_transform.get().get_as_degrees() + data.read(|data| data.rotation))
                            .into(),
                    )
                } else {
                    Default::default()
                }
            },
            &deps,
        ));
    }

    pub fn mouse_down(&mut self, ctx: &NodeContext, args: Event<MouseDown>) {
        args.prevent_default();
        super::wireframe_editor::CONTROL_POINT_FUNCS.with_borrow(|funcs| {
            if let Some(funcs) = funcs {
                let pos = Point2::new(args.mouse.x, args.mouse.y);
                let behavior = model::with_action_context(ctx, |ac| {
                    (funcs[self.ind.get().to_int() as usize].tool_factory)(
                        ac,
                        ac.glass_transform().get() * pos,
                    )
                });
                model::perform_action(&SetToolBehaviour(Some(behavior)), ctx);
            } else {
                pax_engine::log::warn!(
                    "tried to trigger control point tool behavior while none exist"
                );
            }
        })
    }

    pub fn double_click(&mut self, ctx: &NodeContext, args: Event<DoubleClick>) {
        args.prevent_default();
        super::wireframe_editor::CONTROL_POINT_FUNCS.with_borrow(|funcs| {
            if let Some(funcs) = funcs {
                model::with_action_context(ctx, |ac| {
                    (funcs[self.ind.get().to_int() as usize].double_click_behavior)(ac)
                });
            } else {
                pax_engine::log::warn!(
                    "tried to grigger control point double click behavior while none exist"
                );
            }
        })
    }

    pub fn mouse_over(&mut self, ctx: &NodeContext, _event: Event<MouseOver>) {
        self.style.read(|style| {
            self.data.read(|data| {
                model::perform_action(
                    &SetCursor(DesignerCursor {
                        cursor_type: style.cursor_type,
                        rotation_degrees: data.cursor_rotation,
                    }),
                    ctx,
                );
            })
        })
    }
    pub fn mouse_out(&mut self, ctx: &NodeContext, _event: Event<MouseOut>) {
        model::perform_action(&SetCursor(DesignerCursor::default()), ctx);
    }
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct ControlPointDef {
    pub point: GlassPoint,
    /// 0.0-1.0
    pub anchor_x: f64,
    /// 0.0-1.0
    pub anchor_y: f64,
    /// Node-local rotation in degrees
    pub rotation: f64,
    // Node-local rotation in degrees
    pub cursor_rotation: f64,
    pub styling_lookup_ind: usize,
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct ControlPointStyling {
    pub affected_by_transform: bool,
    pub round: bool,
    pub stroke_color: Color,
    pub fill_color: Color,
    pub stroke_width_pixels: f64,
    pub width: f64,
    pub height: f64,
    pub cursor_type: DesignerCursorType,
    pub hit_padding: f64,
}
