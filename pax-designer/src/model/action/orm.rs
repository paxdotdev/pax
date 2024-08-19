use std::any::Any;
use std::f64::consts::PI;

use super::{Action, ActionContext};
use crate::glass::wireframe_editor::editor_generation::stacker_control::sizes_to_string;
use crate::math::approx::ApproxEq;
use crate::math::coordinate_spaces::{Glass, SelectionSpace, World};
use crate::math::{
    self, AxisAlignedBox, DecompositionConfiguration, GetUnit, IntoDecompositionConfiguration,
    RotationUnit, SizeUnit,
};
use crate::model::input::InputEvent;
use crate::model::tools::{SelectMode, SelectNodes};
use crate::model::{GlassNode, GlassNodeSnapshot, SelectionStateSnapshot};
use crate::{math::BoxPoint, model, model::AppState};
use anyhow::{anyhow, Context, Result};
use pax_designtime::orm::template::builder::{self, NodeBuilder};
use pax_designtime::orm::{MoveToComponentEntry, SubTrees};
use pax_designtime::{DesigntimeManager, Serializer};
use pax_engine::api::{borrow, borrow_mut, Rotation};
use pax_engine::api::{Axis, Percent};
use pax_engine::layout::{LayoutProperties, TransformAndBounds};
use pax_engine::math::{Generic, Parts, Transform2};
use pax_engine::pax_manifest::{
    NodeLocation, PaxType, TemplateNodeId, TreeIndexPosition, TreeLocation, TypeId,
    UniqueTemplateNodeIdentifier,
};
use pax_engine::pax_runtime::RepeatInstance;
use pax_engine::serde::Serialize;
use pax_engine::{
    api::Size,
    math::{Point2, Space, Vector2},
    serde,
};
use pax_engine::{log, NodeInterface, NodeLocal, Slot};
use pax_std::layout::stacker::Stacker;
pub mod group_ungroup;

pub struct CreateComponent<'a> {
    pub parent_id: &'a UniqueTemplateNodeIdentifier,
    pub parent_index: TreeIndexPosition,
    pub type_id: &'a TypeId,
    pub custom_props: &'a [(&'a str, &'a str)],
    pub mock_children: usize,
    pub node_layout: NodeLayoutSettings<'a, Glass>,
}

impl Action<UniqueTemplateNodeIdentifier> for CreateComponent<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<UniqueTemplateNodeIdentifier> {
        let parent_location = ctx.location(self.parent_id, &self.parent_index);

        let save_data = {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let mut builder = dt.get_orm_mut().build_new_node(
                ctx.app_state.selected_component_id.get().clone(),
                self.type_id.clone(),
            );

            builder.set_location(parent_location);

            for (name, value) in self.custom_props {
                builder.set_property(name, value)?;
            }

            builder
                .save()
                .map_err(|e| anyhow!("could not save: {}", e))?
        };

        SetNodeLayout {
            id: &save_data.unique_id,
            node_layout: &self.node_layout,
        }
        .perform(ctx)?;

        for i in 1..=self.mock_children {
            let c = 210 - 60 * (i % 2);
            CreateComponent {
                parent_id: &save_data.unique_id,
                parent_index: TreeIndexPosition::Top,
                type_id: &TypeId::build_singleton("pax_std::drawing::rectangle::Rectangle", None),
                custom_props: &[("fill", &format!("rgb({}, {}, {})", c, c, c))],
                mock_children: 0,
                node_layout: NodeLayoutSettings::Fill,
            }
            .perform(ctx)?;
        }

        SelectNodes {
            ids: &[save_data.unique_id.get_template_node_id()],
            mode: SelectMode::DiscardOthers,
        }
        .perform(ctx)?;

        Ok(save_data.unique_id)
    }
}

pub struct SelectedIntoNewComponent {}

impl Action for SelectedIntoNewComponent {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
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
        let t = ctx.transaction("moving selected into new component");
        t.run(|| {
            dt.get_orm_mut()
                .move_to_new_component(&entries, o.x, o.y, u.length(), v.length())
                .map_err(|e| anyhow!("couldn't move to component: {}", e))
        })
    }
}

