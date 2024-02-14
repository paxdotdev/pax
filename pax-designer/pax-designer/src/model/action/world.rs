use super::{pointer::Pointer, Action, ActionContext, CanUndo};
use crate::model::{AppState, ToolState};
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::{
    api::{Size, Transform2D},
    rendering::{kurbo, Point2D},
    serde,
};

pub struct Pan {
    pub event: Pointer,
    pub point: Point2D,
}

impl Action for Pan {
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        match self.event {
            Pointer::Down => {
                let world = ctx.app_state.glass_to_world_transform.translation();
                let base = Point2D {
                    x: world.x - self.point.x,
                    y: world.y - self.point.y,
                };
                ctx.app_state.tool_state = ToolState::Movement { delta: base };
            }
            Pointer::Move => {
                if let ToolState::Movement { delta } = ctx.app_state.tool_state {
                    let point = self.point + delta;
                    ctx.app_state.glass_to_world_transform = ctx
                        .app_state
                        .glass_to_world_transform
                        .with_translation(kurbo::Vec2::new(point.x, point.y));
                }
            }
            Pointer::Up => {
                ctx.app_state.tool_state = ToolState::Idle;
            }
        }
        // ctx.app_state.glass_to_world_transform.then_translate(Vec2 {
        //     x: self.delta.x,
        //     y: self.delta.y,
        // });
        Ok(CanUndo::No)
    }
}
