use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Mutex;

use super::model::ToolBehaviour;
use pax_engine::api::*;
use pax_engine::math::{Point2, Vector2};
use pax_engine::Property;
use pax_engine::*;
use pax_manifest::{TemplateNodeId, TypeId, UniqueTemplateNodeIdentifier};
use pax_std::primitives::{Group, Path, Rectangle, Text, Textbox};
use pax_std::types::text::TextStyle;
use pax_std::types::Fill;
use serde::Deserialize;

use super::control_point::{ControlPoint, ControlPointBehaviour};
use crate::glass::control_point::{
    ControlPointBehaviourFactory, ControlPointDef, ControlPointStyling,
};
use crate::math::coordinate_spaces::Glass;
use crate::math::{AxisAlignedBox, BoxPoint};
use crate::model::action::ActionContext;
use crate::model::{self, action, SelectionState};

#[pax]
#[file("glass/object_editor.pax")]
pub struct ObjectEditor {
    pub ids: Property<Vec<TemplateNodeId>>,
    pub control_points: Property<Vec<ControlPointDef>>,
    pub anchor_point: Property<GlassPoint>,
    pub bounding_segments: Property<Vec<BoundingSegment>>,
    pub editor_id: Property<Numeric>,
    //editor bounds
    pub x: Property<f64>,
    pub y: Property<f64>,
    pub width: Property<f64>,
    pub height: Property<f64>,

    pub textbox_editor_style: Property<TextStyle>,
    pub textbox_editor_text: Property<StringBox>,
    pub textbox_editor_original_text: Property<StringBox>,
}

// Temporary solution - can be moved to private field on ObjectEditor
// Once we have private variables/upwards data passing (from ControlPoint)
thread_local!(
    pub static CONTROL_POINT_FUNCS: RefCell<Option<Vec<ControlPointBehaviourFactory>>> =
        RefCell::new(None);
);

