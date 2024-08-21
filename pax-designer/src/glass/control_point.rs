use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use super::wireframe_editor::GlassPoint;
use super::ToolVisualizationState;
use crate::glass;
use crate::math::intent_snapper::{self, IntentSnapper, SnapSet};
use anyhow::Result;
use pax_engine::api::Fill;
use pax_engine::api::*;
use pax_engine::math::{Point2, Transform2};
use pax_engine::pax_manifest::UniqueTemplateNodeIdentifier;
use pax_engine::*;
use pax_std::*;
use serde::Deserialize;

use crate::math::AxisAlignedBox;
use crate::model::{self, action};
use crate::model::{AppState, ToolBehavior};

use crate::math;
use crate::math::coordinate_spaces::{self, Glass, World};
use crate::model::action::pointer::Pointer;
use crate::model::action::{Action, ActionContext, Transaction};
use crate::model::input::Dir;

#[pax]
#[engine_import_path("pax_engine")]
#[file("glass/control_point.pax")]
pub struct ControlPoint {
    pub data: Property<ControlPointDef>,
    pub ind: Property<Numeric>,
    // the transform of the currently selected object
    pub object_rotation: Property<Rotation>,
    // the transform to be applied to this control point
    pub applied_rotation: Property<Rotation>,
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
}

pub enum Snap<'a> {
    No,
    Yes(&'a [UniqueTemplateNodeIdentifier]),
}

impl ControlPointTool {
    pub fn new(
        transaction: Transaction,
        snapper: Option<IntentSnapper>,
        behaviour: impl ControlPointBehavior + 'static,
    ) -> Self {
        Self {
            transaction,
            behaviour: Box::new(behaviour),
            snapper,
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
                let offset = intent_snapper.snap(&[point]);
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
        if let Some(intent_snapper) = &self.snapper {
            let snap_lines = intent_snapper.get_snap_lines_prop();
            let deps = [snap_lines.untyped()];
            Property::computed(
                move || ToolVisualizationState {
                    rect_tool: Default::default(),
                    outline: Default::default(),
                    snap_lines: snap_lines.get(),
                },
                &deps,
            )
        } else {
            Property::default()
        }
    }
}

pub struct ActivateControlPoint {
    behavior: Rc<RefCell<dyn ToolBehavior>>,
}

impl Action for ActivateControlPoint {
    fn perform(&self, ctx: &mut ActionContext) -> anyhow::Result<()> {
        ctx.app_state
            .tool_behavior
            .set(Some(Rc::clone(&self.behavior)));
        Ok(())
    }
}

impl ControlPoint {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        let data = self.data.clone();
        let object_transform = self.object_rotation.clone();
        let deps = [data.untyped(), object_transform.untyped()];
        self.applied_rotation.replace_with(Property::computed(
            move || {
                if data.get().styling.affected_by_transform {
                    object_transform.get()
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
                    // save-point before we start executing control point behavior
                    let before_undo_id = borrow!(ac.engine_context.designtime)
                        .get_orm()
                        .get_last_undo_id()
                        .unwrap_or(0);
                    ac.undo_stack.push(before_undo_id);

                    (funcs[self.ind.get().to_int() as usize].tool_factory)(
                        ac,
                        ac.glass_transform().get() * pos,
                    )
                });
                model::perform_action(&ActivateControlPoint { behavior }, ctx);
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
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct ControlPointDef {
    pub point: GlassPoint,
    pub styling: ControlPointStyling,
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct ControlPointStyling {
    pub affected_by_transform: bool,
    pub round: bool,
    pub stroke: Color,
    pub fill: Color,
    pub stroke_width_pixels: f64,
    pub width: f64,
    pub height: f64,
}
