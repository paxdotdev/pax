use anyhow::anyhow;
use std::cell::RefCell;
use std::ops::ControlFlow;
use std::rc::Rc;
use std::sync::Mutex;

use super::model::ToolBehavior;
use pax_engine::api::*;
use pax_engine::layout::TransformAndBounds;
use pax_engine::math::{Generic, Parts, Point2, Transform2, Vector2};
use pax_engine::Property;
use pax_engine::*;
use pax_manifest::{TemplateNodeId, TypeId, UniqueTemplateNodeIdentifier};
use pax_std::*;
use serde::Deserialize;

use super::control_point::{ControlPoint, ControlPointBehavior};
use crate::glass::control_point::{ControlPointDef, ControlPointStyling, ControlPointToolFactory};
use crate::math::coordinate_spaces::{Glass, SelectionSpace};
use crate::math::{AxisAlignedBox, BoxPoint, GetUnit, SizeUnit};
use crate::model::action::orm::SetAnchor;
use crate::model::action::ActionContext;
use crate::model::{self, action, GlassNodeSnapshot, SelectionState, SelectionStateSnapshot};
use pax_engine::api::Fill;

pub mod editor_generation;
use self::editor_generation::Editor;

#[pax]
#[engine_import_path("pax_engine")]
#[file("glass/wireframe_editor.pax")]
pub struct WireframeEditor {
    pub control_points: Property<Vec<ControlPointDef>>,
    pub bounding_segments: Property<Vec<BoundingSegment>>,
    pub on_selection_changed: Property<bool>,
    pub object_rotation: Property<Rotation>,
}

// Temporary solution - can be moved to private field on ObjectEditor
// Once we have private variables/upwards data passing (from ControlPoint)
thread_local!(
    pub static CONTROL_POINT_FUNCS: RefCell<Option<Vec<ControlPointToolFactory>>> =
        RefCell::new(None);
);

#[allow(unused)]
impl WireframeEditor {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let selected = model::read_app_state_with_derived(|_, derived_state| {
            derived_state.selection_state.clone()
        });
        let selected_cp = selected.clone();
        let deps = [selected.untyped()];

        let control_points = self.control_points.clone();
        let bounding_segments = self.bounding_segments.clone();
        // This is doing _hierarchical_ binding:
        // whenever the selection ID changes, the transform and bounds of
        // the editor (among other things) are re-bound to the engine node
        // corresponding to that id. "bindings inside bindings"
        let ctx = ctx.clone();
        let object_rotation = self.object_rotation.clone();
        self.on_selection_changed.replace_with(Property::computed(
            move || {
                let selected = selected_cp.get();
                if selected.items.len() > 0 {
                    Self::bind_object_transform(object_rotation.clone(), &selected);
                    let editor = Editor::new(ctx.clone(), selected);
                    Self::bind_editor(control_points.clone(), bounding_segments.clone(), editor);
                } else {
                    control_points.replace_with(Property::new(vec![]));
                    bounding_segments.replace_with(Property::new(vec![]));
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

    fn bind_editor(
        control_points: Property<Vec<ControlPointDef>>,
        bounding_segments: Property<Vec<BoundingSegment>>,
        editor: Property<Editor>,
    ) {
        let editorcp = editor.clone();
        let deps = [editor.untyped()];

        control_points.replace_with(Property::computed(
            move || {
                let mut control_points = vec![];
                let mut behaviors = vec![];
                for control_set in editorcp.get().controls {
                    let (control_points_set, behavior_set): (Vec<_>, Vec<_>) = control_set
                        .points
                        .into_iter()
                        .map(|c_point| (c_point.point, c_point.behavior))
                        .unzip();
                    let control_points_from_set: Vec<ControlPointDef> = control_points_set
                        .into_iter()
                        .map(|p| ControlPointDef {
                            point: p.into(),
                            styling: control_set.styling.clone(),
                        })
                        .collect();
                    control_points.extend(control_points_from_set);
                    behaviors.extend(behavior_set);
                }

                CONTROL_POINT_FUNCS.with_borrow_mut(|funcs| {
                    *funcs = Some(behaviors);
                });
                control_points
            },
            &deps,
        ));

        bounding_segments.replace_with(Property::computed(
            move || editor.get().segments.into_iter().map(Into::into).collect(),
            &deps,
        ));
    }

    pub fn bind_object_transform(object_rotation: Property<Rotation>, selection: &SelectionState) {
        let t_and_b = selection.total_bounds.clone();
        let deps = [t_and_b.untyped()];
        object_rotation.replace_with(Property::computed(
            move || {
                let parts: Parts = t_and_b.get().as_transform().into();
                Rotation::Radians(parts.rotation.into())
            },
            &deps,
        ));
    }
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct GlassPoint {
    pub x: f64,
    pub y: f64,
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

#[pax]
#[engine_import_path("pax_engine")]
pub struct BoundingSegment {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}