impl ObjectEditor {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        CONTROL_POINT_FUNCS.with_borrow(|funcs| if funcs.is_some() {
            panic!("can't create more than one ObjectEditor with current architecture (need to move CONTROL_POINTS_FUNCS)");
        });
    }

    pub fn pre_render(&mut self, ctx: &NodeContext) {
        model::read_app_state_with_derived(ctx, |app_state, derived_state| {
            // HACK: if we were editing text, and the selection has changed,
            // commit the text changes to the old selection before
            // selecting this new object
            if &app_state.selected_template_node_ids != self.ids.get() {
                if *self.editor_id.get() == 1 {
                    self.commit_changes(ctx, app_state.selected_component_id.clone());
                }
            }
            self.ids.set(app_state.selected_template_node_ids.clone());
            // HACK: dirty dag manual check if we need to update
            static BOUNDS: Mutex<Option<AxisAlignedBox>> = Mutex::new(None);
            let total_bounds = derived_state.selected_bounds.total_bounds();
            if BOUNDS.lock().unwrap().as_ref() == total_bounds.as_ref() {
                return;
            }
            *BOUNDS.lock().unwrap() = total_bounds;

            //reset state
            self.editor_id.set(0.into());
            // set state that isn't determined by selection type and object
            if let Some(bounds) = &derived_state.selected_bounds.total_bounds() {
                self.x.set(bounds.top_left().x);
                self.y.set(bounds.top_left().y);
                self.width.set(bounds.width());
                self.height.set(bounds.height());
            }

            if let Some(item) = &derived_state.selected_bounds.get_single() {
                let mut dt = ctx.designtime.borrow_mut();

                // figure out type from template_node (can be changed if we expose this on expanded node)
                let node = dt
                    .get_orm_mut()
                    .get_node(item.id.clone())
                    .expect("node exists")
                    .get_type_id()
                    .import_path();

                match node.as_ref().map(|v| v.as_str()) {
                    Some("pax_designer::pax_reexports::pax_std::primitives::Text") => {
                        self.set_generic_object_editor(&item.bounds);
                        self.editor_id.set(1.into());
                        let node = ctx
                            .get_nodes_by_global_id(item.id.clone())
                            .into_iter()
                            .next()
                            .unwrap();

                        node.with_properties(|text: &mut Text| {
                            // HACK: if this textbox isn't already in "editing state, copy over style and content,
                            // and replace content by invisible character to mark as "taken".
                            // NOTE: if we need to hide the underlying object in more places than text,
                            // create a common property "visible" that temporarily hides an expanded node
                            if &text.text.get().string != "\u{2800}" {
                                self.textbox_editor_style.set(text.style.get().clone());
                                self.textbox_editor_text.set(text.text.get().clone());
                                self.textbox_editor_original_text
                                    .set(text.text.get().clone());
                                text.text.set(StringBox::from("\u{2800}".to_string()));
                            }
                        });
                    }
                    path => {
                        self.set_generic_object_editor(&item.bounds);
                    }
                };
            } else if let Some(total_bounds) = derived_state.selected_bounds.total_bounds() {
                let mut editor = Editor::new();
                let [p1, p2, p3, p4] = total_bounds.corners();
                editor.add_bounding_segments(vec![
                    (p1, p2).into(),
                    (p2, p3).into(),
                    (p3, p4).into(),
                    (p4, p1).into(),
                ]);
                self.set_editor(editor);
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
        }
        if bounding_segments.len() == self.bounding_segments.get().len() {
            bounding_segments.push(BoundingSegment::default());
        }
        self.control_points.set(control_points);
        self.bounding_segments.set(bounding_segments);
    }

    fn set_generic_object_editor(&mut self, selection_bounds: &AxisAlignedBox) {
        let [p1, p4, p3, p2] = selection_bounds.corners();

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
                let axis_box_world = item
                    .bounds
                    .try_into_space(ctx.world_transform())
                    .expect("tried to transform axis aligned box to non-axis aligned space");
                let origin_world = ctx.world_transform() * item.origin;
                if let Err(e) = ctx.execute(action::orm::ResizeSelected {
                    attachment_point: self.attachment_point,
                    original_bounds: (axis_box_world, origin_world),
                    point: world_point,
                }) {
                    pax_engine::log::warn!("resize failed: {:?}", e);
                };
            }
        }

        fn resize_factory(anchor: Point2<BoxPoint>) -> ControlPointBehaviourFactory {
            Box::new(move |ac, _p| Box::new(ResizeBehaviour::new(anchor, ac.selection_state())))
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
            Box::new(|ctx, point| {
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
                    .common_properties()
                    .local_rotation;
                let start_dir = point - rotation_anchor;
                Box::new(RotationBehaviour {
                    rotation_anchor,
                    start_dir,
                    start_angle,
                })
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
                size_pixels: 25.0,
            },
        );

        self.set_editor(editor);
    }

    pub fn commit_changes(&mut self, ctx: &NodeContext, type_id: TypeId) {
        let mut dt = ctx.designtime.borrow_mut();
        let Some(templ_id) = self.ids.get().first() else {
            return;
        };
        let Some(mut builder) = dt
            .get_orm_mut()
            .get_node(UniqueTemplateNodeIdentifier::build(
                type_id,
                templ_id.clone(),
            ))
        else {
            return;
        };
        builder
            .set_typed_property("text", self.textbox_editor_text.get().clone())
            .unwrap();
        builder.save().unwrap();
    }

    pub fn text_editor_input(&mut self, _ctx: &NodeContext, event: Event<TextInput>) {
        self.textbox_editor_text
            .set(StringBox::from(event.text.clone()));
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
    behaviour: Box<dyn Fn(&mut ActionContext, Point2<Glass>) -> Box<dyn ToolBehaviour>>,
}

impl CPoint {
    fn new(point: Point2<Glass>, behaviour: ControlPointBehaviourFactory) -> Self {
        Self {
            point,
            behaviour: Box::new(behaviour),
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
