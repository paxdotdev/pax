use std::f64::consts::PI;
use std::{cell::RefCell, rc::Rc};

use pax_engine::api::{borrow, borrow_mut, Color, Interpolatable};
use pax_engine::math::{Generic, TransformParts, Vector2};
use pax_engine::NodeLocal;
use pax_engine::{
    api::NodeContext, log, math::Point2, node_layout::TransformAndBounds, NodeInterface, Property,
};

use crate::glass::control_point::{ControlPointTool, Snap};
use crate::glass::ToolVisualizationState;
use crate::math::intent_snapper::{IntentSnapper, SnapCollection, SnapSet};
use crate::model::tools::ToolBehavior;
use crate::utils::designer_cursor::{DesignerCursor, DesignerCursorType};
use crate::{
    glass::control_point::{ControlPointBehavior, ControlPointStyling, ControlPointToolFactory},
    math::coordinate_spaces::{Glass, SelectionSpace},
    model::{
        action::{self, Action, ActionContext},
        GlassNodeSnapshot, SelectionState, SelectionStateSnapshot,
    },
};

impl Interpolatable for Editor {}

pub mod anchor_control;
pub mod resize_control;
pub mod rotate_control;
pub mod slot_control;
pub mod stacker_control;

#[derive(Clone, Default)]
pub struct Editor {
    pub controls: Vec<ControlPointSet>,
    pub segments: Property<Vec<(Point2<Glass>, Point2<Glass>)>>,
}

impl Editor {
    pub fn new(ctx: NodeContext, selection: SelectionState) -> Self {
        let total_bounds = selection.total_bounds.clone();
        let deps = [total_bounds.untyped()];
        let total_bounds_cp = total_bounds.clone();

        let anchor = selection.total_origin.clone();
        let mut control_point_sets = vec![
            anchor_control::anchor_control_point_set(anchor),
            resize_control::resize_control_points_set(total_bounds.clone()),
            rotate_control::rotate_control_points_set(total_bounds.clone()),
        ];

        control_point_sets.extend(Self::object_specific_control_point_sets(ctx, selection));

        let bounding_segments = Property::computed(
            move || {
                let total_bounds = total_bounds_cp.get();
                let [p1, p4, p3, p2] = total_bounds.corners();
                vec![(p1, p2), (p2, p3), (p3, p4), (p4, p1)]
            },
            &deps,
        );

        Self {
            controls: control_point_sets,
            segments: bounding_segments,
        }
    }

    fn object_specific_control_point_sets(
        ctx: NodeContext,
        selection: SelectionState,
    ) -> Vec<ControlPointSet> {
        if selection.items.len() != 1 {
            return Vec::default();
        }
        let item = selection.items.into_iter().next().unwrap();
        let type_id = {
            let mut dt = borrow_mut!(ctx.designtime);
            let Some(builder) = dt.get_orm_mut().get_node_builder(item.id.clone(), false) else {
                return Vec::default();
            };
            builder.get_type_id()
        };
        let import_path = type_id.import_path();
        match import_path.as_ref().map(|v| v.as_str()) {
            Some("pax_std::layout::stacker::Stacker") => {
                vec![
                    stacker_control::stacker_divider_control_set(ctx.clone(), item.clone()),
                    // add slot control generally for all slot components?
                    slot_control::slot_dot_control_set(ctx.clone(), item.clone()),
                ]
            }
            _ => return Vec::default(),
        }
    }
}

impl Interpolatable for CPoint {}

#[derive(Clone)]
pub struct CPoint {
    /// Where this control point is placed
    pub point: Point2<Glass>,
    /// Node-local rotation in degrees
    pub rotation: f64,
    /// Node-local cursor rotation in degrees
    pub cursor_rotation: f64,
    /// Anchor of control point (x/y 0.0-1.0, 0.5 is center)
    pub anchor: Point2<NodeLocal>,
    /// behavior on click/double click
    pub behavior: ControlPointToolFactory,
}

impl Default for CPoint {
    fn default() -> Self {
        Self {
            point: Default::default(),
            rotation: 0.0,
            cursor_rotation: 0.0,
            anchor: Point2::new(0.5, 0.5),
            // TODO we want this to be required most likely,
            // make builder instead of derive of Default?
            behavior: ControlPointToolFactory {
                tool_factory: Rc::new(|_, _| {
                    Rc::new(RefCell::new({
                        struct NoOpToolBehavior;
                        impl ToolBehavior for NoOpToolBehavior {
                            fn pointer_down(
                                &mut self,
                                _point: Point2<Glass>,
                                _ctx: &mut ActionContext,
                            ) -> std::ops::ControlFlow<()> {
                                std::ops::ControlFlow::Break(())
                            }

                            fn pointer_move(
                                &mut self,
                                _point: Point2<Glass>,
                                _ctx: &mut ActionContext,
                            ) -> std::ops::ControlFlow<()> {
                                std::ops::ControlFlow::Break(())
                            }

                            fn pointer_up(
                                &mut self,
                                _point: Point2<Glass>,
                                _ctx: &mut ActionContext,
                            ) -> std::ops::ControlFlow<()> {
                                std::ops::ControlFlow::Break(())
                            }

                            fn finish(&mut self, _ctx: &mut ActionContext) -> anyhow::Result<()> {
                                Ok(())
                            }

                            fn keyboard(
                                &mut self,
                                _event: crate::model::input::InputEvent,
                                _dir: crate::model::input::Dir,
                                _ctx: &mut ActionContext,
                            ) -> std::ops::ControlFlow<()> {
                                std::ops::ControlFlow::Break(())
                            }

                            fn get_visual(&self) -> Property<ToolVisualizationState> {
                                Property::default()
                            }
                        }
                        NoOpToolBehavior
                    }))
                }),
                double_click_behavior: Rc::new(|_| ()),
            },
        }
    }
}

impl Interpolatable for ControlPointSet {}

#[derive(Clone, Default)]
pub struct ControlPointSet {
    pub points: Property<Vec<CPoint>>,
    pub styling: ControlPointStyling,
}
