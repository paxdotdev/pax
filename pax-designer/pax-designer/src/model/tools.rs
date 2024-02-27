use std::ops::ControlFlow;
use std::rc::Rc;

use super::action::orm::{CreateRectangle, MoveSelected};
use super::action::pointer::Pointer;
use super::action::{Action, ActionContext, CanUndo};
use crate::model::math::coordinate_spaces::Glass;
use crate::model::Tool;
use crate::model::{AppState, ToolBehaviour};
use crate::USERLAND_PROJECT_ID;
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::api::Size;
use pax_engine::math::Point2;
use pax_engine::math::Vector2;
use pax_engine::rendering::TransformAndBounds;
use pax_std::types::Color;

pub struct RectangleTool {
    p1: Point2<Glass>,
    p2: Point2<Glass>,
}

impl RectangleTool {
    pub fn new(_ctx: &mut ActionContext, point: Point2<Glass>) -> Self {
        Self {
            p1: point,
            p2: point,
        }
    }
}

impl ToolBehaviour for RectangleTool {
    fn pointer_down(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_move(
        &mut self,
        point: Point2<Glass>,
        _ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        self.p2 = point;
        ControlFlow::Continue(())
    }

    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        self.p2 = point;
        let world_origin = ctx.world_transform() * self.p1;
        let world_dims = ctx.world_transform() * (self.p2 - self.p1);
        ctx.execute(CreateRectangle {
            origin: world_origin,
            dims: world_dims,
        })
        .unwrap();
        ControlFlow::Break(())
    }

    fn keyboard(
        &mut self,
        _event: crate::model::input::InputEvent,
        _dir: crate::model::input::Dir,
        _ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn visualize(&self, glass: &mut crate::glass::Glass) {
        glass.is_rect_tool_active.set(true);
        glass.rect_tool.set(crate::glass::RectTool {
            x: Size::Pixels(self.p1.x.into()),
            y: Size::Pixels(self.p1.y.into()),
            width: Size::Pixels((self.p2.x - self.p2.x).into()),
            height: Size::Pixels((self.p2.y - self.p2.y).into()),
            stroke: Color::rgba(0.into(), 0.into(), 1.into(), 0.7.into()),
            fill: Color::rgba(0.into(), 0.into(), 0.into(), 0.2.into()),
        });
    }
}

pub struct PointerTool {
    state: PointerToolState,
}

enum PointerToolState {
    Moving {
        offset: Vector2<Glass>,
    },
    Selecting {
        p1: Point2<Glass>,
        p2: Point2<Glass>,
    },
}

impl PointerTool {
    pub fn new(ctx: &mut ActionContext, point: Point2<Glass>) -> Self {
        Self {
            state: if let Some(hit) = ctx.raycast_glass(point) {
                ctx.app_state.selected_template_node_id = Some(hit.global_id().1);

                let origin_window = hit.origin().unwrap();
                let object_origin_glass = ctx.glass_transform() * origin_window;
                let offset = point - object_origin_glass;
                PointerToolState::Moving { offset }
            } else {
                PointerToolState::Selecting {
                    p1: point,
                    p2: point,
                }
            },
        }
    }
}

impl ToolBehaviour for PointerTool {
    fn pointer_down(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_move(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        match self.state {
            PointerToolState::Moving { offset } => {
                let world_point = ctx.world_transform() * (point - offset);
                ctx.execute(MoveSelected { point: world_point }).unwrap();
            }
            PointerToolState::Selecting { ref mut p2, .. } => *p2 = point,
        }
        ControlFlow::Continue(())
    }

    fn pointer_up(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        // TODO select multiple objects if in PointerToolState::Selecting state
        ControlFlow::Break(())
    }

    fn keyboard(
        &mut self,
        _event: crate::model::input::InputEvent,
        _dir: crate::model::input::Dir,
        _ctx: &mut ActionContext,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn visualize(&self, glass: &mut crate::glass::Glass) {
        if let PointerToolState::Selecting { p1, p2 } = self.state {
            glass.is_rect_tool_active.set(true);
            glass.rect_tool.set(crate::glass::RectTool {
                x: Size::Pixels(p1.x.into()),
                y: Size::Pixels(p1.y.into()),
                width: Size::Pixels((p2.x - p2.x).into()),
                height: Size::Pixels((p2.y - p2.y).into()),
                stroke: Color::rgba(0.into(), 1.into(), 1.into(), 0.7.into()),
                fill: Color::rgba(0.into(), 1.into(), 1.into(), 0.1.into()),
            });
        }
    }
}
