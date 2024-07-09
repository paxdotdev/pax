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
use crate::model::{self, action, GlassNodeSnapshot, SelectionState, SelectionStateSnapshot};
use pax_engine::api::Fill;

pub mod editor_generation;
use self::editor_generation::Editor;

#[pax]
#[file("glass/wireframe_editor.pax")]
pub struct WireframeEditor {
    pub control_points: Property<Vec<ControlPointDef>>,
    pub bounding_segments: Property<Vec<BoundingSegment>>,
    pub on_selection_changed: Property<bool>,
}

// Temporary solution - can be moved to private field on ObjectEditor
// Once we have private variables/upwards data passing (from ControlPoint)
thread_local!(
    pub static CONTROL_POINT_FUNCS: RefCell<Option<Vec<ControlPointBehaviourFactory>>> =
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
        self.on_selection_changed.replace_with(Property::computed(
            move || {
                let selected = selected_cp.get();
                if selected.items.len() > 0 {
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

        bounding_segments.replace_with(Property::computed(
            move || editor.get().segments.into_iter().map(Into::into).collect(),
            &deps,
        ));
    }
}

#[pax]
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
pub struct BoundingSegment {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}
