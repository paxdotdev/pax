use super::CanUndo;
use super::{Action, ActionContext};
use crate::model::action;
use crate::model::AppState;
use crate::model::ToolState;
use crate::USERLAND_PROJECT_ID;
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::api::MouseButton;
use pax_engine::rendering::Point2D;

pub struct PointerAction {
    pub event: Pointer,
    pub button: MouseButton,
    pub screenspace_point: Point2D,
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
                point: self.screenspace_point,
            }),
            MouseButton::Middle => ctx.execute(action::world::Pan {
                event: self.event,
                point: self.screenspace_point,
            }),
            _ => Ok(()),
        }?;
        Ok(CanUndo::No)
    }
}
