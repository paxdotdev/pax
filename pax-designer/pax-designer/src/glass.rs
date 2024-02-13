use pax_engine::api::*;
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
        pax_engine::log::debug!("key down");
        //TODO: handle keydowns and pass into InputMapper
    }

    pub fn update_view(&mut self, ctx: &NodeContext) {
        model::read_app_state(|app_state| {
            // selection state visual
            if let Some(id) = app_state.selected_template_node_id {
                // TODO let bounding box = ctx.get_template_node_bounding_box(id);
                self.selection_active.set(true);
                let (x1, y1, x2, y2) = app_state.TEMP_TODO_REMOVE_bounds;
                let sv = SelectionVisual::new_from_box_bounds(x1, y1, x2, y2);
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
                }
            } else {
                self.rect_tool_active.set(false);
            };
        });
    }
}

impl Default for Glass {
    fn default() -> Self {
        let sv = SelectionVisual::new_from_box_bounds(300.0, 100.0, 400.0, 200.0);

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

#[pax]
pub struct BoundingSegment {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

#[pax]
pub struct SelectionVisual {
    pub control_points: Vec<ControlPoint>,
    pub anchor_point: ControlPoint,
    pub bounding_segments: Vec<BoundingSegment>,
}

impl SelectionVisual {
    fn new_from_box_bounds(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        Self {
            control_points: vec![
                ControlPoint { x: x0, y: y0 },
                ControlPoint {
                    x: (x0 + x1) / 2.0,
                    y: y0,
                },
                ControlPoint { x: x1, y: y0 },
                ControlPoint {
                    x: x0,
                    y: (y0 + y1) / 2.0,
                },
                //
                // anchor point
                //
                ControlPoint {
                    x: x1,
                    y: (y0 + y1) / 2.0,
                },
                ControlPoint { x: x0, y: y1 },
                ControlPoint {
                    x: (x0 + x1) / 2.0,
                    y: y1,
                },
                ControlPoint { x: x1, y: y1 },
            ],
            bounding_segments: vec![
                BoundingSegment {
                    x0: x0,
                    y0: y0,
                    x1: x1,
                    y1: y0,
                },
                BoundingSegment {
                    x0: x0,
                    y0: y0,
                    x1: x0,
                    y1: y1,
                },
                BoundingSegment {
                    x0: x1,
                    y0: y1,
                    x1: x1,
                    y1: y0,
                },
                BoundingSegment {
                    x0: x1,
                    y0: y1,
                    x1: x0,
                    y1: y1,
                },
            ],
            anchor_point: ControlPoint {
                x: (x0 + x1) / 2.0,
                y: (y0 + y1) / 2.0,
            },
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
