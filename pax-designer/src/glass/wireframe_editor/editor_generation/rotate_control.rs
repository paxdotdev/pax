use std::{cell::RefCell, rc::Rc};

use pax_engine::{api::Color, math::Point2, pax_runtime::TransformAndBounds, Property};

use crate::{
    glass::{
        control_point::{
            ControlPointBehavior, ControlPointStyling, ControlPointTool, ControlPointToolFactory,
        },
        wireframe_editor::editor_generation::CPoint,
    },
    math::coordinate_spaces::{Glass, SelectionSpace},
    model::{
        action::{self, Action, ActionContext},
        SelectionStateSnapshot,
    },
    utils::designer_cursor::DesignerCursorType,
};

use super::ControlPointSet;

pub fn rotate_control_points_set(
    total_selection_bounds: Property<TransformAndBounds<SelectionSpace, Glass>>,
) -> Property<ControlPointSet> {
    let rotate_control_point_styling = ControlPointStyling {
        round: true,
        stroke_color: Color::TRANSPARENT,
        fill_color: Color::TRANSPARENT,
        stroke_width_pixels: 0.0,
        width: 20.0,
        height: 20.0,
        affected_by_transform: true,
        cursor_type: DesignerCursorType::Rotation,
        hit_padding: 0.0,
    };

    let deps = [total_selection_bounds.untyped()];
    Property::computed(
        move || {
            let total_bounds = total_selection_bounds.get();
            // clockwise from top left:
            let [p1, p4, p3, p2] = total_bounds.corners();
            let rotate_control_points = vec![
                // each section of three sits in one corner, and fills out an
                // "L" shape that "fits" the selected object. first CPoint in
                // each section is the one pointing diagonally outwards from
                // the selection.

                // -------- section 1 --------
                CPoint {
                    point: p1,
                    behavior: rotate_factory(),
                    rotation: 225.0,
                    anchor: Point2::new(1.0, 1.0),
                },
                CPoint {
                    point: p1,
                    behavior: rotate_factory(),
                    rotation: 225.0,
                    anchor: Point2::new(0.0, 1.0),
                },
                CPoint {
                    point: p1,
                    behavior: rotate_factory(),
                    rotation: 225.0,
                    anchor: Point2::new(1.0, 0.0),
                },
                // -------- section 2 --------
                CPoint {
                    point: p2,
                    behavior: rotate_factory(),
                    rotation: 315.0,
                    anchor: Point2::new(0.0, 1.0),
                },
                CPoint {
                    point: p2,
                    behavior: rotate_factory(),
                    rotation: 315.0,
                    anchor: Point2::new(1.0, 1.0),
                },
                CPoint {
                    point: p2,
                    behavior: rotate_factory(),
                    rotation: 315.0,
                    anchor: Point2::new(0.0, 0.0),
                },
                // -------- section 3 --------
                CPoint {
                    point: p3,
                    behavior: rotate_factory(),
                    rotation: 55.0,
                    anchor: Point2::new(0.0, 0.0),
                },
                CPoint {
                    point: p3,
                    behavior: rotate_factory(),
                    rotation: 55.0,
                    anchor: Point2::new(1.0, 0.0),
                },
                CPoint {
                    point: p3,
                    behavior: rotate_factory(),
                    rotation: 55.0,
                    anchor: Point2::new(0.0, 1.0),
                },
                // -------- section 4 --------
                CPoint {
                    point: p4,
                    behavior: rotate_factory(),
                    rotation: 145.0,
                    anchor: Point2::new(1.0, 0.0),
                },
                CPoint {
                    point: p4,
                    behavior: rotate_factory(),
                    rotation: 145.0,
                    anchor: Point2::new(0.0, 0.0),
                },
                CPoint {
                    point: p4,
                    behavior: rotate_factory(),
                    rotation: 145.0,
                    anchor: Point2::new(1.0, 1.0),
                },
            ];
            ControlPointSet {
                points: rotate_control_points,
                styling: rotate_control_point_styling.clone(),
            }
        },
        &deps,
    )
}

fn rotate_factory() -> ControlPointToolFactory {
    ControlPointToolFactory {
        tool_factory: Rc::new(|ctx, point| {
            let initial_selection = (&ctx.derived_state.selection_state.get()).into();
            Rc::new(RefCell::new(ControlPointTool::new(
                ctx,
                "rotating",
                None,
                RotationBehavior {
                    start_pos: point,
                    initial_selection,
                },
            )))
        }),
        double_click_behavior: Rc::new(|_| ()),
    }
}

struct RotationBehavior {
    initial_selection: SelectionStateSnapshot,
    start_pos: Point2<Glass>,
}

impl ControlPointBehavior for RotationBehavior {
    fn step(&self, ctx: &mut ActionContext, point: Point2<Glass>) -> anyhow::Result<()> {
        action::orm::space_movement::RotateFromSnapshot {
            curr_pos: point,
            start_pos: self.start_pos,
            initial_selection: &self.initial_selection,
        }
        .perform(ctx)?;
        Ok(())
    }
}
