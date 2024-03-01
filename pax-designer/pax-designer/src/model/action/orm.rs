use std::f64::consts::PI;

use super::{Action, ActionContext, CanUndo};
use crate::math::AxisAlignedBox;
use crate::model::input::InputEvent;
use crate::{
    math::BoxPoint,
    model::{
        math::coordinate_spaces::{Glass, World},
        AppState,
    },
};
use anyhow::{anyhow, Context, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::api::Rotation;
use pax_engine::NodeInterface;
use pax_engine::{
    api::Size,
    math::{Point2, Space, Vector2},
    serde,
};
use pax_manifest::{TypeId, UniqueTemplateNodeIdentifier};

pub struct CreateComponent {
    pub bounds: AxisAlignedBox<World>,
    pub type_id: TypeId,
}
impl Action for CreateComponent {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let mut dt = ctx.engine_context.designtime.borrow_mut();
        let mut builder = dt
            .get_orm_mut()
            .build_new_node(ctx.app_state.selected_component_id.clone(), self.type_id);
        builder.set_property("x", &to_pixels(self.bounds.top_left().x))?;
        builder.set_property("y", &to_pixels(self.bounds.top_left().y))?;
        builder.set_property("width", &to_pixels(self.bounds.width()))?;
        builder.set_property("height", &to_pixels(self.bounds.height()))?;

        builder
            .save()
            .map_err(|e| anyhow!("could not save: {}", e))?;

        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            let mut dt = ctx.engine_context.designtime.borrow_mut();
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}

pub struct MoveSelected {
    pub point: Point2<World>,
}

impl Action for MoveSelected {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let selected = ctx
            .app_state
            .selected_template_node_id
            .clone()
            .expect("executed action MoveSelected without a selected object");
        let mut dt = ctx.engine_context.designtime.borrow_mut();

        let mut builder = dt
            .get_orm_mut()
            .get_node(UniqueTemplateNodeIdentifier::build(
                ctx.app_state.selected_component_id.clone(),
                selected,
            ));

        builder.set_property("x", &to_pixels(self.point.x))?;
        builder.set_property("y", &to_pixels(self.point.y))?;
        builder
            .save()
            .map_err(|e| anyhow!("could not move thing: {}", e))?;

        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            let mut dt = ctx.engine_context.designtime.borrow_mut();
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}

pub struct ResizeSelected {
    pub attachment_point: Point2<BoxPoint>,
    pub original_bounds: (AxisAlignedBox<World>, Point2<World>),
    pub point: Point2<World>,
}

impl Action for ResizeSelected {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let (bounds, origin) = self.original_bounds;

        let is_shift_key_down = ctx.app_state.keys_pressed.contains(&InputEvent::Shift);
        let is_alt_key_down = ctx.app_state.keys_pressed.contains(&InputEvent::Alt);

        let world_anchor = bounds.from_inner_space(self.attachment_point);
        let new_bounds =
            bounds.morph_constrained(self.point, world_anchor, is_alt_key_down, is_shift_key_down);

        let origin_relative: Point2<BoxPoint> = bounds.to_inner_space(origin);
        let new_origin_relative = new_bounds.from_inner_space(origin_relative);

        let mut dt = ctx.engine_context.designtime.borrow_mut();
        let selected = ctx
            .app_state
            .selected_template_node_id
            .clone()
            .expect("executed action ResizeSelected without a selected object");
        let mut builder = dt
            .get_orm_mut()
            .get_node(UniqueTemplateNodeIdentifier::build(
                ctx.app_state.selected_component_id.clone(),
                selected,
            ));

        if self.attachment_point.y.abs() > f64::EPSILON {
            builder.set_property("y", &to_pixels(new_origin_relative.y))?;
            builder.set_property("height", &to_pixels(new_bounds.height()))?;
        }

        if self.attachment_point.x.abs() > f64::EPSILON {
            builder.set_property("x", &to_pixels(new_origin_relative.x))?;
            builder.set_property("width", &to_pixels(new_bounds.width()))?;
        }

        builder
            .save()
            .map_err(|e| anyhow!("could not move thing: {}", e))?;

        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            let mut dt = ctx.engine_context.designtime.borrow_mut();
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}

const ANGLE_SNAP_DEG: f64 = 45.0;

pub struct RotateSelected {
    pub rotation_anchor: Point2<Glass>,
    pub moving_from: Vector2<Glass>,
    pub moving_to: Vector2<Glass>,
    pub start_angle: Rotation,
}

impl Action for RotateSelected {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let angle_diff = self.moving_from.angle_to(self.moving_to);
        let new_rot = angle_diff + self.start_angle;

        let mut angle_deg = new_rot.get_as_degrees().rem_euclid(360.0);
        if ctx.app_state.keys_pressed.contains(&InputEvent::Shift) {
            angle_deg = (angle_deg / ANGLE_SNAP_DEG).round() * ANGLE_SNAP_DEG;
            if angle_deg >= 360.0 - f64::EPSILON {
                angle_deg = 0.0;
            }
        }

        let mut dt = ctx.engine_context.designtime.borrow_mut();
        let selected = ctx
            .app_state
            .selected_template_node_id
            .clone()
            .expect("executed action ResizeSelected without a selected object");
        let mut builder = dt
            .get_orm_mut()
            .get_node(UniqueTemplateNodeIdentifier::build(
                ctx.app_state.selected_component_id.clone(),
                selected,
            ));

        builder.set_property("rotate", &format!("{}deg", angle_deg))?;
        builder
            .save()
            .map_err(|e| anyhow!("could not move thing: {}", e))?;
        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            let mut dt = ctx.engine_context.designtime.borrow_mut();
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}
fn to_pixels(v: f64) -> String {
    format!("{:?}px", v.round())
}
