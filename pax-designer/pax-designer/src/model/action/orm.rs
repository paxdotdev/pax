use std::f64::consts::PI;

use super::{Action, ActionContext, CanUndo};
use crate::math::coordinate_spaces::{Glass, SelectionSpace, World};
use crate::math::{
    self, AxisAlignedBox, DecompositionConfiguration, GetUnit, IntoDecompositionConfiguration,
    RotationUnit, SizeUnit,
};
use crate::model::input::InputEvent;
use crate::model::tools::SelectNodes;
use crate::model::{GlassNodeSnapshot, SelectionStateSnapshot};
use crate::ROOT_PROJECT_ID;
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
use pax_manifest::{
    NodeLocation, TreeIndexPosition, TreeLocation, TypeId, UniqueTemplateNodeIdentifier,
};
use pax_runtime_api::{Axis, Percent};
pub mod group_ungroup;

pub struct CreateComponent<'a> {
    pub bounds: AxisAlignedBox<World>,
    pub type_id: TypeId,
    pub custom_props: Vec<(&'a str, &'a str)>,
}

impl Action for CreateComponent<'_> {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let save_data = {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let mut builder = dt.get_orm_mut().build_new_node(
                ctx.app_state.selected_component_id.get().clone(),
                self.type_id,
            );

            for (name, value) in self.custom_props {
                builder.set_property(name, value)?;
            }

            builder
                .save()
                .map_err(|e| anyhow!("could not save: {}", e))?
        };
        let stage = ctx.app_state.stage.get();

        ctx.execute(SetNodePropertiesFromTransform::<World> {
            id: save_data.unique_id.clone(),
            transform_and_bounds: TransformAndBounds {
                transform: self.bounds.as_transform().cast_spaces(),
                bounds: (1.0, 1.0),
            }
            .as_pure_size(),
            parent_transform_and_bounds: TransformAndBounds {
                transform: Transform2::identity(),
                bounds: (stage.width as f64, stage.height as f64),
            },
            decomposition_config: Default::default(),
        })?;

        ctx.execute(SelectNodes {
            ids: &[save_data.unique_id.get_template_node_id()],
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
        let selection = ctx.derived_state.selection_state.get();
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

pub struct SetNodeProperties {
    id: UniqueTemplateNodeIdentifier,
    properties: LayoutProperties,
}

impl Action for SetNodeProperties {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        let Some(mut builder) = dt.get_orm_mut().get_node(self.id) else {
            return Err(anyhow!("can't move: node doesn't exist in orm"));
        };
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
        } = self.properties;

        const EPS: f64 = 1e-3;
        let is_size_default = |s: &Size, d_p: f64| match s {
            Size::Pixels(p) => p.to_float().abs() < EPS,
            Size::Percent(p) => (p.to_float() - d_p).abs() < EPS,
            Size::Combined(pix, per) => pix.to_float() < EPS && (per.to_float() - d_p).abs() < EPS,
        };
        let is_size_default_100 = |s: &Size| is_size_default(s, 100.0);
        let is_size_default_0 = |s: &Size| is_size_default(s, 0.0);
        let is_rotation_default = |r: &Rotation| (r.get_as_degrees() % 360.0).abs() < EPS;

        write_to_orm(&mut builder, "x", x, is_size_default_0)?;
        write_to_orm(&mut builder, "y", y, is_size_default_0)?;
        write_to_orm(&mut builder, "width", width, is_size_default_100)?;
        write_to_orm(&mut builder, "height", height, is_size_default_100)?;
        write_to_orm(
            &mut builder,
            "scale_x",
            scale_x.map(|v| Size::Percent(v.0)),
            is_size_default_100,
        )?;
        write_to_orm(
            &mut builder,
            "scale_y",
            scale_y.map(|v| Size::Percent(v.0)),
            is_size_default_100,
        )?;
        write_to_orm(&mut builder, "rotate", rotate, is_rotation_default)?;
        write_to_orm(&mut builder, "skew_x", skew_x, is_rotation_default)?;
        write_to_orm(&mut builder, "skew_y", skew_y, is_rotation_default)?;
        write_to_orm(&mut builder, "anchor_x", anchor_x, |_| false)?;
        write_to_orm(&mut builder, "anchor_y", anchor_y, |_| false)?;

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
pub struct SetNodePropertiesFromTransform<T> {
    pub id: UniqueTemplateNodeIdentifier,
    pub transform_and_bounds: TransformAndBounds<NodeLocal, T>,
    pub parent_transform_and_bounds: TransformAndBounds<NodeLocal, T>,
    pub decomposition_config: DecompositionConfiguration,
}

impl<T: Space> Action for SetNodePropertiesFromTransform<T> {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let new_props: LayoutProperties = math::transform_and_bounds_decomposition(
            self.decomposition_config,
            self.parent_transform_and_bounds,
            self.transform_and_bounds,
        );

        ctx.execute(SetNodeProperties {
            id: self.id,
            properties: new_props,
        })?;
        Ok(CanUndo::No)
    }
}

pub struct SetAnchor<'a> {
    pub object: &'a GlassNodeSnapshot,
    pub point: Point2<NodeLocal>,
}

