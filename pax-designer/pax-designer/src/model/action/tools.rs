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
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        match ctx.app_state.selected_tool {
            Tool::Rectangle => ctx.execute(RectangleTool {
                event: self.event,
                point: self.point,
            }),
            Tool::Pointer => ctx.execute(PointerTool {
                point: self.point,
                event: self.event,
            }),
        };

        Ok(CanUndo::No)
    }
}

pub struct RectangleTool {
    pub event: Pointer,
    pub point: Point2<Glass>,
}

impl Action for RectangleTool {
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        match self.event {
            Pointer::Down => {
                ctx.app_state.tool_state = ToolState::Box {
                    p1: self.point,
                    p2: self.point,
                    stroke: Color::rgba(0.into(), 0.into(), 1.into(), 0.7.into()),
                    fill: Color::rgba(0.into(), 0.into(), 0.into(), 0.2.into()),
                };
            }
            Pointer::Move => {
                if let ToolState::Box { ref mut p2, .. } = ctx.app_state.tool_state {
                    *p2 = self.point;
                }
            }
            Pointer::Up => {
                if let ToolState::Box { p1, p2, .. } = std::mem::take(&mut ctx.app_state.tool_state)
                {
                    let glass_to_world = ctx.world_transform();
                    let world_origin = glass_to_world * p1;
                    let world_dims = glass_to_world * (p2 - p1);
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
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        match self.event {
            Pointer::Down => {
                let world_point = ctx.world_transform() * self.point;
                if let Some(hit) = ctx.raycast_world(world_point) {
                    ctx.app_state.selected_template_node_id = Some(hit.global_id().1);

                    let origin_window = hit.origin().unwrap();
                    let object_origin_glass = ctx.glass_transform() * origin_window;
                    let offset = ctx.world_transform() * (object_origin_glass - self.point);
                    let curr_pos = ctx.app_state.tool_state = ToolState::Movement { offset };
                } else {
                    ctx.app_state.tool_state = ToolState::Box {
                        p1: self.point,
                        p2: self.point,
                        stroke: Color::rgba(0.into(), 1.into(), 1.into(), 0.7.into()),
                        fill: Color::rgba(0.into(), 1.into(), 1.into(), 0.1.into()),
                    };
                }
            }
            Pointer::Move => match ctx.app_state.tool_state {
                ToolState::Box { p2, .. } => {
                    let ToolState::Box { ref mut p2, .. } = ctx.app_state.tool_state else {
                        unreachable!();
                    };
                    *p2 = self.point;
                }
                ToolState::Movement { offset } => {
                    let world_point = ctx.world_transform() * self.point + offset;
                    ctx.execute(MoveSelected { point: world_point });
                }
                _ => (),
            },
            Pointer::Up => {
                if let ToolState::Box { p1, p2, .. } = std::mem::take(&mut ctx.app_state.tool_state)
                {
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
