use std::f64::consts::PI;

use super::{Action, ActionContext, CanUndo};
use crate::math::coordinate_spaces::{Glass, World};
use crate::math::{self, AxisAlignedBox, GetUnit, InversionConfiguration, RotationUnit, SizeUnit};
use crate::model::input::InputEvent;
use crate::model::tools::SelectNode;
use crate::{math::BoxPoint, model, model::AppState};
use anyhow::{anyhow, Context, Result};
use pax_designtime::orm::MoveToComponentEntry;
use pax_designtime::DesigntimeManager;
use pax_engine::api::{borrow_mut, Rotation};
use pax_engine::layout::{LayoutProperties, TransformAndBounds};
use pax_engine::math::{Parts, Transform2};
use pax_engine::{
    api::Size,
    math::{Point2, Space, Vector2},
    serde,
};
use pax_engine::{log, NodeInterface, NodeLocal};
use pax_manifest::{TypeId, UniqueTemplateNodeIdentifier};
use pax_runtime_api::{Axis, Percent};

pub struct CreateComponent {
    pub bounds: AxisAlignedBox<World>,
    pub type_id: TypeId,
}

impl Action for CreateComponent {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        let mut builder = dt.get_orm_mut().build_new_node(
            ctx.app_state.selected_component_id.get().clone(),
            self.type_id,
        );
        builder.set_property("x", &to_pixels(self.bounds.top_left().x))?;
        builder.set_property("y", &to_pixels(self.bounds.top_left().y))?;
        builder.set_property("width", &to_pixels(self.bounds.width()))?;
        builder.set_property("height", &to_pixels(self.bounds.height()))?;

        let save_data = builder
            .save()
            .map_err(|e| anyhow!("could not save: {}", e))?;
        ctx.execute(SelectNode {
            id: save_data.unique_id.get_template_node_id(),
            overwrite: true,
        })?;

        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}

pub struct SelectedIntoNewComponent {}

impl Action for SelectedIntoNewComponent {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let selection = ctx.selection_state();
        if selection.selected_count() == 0 {
            return Err(anyhow!("can't create new embty component"));
        };
        let mut dt = borrow_mut!(ctx.engine_context.designtime);

        let world_transform = ctx.world_transform();
        let entries: Vec<_> = selection
            .items
            .iter()
            .map(|e| {
                let b = TransformAndBounds {
                    transform: world_transform,
                    bounds: (1.0, 1.0),
                } * e.transform_and_bounds.get();
                let parts: Parts = b.transform.into();
                MoveToComponentEntry {
                    x: parts.origin.x,
                    y: parts.origin.y,
                    width: parts.scale.x * b.bounds.0,
                    height: parts.scale.y * b.bounds.1,
                    id: e.id.clone(),
                }
            })
            .collect();

        let tb = TransformAndBounds {
            transform: world_transform,
            bounds: (1.0, 1.0),
        } * selection.total_bounds.get();
        let (o, u, v) = tb.transform.decompose();
        let u = u * tb.bounds.0;
        let v = v * tb.bounds.0;
        dt.get_orm_mut()
            .move_to_new_component(&entries, o.x, o.y, u.length(), v.length())
            .map_err(|e| anyhow!("couldn't move to component: {}", e))?;
        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}

pub struct SetBoxSelected<'a> {
    pub node_box: TransformAndBounds<NodeLocal, Glass>,
    pub old_props: &'a LayoutProperties,
}

impl<'a> Action for SetBoxSelected<'a> {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        if ctx.app_state.selected_template_node_ids.get().len() > 1 {
            // TODO support multi-selection movement
            return Ok(CanUndo::No);
        }
        let Some(selected) = ctx
            .app_state
            .selected_template_node_ids
            .read(|ids| ids.get(0).cloned())
        else {
            return Err(anyhow!("tried to move selected but no selected object"));
        };
        let mut dt = borrow_mut!(ctx.engine_context.designtime);

        let Some(mut builder) = dt
            .get_orm_mut()
            .get_node(UniqueTemplateNodeIdentifier::build(
                ctx.app_state.selected_component_id.get(),
                selected.clone(),
            ))
        else {
            return Err(anyhow!("can't move: selected node doesn't exist in orm"));
        };

        // TODO this should be relative to parent later on (when we have contextual drilling)
        let bounds = ctx
            .app_state
            .stage
            .read(|stage| (stage.width as f64, stage.height as f64));

        let inv_config = InversionConfiguration {
            anchor_x: self.old_props.anchor_x,
            anchor_y: self.old_props.anchor_y,
            container_bounds: bounds,
            unit_width: self.old_props.width.unit(),
            unit_height: self.old_props.height.unit(),
            unit_rotation: self.old_props.rotate.unit(),
            unit_x_pos: self.old_props.x.unit(),
            unit_y_pos: self.old_props.y.unit(),
            unit_skew_x: self.old_props.skew_x.unit(),
        };
        let new_props: LayoutProperties = math::transform_and_bounds_inversion(
            inv_config,
            TransformAndBounds {
                transform: ctx.world_transform(),
                bounds: (1.0, 1.0),
            } * self.node_box,
        );

        let LayoutProperties {
            x,
            y,
            width,
            height,
            rotate,
            scale_x,
            scale_y,
            anchor_x,
            anchor_y,
            skew_x,
            skew_y,
        } = new_props;

