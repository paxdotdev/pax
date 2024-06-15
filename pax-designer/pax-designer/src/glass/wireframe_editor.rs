use anyhow::anyhow;
use std::cell::RefCell;
use std::ops::ControlFlow;
use std::rc::Rc;
use std::sync::Mutex;

use super::model::ToolBehaviour;
use pax_engine::api::*;
use pax_engine::layout::TransformAndBounds;
use pax_engine::math::{Generic, Point2, Transform2, Vector2};
use pax_engine::Property;
use pax_engine::*;
use pax_manifest::{TemplateNodeId, TypeId, UniqueTemplateNodeIdentifier};
use pax_std::primitives::Ellipse;
use pax_std::primitives::{Group, Path, Rectangle, Text, Textbox};
use pax_std::types::text::TextStyle;
use serde::Deserialize;

use super::control_point::{ControlPoint, ControlPointBehaviour};
use crate::glass::control_point::{
    ControlPointBehaviourFactory, ControlPointDef, ControlPointStyling,
};
use crate::math::coordinate_spaces::{Glass, SelectionSpace};
use crate::math::{AxisAlignedBox, BoxPoint, GetUnit, SizeUnit};
use crate::model::action::orm::{write_to_orm, SetAnchor};
use crate::model::action::ActionContext;
use crate::model::{self, action, RuntimeNodeInfo, SelectionState, SelectionStateSnapshot};
use pax_engine::api::Fill;

#[pax]
#[file("glass/wireframe_editor.pax")]
pub struct WireframeEditor {
    pub control_points: Property<Vec<ControlPointDef>>,
    pub anchor_point: Property<GlassPoint>,
    pub bounding_segments: Property<Vec<BoundingSegment>>,
    pub on_selection_changed: Property<bool>,
    pub anchor_x: Property<f64>,
    pub anchor_y: Property<f64>,
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
        let anchor_x = self.anchor_x.clone();
        let anchor_y = self.anchor_y.clone();
        // This is an example of hierarchical binding.
        // whenever the selection ID changes,
        // the selection bounds (among other things)
        // are re-bound to the engine node corresponding
        // to that id
        self.on_selection_changed.replace_with(Property::computed(
            move || {
                let selected = selected_cp.get();

                if selected.items.len() > 0 {
                    // hook up anchor point
                    let orig = selected.total_origin.clone();
                    let deps = [orig.untyped()];
                    let cp_orig = orig.clone();
                    anchor_x.replace_with(Property::computed(move || cp_orig.get().x, &deps));
                    anchor_y.replace_with(Property::computed(move || orig.get().y, &deps));
                    // hook up outline + control points
                    let t_and_b = selected.total_bounds;
                    let deps = [t_and_b.untyped()];
                    let editor = Property::computed(
                        move || get_generic_object_editor(&t_and_b.get()),
                        &deps,
                    );
                    bind_props_to_editor(editor, control_points.clone(), bounding_segments.clone());
                } else {
                    control_points.replace_with(Property::new(vec![]));
                    bounding_segments.replace_with(Property::new(vec![]));
                    anchor_x.replace_with(Property::new(-100.0));
                    anchor_y.replace_with(Property::new(-100.0));
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

    pub fn anchor_move(&mut self, ctx: &NodeContext, _event: Event<MouseDown>) {
        model::with_action_context(ctx, |ac| {
            let selection: SelectionStateSnapshot = (&ac.selection_state()).into();
            if selection.items.len() == 1 {
                let item = selection.items.into_iter().next().unwrap();
                ac.app_state
                    .tool_behaviour
                    .set(Some(Rc::new(RefCell::new(MoveAnchorTool { object: item }))));
            }
        });
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

fn get_generic_object_editor(
    selection_bounds: &TransformAndBounds<SelectionSpace, Glass>,
) -> Editor {
    let (o, u, v) = selection_bounds.transform.decompose();
    let u = u * selection_bounds.bounds.0;
    let v = v * selection_bounds.bounds.1;
    let [p1, p4, p3, p2] = [o, o + v, o + u + v, o + u];

    let mut editor = Editor::new();

    struct ResizeBehaviour {
        attachment_point: Point2<BoxPoint>,
        initial_selection: SelectionStateSnapshot,
    }

    impl ResizeBehaviour {
        fn new(
            attachment_point: Point2<BoxPoint>,
            initial_selection_state: &SelectionState,
        ) -> Self {
            Self {
                attachment_point,
                initial_selection: initial_selection_state.into(),
            }
        }
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
            Rc::new(RefCell::new(ResizeBehaviour::new(
                anchor,
                &ac.selection_state(),
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

struct MoveAnchorTool {
    object: RuntimeNodeInfo,
}

impl ToolBehaviour for MoveAnchorTool {
    fn pointer_down(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Break(())
    }

    fn pointer_move(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        let t_and_b = self.object.transform_and_bounds;
        let point_in_space = t_and_b.transform.inverse() * point;
        if ctx
            .execute(SetAnchor {
                object: &self.object,
                point: point_in_space,
            })
            .is_err()
        {
            log::warn!("failed to move anchor");
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }

    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        self.pointer_move(point, ctx);
        ControlFlow::Break(())
    }

    fn keyboard(
        &mut self,
        _event: model::input::InputEvent,
        _dir: model::input::Dir,
        _ctx: &mut ActionContext,
    ) -> ControlFlow<()> {
        ControlFlow::Break(())
    }

    fn visualize(&self, _glass: &mut crate::glass::Glass) {}
}
