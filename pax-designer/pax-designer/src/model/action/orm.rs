use std::f64::consts::PI;

use super::{Action, ActionContext, CanUndo};
use crate::math::coordinate_spaces::{Glass, SelectionSpace, World};
use crate::math::{self, AxisAlignedBox, GetUnit, InversionConfiguration, RotationUnit, SizeUnit};
use crate::model::input::InputEvent;
use crate::model::tools::SelectNode;
use crate::{math::BoxPoint, model, model::AppState};
use anyhow::{anyhow, Context, Result};
use pax_designtime::orm::MoveToComponentEntry;
use pax_designtime::DesigntimeManager;
use pax_engine::api::{borrow_mut, Rotation};
use pax_engine::layout::{LayoutProperties, TransformAndBounds};
use pax_engine::math::{Generic, Parts, Transform2};
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
        let v = v * tb.bounds.1;
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

pub struct SetBoxSelected {
    pub id: UniqueTemplateNodeIdentifier,
    pub node_box: TransformAndBounds<NodeLocal, Glass>,
    pub inv_config: InversionConfiguration,
}

impl Action for SetBoxSelected {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        let Some(mut builder) = dt.get_orm_mut().get_node(self.id) else {
            return Err(anyhow!("can't move: node doesn't exist in orm"));
        };

        let new_props: LayoutProperties = math::transform_and_bounds_inversion(
            self.inv_config,
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

pub struct Resize<'a> {
    pub fixed_point: Point2<BoxPoint>,
    pub new_point: Point2<Glass>,
    pub selection_transform_and_bounds: TransformAndBounds<SelectionSpace, Glass>,
    pub objects: &'a [SelectedObject],
}

pub struct SelectedObject {
    pub id: UniqueTemplateNodeIdentifier,
    pub transform_and_bounds: TransformAndBounds<NodeLocal, Glass>,
    pub layout_properties: LayoutProperties,
}

impl Action for Resize<'_> {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let mut is_shift_key_down = false;
        let mut is_alt_key_down = false;
        ctx.app_state.keys_pressed.read(|keys| {
            is_shift_key_down = keys.contains(&InputEvent::Shift);
            is_alt_key_down = keys.contains(&InputEvent::Alt);
        });

        let bounds = self.selection_transform_and_bounds.bounds;
        let selection_space = self.selection_transform_and_bounds.transform
            * Transform2::scale_sep(Vector2::new(bounds.0, bounds.1));
        let fixed: Point2<SelectionSpace> = self.fixed_point.cast_space();
        let grab = (Vector2::new(1.0, 1.0) - fixed.to_vector()).to_point();
        let new_in_selec = selection_space.inverse() * self.new_point;
        let diff_start = fixed - grab;
        let diff_now = fixed - new_in_selec;

        let scale = diff_now / diff_start;
        let anchor: Transform2<SelectionSpace> = Transform2::translate(fixed.to_vector());

        // this is the transform to apply to all of the objects that are being resized
        let to_local = TransformAndBounds {
            transform: selection_space * anchor,
            bounds: (1.0, 1.0),
        };
        // let local_resize = TransformAndBounds {
        //     transform: Transform2::scale_sep(scale),
        //     bounds: (1.0, 1.0),
        // };
        let local_resize = TransformAndBounds {
            transform: Transform2::identity(),
            bounds: (scale.x, scale.y),
        };

        let resize = to_local * local_resize * to_local.inverse();

        // TODO this should be relative to each nodes parent later on (when we have contextual drilling)
        // (most likely there's always only one parent?)
        let container_bounds = ctx
            .app_state
            .stage
            .read(|stage| (stage.width as f64, stage.height as f64));

        for object in self.objects {
            let inv_config = InversionConfiguration {
                container_bounds,
                anchor_x: object.layout_properties.anchor_x,
                anchor_y: object.layout_properties.anchor_y,
                // TODO override some units here
                unit_width: object.layout_properties.width.unit(),
                unit_height: object.layout_properties.height.unit(),
                unit_rotation: object.layout_properties.rotate.unit(),
                unit_x_pos: object.layout_properties.x.unit(),
                unit_y_pos: object.layout_properties.y.unit(),
                unit_skew_x: object.layout_properties.skew_x.unit(),
            };
            ctx.execute(SetBoxSelected {
                id: object.id.clone(),
                node_box: resize * object.transform_and_bounds,
                inv_config,
            })?;
        }

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
