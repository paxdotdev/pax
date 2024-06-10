use std::cell::RefCell;
use std::rc::Rc;

use super::CanUndo;
use super::{Action, ActionContext};
use crate::context_menu::ContextMenuMessage;
use crate::math::coordinate_spaces::Glass;
use crate::model::action::world::Pan;
use crate::model::input::InputEvent;
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

pub struct PointerAction<'a> {
    pub prevent_default: &'a dyn Fn(),
    pub event: Pointer,
    pub button: MouseButton,
    pub point: Point2<Window>,
}

#[derive(Clone, PartialEq, Copy)]
pub enum Pointer {
    Down,
    Move,
    Up,
}

impl Action for PointerAction<'_> {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let point_glass = ctx.glass_transform().get() * self.point;
        ctx.app_state.mouse_position.set(point_glass);
        let spacebar = ctx
            .app_state
            .keys_pressed
            .get()
            .contains(&InputEvent::Space);
        let tool_behaviour = ctx.app_state.tool_behaviour.clone();
        // Open context menu on right mouse button click no matter what
        if matches!(
            (self.event, self.button.clone()),
            (Pointer::Down, MouseButton::Right)
        ) {
            ctx.execute(ContextMenuMessage::Open { pos: point_glass })?;
            return Ok(CanUndo::No);
        }

        if matches!(self.event, Pointer::Down) {
            ctx.execute(ContextMenuMessage::Close)?;
        }

        // If no tool is active, activate a tool on mouse down
        if matches!(self.event, Pointer::Down) && tool_behaviour.get().is_none() {
            match (self.button, spacebar) {
                (MouseButton::Left, false) => match ctx.app_state.selected_tool.get() {
                    Tool::Pointer => {
                        (self.prevent_default)();
                        tool_behaviour.set(Some(Rc::new(RefCell::new(PointerTool::new(
                            ctx,
                            point_glass,
                        )))));
                    }
                    Tool::CreateComponent(component) => {
                        let primitive_name = match component {
                            Component::Rectangle => "Rectangle",
                            Component::Ellipse => "Ellipse",
                            Component::Text => "Text",
                        };
                        tool_behaviour.set(Some(Rc::new(RefCell::new(CreateComponentTool::new(
                            ctx,
                            point_glass,
                            &TypeId::build_singleton(
                                &format!(
                                    "pax_designer::pax_reexports::pax_std::primitives::{}",
                                    primitive_name
                                ),
                                None,
                            ),
                        )))));
                    }
                    Tool::TodoTool => todo!(),
                },
                (MouseButton::Left, true) | (MouseButton::Middle, _) => {
                    tool_behaviour.set(Some(Rc::new(RefCell::new(Pan {
                        start_point: point_glass,
                        original_transform: ctx.app_state.glass_to_world_transform.get(),
                    }))));
                }
                _ => (),
            };
        }

        // Whatever tool behaviour exists, let it do it's thing
        let point = ctx.app_state.mouse_position.get();
        tool_behaviour.update(|tool_behaviour| {
            let res = if let Some(tool) = tool_behaviour {
                let mut tool = tool.borrow_mut();
                let res = match self.event {
                    Pointer::Down => tool.pointer_down(point, ctx),
                    Pointer::Move => tool.pointer_move(point, ctx),
                    Pointer::Up => tool.pointer_up(point, ctx),
                };
                Some(res)
            } else {
                None
            };
            // Check if this tool is done and is returning control flow to main app
            if let Some(res) = res {
                match res {
                    std::ops::ControlFlow::Continue(_) => (),
                    std::ops::ControlFlow::Break(_) => {
                        *tool_behaviour = None;
                        ctx.app_state.selected_tool.set(Tool::Pointer);
                    }
                }
            }
        });
        Ok(CanUndo::No)
    }
}
