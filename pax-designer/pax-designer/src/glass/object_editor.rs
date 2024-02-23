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
use crate::math::{AxisAlignedBox, BoxPoint};
use crate::model::action::ActionContext;
use crate::model::math::coordinate_spaces::Glass;
use crate::model::{self, action};

#[pax]
#[file("glass/object_editor.pax")]
pub struct ObjectEditor {
    pub control_points: Property<Vec<GlassPoint>>,
    pub anchor_point: Property<GlassPoint>,
    pub bounding_segments: Property<Vec<BoundingSegment>>,
}

type ControlPointFuncs = Vec<Rc<ControlPointBehaviour>>;

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

    fn set_generic_object_editor(&mut self, selection_bounds: &AxisAlignedBox) {
        let [p1, p4, p3, p2] = selection_bounds.bounding_points();

        fn behaviour(attachment_point: Point2<BoxPoint>) -> Rc<ControlPointBehaviour> {
            Rc::new(move |ctx, original_bounds, new_point| {
                let world_point = ctx.world_transform() * new_point;
                let &(ref axis_box, origin) = original_bounds;
                let axis_box_world = axis_box
                    .try_into_space(ctx.world_transform())
                    .expect("tried to transform axis aligned box to non-axis aligned space");
                let origin_world = ctx.world_transform() * origin;
                if let Err(e) = ctx.execute(action::orm::ResizeSelected {
                    attachment_point,
                    original_bounds: (axis_box_world, origin_world),
                    point: world_point,
                }) {
                    pax_engine::log::warn!("resize failed: {:?}", e);
                };
            })
        }

        let control_points_with_behaviour = vec![
            (p1, behaviour(Point2::new(1.0, 1.0))),
            (p1.midpoint_towards(p2), behaviour(Point2::new(0.0, 1.0))),
            (p2, behaviour(Point2::new(-1.0, 1.0))),
            (p2.midpoint_towards(p3), behaviour(Point2::new(-1.0, 0.0))),
            (p3, behaviour(Point2::new(-1.0, -1.0))),
            (p3.midpoint_towards(p4), behaviour(Point2::new(0.0, -1.0))),
            (p4, behaviour(Point2::new(1.0, -1.0))),
            (p4.midpoint_towards(p1), behaviour(Point2::new(1.0, 0.0))),
        ];

        let (control_points, behaviour): (Vec<Point2<Glass>>, Vec<_>) =
            control_points_with_behaviour.into_iter().unzip();
        let mut control_points: Vec<GlassPoint> =
            control_points.into_iter().map(Into::into).collect();

        CONTROL_POINT_FUNCS.with_borrow_mut(|funcs| {
            *funcs = Some(behaviour);
        });

        let mut bounding_segments = vec![
            (p1, p2).into(),
            (p2, p3).into(),
            (p3, p4).into(),
            (p4, p1).into(),
        ];

        // HACK before dirty-dag (to make sure repeat updates)
        if control_points.len() == self.control_points.get().len() {
            control_points.push(GlassPoint {
                x: f64::MIN,
                y: f64::MIN,
            });
            bounding_segments.push(BoundingSegment::default());
        }

        *self = Self {
            control_points: Box::new(PropertyLiteral::new(control_points)),
            bounding_segments: Box::new(PropertyLiteral::new(bounding_segments)),
            anchor_point: Box::new(PropertyLiteral::new(p1.midpoint_towards(p3).into())),
        };
    }
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
