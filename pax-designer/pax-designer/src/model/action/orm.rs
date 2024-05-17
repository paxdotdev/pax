use std::f64::consts::PI;

use super::{Action, ActionContext, CanUndo};
use crate::math::coordinate_spaces::{Glass, World};
use crate::math::AxisAlignedBox;
use crate::model::input::InputEvent;
use crate::model::tools::SelectNode;
use crate::{math::BoxPoint, model, model::AppState};
use anyhow::{anyhow, Context, Result};
use pax_designtime::orm::MoveToComponentEntry;
use pax_designtime::DesigntimeManager;
use pax_engine::api::{borrow_mut, Rotation};
use pax_engine::{
    api::Size,
    math::{Point2, Space, Vector2},
    serde,
};
use pax_engine::{log, NodeInterface, Properties};
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
                let b = e
                    .bounds
                    .get()
                    .try_into_space(world_transform)
                    .expect("non-valid world transform");
                MoveToComponentEntry {
                    x: b.top_left().x,
                    y: b.top_left().y,
                    width: b.width(),
                    height: b.height(),
                    id: e.id.clone(),
                }
            })
            .collect();

        let tb = selection
            .total_bounds
            .get()
            .try_into_space(world_transform)
            .expect("non-valid world transform");
        dt.get_orm_mut()
            .move_to_new_component(
                &entries,
                tb.top_left().x,
                tb.top_left().y,
                tb.width(),
                tb.height(),
            )
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
    pub node_box: AxisAlignedBox<World>,
    pub props: &'a Properties,
    pub ignore_coord: (bool, bool),
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

        let bounds = ctx
            .app_state
            .stage
            .read(|stage| (stage.width as f64, stage.height as f64));

        let width = self.node_box.width();
        let height = self.node_box.height();

        let Point2 { x: dx, y: dy, .. } = self.node_box.top_left();

        let x = if let Some(anchor_x) = self.props.anchor_x {
            dx + anchor_x.evaluate((width, height), Axis::X)
        } else {
            // if anchor is not set to figure out the new "virtual"
            // anchor point based on wanted top left position and width/height.
            // (same thing as bellow is done for the y case)
            // equation for new position (since anchor depends on x, solving for x):
            // x = dx + (width/bounds.0)*x =>
            // x*(1 - (width/bounds.0)) = dx => (if width != bounds.0)
            // x = dx/(1 - (width/bounds.0))
            dx / (1.0 - (width / bounds.0))
        };
        let y = if let Some(anchor_y) = self.props.anchor_y {
            dy + anchor_y.evaluate((width, height), Axis::Y)
        } else {
            // same thing here
            dy / (1.0 - (height / bounds.1))
        };
        let x = if x.is_nan() { dx } else { x };
        let y = if y.is_nan() { dy } else { y };

        let percentage_x = (x / bounds.0) * 100.0;
        let percentage_y = (y / bounds.1) * 100.0;
        let perc_width = (width / bounds.0) * 100.0;
        let perc_height = (height / bounds.1) * 100.0;
        if !self.ignore_coord.0 {
            builder.set_property("x", &to_percent(percentage_x))?;
            if self.props.width.is_some() {
                builder.set_property("width", &to_percent(perc_width))?;
            }
        }
        if !self.ignore_coord.1 {
            builder.set_property("y", &to_percent(percentage_y))?;
            if self.props.height.is_some() {
                builder.set_property("height", &to_percent(perc_height))?;
            }
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
        let new_bounds =
            bounds.morph_constrained(self.point, world_anchor, is_alt_key_down, is_shift_key_down);

        let freeze_x = self.attachment_point.x.abs() <= f64::EPSILON;
        let freeze_y = self.attachment_point.y.abs() <= f64::EPSILON;
        ctx.execute(SetBoxSelected {
            node_box: new_bounds,
            props: self.props,
            ignore_coord: (freeze_x, freeze_y),
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
