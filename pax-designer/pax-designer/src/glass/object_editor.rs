use std::cell::RefCell;
use std::rc::Rc;

use pax_engine::api::*;
use pax_engine::math::{Point2, Vector2};
use pax_engine::Property;
use pax_engine::*;
use pax_std::primitives::{Group, Path, Rectangle};
use pax_std::types::{Color, Fill};
use serde::Deserialize;

use super::control_point::{ControlPoint, ControlPointBehaviour};
use crate::glass::control_point::{ControlPointDef, ControlPointStyling};
use crate::math::{AxisAlignedBox, BoxPoint};
use crate::model::action::ActionContext;
use crate::model::math::coordinate_spaces::Glass;
use crate::model::{self, action};

#[pax]
#[file("glass/object_editor.pax")]
pub struct ObjectEditor {
    pub control_points: Property<Vec<ControlPointDef>>,
    pub anchor_point: Property<GlassPoint>,
    pub bounding_segments: Property<Vec<BoundingSegment>>,
}

type ControlPointFuncs = Vec<Rc<dyn ControlPointBehaviour>>;
// Temporary solution - can be moved to private field on ObjectEditor
// Once we have private variables/upwards data passing (from ControlPoint)
thread_local!(
    pub static CONTROL_POINT_FUNCS: RefCell<Option<ControlPointFuncs>> = RefCell::new(None);
);

impl ObjectEditor {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        CONTROL_POINT_FUNCS.with_borrow(|funcs| if funcs.is_some() {
            panic!("can't create more than one ObjectEditor with current architecture (need to move CONTROL_POINTS_FUNCS)");
        });
    }

    pub fn pre_render(&mut self, ctx: &NodeContext) {
        model::read_app_state_with_derived(ctx, |_app_state, derived_state| {
            if let Some((bounds, _origin)) = &derived_state.selected_bounds {
                self.set_generic_object_editor(bounds);
            } else {
                CONTROL_POINT_FUNCS.with_borrow_mut(|funcs| {
                    *funcs = None;
                });
                self.control_points.set(vec![]);
                self.bounding_segments.set(vec![]);
                self.anchor_point.set(GlassPoint {
                    x: f64::MIN,
                    y: f64::MIN,
                });
            }
        });
    }

    fn set_editor(&mut self, editor: Editor) {
        let mut control_points = vec![];
        let mut behaviours = vec![];

        for control_set in editor.controls {
            let (control_points_set, behaviour_set): (Vec<_>, Vec<_>) = control_set
                .points
                .into_iter()
                .map(|c_point| (c_point.point, c_point.behaviour))
                .unzip();
            let control_points_from_set: Vec<ControlPointDef> = control_points_set
                .into_iter()
                .map(|p| ControlPointDef {
                    point: p.into(),
                    styling: control_set.styling.clone(),
                })
                .collect();
            control_points.extend(control_points_from_set);
            behaviours.extend(behaviour_set);
        }

        CONTROL_POINT_FUNCS.with_borrow_mut(|funcs| {
            *funcs = Some(behaviours);
        });

        let mut bounding_segments = editor.segments;

        // HACK before dirty-dag (to make sure repeat updates)
        if control_points.len() == self.control_points.get().len() {
            control_points.push(ControlPointDef {
                point: GlassPoint {
                    x: f64::MIN,
                    y: f64::MIN,
                },
                styling: ControlPointStyling::default(),
            });
            bounding_segments.push(BoundingSegment::default());
        }
        self.control_points.set(control_points);
        self.bounding_segments.set(bounding_segments);
    }

    fn set_generic_object_editor(&mut self, selection_bounds: &AxisAlignedBox) {
        let [p1, p4, p3, p2] = selection_bounds.bounding_points();

        let mut editor = Editor::new();

        struct ResizeBehaviour {
            attachment_point: Point2<BoxPoint>,
            initial_box_bounds: RefCell<Option<(AxisAlignedBox, Point2<Glass>)>>,
        }

        impl ResizeBehaviour {
            fn new(attachment_point: Point2<BoxPoint>) -> Self {
                Self {
                    attachment_point,
                    initial_box_bounds: RefCell::new(None),
                }
            }
        }

        impl ControlPointBehaviour for ResizeBehaviour {
            fn init(&self, ctx: &mut ActionContext, _point: Point2<Glass>) {
                *self.initial_box_bounds.borrow_mut() = ctx.selected_bounds();
            }

            fn step(&self, ctx: &mut ActionContext, point: Point2<Glass>) {
                let world_point = ctx.world_transform() * point;
                let bounds = self.initial_box_bounds.borrow();
                let &(ref axis_box, origin) = bounds.as_ref().expect("resize has been initialized");
                let axis_box_world = axis_box
                    .try_into_space(ctx.world_transform())
                    .expect("tried to transform axis aligned box to non-axis aligned space");
                let origin_world = ctx.world_transform() * origin;
                if let Err(e) = ctx.execute(action::orm::ResizeSelected {
                    attachment_point: self.attachment_point,
                    original_bounds: (axis_box_world, origin_world),
                    point: world_point,
                }) {
                    pax_engine::log::warn!("resize failed: {:?}", e);
                };
            }
        }
        // resize points
        editor.add_control_set(
            vec![
                CPoint::new(
                    p1, //
                    ResizeBehaviour::new(Point2::new(1.0, 1.0)),
                ),
                CPoint::new(
                    p1.midpoint_towards(p2),
                    ResizeBehaviour::new(Point2::new(0.0, 1.0)),
                ),
                CPoint::new(
                    p2, //
                    ResizeBehaviour::new(Point2::new(-1.0, 1.0)),
                ),
                CPoint::new(
                    p2.midpoint_towards(p3),
                    ResizeBehaviour::new(Point2::new(-1.0, 0.0)),
                ),
                CPoint::new(
                    p3, //
                    ResizeBehaviour::new(Point2::new(-1.0, -1.0)),
                ),
                CPoint::new(
                    p3.midpoint_towards(p4),
                    ResizeBehaviour::new(Point2::new(0.0, -1.0)),
                ),
                CPoint::new(
                    p4, //
                    ResizeBehaviour::new(Point2::new(1.0, -1.0)),
                ),
                CPoint::new(
                    p4.midpoint_towards(p1),
                    ResizeBehaviour::new(Point2::new(1.0, 0.0)),
                ),
            ],
            ControlPointStyling {
                stroke: Color::rgb(0.0.into(), 0.0.into(), 1.0.into()),
                fill: Color::rgb(1.0.into(), 1.0.into(), 1.0.into()),
                stroke_width_pixels: 1.0,
                size_pixels: 7.0,
            },
        );

        editor.add_bounding_segments(vec![
            (p1, p2).into(),
            (p2, p3).into(),
            (p3, p4).into(),
            (p4, p1).into(),
        ]);

        struct RotationBehaviour {
            rotation_anchor: RefCell<Option<Point2<Glass>>>,
            start_dir: RefCell<Option<Vector2<Glass>>>,
            start_angle: RefCell<Option<Rotation>>,
        }

        impl RotationBehaviour {
            fn new() -> Self {
                Self {
                    rotation_anchor: RefCell::new(None),
                    start_dir: RefCell::new(None),
                    start_angle: RefCell::new(None),
                }
            }
        }

        impl ControlPointBehaviour for RotationBehaviour {
            fn init(&self, ctx: &mut ActionContext, point: Point2<Glass>) {
                let rot_anchor = ctx.selected_bounds().expect("an object is selected").1;
                let initial_object_rotation =
                    ctx.selected_node().unwrap().properties().local_rotation;
                let start_dir = point - rot_anchor;
                *self.rotation_anchor.borrow_mut() = Some(rot_anchor);
                *self.start_dir.borrow_mut() = Some(start_dir);
                *self.start_angle.borrow_mut() = Some(initial_object_rotation);
            }

            fn step(&self, ctx: &mut ActionContext, point: Point2<Glass>) {
                let rotation_anchor = self.rotation_anchor.borrow().unwrap();
                let moving_to = point - rotation_anchor;
                if let Err(e) = ctx.execute(action::orm::RotateSelected {
                    rotation_anchor,
                    moving_from: self.start_dir.borrow().unwrap(),
                    moving_to,
                    start_angle: self.start_angle.borrow().clone().unwrap(),
                }) {
                    pax_engine::log::warn!("rotation failed: {:?}", e);
                };
            }
        }

        editor.add_control_set(
            vec![
                CPoint::new(p1, RotationBehaviour::new()),
                CPoint::new(p2, RotationBehaviour::new()),
                CPoint::new(p3, RotationBehaviour::new()),
                CPoint::new(p4, RotationBehaviour::new()),
            ],
            ControlPointStyling {
                stroke: Color::rgb(0.0.into(), 0.0.into(), 1.0.into()),
                fill: Color::rgba(0.0.into(), 0.0.into(), 0.0.into(), 0.0.into()),
                stroke_width_pixels: 0.0,
                size_pixels: 25.0,
            },
        );

        self.set_editor(editor);
    }
}