        // Write new_prop values to ORM
        if let Some(x) = x {
            builder.set_property("x", &x.to_string())?;
        }
        if let Some(width) = width {
            builder.set_property("width", &width.to_string())?;
        }
        if let Some(y) = y {
            builder.set_property("y", &y.to_string())?;
        }
        if let Some(height) = height {
            builder.set_property("height", &height.to_string())?;
        }
        if let Some(scale_x) = scale_x {
            builder.set_property("scale_x", &scale_x.to_string())?;
        }
        if let Some(scale_y) = scale_y {
            builder.set_property("scale_y", &scale_y.to_string())?;
        }
        if let Some(skew_x) = skew_x {
            builder.set_property("skew_x", &skew_x.to_string())?;
        }
        if let Some(skew_y) = skew_y {
            builder.set_property("skew_y", &skew_y.to_string())?;
        }
        if let Some(rotation) = rotate {
            builder.set_property("rotate", &rotation.to_string())?;
        }

        builder
            .save()
            .map_err(|e| anyhow!("could not move thing: {}", e))?;

        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}

pub struct Resize<'props> {
    pub fixed_point: Point2<BoxPoint>,
    pub new_point: Point2<Glass>,
    pub selection_transform_and_bounds: TransformAndBounds<NodeLocal, Glass>,
    pub props: &'props LayoutProperties,
}

impl<'props> Action for Resize<'props> {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let mut is_shift_key_down = false;
        let mut is_alt_key_down = false;
        ctx.app_state.keys_pressed.read(|keys| {
            is_shift_key_down = keys.contains(&InputEvent::Shift);
            is_alt_key_down = keys.contains(&InputEvent::Alt);
        });

        let bounds = self.selection_transform_and_bounds.bounds;
        let (o, vx, vy) = self.selection_transform_and_bounds.transform.decompose();
        let vx = vx * bounds.0;
        let vy = vy * bounds.1;
        let anchor = vx * self.fixed_point.x + vy * self.fixed_point.y;
        let world_anchor: Point2<Glass> = o + anchor;
        let grab_point = o + vx * (1.0 - self.fixed_point.x) + vy * (1.0 - self.fixed_point.y);
        let diff_start = world_anchor - grab_point;
        let diff_now = world_anchor - self.new_point;

        let anchor_relative = self.selection_transform_and_bounds.transform.inverse() * anchor;
        let diff_start_selection_relative =
            self.selection_transform_and_bounds.transform.inverse() * diff_start;
        let diff_now_selection_relative =
            self.selection_transform_and_bounds.transform.inverse() * diff_now;

        let scale = diff_now_selection_relative / diff_start_selection_relative;
        let anchor_shift: Transform2<NodeLocal> = Transform2::translate(anchor_relative);
        let new_box = self.selection_transform_and_bounds
            * TransformAndBounds {
                transform: anchor_shift * Transform2::scale_sep(scale) * anchor_shift.inverse(),
                bounds: (1.0, 1.0),
            };

        ctx.execute(SetBoxSelected {
            node_box: new_box,
            old_props: self.props,
        })?;

        Ok(CanUndo::No)
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
        if ctx
            .app_state
            .keys_pressed
            .get()
            .contains(&InputEvent::Shift)
        {
            angle_deg = (angle_deg / ANGLE_SNAP_DEG).round() * ANGLE_SNAP_DEG;
            if angle_deg >= 360.0 - f64::EPSILON {
                angle_deg = 0.0;
            }
        }

        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        let selected = ctx
            .app_state
            .selected_template_node_ids
            // TODO multi-select
            .get()
            .first()
            .expect("executed action ResizeSelected without a selected object")
            .clone();
        let Some(mut builder) = dt
            .get_orm_mut()
            .get_node(UniqueTemplateNodeIdentifier::build(
                ctx.app_state.selected_component_id.get().clone(),
                selected,
            ))
        else {
            return Err(anyhow!("can't rotate: selected node doesn't exist in orm"));
        };

        builder.set_property("rotate", &format!("{}deg", angle_deg))?;
        builder
            .save()
            .map_err(|e| anyhow!("could not move thing: {}", e))?;
        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}

pub struct DeleteSelected {}

pub struct UndoRequested {}

pub struct SerializeRequested {}

impl Action for SerializeRequested {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        if let Err(e) = dt.send_component_update(&ctx.app_state.selected_component_id.get()) {
            pax_engine::log::error!("failed to save component to file: {:?}", e);
        }
        Ok(CanUndo::No)
    }
}

impl Action for UndoRequested {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        dt.get_orm_mut()
            .undo()
            .map_err(|e| anyhow!("undo failed: {:?}", e))?;
        Ok(CanUndo::No)
    }
}

impl Action for DeleteSelected {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let selected = &ctx.app_state.selected_template_node_ids.get();
        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        for s in selected {
            let uid = UniqueTemplateNodeIdentifier::build(
                ctx.app_state.selected_component_id.get(),
                s.clone(),
            );
            dt.get_orm_mut()
                .remove_node(uid)
                .map_err(|_| anyhow!("couldn't delete node"))?;
        }
        ctx.app_state
            .selected_template_node_ids
            .update(|ids| ids.clear());
        // TODO: this undo doesn't work, need to undo multiple things
        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}

fn to_pixels(v: f64) -> String {
    format!("{:?}px", v.round())
}

fn to_percent(v: f64) -> String {
    format!("{:.2?}%", v)
}
