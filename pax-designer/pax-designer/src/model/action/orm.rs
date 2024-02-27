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

pub struct CreateRectangle {
    pub origin: Point2<World>,
    pub dims: Vector2<World>,
}
impl Action for CreateRectangle {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let mut dt = ctx.engine_context.designtime.borrow_mut();
        let mut builder = dt.get_orm_mut().build_new_node(
            ctx.app_state.selected_component_id.clone(),
            "pax_designer::pax_reexports::pax_std::primitives::Rectangle".to_owned(),
            "Rectangle".to_owned(),
            None,
        );
        builder.set_property("x", &to_pixels(self.origin.x))?;
        builder.set_property("y", &to_pixels(self.origin.y))?;
        builder.set_property("width", &to_pixels(self.dims.x))?;
        builder.set_property("height", &to_pixels(self.dims.y))?;

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
            .expect("executed action MoveSelected without a selected object");
        let mut dt = ctx.engine_context.designtime.borrow_mut();

        let mut builder = dt
            .get_orm_mut()
            .get_node(&ctx.app_state.selected_component_id, selected);

        builder.set_property("x", &to_pixels(self.point.x))?;
        builder.set_property("y", &to_pixels(self.point.y))?;
        builder
            .save()
            .map_err(|e| anyhow!("could not move thing: {}", e))?;

        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            // pax_engine::log::debug!("undid move");
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
        let shift_modifier = |v: Vector2<World>| {
            // Bind the resize direction to be in the same
            // direction as the original aspect ratio of this object
            let aspect_ratio = bounds.bottom_right() - bounds.top_left();
            v.coord_abs()
                .project_axis_aligned(aspect_ratio)
                .to_signums_of(v)
        };

        let new_box = if ctx.app_state.keys_pressed.contains(&InputEvent::Alt) {
            // Resize from center if alt is down
            let center = bounds.from_inner_space(Point2::new(0.0, 0.0));
            let mut v = (center - self.point).coord_abs();
            if is_shift_key_down {
                v = shift_modifier(v);
            }
            AxisAlignedBox::new(center + v, center - v)
        } else {
            // Otherwise resize from attachment point
            let resize_anchor = bounds.from_inner_space(self.attachment_point);
            let mut v = self.point - resize_anchor;
            if is_shift_key_down {
                v = shift_modifier(v);
            }
            AxisAlignedBox::new(resize_anchor + v, resize_anchor)
        };

        let origin_relative: Point2<BoxPoint> = bounds.to_inner_space(origin);
        let new_origin_relative = new_box.from_inner_space(origin_relative);

        let mut dt = ctx.engine_context.designtime.borrow_mut();
        let selected = ctx
            .app_state
            .selected_template_node_id
            .expect("executed action ResizeSelected without a selected object");
        let mut builder = dt
            .get_orm_mut()
            .get_node(&ctx.app_state.selected_component_id, selected);

        if self.attachment_point.y.abs() > f64::EPSILON {
            builder.set_property("y", &to_pixels(new_origin_relative.y))?;
            builder.set_property("height", &to_pixels(new_box.height()))?;
        }

        if self.attachment_point.x.abs() > f64::EPSILON {
            builder.set_property("x", &to_pixels(new_origin_relative.x))?;
            builder.set_property("width", &to_pixels(new_box.width()))?;
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
            .expect("executed action ResizeSelected without a selected object");
        let mut builder = dt
            .get_orm_mut()
            .get_node(&ctx.app_state.selected_component_id, selected);

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
    format!("{:?}px", v)
}
