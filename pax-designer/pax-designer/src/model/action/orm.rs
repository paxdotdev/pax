use std::f64::consts::PI;

use super::{Action, ActionContext, CanUndo};
use crate::math::coordinate_spaces::{Glass, World};
use crate::math::{self, AxisAlignedBox, Unit};
use crate::model::input::InputEvent;
use crate::model::tools::SelectNode;
use crate::{math::BoxPoint, model, model::AppState};
use anyhow::{anyhow, Context, Result};
use pax_designtime::orm::MoveToComponentEntry;
use pax_designtime::DesigntimeManager;
use pax_engine::api::{borrow_mut, Rotation};
use pax_engine::math::{Parts, Transform2};
use pax_engine::{
    api::Size,
    math::{Point2, Space, Vector2},
    serde,
};
use pax_engine::{log, NodeInterface, NodeLocal, Properties};
use pax_manifest::{TypeId, UniqueTemplateNodeIdentifier};
use pax_runtime_api::Axis;

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
                let b = world_transform * e.bounds.get();
                let parts: Parts = b.into();
                MoveToComponentEntry {
                    x: parts.origin.x,
                    y: parts.origin.y,
                    width: parts.scale.x,
                    height: parts.scale.y,
                    id: e.id.clone(),
                }
            })
            .collect();

        let tb = world_transform * selection.total_bounds.get();
        let (o, u, v) = tb.decompose();
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
    pub node_box: Transform2<Glass, NodeLocal>,
    pub props: &'a Properties,
    pub dimension_frozen: (bool, bool),
    pub set_position: bool,
    pub set_size: bool,
    pub unit: Unit,
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
        let new_props: Properties =
            math::transform_to_properties(bounds, self.node_box, self.props);

        // Write new_prop values to ORM
        if let Some(x) = new_props.x {
            builder.set_property("x", &x.to_string())?;
        }
        if let Some(width) = new_props.width {
            builder.set_property("width", &width.to_string())?;
        }
        if let Some(y) = new_props.y {
            builder.set_property("y", &y.to_string())?;
        }
        if let Some(height) = new_props.height {
            builder.set_property("height", &height.to_string())?;
        }
        if let Some(scale_x) = new_props.scale_x {
            builder.set_property("scale_x", &scale_x.to_string())?;
        }
        if let Some(scale_y) = new_props.scale_y {
            builder.set_property("scale_y", &scale_y.to_string())?;
        }
        if let Some(skew_x) = new_props.skew_x {
            builder.set_property("skew_x", &skew_x.to_string())?;
        }
        if let Some(skew_y) = new_props.skew_y {
            builder.set_property("skew_y", &skew_y.to_string())?;
        }
        if let Some(rotation) = new_props.local_rotation {
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

pub struct ResizeSelected<'props> {
    pub attachment_point: Point2<BoxPoint>,
    pub original_bounds: (AxisAlignedBox<World>, Point2<World>),
    pub props: &'props Properties,
    pub point: Point2<World>,
}

impl<'props> Action for ResizeSelected<'props> {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let (bounds, _) = self.original_bounds;

        let mut is_shift_key_down = false;
        let mut is_alt_key_down = false;
        ctx.app_state.keys_pressed.read(|keys| {
            is_shift_key_down = keys.contains(&InputEvent::Shift);
            is_alt_key_down = keys.contains(&InputEvent::Alt);
        });

        let world_anchor = bounds.from_inner_space(self.attachment_point);
        let new_bounds = bounds
            .morph_constrained(self.point, world_anchor, is_alt_key_down, is_shift_key_down)
            .as_transform();

        let freeze_x = self.attachment_point.x.abs() <= f64::EPSILON;
        let freeze_y = self.attachment_point.y.abs() <= f64::EPSILON;

        let unit = if ctx
            .app_state
            .keys_pressed
            .read(|keys| keys.contains(&InputEvent::Meta))
        {
            Unit::Pixels
        } else {
            Unit::Percent
        };

        ctx.execute(SetBoxSelected {
            node_box: new_bounds.cast_spaces(),
            props: self.props,
            dimension_frozen: (freeze_x, freeze_y),
            unit,
            set_position: true,
            set_size: true,
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
