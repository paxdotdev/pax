use pax_engine::api::*;
use pax_engine::math::Point2;
use pax_engine::*;
use pax_std::primitives::{Group, Path, Rectangle};
use pax_std::types::{Color, Fill};
use serde::Deserialize;

use crate::model;
use crate::model::AppState;
use crate::model::ToolState;

use crate::model::action::pointer::Pointer;
use crate::model::input::Dir;
use crate::model::math;
use crate::model::math::coordinate_spaces::{self, World};

pub mod control_point;
pub mod object_editor;
use control_point::ControlPoint;

use object_editor::ObjectEditor;

#[pax]
#[custom(Default)]
#[file("glass/mod.pax")]
pub struct Glass {
    // rect tool state
    pub is_rect_tool_active: Property<bool>,
    pub rect_tool: Property<RectTool>,
}

impl Glass {
    pub fn handle_mouse_down(&mut self, ctx: &NodeContext, args: ArgsMouseDown) {
        model::perform_action(
            crate::model::action::pointer::PointerAction {
                event: Pointer::Down,
                button: args.mouse.button,
                point: Point2::new(args.mouse.x, args.mouse.y),
            },
            ctx,
        );
    }

    pub fn handle_mouse_move(&mut self, ctx: &NodeContext, args: ArgsMouseMove) {
        model::perform_action(
            crate::model::action::pointer::PointerAction {
                event: Pointer::Move,
                button: args.mouse.button,
                point: Point2::new(args.mouse.x, args.mouse.y),
            },
            ctx,
        );
    }

    pub fn handle_mouse_up(&mut self, ctx: &NodeContext, args: ArgsMouseUp) {
        model::perform_action(
            crate::model::action::pointer::PointerAction {
                event: Pointer::Up,
                button: args.mouse.button,
                point: Point2::new(args.mouse.x, args.mouse.y),
            },
            ctx,
        );
    }

    pub fn handle_key_down(&mut self, ctx: &NodeContext, args: ArgsKeyDown) {
        model::process_keyboard_input(ctx, Dir::Down, args.keyboard.key);
    }

    pub fn handle_key_up(&mut self, ctx: &NodeContext, args: ArgsKeyUp) {
        model::process_keyboard_input(ctx, Dir::Up, args.keyboard.key);
    }

    pub fn update_view(&mut self, _ctx: &NodeContext) {
        model::read_app_state(|app_state| {
            // Draw current tool visuals
            // this could be factored out into it's own component as well eventually
            match &app_state.tool_state {
                ToolState::BoxSelect {
                    p1,
                    p2,
                    fill,
                    stroke,
                } => {
                    self.is_rect_tool_active.set(true);
                    self.rect_tool.set(RectTool {
                        x: Size::Pixels(p1.x.into()),
                        y: Size::Pixels(p1.y.into()),
                        width: Size::Pixels((p2.x - p1.x).into()),
                        height: Size::Pixels((p2.y - p1.y).into()),
                        fill: fill.clone(),
                        stroke: stroke.clone(),
                    });
                }
                ToolState::MovingObject { .. } => (),
                _ => {
                    // reset all tool visuals
                    self.is_rect_tool_active.set(false);
                }
            }
        });
    }
}

impl Default for Glass {
    fn default() -> Self {
        Self {
            is_rect_tool_active: Box::new(PropertyLiteral::new(false)),
            rect_tool: Default::default(),
        }
    }
}

#[pax]
pub struct RectTool {
    pub x: Size,
    pub y: Size,
    pub width: Size,
    pub height: Size,
    pub fill: Color,
    pub stroke: Color,
}
