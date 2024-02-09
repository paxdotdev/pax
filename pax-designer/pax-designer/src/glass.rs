use pax_lang::api::*;
use pax_lang::*;
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
    pub show_selection_controls: Property<bool>,
    pub control_points: Property<Vec<ControlPoint>>,
    pub anchor_point: Property<ControlPoint>,
    pub selection_bounding_segments: Property<Vec<BoundingSegment>>,
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
        pax_lang::log("key down");
        //TODO: handle keydowns and pass into InputMapper
    }

    pub fn update_view(&mut self, ctx: &NodeContext) {
        model::with_app_state(|app_state| {
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
        Self {
            show_selection_controls: Box::new(PropertyLiteral::new(true)),
            control_points: Box::new(PropertyLiteral::new(vec![
                ControlPoint { x: 300.0, y: 100.0 },
                ControlPoint { x: 350.0, y: 100.0 },
                ControlPoint { x: 400.0, y: 100.0 },
                ControlPoint { x: 300.0, y: 150.0 },
                //
                // anchor point
                //
                ControlPoint { x: 400.0, y: 150.0 },
                ControlPoint { x: 300.0, y: 200.0 },
                ControlPoint { x: 350.0, y: 200.0 },
                ControlPoint { x: 400.0, y: 200.0 },
            ])),
            selection_bounding_segments: Box::new(PropertyLiteral::new(vec![
                BoundingSegment {
                    x0: 300.0,
                    y0: 100.0,
                    x1: 400.0,
                    y1: 100.0,
                },
                BoundingSegment {
                    x0: 400.0,
                    y0: 100.0,
                    x1: 400.0,
                    y1: 200.0,
                },
                BoundingSegment {
                    x0: 400.0,
                    y0: 200.0,
                    x1: 300.0,
                    y1: 200.0,
                },
                BoundingSegment {
                    x0: 300.0,
                    y0: 200.0,
                    x1: 300.0,
                    y1: 100.0,
                },
            ])),
            anchor_point: Box::new(PropertyLiteral::new(ControlPoint { x: 350.0, y: 150.0 })),
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
pub struct RectTool {
    pub x: Size,
    pub y: Size,
    pub width: Size,
    pub height: Size,
    pub fill: Color,
    pub stroke: Color,
}
