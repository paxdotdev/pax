use super::CanUndo;
use super::{Action, ActionContext};
use crate::model::action;
use crate::model::math::{Glass, Screen};
use crate::model::AppState;
use crate::model::ToolState;
use crate::USERLAND_PROJECT_ID;
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::api::MouseButton;
use pax_engine::math::Point2;

pub struct PointerAction {
    pub event: Pointer,
    pub button: MouseButton,
    pub point: Point2<Screen>,
}

#[derive(Clone, Copy)]
pub enum Pointer {
    Down,
    Move,
    Up,
}

impl Action for PointerAction {
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        match self.button {
            MouseButton::Left => ctx.execute(action::tools::ToolAction {
                event: self.event,
                point: self.point.to_world::<Glass>(),
            }),
            MouseButton::Middle => ctx.execute(action::world::Pan {
                event: self.event,
                point: self.point.to_world::<Glass>(),
            }),
            _ => Ok(()),
        }?;
        Ok(CanUndo::No)
    }
}
