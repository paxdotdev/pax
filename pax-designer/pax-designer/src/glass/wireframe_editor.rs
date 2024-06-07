use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Mutex;

use super::model::ToolBehaviour;
use pax_engine::api::*;
use pax_engine::math::{Generic, Point2, Transform2, Vector2};
use pax_engine::Property;
use pax_engine::*;
use pax_manifest::{TemplateNodeId, TypeId, UniqueTemplateNodeIdentifier};
use pax_std::primitives::{Group, Path, Rectangle, Text, Textbox};
use pax_std::types::text::TextStyle;
use serde::Deserialize;

use super::control_point::{ControlPoint, ControlPointBehaviour};
use crate::glass::control_point::{
    ControlPointBehaviourFactory, ControlPointDef, ControlPointStyling,
};
use crate::math::coordinate_spaces::Glass;
use crate::math::{AxisAlignedBox, BoxPoint};
use crate::model::action::ActionContext;
use crate::model::{self, action, SelectionState};
use pax_engine::api::Fill;

#[pax]
#[file("glass/wireframe_editor.pax")]
pub struct WireframeEditor {
    pub control_points: Property<Vec<ControlPointDef>>,
    pub anchor_point: Property<GlassPoint>,
    pub bounding_segments: Property<Vec<BoundingSegment>>,
    pub on_selection_changed: Property<bool>,
}

// Temporary solution - can be moved to private field on ObjectEditor
// Once we have private variables/upwards data passing (from ControlPoint)
thread_local!(
    pub static CONTROL_POINT_FUNCS: RefCell<Option<Vec<ControlPointBehaviourFactory>>> =
        RefCell::new(None);
);

impl WireframeEditor {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        let selected = model::read_app_state_with_derived(|_, derived_state| {
            derived_state.selected_bounds.clone()
        });
        let selected_cp = selected.clone();
        let deps = [selected.untyped()];

        let control_points = self.control_points.clone();
        let bounding_segments = self.bounding_segments.clone();
        // This is an example of hierarchical binding.
        // whenever the selection ID changes,
        // the selection bounds (among other things)
        // are re-bound to the engine node corresponding
        // to that id
        self.on_selection_changed.replace_with(Property::computed(
            move || {
                let selected = selected_cp.get();
                let bounds = selected.total_bounds();
                if let Some(bounds) = bounds {
                    let deps = [bounds.untyped()];
                    let editor =
                        Property::computed(move || get_generic_object_editor(&bounds.get()), &deps);
                    bind_props_to_editor(editor, control_points.clone(), bounding_segments.clone());
                }

                true
            },
            &deps,
        ));
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        // Fire lazy prop if dirty every tick
        self.on_selection_changed.get();
    }
}

