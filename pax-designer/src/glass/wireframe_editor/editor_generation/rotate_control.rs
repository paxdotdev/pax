use std::{cell::RefCell, rc::Rc};

use pax_engine::{
    api::Color,
    math::{Point2, Vector2},
    pax_runtime::TransformAndBounds,
    Property,
};

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
) -> ControlPointSet {
    let rotate_control_point_styling = ControlPointStyling {
        round: false,
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
    let rotate_control_points = Property::computed(
        move || {
            let total_selection_bounds = total_selection_bounds.get().as_transform();
            let rotate_factory = rotate_factory();
            let c_point = |x: f64, y: f64| {
                let local_point = Point2::new(x, y);
                let cursor_rotation = Vector2::new(1.0, 0.0)
                    .angle_to(local_point - Point2::new(0.5, 0.5))
                    .get_as_degrees();
                // Create three control points per corner in an "L" shape
                [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]
                    .into_iter()
                    .map(|(x, y)| Point2::new(x, y))
                    .filter(|p| p != &local_point)
                    .map(|anchor| CPoint {
                        point: total_selection_bounds * local_point,
                        cursor_rotation,
                        anchor: anchor.cast_space(),
                        behavior: rotate_factory.clone(),
                        ..Default::default()
                    })
                    .collect::<Vec<_>>()
            };
            [
                c_point(1.0, 1.0),
                c_point(0.0, 1.0),
                c_point(1.0, 0.0),
                c_point(0.0, 0.0),
            ]
            .into_iter()
            .flatten()
            .collect()
        },
        &deps,
    );
    ControlPointSet {
        points: rotate_control_points,
        styling: rotate_control_point_styling,
    }
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
