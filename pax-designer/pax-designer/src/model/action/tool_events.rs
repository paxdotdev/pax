use super::pointer_events::Pointer;
use super::{Action, ActionContext, OneshotAction};
use crate::model::AppState;
use crate::model::{Tool, ToolVisual};
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;

pub struct ToolAction {
    pub tool: Tool,
    pub event: Pointer,
    pub x: f64,
    pub y: f64,
}

impl OneshotAction for ToolAction {
    fn perform(&mut self, ctx: &mut ActionContext) -> Result<()> {
        match self.tool {
            Tool::Rectangle => ctx.perform(RectangleTool {
                x: self.x,
                y: self.y,
                event: self.event,
            }),
        }
    }
}

impl From<ToolAction> for Action {
    fn from(value: ToolAction) -> Self {
        Action::Oneshot(Box::new(value))
    }
}

pub struct RectangleTool {
    pub event: Pointer,
    pub x: f64,
    pub y: f64,
}

impl OneshotAction for RectangleTool {
    fn perform(&mut self, ctx: &mut ActionContext) -> Result<()> {
        match self.event {
            Pointer::Down => {
                ctx.app_state.tool_visual = Some(ToolVisual::Box {
                    x1: self.x,
                    y1: self.y,
                    x2: self.x,
                    y2: self.y,
                });
            }
            Pointer::Move => {
                if let Some(ToolVisual::Box {
                    ref mut x2,
                    ref mut y2,
                    ..
                }) = ctx.app_state.tool_visual.as_mut()
                {
                    *x2 = self.x;
                    *y2 = self.y;
                }
            }
            Pointer::Up => {
                if let Some(ToolVisual::Box { x1, y1, x2, y2 }) = ctx.app_state.tool_visual.take() {
                    ctx.perform(super::create_components::CreateRectangle {})?;
                }
            }
        }
        Ok(())
    }
}

impl From<RectangleTool> for Action {
    fn from(value: RectangleTool) -> Self {
        Action::Oneshot(Box::new(value))
    }
}
