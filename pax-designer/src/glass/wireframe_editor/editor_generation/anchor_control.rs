use std::{cell::RefCell, rc::Rc};

use pax_engine::{api::Color, math::Point2, Property};

use crate::{
    glass::{
        control_point::{
            ControlPointBehavior, ControlPointStyling, ControlPointTool, ControlPointToolFactory,
        },
        wireframe_editor::editor_generation::CPoint,
    },
    math::{
        coordinate_spaces::Glass,
        intent_snapper::{IntentSnapper, SnapCollection, SnapSet},
    },
    model::{
        action::{self, Action, ActionContext},
        GlassNodeSnapshot,
    },
    utils::designer_cursor::DesignerCursorType,
};

use super::ControlPointSet;

pub fn anchor_control_point_set(anchor: Property<Point2<Glass>>) -> ControlPointSet {
    let anchor_control_point_styling = ControlPointStyling {
        round: true,
        stroke_color: Color::BLUE,
        fill_color: Color::rgba(255.into(), 255.into(), 255.into(), 150.into()),
        stroke_width_pixels: 1.0,
        width: 10.0,
        height: 10.0,
        affected_by_transform: false,
        cursor_type: DesignerCursorType::Move,
        hit_padding: 10.0,
    };

    let deps = [anchor.untyped()];
    let anchor_control_point = Property::computed(
        move || {
            let anchor = anchor.get();
            vec![CPoint {
                point: anchor,
                behavior: anchor_factory(),
                ..Default::default()
            }]
        },
        &deps,
    );

    ControlPointSet {
        points: anchor_control_point,
        styling: anchor_control_point_styling.clone(),
    }
}

fn anchor_factory() -> ControlPointToolFactory {
    ControlPointToolFactory {
        tool_factory: Rc::new(move |ac, _p| {
            let selection_state = (&ac.derived_state.selection_state.get())
                .items
                .first()
                .map(Into::into);
            Rc::new(RefCell::new(ControlPointTool::new(
                ac,
                "moving anchor point",
                Some(IntentSnapper::new(
                    ac,
                    SnapCollection {
                        sets: selection_state
                            .as_ref()
                            .map(|ss: &GlassNodeSnapshot| {
                                vec![SnapSet::points_from_transform_and_bounds(
                                    ss.transform_and_bounds,
                                    Color::BLUE,
                                )]
                            })
                            .unwrap_or_default(),
                    },
                )),
                AnchorBehavior {
                    initial_object: selection_state,
                },
            )))
        }),
        double_click_behavior: Rc::new(|_| ()),
    }
}

struct AnchorBehavior {
    initial_object: Option<GlassNodeSnapshot>,
}

impl ControlPointBehavior for AnchorBehavior {
    fn step(&self, ctx: &mut ActionContext, point: Point2<Glass>) -> anyhow::Result<()> {
        if let Some(initial_object) = &self.initial_object {
            let t_and_b = initial_object.transform_and_bounds;
            let point_in_space = t_and_b.transform.inverse() * point;
            action::orm::space_movement::SetAnchor {
                object: &initial_object,
                point: point_in_space,
            }
            .perform(ctx)?;
        }
        Ok(())
    }
}