impl Action for SetAnchor<'_> {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let t_and_b = self.object.transform_and_bounds;
        let anchor_x = match self.object.layout_properties.anchor_x.unit() {
            SizeUnit::Pixels => Size::Pixels(self.point.x.into()),
            SizeUnit::Percent => Size::Percent((100.0 * self.point.x / t_and_b.bounds.0).into()),
        };
        let anchor_y = match self.object.layout_properties.anchor_y.unit() {
            SizeUnit::Pixels => Size::Pixels(self.point.y.into()),
            SizeUnit::Percent => Size::Percent((100.0 * self.point.y / t_and_b.bounds.1).into()),
        };

        let new_anchor = LayoutProperties {
            anchor_x: Some(anchor_x),
            anchor_y: Some(anchor_y),
            ..self.object.layout_properties.clone()
        };
        ctx.execute(SetNodePropertiesFromTransform {
            id: self.object.id.clone(),
            transform_and_bounds: self.object.transform_and_bounds,
            parent_transform_and_bounds: self.object.parent_transform_and_bounds,
            decomposition_config: new_anchor.into_decomposition_config(),
        })?;

        Ok(CanUndo::No)
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

        for item in &self.initial_selection.items {
            ctx.execute(SetNodePropertiesFromTransform {
                id: item.id.clone(),
                transform_and_bounds: resize * item.transform_and_bounds,
                parent_transform_and_bounds: item.parent_transform_and_bounds,
                decomposition_config: item.layout_properties.into_decomposition_config(),
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

        let local_rotation = TransformAndBounds {
            transform: Transform2::rotate(rotation.to_radians()),
            bounds: (1.0, 1.0),
        };

        // nove to "frame of reference", perform operation, move back
        // TODO refactor so that things like rotation are also just a "local_resize" transform that is performing a rotation,
        // most likely from center of selection (at least when multiple)?
        let rotate = to_local * local_rotation * to_local.inverse();

        for item in &self.initial_selection.items {
            ctx.execute(SetNodePropertiesFromTransform {
                id: item.id.clone(),
                transform_and_bounds: rotate * item.transform_and_bounds,
                parent_transform_and_bounds: item.parent_transform_and_bounds,
                decomposition_config: item.layout_properties.into_decomposition_config(),
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
        let selected = ctx.app_state.selected_template_node_ids.get();
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

pub fn write_to_orm<T: Serialize + Default + PartialEq>(
    builder: &mut NodeBuilder,
    name: &str,
    value: Option<T>,
    is_close_to_default: impl FnOnce(&T) -> bool,
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

pub struct MoveNode<'a, S> {
    pub node_id: &'a UniqueTemplateNodeIdentifier,
    pub node_transform_and_bounds: &'a TransformAndBounds<NodeLocal, S>,
    pub node_inv_config: DecompositionConfiguration,
    pub new_parent_transform_and_bounds: &'a TransformAndBounds<NodeLocal, S>,
    pub new_parent_uid: &'a UniqueTemplateNodeIdentifier,
    pub index: TreeIndexPosition,
    pub resize_mode: ResizeNode,
}

pub enum ResizeNode {
    Fill,
    KeepScreenBounds,
}

impl<S: Space> Action for MoveNode<'_, S> {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        match self.resize_mode {
            ResizeNode::KeepScreenBounds => ctx.execute(SetNodePropertiesFromTransform {
                id: self.node_id.clone(),
                transform_and_bounds: self.node_transform_and_bounds.clone(),
                parent_transform_and_bounds: self.new_parent_transform_and_bounds.clone(),
                decomposition_config: self.node_inv_config,
            })?,
            ResizeNode::Fill => ctx.execute(SetNodeProperties {
                id: self.node_id.clone(),
                properties: LayoutProperties::fill(),
            })?,
        }

        let parent_location = if ctx
            .engine_context
            .get_nodes_by_id(ROOT_PROJECT_ID)
            .first()
            .unwrap()
            .global_id()
            == Some(self.new_parent_uid.clone())
        {
            NodeLocation::new(
                self.node_id.get_containing_component_type_id(),
                TreeLocation::Root,
                self.index,
            )
        } else {
            NodeLocation::new(
                self.node_id.get_containing_component_type_id(),
                TreeLocation::Parent(self.new_parent_uid.get_template_node_id()),
                self.index,
            )
        };

        {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let _undo_id = dt
                .get_orm_mut()
                .move_node(self.node_id.clone(), parent_location)
                .map_err(|e| anyhow!("couldn't move child node {:?}", e))?;
        }

        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}
