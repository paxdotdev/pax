use std::f64::consts::PI;

use super::{Action, ActionContext, CanUndo};
use crate::math::coordinate_spaces::{Glass, SelectionSpace, World};
use crate::math::{self, AxisAlignedBox, GetUnit, InversionConfiguration, RotationUnit, SizeUnit};
use crate::model::input::InputEvent;
use crate::model::tools::SelectNode;
use crate::model::SelectionStateSnapshot;
use crate::{math::BoxPoint, model, model::AppState};
use anyhow::{anyhow, Context, Result};
use pax_designtime::orm::template::builder::NodeBuilder;
use pax_designtime::orm::MoveToComponentEntry;
use pax_designtime::{DesigntimeManager, Serializer};
use pax_engine::api::{borrow_mut, Rotation};
use pax_engine::layout::{LayoutProperties, TransformAndBounds};
use pax_engine::math::{Generic, Parts, Transform2};
use pax_engine::serde::Serialize;
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
        if selection.items.len() == 0 {
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
            anchor_x: _,
            anchor_y: _,
            skew_x,
            skew_y,
        } = new_props;

        fn write_to_orm<T: Serialize + Default + PartialEq>(
            builder: &mut NodeBuilder,
            name: &str,
            value: Option<T>,
            is_close_to_default: impl Fn(&T) -> bool + 'static,
        ) -> Result<()> {
            if let Some(val) = value {
                if !is_close_to_default(&val) {
                    let val = pax_designtime::to_pax(&val)?;
                    builder.set_property(name, &val)?;
                } else {
                    builder.set_property(name, "")?;
                }
            };
            Ok(())
        }
        const EPS: f64 = 1e-3;
        fn is_size_default(s: &Size) -> bool {
            match s {
                Size::Pixels(p) => p.to_float().abs() < EPS,
                Size::Percent(p) => (p.to_float() - 100.0).abs() < EPS,
                Size::Combined(pix, per) => {
                    pix.to_float() < EPS && (per.to_float() - 100.0).abs() < EPS
                }
            }
        }
        let is_rotation_default = |r: &Rotation| (r.get_as_degrees() % 360.0).abs() < EPS;

        write_to_orm(&mut builder, "x", x, is_size_default)?;
        write_to_orm(&mut builder, "y", y, is_size_default)?;
        write_to_orm(&mut builder, "width", width, is_size_default)?;
        write_to_orm(&mut builder, "height", height, is_size_default)?;
        write_to_orm(&mut builder, "rotate", rotate, is_rotation_default)?;
        write_to_orm(
            &mut builder,
            "scale_x",
            scale_x.map(|v| Size::Percent(v.0)),
            is_size_default,
        )?;
        write_to_orm(
            &mut builder,
            "scale_y",
            scale_y.map(|v| Size::Percent(v.0)),
            is_size_default,
        )?;
        write_to_orm(&mut builder, "skew_x", skew_x, is_rotation_default)?;
        write_to_orm(&mut builder, "skew_y", skew_y, is_rotation_default)?;
        // always skip writing anchor? (separate method)

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
    pub initial_selection: &'a SelectionStateSnapshot,
}

