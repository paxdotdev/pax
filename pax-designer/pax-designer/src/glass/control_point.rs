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
    pub location: Property<GlassPoint>,
    pub ind: Property<Numeric>,
}

pub type ControlPointBehaviour =
    dyn Fn(&mut ActionContext, &(AxisAlignedBox, Point2<Glass>), Point2<Glass>);

pub struct ActivateControlPoint {
    behaviour: Rc<ControlPointBehaviour>,
    original_bounds: (AxisAlignedBox, Point2<Glass>),
}

impl Action for ActivateControlPoint {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> anyhow::Result<CanUndo> {
        ctx.app_state.tool_state = ToolState::MovingControlPoint {
            move_func: self.behaviour,
            original_bounds: self.original_bounds,
        };
        Ok(CanUndo::No)
    }
}

impl ControlPoint {
    pub fn mouse_down(&mut self, ctx: &NodeContext, _args: ArgsMouseDown) {
        let bounds = model::with_action_context(ctx, |ac| ac.selected_bounds())
            .expect("selection bounds exist");
        super::object_editor::CONTROL_POINT_FUNCS.with_borrow(|funcs| {
            if let Some(funcs) = funcs {
                model::perform_action(
                    ActivateControlPoint {
                        behaviour: Rc::clone(&funcs[self.ind.get().get_as_int() as usize]),
                        original_bounds: bounds,
                    },
                    ctx,
                );
            } else {
                pax_engine::log::error!("tried to grigger control point while none exist");
            }
        })
    }
}
