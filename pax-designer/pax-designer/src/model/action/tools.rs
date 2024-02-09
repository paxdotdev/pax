use super::pointer::Pointer;
use super::{Action, ActionContext, CanUndo};
use crate::model::AppState;
use crate::model::{Tool, ToolVisual};
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_std::types::Color;

pub struct ToolAction {
    pub tool: Tool,
    pub event: Pointer,
    pub x: f64,
    pub y: f64,
}

impl Action for ToolAction {
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        match self.tool {
            Tool::Rectangle => ctx.perform(RectangleTool {
                x: self.x,
                y: self.y,
                event: self.event,
            }),
            Tool::Pointer => ctx.perform(PointerTool {
                x: self.x,
                y: self.y,
                event: self.event,
            }),
        };

        Ok(CanUndo::No)
    }
}

pub struct RectangleTool {
    pub event: Pointer,
    pub x: f64,
    pub y: f64,
}

impl Action for RectangleTool {
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        match self.event {
            Pointer::Down => {
                ctx.app_state.tool_visual = Some(ToolVisual::Box {
                    x1: self.x,
                    y1: self.y,
                    x2: self.x,
                    y2: self.y,
                    stroke: Color::rgba(0.into(), 0.into(), 1.into(), 0.7.into()),
                    fill: Color::rgba(0.into(), 0.into(), 0.into(), 0.2.into()),
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
                if let Some(ToolVisual::Box { x1, y1, x2, y2, .. }) =
                    ctx.app_state.tool_visual.take()
                {
                    ctx.perform(super::orm::CreateRectangle {})?;
                }
            }
        }
        Ok(CanUndo::No)
    }
}

pub struct PointerTool {
    pub event: Pointer,
    pub x: f64,
    pub y: f64,
}

impl Action for PointerTool {
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        match self.event {
            Pointer::Down => {
                // TODO don't always go into selection state.
                // if on top of object (query the engine for this)
                // then set "dragged object" to that object, and check for that in the below cases
                // or refactor into one "selection" tool and one "drag" tool
                ctx.app_state.tool_visual = Some(ToolVisual::Box {
                    x1: self.x,
                    y1: self.y,
                    x2: self.x,
                    y2: self.y,
                    stroke: Color::rgba(0.into(), 1.into(), 1.into(), 0.7.into()),
                    fill: Color::rgba(0.into(), 1.into(), 1.into(), 0.1.into()),
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
                if let Some(ToolVisual::Box { x1, y1, x2, y2, .. }) =
                    ctx.app_state.tool_visual.take()
                {
                    // TODO get objects within rectangle from engine, and find their
                    // TemplateNode ids to set selection state.
                }
            }
        }
        Ok(CanUndo::No)
    }
}
