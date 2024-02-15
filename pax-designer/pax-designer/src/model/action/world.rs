use super::{pointer::Pointer, Action, ActionContext, CanUndo};
use crate::model::{math::Glass, AppState, ToolState};
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::{
    api::{Size, Transform2D},
    math::{Generic, Point2, Vector2},
    rendering::kurbo,
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
                let world = ctx.app_state.glass_to_world_transform.translation();
                let base = Point2::<Generic>::new(world.x - self.point.x, world.y - self.point.y);
                ctx.app_state.tool_state = ToolState::Movement {
                    x: base.x,
                    y: base.y,
                };
            }
            Pointer::Move => {
                if let ToolState::Movement { x, y } = ctx.app_state.tool_state {
                    let point = self.point + Vector2::new(x, y);
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
