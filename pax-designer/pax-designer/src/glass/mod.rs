use pax_engine::api::*;
use pax_engine::math::Point2;
use pax_engine::*;
use pax_std::primitives::{Group, Path, Rectangle};
use pax_std::types::Fill;
use serde::Deserialize;

use crate::model;
use crate::model::AppState;

use crate::math;
use crate::math::coordinate_spaces::{self, World};
use crate::model::action::pointer::Pointer;
use crate::model::input::Dir;

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
    pub fn context_menu(&mut self, ctx: &NodeContext, args: Event<ContextMenu>) {
        args.prevent_default();
    }

    pub fn handle_mouse_down(&mut self, ctx: &NodeContext, args: Event<MouseDown>) {
        model::perform_action(
            crate::model::action::pointer::PointerAction {
                event: Pointer::Down,
                button: args.mouse.button.clone(),
                point: Point2::new(args.mouse.x, args.mouse.y),
            },
            ctx,
        );
    }

    pub fn handle_mouse_move(&mut self, ctx: &NodeContext, args: Event<MouseMove>) {
        model::perform_action(
            crate::model::action::pointer::PointerAction {
                event: Pointer::Move,
                button: args.mouse.button.clone(),
                point: Point2::new(args.mouse.x, args.mouse.y),
            },
            ctx,
        );
    }

    pub fn handle_mouse_up(&mut self, ctx: &NodeContext, args: Event<MouseUp>) {
        model::perform_action(
            crate::model::action::pointer::PointerAction {
                event: Pointer::Up,
                button: args.mouse.button.clone(),
                point: Point2::new(args.mouse.x, args.mouse.y),
            },
            ctx,
        );
    }

    pub fn handle_key_down(&mut self, ctx: &NodeContext, args: Event<KeyDown>) {
        model::process_keyboard_input(ctx, Dir::Down, args.keyboard.key.clone());
    }

    pub fn handle_key_up(&mut self, ctx: &NodeContext, args: Event<KeyUp>) {
        model::process_keyboard_input(ctx, Dir::Up, args.keyboard.key.clone());
    }

    pub fn update_view(&mut self, _ctx: &NodeContext) {
        model::read_app_state(|app_state| {
            // Draw current tool visuals
            // this could be factored out into it's own component as well eventually
            if let Some(tool) = app_state.tool_behaviour.borrow().as_ref() {
                tool.visualize(self);
            } else {
                self.is_rect_tool_active.set(false);
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
#[derive(Debug)]
pub struct RectTool {
    pub x: Size,
    pub y: Size,
    pub width: Size,
    pub height: Size,
    pub fill: Color,
    pub stroke: Color,
}
