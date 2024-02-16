use super::CanUndo;
use super::{Action, ActionContext};
use crate::model::action;
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
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        let point_glass = ctx.glass_transform() * self.point;
        match self.button {
            MouseButton::Left => ctx.execute(action::tools::ToolAction {
                event: self.event,
                point: point_glass,
            }),
            MouseButton::Middle => ctx.execute(action::world::Pan {
                event: self.event,
                point: point_glass,
            }),
            _ => Ok(()),
        }?;
        Ok(CanUndo::No)
    }
}
