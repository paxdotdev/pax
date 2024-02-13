use pax_engine::api::*;
use pax_engine::rendering::Point2D;
use pax_engine::*;
use pax_std::primitives::{Group, Path, Rectangle};
use pax_std::types::{Color, Fill};
use serde::Deserialize;

use crate::model;
use crate::model::AppState;
use crate::model::ToolVisual;

use crate::model::action::pointer::Pointer;

#[pax]
#[custom(Default)]
#[file("glass.pax")]
pub struct Glass {
    // selection state
    pub selection_active: Property<bool>,
    pub control_points: Property<Vec<ControlPoint>>,
    pub anchor_point: Property<ControlPoint>,
    pub bounding_segments: Property<Vec<BoundingSegment>>,
    // pub selection_visual: Property<SelectionVisual>,

    // rect tool state
    pub rect_tool_active: Property<bool>,
    pub rect_tool: Property<RectTool>,
}

impl Glass {
    pub fn handle_mouse_down(&mut self, ctx: &NodeContext, args: ArgsMouseDown) {
        model::perform_action(
            crate::model::action::pointer::PointerAction {
                event: Pointer::Down,
                x: args.mouse.x,
                y: args.mouse.y,
            },
            ctx,
        );
    }

    pub fn handle_mouse_move(&mut self, ctx: &NodeContext, args: ArgsMouseMove) {
        model::perform_action(
            crate::model::action::pointer::PointerAction {
                event: Pointer::Move,
                x: args.mouse.x,
                y: args.mouse.y,
            },
            ctx,
        );
    }

    pub fn handle_mouse_up(&mut self, ctx: &NodeContext, args: ArgsMouseUp) {
        model::perform_action(
            crate::model::action::pointer::PointerAction {
                event: Pointer::Up,
                x: args.mouse.x,
                y: args.mouse.y,
            },
            ctx,
        );
    }

    pub fn handle_key_down(&mut self, ctx: &NodeContext, args: ArgsKeyDown) {
        // pax_engine::log::debug!("key down");
        //TODO: handle keydowns and pass into InputMapper
    }

    pub fn update_view(&mut self, ctx: &NodeContext) {
        model::read_app_state(|app_state| {
            if let Some(id) = app_state.selected_template_node_id {
                self.selection_active.set(true);
                let points = app_state.TEMP_TODO_REMOVE_bounds;
                let sv = SelectionVisual::new_from_box_bounds(points);
                self.control_points.set(sv.control_points);
                self.anchor_point.set(sv.anchor_point);
                self.bounding_segments.set(sv.bounding_segments);
            } else {
                self.selection_active.set(false);
            }

            // tool use visual
            if let Some(visual) = &app_state.tool_visual {
                match visual {
                    ToolVisual::Box {
                        x1,
                        y1,
                        x2,
                        y2,
                        fill,
                        stroke,
                    } => {
                        self.rect_tool_active.set(true);
                        self.rect_tool.set(RectTool {
                            x: Size::Pixels(x1.into()),
                            y: Size::Pixels(y1.into()),
                            width: Size::Pixels((x2 - x1).into()),
                            height: Size::Pixels((y2 - y1).into()),
                            fill: fill.clone(),
                            stroke: stroke.clone(),
                        });
                    }
                    ToolVisual::MovingNode { .. } => (),
                }
            } else {
                self.rect_tool_active.set(false);
            };
        });
    }
}

impl Default for Glass {
    fn default() -> Self {
        let sv = SelectionVisual::default();

        Self {
            selection_active: Default::default(),
            control_points: Box::new(PropertyLiteral::new(sv.control_points)),
            anchor_point: Box::new(PropertyLiteral::new(sv.anchor_point)),
            bounding_segments: Box::new(PropertyLiteral::new(sv.bounding_segments)),
            rect_tool_active: Box::new(PropertyLiteral::new(false)),
            rect_tool: Default::default(),
        }
    }
}

#[pax]
pub struct ControlPoint {
    pub x: f64,
    pub y: f64,
}

impl From<pax_engine::rendering::Point2D> for ControlPoint {
    fn from(value: pax_engine::rendering::Point2D) -> Self {
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

impl From<(Point2D, Point2D)> for BoundingSegment {
    fn from(value: (Point2D, Point2D)) -> Self {
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
    fn new_from_box_bounds(points: [Point2D; 4]) -> Self {
        let [p1, p2, p3, p4] = points;
        Self {
            control_points: vec![
                p1.into(),
                ((p1 + p2) / 2.0).into(),
                p2.into(),
                ((p1 + p4) / 2.0).into(),
                //
                // anchor point
                //
                ((p2 + p3) / 2.0).into(),
                p3.into(),
                ((p3 + p4) / 2.0).into(),
                p4.into(),
            ],
            bounding_segments: vec![
                (p1, p2).into(),
                (p2, p3).into(),
                (p3, p4).into(),
                (p4, p1).into(),
            ],
            anchor_point: ((p1 + p3) / 2.0).into(),
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
