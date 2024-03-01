use std::rc::Rc;

use super::CanUndo;
use super::{Action, ActionContext};
use crate::model::action::world::Pan;
use crate::model::input::InputEvent;
use crate::model::math::coordinate_spaces::Glass;
use crate::model::tools::{CreateComponentTool, PointerTool};
use crate::model::AppState;
use crate::model::Component;
use crate::model::{action, Tool};
use crate::USERLAND_PROJECT_ID;
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::api::{MouseButton, Window};
use pax_engine::math::Point2;
use pax_manifest::TypeId;

pub struct PointerAction {
    pub event: Pointer,
    pub button: MouseButton,
    pub point: Point2<Window>,
}

#[derive(Clone, Copy)]
pub enum Pointer {
    Down,
    Move,
    Up,
}

impl Action for PointerAction {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let point_glass = ctx.glass_transform() * self.point;
        ctx.app_state.mouse_position = point_glass;
        let spacebar = ctx.app_state.keys_pressed.contains(&InputEvent::Space);
        let tool_behaviour = Rc::clone(&ctx.app_state.tool_behaviour);
        let mut tool_behaviour = tool_behaviour.borrow_mut();

        // If no tool is active, activate a tool on mouse down
        if matches!(self.event, Pointer::Down) && tool_behaviour.is_none() {
            *tool_behaviour = Some(match (self.button, spacebar) {
                (MouseButton::Left, false) => match ctx.app_state.selected_tool {
                    Tool::Pointer => Box::new(PointerTool::new(ctx, point_glass)),
                    Tool::CreateComponent(component) => {
                        let primitive_name = match component {
                            Component::Rectangle => "Rectangle",
                            Component::Ellipse => "Ellipse",
                        };
                        Box::new(CreateComponentTool::new(
                            ctx,
                            point_glass,
                            &TypeId::build_primitive(primitive_name),
                        ))
                    }
                    Tool::TodoTool => todo!(),
                },
                (MouseButton::Left, true) | (MouseButton::Middle, _) => Box::new(Pan {
                    start_point: point_glass,
                    original_transform: ctx.app_state.glass_to_world_transform,
                }),
                _ => panic!("unhandled mouse event"),
            });
        }

        // Whatever tool behaviour exists, let it do it's thing
        let point = ctx.app_state.mouse_position;
        if let Some(tool) = tool_behaviour.as_mut() {
            let res = match self.event {
                Pointer::Down => tool.pointer_down(point, ctx),
                Pointer::Move => tool.pointer_move(point, ctx),
                Pointer::Up => tool.pointer_up(point, ctx),
            };

            // Check if this tool is done and is returning control flow to main app
            match res {
                std::ops::ControlFlow::Continue(_) => (),
                std::ops::ControlFlow::Break(_) => {
                    *tool_behaviour = None;
                    ctx.app_state.selected_tool = Tool::Pointer;
                }
            }
        }
        Ok(CanUndo::No)
    }
}
