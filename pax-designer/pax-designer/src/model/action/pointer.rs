use super::CanUndo;
use super::{Action, ActionContext};
use crate::model::action;
use crate::model::input::InputEvent;
use crate::model::math::coordinate_spaces::Glass;
use crate::model::AppState;
use crate::model::ToolState;
use crate::USERLAND_PROJECT_ID;
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::api::{MouseButton, Window};
use pax_engine::math::Point2;

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
        match (self.button, spacebar) {
            (MouseButton::Left, false) => ctx.execute(action::tools::ToolAction {
                event: self.event,
                point: point_glass,
            }),
            (MouseButton::Left, true) | (MouseButton::Middle, _) => {
                ctx.execute(action::world::Pan {
                    event: self.event,
                    point: point_glass,
                })
            }
            _ => Ok(()),
        }?;
        Ok(CanUndo::No)
    }
}
