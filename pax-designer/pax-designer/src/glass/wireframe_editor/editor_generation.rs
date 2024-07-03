use std::{cell::RefCell, rc::Rc};

use pax_engine::{layout::TransformAndBounds, math::Point2};
use pax_runtime_api::{Color, Interpolatable};

use crate::{
    glass::control_point::{
        ControlPointBehaviour, ControlPointBehaviourFactory, ControlPointStyling,
    },
    math::{
        coordinate_spaces::{Glass, SelectionSpace},
        BoxPoint,
    },
    model::{
        action::{self, ActionContext},
        RuntimeNodeInfo, SelectionState, SelectionStateSnapshot, ToolBehaviour,
    },
};

impl Interpolatable for Editor {}

#[derive(Clone, Default)]
pub struct Editor {
    pub controls: Vec<ControlPointSet>,
    pub segments: Vec<(Point2<Glass>, Point2<Glass>)>,
}

impl Editor {
    pub fn new(
        selection_bounds: &TransformAndBounds<SelectionSpace, Glass>,
        anchor: Point2<Glass>,
    ) -> Self {
        let (o, u, v) = selection_bounds.transform.decompose();
        let u = u * selection_bounds.bounds.0;
        let v = v * selection_bounds.bounds.1;
        let [p1, p4, p3, p2] = [o, o + v, o + u + v, o + u];

        let bounding_segments = vec![(p1, p2), (p2, p3), (p3, p4), (p4, p1)];

        Self {
            controls: vec![
                Self::resize_control_points_set(p1, p2, p3, p4),
                Self::rotate_control_points_set(p1, p2, p3, p4),
                Self::anchor_control_point_set(anchor),
            ],
            segments: bounding_segments,
        }
    }

    fn rotate_control_points_set(
        p1: Point2<Glass>,
        p2: Point2<Glass>,
        p3: Point2<Glass>,
        p4: Point2<Glass>,
    ) -> ControlPointSet {
        struct RotationBehaviour {
            initial_selection: SelectionStateSnapshot,
            start_pos: Point2<Glass>,
        }

        impl ControlPointBehaviour for RotationBehaviour {
            fn step(&self, ctx: &mut ActionContext, point: Point2<Glass>) {
                if let Err(e) = ctx.execute(action::orm::RotateSelected {
                    curr_pos: point,
                    start_pos: self.start_pos,
                    initial_selection: &self.initial_selection,
                }) {
                    pax_engine::log::warn!("rotation failed: {:?}", e);
                };
            }
        }

        fn rotate_factory() -> ControlPointBehaviourFactory {
            Rc::new(|ctx, point| {
                let initial_selection = (&ctx.selection_state()).into();
                Rc::new(RefCell::new(RotationBehaviour {
                    start_pos: point,
                    initial_selection,
                }))
            })
        }
        let rotate_control_points = vec![
            CPoint::new(p1, rotate_factory()),
            CPoint::new(p2, rotate_factory()),
            CPoint::new(p3, rotate_factory()),
            CPoint::new(p4, rotate_factory()),
        ];
        let rotate_control_point_styling = ControlPointStyling {
            round: false,
            stroke: Color::TRANSPARENT,
            fill: Color::TRANSPARENT,
            stroke_width_pixels: 0.0,
            size_pixels: 27.0,
        };

        ControlPointSet {
            points: rotate_control_points,
            styling: rotate_control_point_styling,
        }
    }

