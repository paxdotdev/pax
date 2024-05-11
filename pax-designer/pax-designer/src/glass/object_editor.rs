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
#[file("glass/object_editor.pax")]
pub struct ObjectEditor {
    pub editor_id: Property<i64>,
    pub control_points: Property<Vec<ControlPointDef>>,
    pub anchor_point: Property<GlassPoint>,
    pub bounding_segments: Property<Vec<BoundingSegment>>,
    //editor bounds
    pub x: Property<f64>,
    pub y: Property<f64>,
    pub width: Property<f64>,
    pub height: Property<f64>,

    pub textbox_editor_style: Property<TextStyle>,
    pub textbox_editor_text: Property<String>,
    pub textbox_editor_original_text: Property<String>,
}

// Temporary solution - can be moved to private field on ObjectEditor
// Once we have private variables/upwards data passing (from ControlPoint)
thread_local!(
    pub static CONTROL_POINT_FUNCS: RefCell<Option<Vec<ControlPointBehaviourFactory>>> =
        RefCell::new(None);
);

impl ObjectEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        model::read_app_state(|app_state| {
            let comp_id = app_state.selected_component_id.clone();
            let node_ids = app_state.selected_template_node_ids.clone();
            let transform = app_state.glass_to_world_transform.clone();
            let manifest_ver = borrow!(ctx.designtime).get_manifest_version();
            let ctx = ctx.clone();

            let deps = [
                comp_id.untyped(),
                node_ids.untyped(),
                transform.untyped(),
                manifest_ver.untyped(),
            ];
            let editor = Property::computed(
                move || {
                    let selected_bounds =
                        model::with_action_context(&ctx, |ac| ac.selection_state());
                    if let Some(total_bounds) = selected_bounds.total_bounds() {
                        get_generic_object_editor(&total_bounds)
                    } else {
                        Editor::new()
                    }
                },
                &deps,
            );

            bind_props_to_editor(
                editor,
                self.control_points.clone(),
                self.bounding_segments.clone(),
            );
        });
    }

    pub fn pre_render(&mut self, ctx: &NodeContext) {
        // model::read_app_state_with_derived(ctx, |app_state, derived_state| {
        //     // HACK: if we were editing text, and the selection has changed,
        //     // commit the text changes to the old selection before
        //     // selecting this new object
        //     if app_state.selected_template_node_ids.get() != self.ids.get() {
        //         if self.editor_id.get() == 1 {
        //             self.commit_changes(ctx, app_state.selected_component_id.get().clone());
        //         }
        //     }
        //     self.ids
        //         .set(app_state.selected_template_node_ids.get().clone());
        //     // HACK: dirty dag manual check if we need to update
        //     static BOUNDS: Mutex<Option<AxisAlignedBox>> = Mutex::new(None);
        //     let total_bounds = derived_state.selected_bounds.total_bounds();
        //     if BOUNDS.lock().unwrap().as_ref() == total_bounds.as_ref() {
        //         return;
        //     }
        //     *BOUNDS.lock().unwrap() = total_bounds;

        //     //reset state
        //     self.editor_id.set(0.into());
        //     // set state that isn't determined by selection type and object
        //     if let Some(bounds) = &derived_state.selected_bounds.total_bounds() {
        //         self.x.set(bounds.top_left().x);
        //         self.y.set(bounds.top_left().y);
        //         self.width.set(bounds.width());
        //         self.height.set(bounds.height());
        //     }

        //     if let Some(item) = &derived_state.selected_bounds.get_single() {
        //         let mut dt = borrow_mut!(ctx.designtime);

        //         // figure out type from template_node (can be changed if we expose this on expanded node)
        //         let node = dt
        //             .get_orm_mut()
        //             .get_node(item.id.clone())
        //             .expect("node exists")
        //             .get_type_id()
        //             .import_path();

        //         match node.as_ref().map(|v| v.as_str()) {
        //             Some("pax_designer::pax_reexports::pax_std::primitives::Text") => {
        //                 self.set_generic_object_editor(&item.bounds);
        //                 self.editor_id.set(1.into());
        //                 let node = ctx
        //                     .get_nodes_by_global_id(item.id.clone())
        //                     .into_iter()
        //                     .next()
        //                     .unwrap();

        //                 node.with_properties(|text: &mut Text| {
        //                     // HACK: if this textbox isn't already in "editing state, copy over style and content,
        //                     // and replace content by invisible character to mark as "taken".
        //                     // NOTE: if we need to hide the underlying object in more places than text,
        //                     // create a common property "visible" that temporarily hides an expanded node
        //                     if &text.text.get() != "\u{2800}" {
        //                         self.textbox_editor_style.set(text.style.get().clone());
        //                         self.textbox_editor_text.set(text.text.get());
        //                         self.textbox_editor_original_text.set(text.text.get());
        //                         text.text.set(String::from("\u{2800}"));
        //                     }
        //                 });
        //             }
        //             path => {
        //                 self.set_generic_object_editor(&item.bounds);
        //             }
        //         };
        //     } else if let Some(total_bounds) = derived_state.selected_bounds.total_bounds() {
        //         let mut editor = Editor::new();
        //         let [p1, p2, p3, p4] = total_bounds.corners();
        //         editor.add_bounding_segments(vec![
        //             (p1, p2).into(),
        //             (p2, p3).into(),
        //             (p3, p4).into(),
        //             (p4, p1).into(),
        //         ]);
        //         self.set_editor(editor);
        //     } else {
        //         CONTROL_POINT_FUNCS.with_borrow_mut(|funcs| {
        //             *funcs = None;
        //         });
        //         self.control_points.set(vec![]);
        //         self.bounding_segments.set(vec![]);
        //         self.anchor_point.set(GlassPoint {
        //             x: f64::MIN,
        //             y: f64::MIN,
        //         });
        //     }
        // });
    }

    pub fn commit_changes(&mut self, ctx: &NodeContext, type_id: TypeId) {
        // let mut dt = borrow_mut!(ctx.designtime);
        // let ids = self.ids.get();
        // let Some(templ_id) = ids.first() else {
        //     return;
        // };
        // let Some(mut builder) = dt
        //     .get_orm_mut()
        //     .get_node(UniqueTemplateNodeIdentifier::build(
        //         type_id,
        //         templ_id.clone(),
        //     ))
        // else {
        //     return;
        // };
        // builder
        //     .set_typed_property("text", self.textbox_editor_text.get().clone())
        //     .unwrap();
        // builder.save().unwrap();
    }

    pub fn text_editor_input(&mut self, _ctx: &NodeContext, event: Event<TextInput>) {
        self.textbox_editor_text.set(String::from(&event.text));
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

fn get_generic_object_editor(selection_bounds: &AxisAlignedBox) -> Editor {
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
                .common_properties()
                .local_rotation;
            let start_dir = point - rotation_anchor;
            Rc::new(RefCell::new(RotationBehaviour {
                rotation_anchor,
                start_dir,
                start_angle,
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
