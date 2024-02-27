use std::ops::ControlFlow;

use super::{pointer::Pointer, Action, ActionContext, CanUndo};
use crate::model::{
    input::InputEvent,
    math::coordinate_spaces::{Glass, World},
    AppState, ToolBehaviour,
};
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::{
    api::{Size, Transform2D},
    math::{Generic, Point2, Transform2, Vector2},
    serde,
};

pub struct Pan {
    pub start_point: Point2<Glass>,
    pub original_transform: Transform2<Glass, World>,
}

impl ToolBehaviour for Pan {
    fn pointer_down(
        &mut self,
        _point: Point2<Glass>,
        _ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_move(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        let diff = ctx.world_transform() * (self.start_point - point);
        let translation = Transform2::translate(diff);
        ctx.app_state.glass_to_world_transform = translation * self.original_transform;
        ControlFlow::Continue(())
    }

    fn pointer_up(
        &mut self,
        _point: Point2<Glass>,
        _ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        ControlFlow::Break(())
    }

    fn keyboard(
        &mut self,
        _event: InputEvent,
        _dir: crate::model::input::Dir,
        _ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn visualize(&self, _glass: &mut crate::glass::Glass) {}
}

pub struct Zoom {
    pub closer: bool,
}

impl Action for Zoom {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        if ctx.app_state.keys_pressed.contains(&InputEvent::Control) {
            let scale = if self.closer { 1.0 / 1.4 } else { 1.4 };
            ctx.app_state.glass_to_world_transform =
                ctx.app_state.glass_to_world_transform * Transform2::scale(scale);
        }
        Ok(CanUndo::No)
    }
}