impl Action for Resize<'_> {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let mut is_shift_key_down = false;
        let mut is_alt_key_down = false;
        ctx.app_state.keys_pressed.read(|keys| {
            is_shift_key_down = keys.contains(&InputEvent::Shift);
            is_alt_key_down = keys.contains(&InputEvent::Alt);
        });

        let bounds = self.initial_selection.total_bounds.bounds;
        let selection_space = self.initial_selection.total_bounds.transform
            * Transform2::scale_sep(Vector2::new(bounds.0, bounds.1));
        let grab = (Vector2::new(1.0, 1.0) - self.fixed_point.cast_space().to_vector()).to_point();
        let new_in_selec = selection_space.inverse() * self.new_point;

        // if alt key is down, resize from anchor instead
        let fixed: Point2<SelectionSpace> = if is_alt_key_down {
            let anchor = self.initial_selection.total_origin;
            selection_space.inverse() * anchor
        } else {
            self.fixed_point.cast_space()
        };

        let diff_start = fixed - grab;
        let mut diff_now = fixed - new_in_selec;

        // if shift key down, the project diff_now on diff start
        // either along x or y axis (whichever is closest)
        if is_shift_key_down {
            diff_now = diff_now
                .coord_abs()
                .project_axis_aligned(diff_start)
                .to_signums_of(diff_now);
        }

        let mut scale = diff_now / diff_start;

        // if grabbing from sides, only resize in one direciton
        if self.fixed_point.x == 0.5 {
            scale.x = 1.0;
        }
        if self.fixed_point.y == 0.5 {
            scale.y = 1.0;
        }

        let anchor: Transform2<SelectionSpace> = Transform2::translate(fixed.to_vector());

        // This is the "frame of refernce" from which all objects that
        // are currently selected should be resized
        let to_local = TransformAndBounds {
            transform: selection_space * anchor,
            bounds: (1.0, 1.0),
        };

        // TODO hook up switching between scaling and resizing mode (commented out scaling for now):
        // this is the transform to apply to all of the objects that are being resized
        let local_resize = TransformAndBounds {
            transform: Transform2::identity(),
            bounds: (scale.x, scale.y),
        };
        // let local_resize = TransformAndBounds {
        //     transform: Transform2::scale_sep(scale),
        //     bounds: (1.0, 1.0),
        // };

        // nove to "frame of reference", perform operation, move back
        // TODO refactor so that things like rotation are also just a "local_resize" transform that is performing a rotation,
        // most likely from center of selection (at least when multiple)?
        let resize = to_local * local_resize * to_local.inverse();

        // TODO this should be relative to each nodes parent later on (when we have contextual drilling)
        // (most likely there's always only one parent?)
        let container_bounds = ctx
            .app_state
            .stage
            .read(|stage| (stage.width as f64, stage.height as f64));

        for item in &self.initial_selection.items {
            let inv_config = InversionConfiguration {
                container_bounds,
                anchor_x: item.layout_properties.anchor_x,
                anchor_y: item.layout_properties.anchor_y,
                // TODO override some units here
                unit_width: item.layout_properties.width.unit(),
                unit_height: item.layout_properties.height.unit(),
                unit_rotation: item.layout_properties.rotate.unit(),
                unit_x_pos: item.layout_properties.x.unit(),
                unit_y_pos: item.layout_properties.y.unit(),
                unit_skew_x: item.layout_properties.skew_x.unit(),
            };
            ctx.execute(SetBoxSelected {
                id: item.id.clone(),
                node_box: resize * item.transform_and_bounds,
                inv_config,
            })?;
        }

        Ok(CanUndo::No)
    }
}

const ANGLE_SNAP_DEG: f64 = 45.0;

pub struct RotateSelected<'a> {
    pub start_pos: Point2<Glass>,
    pub curr_pos: Point2<Glass>,
    pub initial_selection: &'a SelectionStateSnapshot,
}

impl Action for RotateSelected<'_> {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let anchor_point = self.initial_selection.total_origin;
        let start = self.start_pos - anchor_point;
        let curr = self.curr_pos - anchor_point;
        let mut rotation = start.angle_to(curr).get_as_degrees();

        if ctx
            .app_state
            .keys_pressed
            .get()
            .contains(&InputEvent::Shift)
        {
            rotation = (rotation / ANGLE_SNAP_DEG).round() * ANGLE_SNAP_DEG;
            if rotation >= 360.0 - f64::EPSILON {
                rotation = 0.0;
            }
        }

        // This is the "frame of refernce" from which all objects that
        // are currently selected should be resized
        let to_local = TransformAndBounds {
            transform: Transform2::<SelectionSpace, Glass>::translate(anchor_point.to_vector()),
            bounds: (1.0, 1.0),
        };

        let local_resize = TransformAndBounds {
            transform: Transform2::rotate(rotation.to_radians()),
            bounds: (1.0, 1.0),
        };

        // nove to "frame of reference", perform operation, move back
        // TODO refactor so that things like rotation are also just a "local_resize" transform that is performing a rotation,
        // most likely from center of selection (at least when multiple)?
        let resize = to_local * local_resize * to_local.inverse();

        // TODO this should be relative to each nodes parent later on (when we have contextual drilling)
        // (most likely there's always only one parent?)
        let container_bounds = ctx
            .app_state
            .stage
            .read(|stage| (stage.width as f64, stage.height as f64));

        for item in &self.initial_selection.items {
            let inv_config = InversionConfiguration {
                container_bounds,
                anchor_x: item.layout_properties.anchor_x,
                anchor_y: item.layout_properties.anchor_y,
                // TODO override some units here
                unit_width: item.layout_properties.width.unit(),
                unit_height: item.layout_properties.height.unit(),
                unit_rotation: item.layout_properties.rotate.unit(),
                unit_x_pos: item.layout_properties.x.unit(),
                unit_y_pos: item.layout_properties.y.unit(),
                unit_skew_x: item.layout_properties.skew_x.unit(),
            };
            ctx.execute(SetBoxSelected {
                id: item.id.clone(),
                node_box: resize * item.transform_and_bounds,
                inv_config,
            })?;
        }

        Ok(CanUndo::No)
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
