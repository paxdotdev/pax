use super::CanUndo;
use super::{Action, ActionContext};
use crate::model::action;
use crate::model::AppState;
use crate::model::ToolVisual;
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;

pub struct PointerAction {
    pub event: Pointer,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy)]
pub enum Pointer {
    Down,
    Move,
    Up,
}

impl Action for PointerAction {
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        if let Some(tool) = ctx.app_state.selected_tool {
            ctx.execute(action::tools::ToolAction {
                tool,
                event: self.event,
                x: self.x,
                y: self.y,
            })?;
        } else {
            return Err(anyhow!("only rect tool supported at the moment"));
        }
        Ok(CanUndo::No)
    }
}
