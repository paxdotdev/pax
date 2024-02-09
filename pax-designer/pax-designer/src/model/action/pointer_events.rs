use super::{Action, ActionContext, OneshotAction};
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

impl OneshotAction for PointerAction {
    fn perform(&mut self, ctx: &mut ActionContext) -> Result<()> {
        if let Some(tool) = ctx.app_state.selected_tool {
            ctx.perform(action::tool_events::ToolAction {
                tool,
                event: self.event,
                x: self.x,
                y: self.y,
            })
        } else {
            Err(anyhow!("only rect tool supported at the moment"))
        }
    }
}

impl From<PointerAction> for Action {
    fn from(value: PointerAction) -> Self {
        Action::Oneshot(Box::new(value))
    }
}
