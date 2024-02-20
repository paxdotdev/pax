use pax_engine::api::*;
use pax_engine::math::Point2;
use pax_engine::*;
use pax_std::primitives::{Group, Path, Rectangle};
use pax_std::types::{Color, Fill};
use serde::Deserialize;

use crate::model;
use crate::model::AppState;
use crate::model::ToolState;

use crate::model::action::pointer::Pointer;
use crate::model::math;
use crate::model::math::coordinate_spaces;

#[pax]
#[custom(Default)]
#[file("glass.pax")]
pub struct Glass {
    // selection state
    pub is_selection_active: Property<bool>,
    pub control_points: Property<Vec<ControlPoint>>,
    pub anchor_point: Property<ControlPoint>,
    pub bounding_segments: Property<Vec<BoundingSegment>>,
    // pub selection_visual: Property<SelectionVisual>,

    // rect tool state
    pub is_rect_tool_active: Property<bool>,
    pub rect_tool: Property<RectTool>,
}

impl Glass {
    pub fn handle_mouse_down(&mut self, ctx: &NodeContext, args: ArgsMouseDown) {
        model::perform_action(
            crate::model::action::pointer::PointerAction {
                event: Pointer::Down,
                button: args.mouse.button,
                point: Point2::new(args.mouse.x, args.mouse.y),
            },
            ctx,
        );
    }

    pub fn handle_mouse_move(&mut self, ctx: &NodeContext, args: ArgsMouseMove) {
        model::perform_action(
            crate::model::action::pointer::PointerAction {
                event: Pointer::Move,
                button: args.mouse.button,
                point: Point2::new(args.mouse.x, args.mouse.y),
            },
            ctx,
        );
    }

    pub fn handle_mouse_up(&mut self, ctx: &NodeContext, args: ArgsMouseUp) {
        model::perform_action(
            crate::model::action::pointer::PointerAction {
                event: Pointer::Up,
                button: args.mouse.button,
                point: Point2::new(args.mouse.x, args.mouse.y),
            },
            ctx,
        );
    }

    pub fn update_view(&mut self, ctx: &NodeContext) {
        model::read_app_state_with_derived(ctx, |app_state, derived_state| {
            // Selection state drawing
            if let Some(bounds) = derived_state.selected_bounds {
                self.is_selection_active.set(true);
                let mut sv = SelectionVisual::new_from_box_bounds(bounds);

                // HACK before dirty-dag (to make sure repeat updates)
                if self.control_points.get().len() == sv.control_points.len() {
                    sv.control_points.push(ControlPoint {
                        x: f64::MIN,
                        y: f64::MIN,
                    });
                    sv.bounding_segments.push(BoundingSegment::default());
                }
                self.control_points.set(sv.control_points);
                self.anchor_point.set(sv.anchor_point);
                self.bounding_segments.set(sv.bounding_segments);
            } else {
                self.is_selection_active.set(false);
            }

            // Tool drawing
            match &app_state.tool_state {
                ToolState::Box {
                    p1,
                    p2,
                    fill,
                    stroke,
                } => {
                    self.is_rect_tool_active.set(true);
                    self.rect_tool.set(RectTool {
                        x: Size::Pixels(p1.x.into()),
                        y: Size::Pixels(p1.y.into()),
                        width: Size::Pixels((p2.x - p1.x).into()),
                        height: Size::Pixels((p2.y - p1.y).into()),
                        fill: fill.clone(),
                        stroke: stroke.clone(),
                    });
                }
                ToolState::Movement { .. } => (),
                _ => {
                    // reset all tool visuals
                    self.is_rect_tool_active.set(false);
                }
            }
        });
    }
}

impl Default for Glass {
    fn default() -> Self {
        let sv = SelectionVisual::default();

        Self {
            is_selection_active: Default::default(),
            control_points: Box::new(PropertyLiteral::new(sv.control_points)),
            anchor_point: Box::new(PropertyLiteral::new(sv.anchor_point)),
            bounding_segments: Box::new(PropertyLiteral::new(sv.bounding_segments)),
            is_rect_tool_active: Box::new(PropertyLiteral::new(false)),
            rect_tool: Default::default(),
        }
    }
}

#[pax]
pub struct ControlPoint {
    pub x: f64,
    pub y: f64,
}

impl From<Point2<coordinate_spaces::Glass>> for ControlPoint {
    fn from(value: Point2<coordinate_spaces::Glass>) -> Self {
        Self {
            x: value.x,
            y: value.y,
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

impl
    From<(
        Point2<coordinate_spaces::Glass>,
        Point2<coordinate_spaces::Glass>,
    )> for BoundingSegment
{
    fn from(
        value: (
            Point2<coordinate_spaces::Glass>,
            Point2<coordinate_spaces::Glass>,
        ),
    ) -> Self {
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
pub struct SelectionVisual {
    pub control_points: Vec<ControlPoint>,
    pub anchor_point: ControlPoint,
    pub bounding_segments: Vec<BoundingSegment>,
}

impl SelectionVisual {
    fn new_from_box_bounds(points: [Point2<coordinate_spaces::Glass>; 4]) -> Self {
        let [p1, p2, p3, p4] = points;
        Self {
            control_points: vec![
                p1.into(),
                p1.midpoint_towards(p2).into(),
                p2.into(),
                p1.midpoint_towards(p4).into(),
                //
                // anchor point
                //
                p2.midpoint_towards(p3).into(),
                p3.into(),
                p3.midpoint_towards(p4).into(),
                p4.into(),
            ],
            bounding_segments: vec![
                (p1, p2).into(),
                (p2, p3).into(),
                (p3, p4).into(),
                (p4, p1).into(),
            ],
            anchor_point: p1.midpoint_towards(p3).into(),
        }
    }
}

#[pax]
pub struct RectTool {
    pub x: Size,
    pub y: Size,
    pub width: Size,
    pub height: Size,
    pub fill: Color,
    pub stroke: Color,
}
