use super::{pointer::Pointer, Action, ActionContext, CanUndo};
use crate::model::{
    math::coordinate_spaces::{Glass, Window},
    AppState, ToolState,
};
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
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        match self.event {
            Pointer::Down => {
                let original_offset = ctx.world_transform().get_translation();
                ctx.app_state.tool_state = ToolState::Pan {
                    offset: original_offset,
                    point: self.point,
                };
            }
            Pointer::Move => {
                if let ToolState::Pan { point, offset } = ctx.app_state.tool_state {
                    let diff = self.point - point;
                    ctx.app_state.glass_to_world_transform =
                        Transform2::translate(-diff + offset.to_world());
                }
            }
            Pointer::Up => {
                ctx.app_state.tool_state = ToolState::Idle;
            }
        }
        Ok(CanUndo::No)
    }
}
