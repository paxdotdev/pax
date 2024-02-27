use std::any::Any;
use std::rc::Rc;

use super::object_editor::GlassPoint;
use pax_engine::api::*;
use pax_engine::math::Point2;
use pax_engine::*;
use pax_std::primitives::{Group, Path, Rectangle};
use pax_std::types::{Color, Fill};
use serde::Deserialize;

use crate::math::AxisAlignedBox;
use crate::model::AppState;
use crate::model::ToolState;
use crate::model::{self, action};

use crate::model::action::pointer::Pointer;
use crate::model::action::{Action, ActionContext, CanUndo};
use crate::model::input::Dir;
use crate::model::math;
use crate::model::math::coordinate_spaces::{self, Glass, World};

#[pax]
#[file("glass/control_point.pax")]
pub struct ControlPoint {
    pub data: Property<ControlPointDef>,
    pub ind: Property<Numeric>,
}

pub trait ControlPointBehaviour {
    fn init(&self, ctx: &mut ActionContext, point: Point2<Glass>);
    fn step(&self, ctx: &mut ActionContext, point: Point2<Glass>);
}

pub struct ActivateControlPoint {
    behaviour: Rc<dyn ControlPointBehaviour>,
    point: Point2<Window>,
}

impl Action for ActivateControlPoint {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> anyhow::Result<CanUndo> {
        self.behaviour.init(ctx, ctx.glass_transform() * self.point);
        ctx.app_state.tool_state = ToolState::MovingControlPoint {
            behaviour: self.behaviour,
        };
        Ok(CanUndo::No)
    }
}

impl ControlPoint {
    pub fn mouse_down(&mut self, ctx: &NodeContext, args: ArgsMouseDown) {
        super::object_editor::CONTROL_POINT_FUNCS.with_borrow(|funcs| {
            if let Some(funcs) = funcs {
                model::perform_action(
                    ActivateControlPoint {
                        behaviour: Rc::clone(&funcs[self.ind.get().get_as_int() as usize]),
                        point: Point2::new(args.mouse.x, args.mouse.y),
                    },
                    ctx,
                );
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
