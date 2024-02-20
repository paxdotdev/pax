use super::{pointer::Pointer, Action, ActionContext, CanUndo};
use crate::model::{input::InputEvent, math::coordinate_spaces::Glass, AppState, ToolState};
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::{
    api::{Size, Transform2D},
    math::{Generic, Point2, Transform2, Vector2},
    serde,
};

pub struct Pan {
    pub event: Pointer,
    pub point: Point2<Glass>,
}

impl Action for Pan {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        match self.event {
            Pointer::Down => {
                let original_transform = ctx.world_transform();
                ctx.app_state.tool_state = ToolState::Pan {
                    original: original_transform,
                    origin: self.point,
                };
            }
            Pointer::Move => {
                if let ToolState::Pan { original, origin } = ctx.app_state.tool_state {
                    let diff = ctx.world_transform() * (origin - self.point);
                    let translation = Transform2::translate(diff);
                    ctx.app_state.glass_to_world_transform = translation * original;
                }
            }
            Pointer::Up => {
                ctx.app_state.tool_state = ToolState::Idle;
            }
        }
        Ok(CanUndo::No)
    }
}

pub struct Zoom {
    pub closer: bool,
}

impl Action for Zoom {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        pax_engine::log::info!("zoom");
        if ctx.app_state.keys_pressed.contains(&InputEvent::Control) {
            pax_engine::log::info!("zooming");
            let scale = if self.closer { 1.4 } else { 1.0 / 1.4 };
            ctx.app_state.glass_to_world_transform =
                ctx.app_state.glass_to_world_transform * Transform2::scale(scale);
        }
        Ok(CanUndo::No)
    }
}