struct Editor {
    controls: Vec<ControlPointSet>,
    segments: Vec<BoundingSegment>,
}

impl Editor {
    fn new() -> Self {
        Self {
            controls: Default::default(),
            segments: Default::default(),
        }
    }

    fn add_control_set(&mut self, points: Vec<CPoint>, styling: ControlPointStyling) {
        self.controls.push(ControlPointSet { points, styling });
    }

    fn add_bounding_segments(&mut self, segments: Vec<BoundingSegment>) {
        self.segments.extend(segments);
    }
}

struct CPoint {
    point: Point2<Glass>,
    behaviour: Rc<dyn ControlPointBehaviour>,
}

impl CPoint {
    fn new(point: Point2<Glass>, behaviour: impl ControlPointBehaviour + 'static) -> Self {
        Self {
            point,
            behaviour: Rc::new(behaviour),
        }
    }
}

struct ControlPointSet {
    points: Vec<CPoint>,
    styling: ControlPointStyling,
}

#[pax]
pub struct GlassPoint {
    pub x: f64,
    pub y: f64,
}

#[pax]
pub struct BoundingSegment {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl From<Point2<Glass>> for GlassPoint {
    fn from(value: Point2<Glass>) -> Self {
        GlassPoint {
            x: value.x,
            y: value.y,
        }
    }
}

impl From<(Point2<Glass>, Point2<Glass>)> for BoundingSegment {
    fn from(value: (Point2<Glass>, Point2<Glass>)) -> Self {
        let (p0, p1) = value;
        Self {
            x0: p0.x,
            y0: p0.y,
            x1: p1.x,
            y1: p1.y,
        }
    }
}
