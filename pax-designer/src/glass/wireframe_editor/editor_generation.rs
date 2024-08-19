use std::f64::consts::PI;
use std::{cell::RefCell, rc::Rc};

use pax_engine::api::{borrow, borrow_mut, Color, Interpolatable};
use pax_engine::math::Parts;
use pax_engine::{
    api::NodeContext, layout::TransformAndBounds, log, math::Point2, NodeInterface, Property,
};

use crate::glass::control_point::{ControlPointTool, Snap};
use crate::glass::ToolVisualizationState;
use crate::math::intent_snapper::{IntentSnapper, SnapCollection, SnapSet};
use crate::{
    glass::control_point::{ControlPointBehavior, ControlPointStyling, ControlPointToolFactory},
    math::{
        coordinate_spaces::{Glass, SelectionSpace},
        BoxPoint,
    },
    model::{
        action::{self, Action, ActionContext},
        GlassNodeSnapshot, SelectionState, SelectionStateSnapshot, ToolBehavior,
    },
};

impl Interpolatable for Editor {}
pub mod slot_control;
pub mod stacker_control;

#[derive(Clone, Default)]
pub struct Editor {
    pub controls: Vec<ControlPointSet>,
    pub segments: Vec<(Point2<Glass>, Point2<Glass>)>,
}

impl Editor {
    pub fn new(ctx: NodeContext, selection: SelectionState) -> Property<Self> {
        let total_bounds = selection.total_bounds.clone();
        let deps = [total_bounds.untyped()];
        let total_bound_derived = Property::computed(
            move || {
                let total_bounds = total_bounds.get();
                let [p1, p4, p3, p2] = total_bounds.corners();
                (
                    vec![
                        Self::resize_control_points_set(p1, p2, p3, p4),
                        Self::rotate_control_points_set(p1, p2, p3, p4),
                    ],
                    vec![(p1, p2), (p2, p3), (p3, p4), (p4, p1)],
                )
            },
            &deps,
        );

        let anchor = selection.total_origin.clone();
        let deps = [anchor.untyped()];
        let anchor_derived = Property::computed(
            move || {
                let anchor = anchor.get();
                vec![Self::anchor_control_point_set(anchor)]
            },
            &deps,
        );

        let mut deps = vec![total_bound_derived.untyped(), anchor_derived.untyped()];
        let object_specific_derived = Self::object_specific_control_point_sets(ctx, selection);
        deps.extend(object_specific_derived.iter().map(Property::untyped));
        Property::computed(
            move || {
                let (resize_and_rotate_sets, bounding_segments) = total_bound_derived.get();
                let anchor = anchor_derived.get();
                let object_specific_derived =
                    object_specific_derived.iter().map(Property::get).collect();
                Self {
                    controls: [resize_and_rotate_sets, anchor, object_specific_derived]
                        .into_iter()
                        .flatten()
                        .collect(),
                    segments: bounding_segments,
                }
            },
            &deps,
        )
    }

