use super::CanUndo;
use super::{Action, ActionContext};
use crate::model::action;
use crate::model::AppState;
use crate::model::ToolVisual;
use crate::USERLAND_PROJECT_ID;
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::rendering::Point2D;

pub struct PointerAction {
    pub event: Pointer,
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
        ctx.execute(action::tools::ToolAction {
            tool: ctx.app_state.selected_tool,
            event: self.event,
            point: self.screenspace_point,
        })?;
        Ok(CanUndo::No)
    }
}