pub struct SetNodeLayoutProperties<'a> {
    pub id: &'a UniqueTemplateNodeIdentifier,
    pub properties: &'a LayoutProperties,
    // anchor doesn't have a default value (becomes "reactive" in the None case), and so needs
    // to be manually specified to be reset
    pub reset_anchor: bool,
}

impl Action for SetNodeLayoutProperties<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        let Some(mut builder) = dt.get_orm_mut().get_node(
            self.id.clone(),
            ctx.app_state
                .keys_pressed
                .get()
                .contains(&InputEvent::Control),
        ) else {
            return Err(anyhow!("can't move: node doesn't exist in orm"));
        };

        if !matches!(
            builder.get_type_id().get_pax_type(),
            PaxType::Singleton { .. } | PaxType::BlankComponent { .. }
        ) {
            return Ok(());
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

        // compare with the values for the current node in the engine, and
        // only try to write if different (we don't want to try to overwrite a
        // rotation expression if we are only moving an object and not affecting
        // rotation)
        let old_props = ctx
            .engine_context
            .get_nodes_by_global_id(self.id.clone())
            .into_iter()
            .next()
            .map(|n| n.layout_properties())
            .unwrap_or_default();

        write_to_orm(
            &mut builder,
            "x",
            x.as_ref(),
            old_props.x.as_ref(),
            Size::ZERO(),
        )?;
        write_to_orm(
            &mut builder,
            "y",
            y.as_ref(),
            old_props.y.as_ref(),
            Size::ZERO(),
        )?;

        write_to_orm(
            &mut builder,
            "width",
            width.as_ref(),
            old_props.width.as_ref(),
            Size::default(),
        )?;
        write_to_orm(
            &mut builder,
            "height",
            height.as_ref(),
            old_props.height.as_ref(),
            Size::default(),
        )?;

        write_to_orm(
            &mut builder,
            "scale_x",
            scale_x.as_ref().map(|v| Size::Percent(v.0)).as_ref(),
            old_props
                .scale_x
                .as_ref()
                .map(|v| Size::Percent(v.0))
                .as_ref(),
            Size::default(),
        )?;
        write_to_orm(
            &mut builder,
            "scale_y",
            scale_y.as_ref().map(|v| Size::Percent(v.0)).as_ref(),
            old_props
                .scale_y
                .as_ref()
                .map(|v| Size::Percent(v.0))
                .as_ref(),
            Size::default(),
        )?;
        write_to_orm(
            &mut builder,
            "rotate",
            rotate.as_ref(),
            old_props.rotate.as_ref(),
            Rotation::default(),
        )?;
        write_to_orm(
            &mut builder,
            "skew_x",
            skew_x.as_ref(),
            old_props.skew_x.as_ref(),
            Rotation::default(),
        )?;
        write_to_orm(
            &mut builder,
            "skew_y",
            skew_y.as_ref(),
            old_props.skew_y.as_ref(),
            Rotation::default(),
        )?;

        if self.reset_anchor {
            builder.set_property("anchor_x", "")?;
            builder.set_property("anchor_y", "")?;
        } else {
            write_to_orm(
                &mut builder,
                "anchor_x",
                anchor_x.as_ref(),
                old_props.anchor_x.as_ref(),
                // never assume default
                Size::Combined(f64::MAX.into(), f64::MAX.into()),
            )?;
            write_to_orm(
                &mut builder,
                "anchor_y",
                anchor_y.as_ref(),
                old_props.anchor_y.as_ref(),
                // never assume default
                Size::Combined(f64::MAX.into(), f64::MAX.into()),
            )?;
        }

        builder
            .save()
            .map_err(|e| anyhow!("could not move thing: {}", e))?;

        Ok(())
    }
}

struct SetNodeLayoutPropertiesFromTransform<'a, T> {
    pub id: &'a UniqueTemplateNodeIdentifier,
    pub transform_and_bounds: &'a TransformAndBounds<NodeLocal, T>,
    pub parent_transform_and_bounds: &'a TransformAndBounds<NodeLocal, T>,
    pub decomposition_config: &'a DecompositionConfiguration,
}