    fn rotate_control_points_set(
        p1: Point2<Glass>,
        p2: Point2<Glass>,
        p3: Point2<Glass>,
        p4: Point2<Glass>,
    ) -> ControlPointSet {
        struct RotationBehavior {
            initial_selection: SelectionStateSnapshot,
            start_pos: Point2<Glass>,
        }

        impl ControlPointBehavior for RotationBehavior {
            fn step(&self, ctx: &mut ActionContext, point: Point2<Glass>) -> anyhow::Result<()> {
                action::orm::RotateSelected {
                    curr_pos: point,
                    start_pos: self.start_pos,
                    initial_selection: &self.initial_selection,
                }
                .perform(ctx)
            }
        }

        fn rotate_factory() -> ControlPointToolFactory {
            ControlPointToolFactory {
                tool_factory: Rc::new(|ctx, point| {
                    let initial_selection = (&ctx.derived_state.selection_state.get()).into();
                    Rc::new(RefCell::new(ControlPointTool::new(
                        ctx.transaction("rotating"),
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

        let rotate_control_points = vec![
            CPoint::new(p1, rotate_factory()),
            CPoint::new(p2, rotate_factory()),
            CPoint::new(p3, rotate_factory()),
            CPoint::new(p4, rotate_factory()),
        ];
        let rotate_control_point_styling = ControlPointStyling {
            round: true,
            stroke: Color::TRANSPARENT,
            fill: Color::TRANSPARENT,
            stroke_width_pixels: 0.0,
            width: 38.0,
            height: 38.0,
            affected_by_transform: false,
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
        struct ResizeBehavior {
            attachment_point: Point2<BoxPoint>,
            initial_selection: SelectionStateSnapshot,
        }

        impl ControlPointBehavior for ResizeBehavior {
            fn step(&self, ctx: &mut ActionContext, point: Point2<Glass>) -> anyhow::Result<()> {
                action::orm::Resize {
                    initial_selection: &self.initial_selection,
                    fixed_point: self.attachment_point,
                    new_point: point,
                }
                .perform(ctx)
            }
        }

        fn resize_factory(anchor: Point2<BoxPoint>) -> ControlPointToolFactory {
            ControlPointToolFactory {
                tool_factory: Rc::new(move |ac, _p| {
                    let initial_selection: SelectionStateSnapshot =
                        (&ac.derived_state.selection_state.get()).into();

                    // only snap if either no rotation or this is a corner point
                    let rot = Into::<Parts>::into(initial_selection.total_bounds.transform)
                        .rotation
                        .rem_euclid(PI / 2.0);
                    let should_snap = (rot < 1e-2 || rot > PI / 2.0 - 1e-2)
                        || (anchor.x != 0.5 && anchor.y != 0.5);

                    Rc::new(RefCell::new(ControlPointTool::new(
                        ac.transaction("resize"),
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
            width: 8.0,
            height: 8.0,
            affected_by_transform: false,
        };

        ControlPointSet {
            points: resize_control_points,
            styling: resize_control_point_styling,
        }
    }

    fn anchor_control_point_set(anchor: Point2<Glass>) -> ControlPointSet {
        struct AnchorBehavior {
            initial_object: Option<GlassNodeSnapshot>,
        }

        impl ControlPointBehavior for AnchorBehavior {
            fn step(&self, ctx: &mut ActionContext, point: Point2<Glass>) -> anyhow::Result<()> {
                if let Some(initial_object) = &self.initial_object {
                    let t_and_b = initial_object.transform_and_bounds;
                    let point_in_space = t_and_b.transform.inverse() * point;
                    action::orm::SetAnchor {
                        object: &initial_object,
                        point: point_in_space,
                    }
                    .perform(ctx)
                } else {
                    Ok(())
                }
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
                        ac.transaction("moving anchor point"),
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

        let anchor_control_point = vec![CPoint::new(anchor, anchor_factory())];

        let anchor_control_point_styling = ControlPointStyling {
            round: true,
            stroke: Color::BLUE,
            fill: Color::rgba(255.into(), 255.into(), 255.into(), 150.into()),
            stroke_width_pixels: 1.0,
            width: 10.0,
            height: 10.0,
            affected_by_transform: false,
        };

        ControlPointSet {
            points: anchor_control_point,
            styling: anchor_control_point_styling,
        }
    }

    fn object_specific_control_point_sets(
        ctx: NodeContext,
        selection: SelectionState,
    ) -> Vec<Property<ControlPointSet>> {
        if selection.items.len() != 1 {
            return Vec::default();
        }
        let item = selection.items.into_iter().next().unwrap();
        let type_id = {
            let mut dt = borrow_mut!(ctx.designtime);
            let Some(builder) = dt.get_orm_mut().get_node(item.id.clone(), false) else {
                return Vec::default();
            };
            builder.get_type_id()
        };
        let import_path = type_id.import_path();
        match import_path.as_ref().map(|v| v.as_str()) {
            Some("pax_std::layout::stacker::Stacker") => {
                vec![
                    slot_control::slot_dot_control_set(ctx.clone(), item.clone()),
                    stacker_control::stacker_divider_control_set(ctx.clone(), item.clone()),
                ]
            }
            _ => return Vec::default(),
        }
    }
}

impl Interpolatable for CPoint {}

#[derive(Clone)]
pub struct CPoint {
    // make this point a prop?
    pub point: Point2<Glass>,
    pub behavior: ControlPointToolFactory,
}

impl CPoint {
    fn new(point: Point2<Glass>, behavior: ControlPointToolFactory) -> Self {
        Self { point, behavior }
    }
}

impl Interpolatable for ControlPointSet {}

#[derive(Clone, Default)]
pub struct ControlPointSet {
    pub points: Vec<CPoint>,
    pub styling: ControlPointStyling,
}
