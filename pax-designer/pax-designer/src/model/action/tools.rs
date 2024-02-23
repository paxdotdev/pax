use std::rc::Rc;

use super::orm::MoveSelected;
use super::pointer::Pointer;
use super::{Action, ActionContext, CanUndo};
use crate::model::math::coordinate_spaces::Glass;
use crate::model::AppState;
use crate::model::{Tool, ToolState};
use crate::USERLAND_PROJECT_ID;
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::math::Point2;
use pax_engine::math::Vector2;
use pax_engine::rendering::TransformAndBounds;
use pax_std::types::Color;

pub struct ToolAction {
    pub event: Pointer,
    pub point: Point2<Glass>,
}

impl Action for ToolAction {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        // Moving of control point interrupts other tool use
        if matches!(
            ctx.app_state.tool_state,
            ToolState::MovingControlPoint { .. }
        ) {
            match self.event {
                Pointer::Down | Pointer::Move => {
                    let ToolState::MovingControlPoint {
                        ref mut move_func,
                        ref original_bounds,
                    } = ctx.app_state.tool_state
                    else {
                        unreachable!();
                    };
                    let bounds = original_bounds.clone();
                    Rc::clone(move_func)(ctx, &bounds, self.point);
                }
                Pointer::Up => ctx.app_state.tool_state = ToolState::Idle,
            }
        }

        match ctx.app_state.selected_tool {
            Tool::Rectangle => ctx.execute(RectangleTool {
                event: self.event,
                point: self.point,
            }),
            Tool::Pointer => ctx.execute(PointerTool {
                point: self.point,
                event: self.event,
            }),
        }?;

        Ok(CanUndo::No)
    }
}

pub struct RectangleTool {
    pub event: Pointer,
    pub point: Point2<Glass>,
}

impl Action for RectangleTool {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        match self.event {
            Pointer::Down => {
                ctx.app_state.tool_state = ToolState::BoxSelect {
                    p1: self.point,
                    p2: self.point,
                    stroke: Color::rgba(0.into(), 0.into(), 1.into(), 0.7.into()),
                    fill: Color::rgba(0.into(), 0.into(), 0.into(), 0.2.into()),
                };
            }
            Pointer::Move => {
                if let ToolState::BoxSelect { ref mut p2, .. } = ctx.app_state.tool_state {
                    *p2 = self.point;
                }
            }
            Pointer::Up => {
                if let ToolState::BoxSelect { p1, p2, .. } =
                    std::mem::take(&mut ctx.app_state.tool_state)
                {
                    let world_origin = ctx.world_transform() * p1;
                    let world_dims = ctx.world_transform() * (p2 - p1);
                    ctx.execute(super::orm::CreateRectangle {
                        origin: world_origin,
                        dims: world_dims,
                    })?;

                    ctx.app_state.selected_tool = Tool::Pointer;
                }
            }
        }
        Ok(CanUndo::No)
    }
}

pub struct PointerTool {
    pub event: Pointer,
    pub point: Point2<Glass>,
}

impl Action for PointerTool {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        match self.event {
            Pointer::Down => {
                if let Some(hit) = ctx.raycast_glass(self.point) {
                    ctx.app_state.selected_template_node_id = Some(hit.global_id().1);

                    let origin_window = hit.origin().unwrap();
                    let object_origin_glass = ctx.glass_transform() * origin_window;
                    let offset = self.point - object_origin_glass;
                    ctx.app_state.tool_state = ToolState::MovingObject { offset };
                } else {
                    ctx.app_state.tool_state = ToolState::BoxSelect {
                        p1: self.point,
                        p2: self.point,
                        stroke: Color::rgba(0.into(), 1.into(), 1.into(), 0.7.into()),
                        fill: Color::rgba(0.into(), 1.into(), 1.into(), 0.1.into()),
                    };
                }
            }
            Pointer::Move => match ctx.app_state.tool_state {
                ToolState::BoxSelect { .. } => {
                    let ToolState::BoxSelect { ref mut p2, .. } = ctx.app_state.tool_state else {
                        unreachable!();
                    };
                    *p2 = self.point;
                }
                ToolState::MovingObject { offset } => {
                    let world_point = ctx.world_transform() * (self.point - offset);
                    ctx.execute(MoveSelected { point: world_point })?;
                }
                _ => (),
            },
            Pointer::Up => {
                if let ToolState::BoxSelect { .. } = std::mem::take(&mut ctx.app_state.tool_state) {
                    // TODO get objects within rectangle from engine, and find their
                    // TemplateNode ids to set selection state.
                    let something_in_rectangle = true;
                    if something_in_rectangle {
                        ctx.app_state.selected_template_node_id = None;
                        //select things
                    }
                }
            }
        }
        Ok(CanUndo::No)
    }
}