impl<T: Space> Action for SetNodeLayoutPropertiesFromTransform<'_, T> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let new_props: LayoutProperties = math::transform_and_bounds_decomposition(
            self.decomposition_config,
            self.parent_transform_and_bounds,
            self.transform_and_bounds,
        );

        SetNodeLayoutProperties {
            id: self.id,
            properties: &new_props,
            reset_anchor: false,
        }
        .perform(ctx)?;
        Ok(())
    }
}

pub struct SetAnchor<'a> {
    pub object: &'a GlassNodeSnapshot,
    pub point: Point2<NodeLocal>,
}

impl Action for SetAnchor<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
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
        SetNodeLayoutPropertiesFromTransform {
            id: &self.object.id,
            transform_and_bounds: &self.object.transform_and_bounds,
            parent_transform_and_bounds: &self.object.parent_transform_and_bounds,
            decomposition_config: &new_anchor.into_decomposition_config(),
        }
        .perform(ctx)?;

        Ok(())
    }
}

pub struct Resize<'a> {
    pub fixed_point: Point2<BoxPoint>,
    pub new_point: Point2<Glass>,
    pub initial_selection: &'a SelectionStateSnapshot,
}

impl Action for Resize<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
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

        // move to "frame of reference", perform operation, move back
        let resize = to_local * local_resize * to_local.inverse();

        // when resizing, override to % if not meta key is pressed, then use px
        let unit = ctx.app_state.unit_mode.get();

        for item in &self.initial_selection.items {
            SetNodeLayoutPropertiesFromTransform {
                id: &item.id,
                transform_and_bounds: &(resize * item.transform_and_bounds),
                parent_transform_and_bounds: &item.parent_transform_and_bounds,
                decomposition_config: &&DecompositionConfiguration {
                    unit_width: unit,
                    unit_height: unit,
                    unit_x_pos: unit,
                    unit_y_pos: unit,
                    ..item.layout_properties.into_decomposition_config()
                },
            }
            .perform(ctx)?;
        }
        Ok(())
    }
}

const ANGLE_SNAP_DEG: f64 = 45.0;

pub struct RotateSelected<'a> {
    pub start_pos: Point2<Glass>,
    pub curr_pos: Point2<Glass>,
    pub initial_selection: &'a SelectionStateSnapshot,
}

impl Action for RotateSelected<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
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
            let original_rotation =
                Into::<Parts>::into(self.initial_selection.total_bounds.transform)
                    .rotation
                    .to_degrees();
            let total_rotation = (rotation + original_rotation).rem_euclid(360.0 - f64::EPSILON);
            let mut snapped_rotation = (total_rotation / ANGLE_SNAP_DEG).round() * ANGLE_SNAP_DEG;
            if snapped_rotation >= 360.0 - f64::EPSILON {
                snapped_rotation = 0.0;
            }
            rotation = snapped_rotation - original_rotation;
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

        // move to "frame of reference", perform operation, move back
        let rotate = to_local * local_rotation * to_local.inverse();

        for item in &self.initial_selection.items {
            SetNodeLayoutPropertiesFromTransform {
                id: &item.id,
                transform_and_bounds: &(rotate * item.transform_and_bounds),
                parent_transform_and_bounds: &item.parent_transform_and_bounds,
                decomposition_config: &item.layout_properties.into_decomposition_config(),
            }
            .perform(ctx)?;
        }

        Ok(())
    }
}

pub struct SerializeRequested;

impl Action for SerializeRequested {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        if let Err(e) = dt.send_component_update(&ctx.app_state.selected_component_id.get()) {
            pax_engine::log::error!("failed to save component to file: {:?}", e);
        }
        Ok(())
    }
}

pub struct UndoRequested;

impl Action for UndoRequested {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        ctx.undo_stack.undo(dt.get_orm_mut());
        Ok(())
    }
}

pub struct RedoRequested;

impl Action for RedoRequested {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        ctx.undo_stack.redo(dt.get_orm_mut());
        Ok(())
    }
}

pub struct DeleteSelected;

impl Action for DeleteSelected {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
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
        Ok(())
    }
}

pub struct Copy<'a> {
    pub ids: &'a [TemplateNodeId],
}

