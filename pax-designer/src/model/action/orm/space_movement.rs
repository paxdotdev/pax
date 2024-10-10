use pax_engine::{
    math::{Point2, Transform2, TransformParts, Vector2},
    pax_runtime::{LayoutProperties, TransformAndBounds},
    NodeLocal,
};
use pax_std::Size;

use crate::{
    math::{
        coordinate_spaces::{Glass, SelectionSpace},
        BoxPoint, DecompositionConfiguration, GetUnit, IntoDecompositionConfiguration, SizeUnit,
    },
    model::{
        action::{Action, ActionContext},
        input::ModifierKey,
        GlassNodeSnapshot, SelectionStateSnapshot,
    },
};
use anyhow::{anyhow, Result};

use super::{NodeLayoutSettings, SetNodeLayout};

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
        SetNodeLayout {
            id: &self.object.id,
            node_layout: &NodeLayoutSettings::KeepScreenBounds {
                node_transform_and_bounds: &self.object.transform_and_bounds,
                parent_transform_and_bounds: &self.object.parent_transform_and_bounds,
                node_decomposition_config: &new_anchor.into_decomposition_config(),
            },
        }
        .perform(ctx)?;

        Ok(())
    }
}

pub struct ResizeFromSnapshot<'a> {
    pub fixed_point: Point2<BoxPoint>,
    pub new_point: Point2<Glass>,
    pub initial_selection: &'a SelectionStateSnapshot,
}

impl Action for ResizeFromSnapshot<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let (is_shift_key_down, is_alt_key_down) = ctx.app_state.modifiers.read(|keys| {
            (
                keys.contains(&ModifierKey::Shift),
                keys.contains(&ModifierKey::Alt),
            )
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
        let diff_now = fixed - new_in_selec;
        let mut scale = diff_now / diff_start;

        // if shift key down, uniformly scale
        if is_shift_key_down {
            let uniform_scale = if self.fixed_point.x == 0.5 {
                scale.y.abs()
            } else if self.fixed_point.y == 0.5 {
                scale.x.abs()
            } else {
                scale.x.abs().max(scale.y.abs())
            };

            scale = Vector2::new(
                uniform_scale * scale.x.signum(),
                uniform_scale * scale.y.signum(),
            );
        } else {
            // When shift is not down, constrain scaling if using side handles
            if self.fixed_point.x == 0.5 {
                scale.x = 1.0;
            }
            if self.fixed_point.y == 0.5 {
                scale.y = 1.0;
            }
        }

        let anchor: Transform2<SelectionSpace> = Transform2::translate(fixed.to_vector());

        // This is the "frame of reference" from which all objects that
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
            SetNodeLayout {
                id: &item.id,
                node_layout: &NodeLayoutSettings::KeepScreenBounds {
                    node_transform_and_bounds: &(resize * item.transform_and_bounds),
                    parent_transform_and_bounds: &item.parent_transform_and_bounds,
                    node_decomposition_config: &DecompositionConfiguration {
                        unit_width: unit,
                        unit_height: unit,
                        unit_x_pos: unit,
                        unit_y_pos: unit,
                        ..item.layout_properties.into_decomposition_config()
                    },
                },
            }
            .perform(ctx)?;
        }
        Ok(())
    }
}

const ANGLE_SNAP_DEG: f64 = 45.0;

pub struct RotateFromSnapshot<'a> {
    pub start_pos: Point2<Glass>,
    pub curr_pos: Point2<Glass>,
    pub initial_selection: &'a SelectionStateSnapshot,
}

impl Action for RotateFromSnapshot<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let anchor_point = self.initial_selection.total_origin;
        let start = self.start_pos - anchor_point;
        let curr = self.curr_pos - anchor_point;
        let mut rotation = start.angle_to(curr).get_as_degrees();

        if ctx.app_state.modifiers.get().contains(&ModifierKey::Shift) {
            let original_rotation =
                Into::<TransformParts>::into(self.initial_selection.total_bounds.transform)
                    .rotation
                    .to_degrees();
            let total_rotation = (rotation + original_rotation).rem_euclid(360.0 - f64::EPSILON);
            let mut snapped_rotation = (total_rotation / ANGLE_SNAP_DEG).round() * ANGLE_SNAP_DEG;
            if snapped_rotation >= 360.0 - f64::EPSILON {
                snapped_rotation = 0.0;
            }
            rotation = snapped_rotation - original_rotation;
        }

        // This is the "frame of reference" from which all objects that
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
            SetNodeLayout {
                id: &item.id,
                node_layout: &NodeLayoutSettings::KeepScreenBounds {
                    node_transform_and_bounds: &(rotate * item.transform_and_bounds),
                    parent_transform_and_bounds: &item.parent_transform_and_bounds,
                    node_decomposition_config: &item.layout_properties.into_decomposition_config(),
                },
            }
            .perform(ctx)?;
        }

        Ok(())
    }
}

pub struct TranslateFromSnapshot<'a> {
    pub translation: Vector2<Glass>,
    pub initial_selection: &'a SelectionStateSnapshot,
}

impl Action for TranslateFromSnapshot<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let move_translation = TransformAndBounds {
            transform: Transform2::translate(self.translation),
            bounds: (1.0, 1.0),
        };
        let unit = match ctx.app_state.unit_mode.get() {
            SizeUnit::Pixels => SizeUnit::Pixels,
            SizeUnit::Percent => SizeUnit::Percent,
        };
        for item in &self.initial_selection.items {
            if let Ok(curr_item) = ctx.get_glass_node_by_global_id(&item.id) {
                SetNodeLayout {
                    id: &item.id,
                    node_layout: &NodeLayoutSettings::KeepScreenBounds {
                        // NOTE: use the engine nodes parent, but the initial bounds of
                        // the selected node
                        node_transform_and_bounds: &(move_translation * item.transform_and_bounds),
                        parent_transform_and_bounds: &curr_item.parent_transform_and_bounds.get(),
                        node_decomposition_config: &DecompositionConfiguration {
                            unit_x_pos: unit,
                            unit_y_pos: unit,
                            ..item.layout_properties.into_decomposition_config()
                        },
                    },
                }
                .perform(ctx)?
            }
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum NudgeDir {
    Up,
    Down,
    Left,
    Right,
}

pub struct Nudge(pub NudgeDir);

impl Action for Nudge {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let initial_selection: SelectionStateSnapshot =
            (&ctx.derived_state.selection_state.get()).into();
        const GLASS_PIXELS: f64 = 3.0;

        let t = ctx.transaction("nudging selection");
        t.run(|| {
            TranslateFromSnapshot {
                translation: match self.0 {
                    NudgeDir::Up => Vector2::new(0.0, -GLASS_PIXELS),
                    NudgeDir::Down => Vector2::new(0.0, GLASS_PIXELS),
                    NudgeDir::Left => Vector2::new(-GLASS_PIXELS, 0.0),
                    NudgeDir::Right => Vector2::new(GLASS_PIXELS, 0.0),
                },
                initial_selection: &initial_selection,
            }
            .perform(ctx)
        })
    }
}
