use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use super::wireframe_editor::GlassPoint;
use super::ToolVisualizationState;
use crate::glass;
use pax_engine::api::Fill;
use pax_engine::api::*;
use pax_engine::math::{Point2, Transform2};
use pax_engine::*;
use pax_std::*;
use serde::Deserialize;

use crate::math::AxisAlignedBox;
use crate::model::{self, action};
use crate::model::{AppState, ToolBehaviour};

use crate::math;
use crate::math::coordinate_spaces::{self, Glass, World};
use crate::model::action::pointer::Pointer;
use crate::model::action::{Action, ActionContext};
use crate::model::input::Dir;

#[pax]
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
pub struct ControlPointBehaviourFactory {
    pub tool_behaviour:
        Rc<dyn Fn(&mut ActionContext, Point2<Glass>) -> Rc<RefCell<dyn ToolBehaviour>>>,
    pub double_click_behaviour: Rc<dyn Fn(&mut ActionContext)>,
}

pub trait ControlPointBehaviour {
    fn step(&self, ctx: &mut ActionContext, point: Point2<Glass>);
    // used for pushing an undo id to the stack
}

impl<C: ControlPointBehaviour> ToolBehaviour for C {
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
        self.step(ctx, point);
        std::ops::ControlFlow::Continue(())
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

    fn get_visual(&self) -> Property<glass::ToolVisualizationState> {
        Property::new(ToolVisualizationState::default())
    }
}

pub struct ActivateControlPoint {
    behaviour: Rc<RefCell<dyn ToolBehaviour>>,
}

impl Action for ActivateControlPoint {
    fn perform(&self, ctx: &mut ActionContext) -> anyhow::Result<()> {
        ctx.app_state
            .tool_behaviour
            .set(Some(Rc::clone(&self.behaviour)));
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
                let behaviour = model::with_action_context(ctx, |ac| {
                    // save-point before we start executing control point behaviour
                    let before_undo_id = borrow!(ac.engine_context.designtime)
                        .get_orm()
                        .get_last_undo_id()
                        .unwrap_or(0);
                    ac.undo_stack.push(before_undo_id);

                    (funcs[self.ind.get().to_int() as usize].tool_behaviour)(
                        ac,
                        ac.glass_transform().get() * pos,
                    )
                });
                model::perform_action(&ActivateControlPoint { behaviour }, ctx);
            } else {
                pax_engine::log::warn!(
                    "tried to grigger control point tool behaviour while none exist"
                );
            }
        })
    }

    pub fn double_click(&mut self, ctx: &NodeContext, args: Event<DoubleClick>) {
        args.prevent_default();
        super::wireframe_editor::CONTROL_POINT_FUNCS.with_borrow(|funcs| {
            if let Some(funcs) = funcs {
                model::with_action_context(ctx, |ac| {
                    (funcs[self.ind.get().to_int() as usize].double_click_behaviour)(ac)
                });
            } else {
                pax_engine::log::warn!(
                    "tried to grigger control point double click behaviour while none exist"
                );
            }
        })
    }
}

#[pax]
pub struct ControlPointDef {
    pub point: GlassPoint,
    pub styling: ControlPointStyling,
}

#[pax]
pub struct ControlPointStyling {
    pub affected_by_transform: bool,
    pub round: bool,
    pub stroke: Color,
    pub fill: Color,
    pub stroke_width_pixels: f64,
    pub width: f64,
    pub height: f64,
}