impl Action<SubTrees> for Copy<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<SubTrees> {
        let comp_id = ctx.app_state.selected_component_id.get();
        let dt = borrow!(ctx.engine_context.designtime);
        let subtree = dt
            .get_orm()
            .copy_subtrees(&comp_id, &self.ids)
            .ok_or_else(|| anyhow!("couldn't copy"))?;
        Ok(subtree)
    }
}

pub struct Paste<'a> {
    pub subtrees: &'a SubTrees,
}

impl Action<Vec<TemplateNodeId>> for Paste<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<Vec<TemplateNodeId>> {
        let parent = ctx.derived_state.open_container.get();
        let loc = ctx.location(&parent, &TreeIndexPosition::Top);
        let t = ctx.transaction("pasting object");
        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        let ids = t.run(|| {
            let ids = dt
                .get_orm_mut()
                .paste_subtrees(loc, self.subtrees.clone())
                .map_err(|e| anyhow!("failed to paste: {e}"))?;
            SelectNodes {
                ids: &ids,
                mode: SelectMode::DiscardOthers,
            }
            .perform(ctx)?;
            Ok(ids)
        });
        ids
    }
}

fn write_to_orm<T: Serialize + ApproxEq>(
    builder: &mut NodeBuilder,
    name: &str,
    value: Option<&T>,
    old_value: Option<&T>,
    default_value: T,
) -> Result<()> {
    if old_value.approx_eq(&value) {
        return Ok(());
    }
    if let Some(val) = value {
        if !default_value.approx_eq(val) {
            let val = pax_designtime::to_pax(&val)?;
            builder.set_property(name, &val)?;
        } else {
            builder.set_property(name, "")?;
        }
    };
    Ok(())
}

pub struct MoveNode<'a, S = Generic> {
    pub node_id: &'a UniqueTemplateNodeIdentifier,
    pub new_parent_uid: &'a UniqueTemplateNodeIdentifier,
    pub index: TreeIndexPosition,
    pub node_layout: NodeLayoutSettings<'a, S>,
}

pub enum NodeLayoutSettings<'a, S> {
    Fill,
    KeepScreenBounds {
        node_transform_and_bounds: &'a TransformAndBounds<NodeLocal, S>,
        node_decomposition_config: &'a DecompositionConfiguration,
        parent_transform_and_bounds: &'a TransformAndBounds<NodeLocal, S>,
    },
    WithProperties(LayoutProperties),
}

pub struct SetNodeLayout<'a, S> {
    pub id: &'a UniqueTemplateNodeIdentifier,
    pub node_layout: &'a NodeLayoutSettings<'a, S>,
}

impl<S: Space> Action for SetNodeLayout<'_, S> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        match &self.node_layout {
            NodeLayoutSettings::KeepScreenBounds {
                node_transform_and_bounds,
                parent_transform_and_bounds: new_parent_transform_and_bounds,
                node_decomposition_config: node_inv_config,
            } => SetNodeLayoutPropertiesFromTransform {
                id: &self.id,
                transform_and_bounds: node_transform_and_bounds,
                parent_transform_and_bounds: new_parent_transform_and_bounds,
                decomposition_config: node_inv_config,
            }
            .perform(ctx),
            NodeLayoutSettings::Fill => SetNodeLayoutProperties {
                id: &self.id,
                properties: &LayoutProperties::fill(),
                reset_anchor: true,
            }
            .perform(ctx),
            NodeLayoutSettings::WithProperties(props) => SetNodeLayoutProperties {
                id: &self.id,
                properties: props,
                reset_anchor: false,
            }
            .perform(ctx),
        }
    }
}

impl<S: Space> Action for MoveNode<'_, S> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        SetNodeLayout {
            id: self.node_id,
            node_layout: &self.node_layout,
        }
        .perform(ctx)?;
        let parent_location = ctx.location(self.new_parent_uid, &self.index);
        {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            let _undo_id = dt
                .get_orm_mut()
                .move_node(self.node_id.clone(), parent_location.clone())
                .map_err(|e| anyhow!("couldn't move child node {:?}", e))?;
        }

        Ok(())
    }
}