    fn resize_control_points_set(
        p1: Point2<Glass>,
        p2: Point2<Glass>,
        p3: Point2<Glass>,
        p4: Point2<Glass>,
    ) -> ControlPointSet {
        struct ResizeBehaviour {
            attachment_point: Point2<BoxPoint>,
            initial_selection: SelectionStateSnapshot,
        }

        impl ControlPointBehaviour for ResizeBehaviour {
            fn step(&self, ctx: &mut ActionContext, point: Point2<Glass>) {
                if let Err(e) = ctx.execute(action::orm::Resize {
                    initial_selection: &self.initial_selection,
                    fixed_point: self.attachment_point,
                    new_point: point,
                }) {
                    pax_engine::log::warn!("resize failed: {:?}", e);
                };
            }
        }

        fn resize_factory(anchor: Point2<BoxPoint>) -> ControlPointBehaviourFactory {
            Rc::new(move |ac, _p| {
                Rc::new(RefCell::new(ResizeBehaviour {
                    attachment_point: anchor,
                    initial_selection: (&ac.selection_state()).into(),
                }))
            })
        }

        // resize points
        let resize_control_points = vec![
            CPoint::new(
                p1, //
                resize_factory(Point2::new(1.0, 1.0)),
            ),
            CPoint::new(
                p1.midpoint_towards(p2),
                resize_factory(Point2::new(0.5, 1.0)),
            ),
            CPoint::new(
                p2, //
                resize_factory(Point2::new(0.0, 1.0)),
            ),
            CPoint::new(
                p2.midpoint_towards(p3),
                resize_factory(Point2::new(0.0, 0.5)),
            ),
            CPoint::new(
                p3, //
                resize_factory(Point2::new(0.0, 0.0)),
            ),
            CPoint::new(
                p3.midpoint_towards(p4),
                resize_factory(Point2::new(0.5, 0.0)),
            ),
            CPoint::new(
                p4, //
                resize_factory(Point2::new(1.0, 0.0)),
            ),
            CPoint::new(
                p4.midpoint_towards(p1),
                resize_factory(Point2::new(1.0, 0.5)),
            ),
        ];

        let resize_control_point_styling = ControlPointStyling {
            round: false,
            stroke: Color::BLUE,
            fill: Color::WHITE,
            stroke_width_pixels: 1.0,
            size_pixels: 7.0,
        };

        ControlPointSet {
            points: resize_control_points,
            styling: resize_control_point_styling,
        }
    }

    fn anchor_control_point_set(anchor: Point2<Glass>) -> ControlPointSet {
        struct AnchorBehaviour {
            initial_object: Option<RuntimeNodeInfo>,
        }

        impl ControlPointBehaviour for AnchorBehaviour {
            fn step(&self, ctx: &mut ActionContext, point: Point2<Glass>) {
                if let Some(initial_object) = &self.initial_object {
                    let t_and_b = initial_object.transform_and_bounds;
                    let point_in_space = t_and_b.transform.inverse() * point;
                    if let Err(e) = ctx.execute(action::orm::SetAnchor {
                        object: &initial_object,
                        point: point_in_space,
                    }) {
                        pax_engine::log::warn!("resize failed: {:?}", e);
                    };
                }
            }
        }

        fn anchor_factory() -> ControlPointBehaviourFactory {
            Rc::new(move |ac, _p| {
                Rc::new(RefCell::new(AnchorBehaviour {
                    initial_object: (&ac.selection_state()).items.first().map(Into::into),
                }))
            })
        }

        // resize points
        let resize_control_points = vec![CPoint::new(anchor, anchor_factory())];

        let resize_control_point_styling = ControlPointStyling {
            round: true,
            stroke: Color::BLUE,
            fill: Color::rgba(255.into(), 255.into(), 255.into(), 150.into()),
            stroke_width_pixels: 1.0,
            size_pixels: 10.0,
        };

        ControlPointSet {
            points: resize_control_points,
            styling: resize_control_point_styling,
        }
    }
}

#[derive(Clone)]
pub struct CPoint {
    pub point: Point2<Glass>,
    pub behaviour: Rc<dyn Fn(&mut ActionContext, Point2<Glass>) -> Rc<RefCell<dyn ToolBehaviour>>>,
}

impl CPoint {
    fn new(point: Point2<Glass>, behaviour: ControlPointBehaviourFactory) -> Self {
        Self { point, behaviour }
    }
}

#[derive(Clone)]
pub struct ControlPointSet {
    pub points: Vec<CPoint>,
    pub styling: ControlPointStyling,
}
