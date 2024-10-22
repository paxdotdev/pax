use std::{cell::RefCell, f64::consts::PI, rc::Rc};

use pax_engine::{
    api::Color,
    math::{Point2, TransformParts, Vector2},
    pax_runtime::TransformAndBounds,
    NodeLocal, Property,
};

use crate::{
    glass::{
        control_point::{
            ControlPointBehavior, ControlPointStyling, ControlPointTool, ControlPointToolFactory,
        },
        wireframe_editor::editor_generation::CPoint,
    },
    math::{
        coordinate_spaces::{Glass, SelectionSpace},
        intent_snapper::IntentSnapper,
    },
    model::{
        action::{self, Action, ActionContext},
        SelectionStateSnapshot,
    },
    utils::designer_cursor::DesignerCursorType,
};

use super::ControlPointSet;

pub fn resize_control_points_set(
    total_selection_bounds: Property<TransformAndBounds<SelectionSpace, Glass>>,
) -> ControlPointSet {
    let deps = [total_selection_bounds.untyped()];
    // resize points
    let resize_control_points = Property::computed(
        move || {
            let total_selection_bounds = total_selection_bounds.get().as_transform();
            let c_point = |x: f64, y: f64| {
                let point = Point2::new(x, y);
                let cursor_rotation = Vector2::new(1.0, 0.0)
                    .angle_to(point - Point2::new(0.5, 0.5))
                    .get_as_degrees();
                CPoint {
                    point: total_selection_bounds * point,
                    cursor_rotation,
                    behavior: resize_factory(point),
                    ..Default::default()
                }
            };
            vec![
                c_point(0.0, 0.0),
                c_point(0.5, 0.0),
                c_point(1.0, 0.0),
                c_point(1.0, 0.5),
                c_point(1.0, 1.0),
                c_point(0.5, 1.0),
                c_point(0.0, 1.0),
                c_point(0.0, 0.5),
            ]
        },
        &deps,
    );

    let resize_control_point_styling = ControlPointStyling {
        round: false,
        stroke_color: Color::BLUE,
        fill_color: Color::WHITE,
        stroke_width_pixels: 1.0,
        width: 8.0,
        height: 8.0,
        affected_by_transform: true,
        cursor_type: DesignerCursorType::Resize,
        hit_padding: 10.0,
    };

    ControlPointSet {
        points: resize_control_points,
        styling: resize_control_point_styling,
    }
}

fn resize_factory(position: Point2<SelectionSpace>) -> ControlPointToolFactory {
    let anchor = (Point2::new(1.0, 1.0) - position).to_point();
    ControlPointToolFactory {
        tool_factory: Rc::new(move |ac, _p| {
            let initial_selection: SelectionStateSnapshot =
                (&ac.derived_state.selection_state.get()).into();

            // only snap if either no rotation or this is a corner point
            let rot = Into::<TransformParts>::into(initial_selection.total_bounds.transform)
                .rotation
                .rem_euclid(PI / 2.0);
            let should_snap =
                (rot < 1e-2 || rot > PI / 2.0 - 1e-2) || (anchor.x != 0.5 && anchor.y != 0.5);

            Rc::new(RefCell::new(ControlPointTool::new(
                ac,
                "resize",
                should_snap.then_some(IntentSnapper::new_from_scene(
                    ac,
                    &initial_selection
                        .items
                        .iter()
                        .map(|i| i.id.clone())
                        .collect::<Vec<_>>(),
                )),
                ResizeBehavior {
                    attachment_point: anchor,
                    initial_selection,
                },
            )))
        }),
        double_click_behavior: Rc::new(|_| ()),
    }
}

struct ResizeBehavior {
    attachment_point: Point2<SelectionSpace>,
    initial_selection: SelectionStateSnapshot,
}

impl ControlPointBehavior for ResizeBehavior {
    fn step(&self, ctx: &mut ActionContext, point: Point2<Glass>) -> anyhow::Result<()> {
        action::orm::space_movement::ResizeFromSnapshot {
            initial_selection: &self.initial_selection,
            fixed_point: self.attachment_point,
            new_point: point,
        }
        .perform(ctx)?;
        Ok(())
    }
}
