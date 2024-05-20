use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use super::wireframe_editor::GlassPoint;
use crate::glass;
use pax_engine::api::Fill;
use pax_engine::api::*;
use pax_engine::math::Point2;
use pax_engine::*;
use pax_std::primitives::{Group, Path, Rectangle};
use serde::Deserialize;

use crate::math::AxisAlignedBox;
use crate::model::{self, action};
use crate::model::{AppState, ToolBehaviour};

use crate::math;
use crate::math::coordinate_spaces::{self, Glass, World};
use crate::model::action::pointer::Pointer;
use crate::model::action::{Action, ActionContext, CanUndo};
use crate::model::input::Dir;

#[pax]
#[file("glass/control_point.pax")]
pub struct ControlPoint {
    pub data: Property<ControlPointDef>,
    pub ind: Property<Numeric>,
}

pub type ControlPointBehaviourFactory =
    Rc<dyn Fn(&mut ActionContext, Point2<Glass>) -> Rc<RefCell<dyn ToolBehaviour>>>;

pub trait ControlPointBehaviour {
    fn step(&self, ctx: &mut ActionContext, point: Point2<Glass>);
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

    fn visualize(&self, _glass: &mut glass::Glass) {}
}

pub struct ActivateControlPoint {
    behaviour: Rc<RefCell<dyn ToolBehaviour>>,
}

impl Action for ActivateControlPoint {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> anyhow::Result<CanUndo> {
        ctx.app_state.tool_behaviour.set(Some(self.behaviour));
        Ok(CanUndo::No)
    }
}

impl ControlPoint {
    pub fn mouse_down(&mut self, ctx: &NodeContext, args: Event<MouseDown>) {
        super::wireframe_editor::CONTROL_POINT_FUNCS.with_borrow(|funcs| {
            if let Some(funcs) = funcs {
                let pos = Point2::new(args.mouse.x, args.mouse.y);
                let behaviour = model::with_action_context(ctx, |ac| {
                    funcs[self.ind.get().to_int() as usize](ac, ac.glass_transform() * pos)
                });
                model::perform_action(ActivateControlPoint { behaviour }, ctx);
            } else {
                pax_engine::log::warn!("tried to grigger control point while none exist");
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
    pub stroke: Color,
    pub fill: Color,
    pub stroke_width_pixels: f64,
    pub size_pixels: f64,
}