fn bind_props_to_editor(
    editor: Property<Editor>,
    control_points_prop: Property<Vec<ControlPointDef>>,
    bounding_segments_prop: Property<Vec<BoundingSegment>>,
) {
    let editorcp = editor.clone();
    let deps = [editor.untyped()];
    control_points_prop.replace_with(Property::computed(
        move || {
            let mut control_points = vec![];
            let mut behaviours = vec![];
            for control_set in editorcp.get().controls {
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
            control_points
        },
        &deps,
    ));

    bounding_segments_prop.replace_with(Property::computed(move || editor.get().segments, &deps));
}

fn get_generic_object_editor(selection_bounds: &Transform2<Generic, Glass>) -> Editor {
    let (o, u, v) = selection_bounds.decompose();
    let [p1, p4, p3, p2] = [o, o + u, o + u + v, o + v];

    let mut editor = Editor::new();

    struct ResizeBehaviour {
        attachment_point: Point2<BoxPoint>,
        initial_selection_state: SelectionState,
    }

    impl ResizeBehaviour {
        fn new(
            attachment_point: Point2<BoxPoint>,
            initial_selection_state: SelectionState,
        ) -> Self {
            Self {
                attachment_point,
                initial_selection_state,
            }
        }
    }

    impl ControlPointBehaviour for ResizeBehaviour {
        fn step(&self, ctx: &mut ActionContext, point: Point2<Glass>) {
            let world_point = ctx.world_transform() * point;
            let Some(item) = self.initial_selection_state.get_single() else {
                // TODO handle multi-selection
                return;
            };
            // todo make conversion nicer
            let axis_box_world = ctx.world_transform() * item.bounds.get();
            let (o, u, v) = axis_box_world.decompose();
            let axis_box_world = AxisAlignedBox::new(o, o + u + v);
            let origin_world = ctx.world_transform() * item.origin;
            if let Err(e) = ctx.execute(action::orm::ResizeSelected {
                attachment_point: self.attachment_point,
                original_bounds: (axis_box_world, origin_world),
                props: &item.props,
                point: world_point,
            }) {
                pax_engine::log::warn!("resize failed: {:?}", e);
            };
        }
    }

    fn resize_factory(anchor: Point2<BoxPoint>) -> ControlPointBehaviourFactory {
        Rc::new(move |ac, _p| {
            Rc::new(RefCell::new(ResizeBehaviour::new(
                anchor,
                ac.selection_state(),
            )))
        })
    }

    // resize points
    editor.add_control_set(
        vec![
            CPoint::new(
                p1, //
                resize_factory(Point2::new(1.0, 1.0)),
            ),
            CPoint::new(
                p1.midpoint_towards(p2),
                resize_factory(Point2::new(0.0, 1.0)),
            ),
            CPoint::new(
                p2, //
                resize_factory(Point2::new(-1.0, 1.0)),
            ),
            CPoint::new(
                p2.midpoint_towards(p3),
                resize_factory(Point2::new(-1.0, 0.0)),
            ),
            CPoint::new(
                p3, //
                resize_factory(Point2::new(-1.0, -1.0)),
            ),
            CPoint::new(
                p3.midpoint_towards(p4),
                resize_factory(Point2::new(0.0, -1.0)),
            ),
            CPoint::new(
                p4, //
                resize_factory(Point2::new(1.0, -1.0)),
            ),
            CPoint::new(
                p4.midpoint_towards(p1),
                resize_factory(Point2::new(1.0, 0.0)),
            ),
        ],
        ControlPointStyling {
            stroke: Color::BLUE,
            fill: Color::WHITE,
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
        rotation_anchor: Point2<Glass>,
        start_dir: Vector2<Glass>,
        start_angle: Rotation,
    }

    impl ControlPointBehaviour for RotationBehaviour {
        fn step(&self, ctx: &mut ActionContext, point: Point2<Glass>) {
            let rotation_anchor = self.rotation_anchor;
            let moving_to = point - rotation_anchor;
            if let Err(e) = ctx.execute(action::orm::RotateSelected {
                rotation_anchor,
                moving_from: self.start_dir,
                moving_to,
                start_angle: self.start_angle.clone(),
            }) {
                pax_engine::log::warn!("rotation failed: {:?}", e);
            };
        }
    }

    fn rotate_factory() -> ControlPointBehaviourFactory {
        Rc::new(|ctx, point| {
            let rotation_anchor = ctx
                .selection_state()
                .get_single()
                .expect("an object is selected")
                .origin;
            let start_angle = ctx
                .selected_nodes()
                .first()
                .unwrap()
                .1
                .layout_properties()
                .rotate;
            let start_dir = point - rotation_anchor;
            Rc::new(RefCell::new(RotationBehaviour {
                rotation_anchor,
                start_dir,
                start_angle: start_angle.unwrap_or(Rotation::Degrees(0.into())),
            }))
        })
    }
    editor.add_control_set(
        vec![
            CPoint::new(p1, rotate_factory()),
            CPoint::new(p2, rotate_factory()),
            CPoint::new(p3, rotate_factory()),
            CPoint::new(p4, rotate_factory()),
        ],
        ControlPointStyling {
            stroke: Color::TRANSPARENT,
            fill: Color::TRANSPARENT,
            stroke_width_pixels: 0.0,
            size_pixels: 27.0,
        },
    );

    editor
}

impl Interpolatable for Editor {}

#[derive(Clone, Default)]
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

#[derive(Clone)]
struct CPoint {
    point: Point2<Glass>,
    behaviour: Rc<dyn Fn(&mut ActionContext, Point2<Glass>) -> Rc<RefCell<dyn ToolBehaviour>>>,
}

impl CPoint {
    fn new(point: Point2<Glass>, behaviour: ControlPointBehaviourFactory) -> Self {
        Self { point, behaviour }
    }
}

#[derive(Clone)]
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
