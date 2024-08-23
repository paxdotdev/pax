use std::ops::ControlFlow;

use super::{pointer::Pointer, Action, ActionContext};
use crate::math::coordinate_spaces::{Glass, World};
use crate::model::input::ModifierKey;
use crate::model::{input::InputEvent, AppState, ToolBehavior};
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::{
    api::{Size, Transform2D},
    math::{Generic, Point2, Transform2, Vector2},
    serde,
};
use pax_engine::{log, Property};

pub struct Pan {
    pub start_point: Point2<Glass>,
    pub original_transform: Transform2<Glass, World>,
}

impl ToolBehavior for Pan {
    fn pointer_down(
        &mut self,
        _point: Point2<Glass>,
        _ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_move(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        let diff = self.start_point - point;
        if let Err(e) = (Translate {
            translation: diff,
            original_transform: self.original_transform,
        }
        .perform(ctx))
        {
            log::warn!("failed to translate: {}", e);
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
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

    fn get_visual(&self) -> pax_engine::Property<crate::glass::ToolVisualizationState> {
        Property::new(crate::glass::ToolVisualizationState::default())
    }
}

pub struct Translate {
    pub translation: Vector2<Glass>,
    pub original_transform: Transform2<Glass, World>,
}

impl Action for Translate {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let diff = ctx.world_transform() * self.translation;
        let translation = Transform2::translate(diff);
        ctx.app_state
            .glass_to_world_transform
            .set(translation * self.original_transform);
        Ok(())
    }
}

pub struct Zoom {
    pub closer: bool,
}

impl Action for Zoom {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        if ctx.app_state.modifiers.get().contains(&ModifierKey::Meta) {
            let scale = if self.closer { 1.0 / 1.4 } else { 1.4 };
            ctx.app_state.glass_to_world_transform.update(|transform| {
                *transform = *transform * Transform2::scale(scale);
            });
        }
        Ok(())
    }
}
