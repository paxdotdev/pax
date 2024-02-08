use pax_designtime::input::{FSMEvent, ScreenspacePoint};
use pax_lang::api::*;
use pax_lang::*;
use pax_std::primitives::{Group, Path, Rectangle};
use pax_std::types::{Color, Fill};
use serde::Deserialize;

#[pax]
#[custom(Default)]
#[file("glass.pax")]
pub struct Glass {
    pub show_selection_controls: Property<bool>,
    pub control_points: Property<Vec<ControlPoint>>,
    pub anchor_point: Property<ControlPoint>,
    pub selection_bounding_segments: Property<Vec<BoundingSegment>>,
}

impl Glass {
    pub fn handle_mouse_down(&mut self, ctx: &NodeContext, args: ArgsMouseDown) {
        let res = ctx
            .designtime
            .borrow_mut()
            .input_transition(FSMEvent::MouseDown(ScreenspacePoint {
                x: args.mouse.x,
                y: args.mouse.y,
            }));
        log(&format!("input result: {:?}", res));
    }

    pub fn handle_mouse_move(&mut self, ctx: &NodeContext, args: ArgsMouseMove) {
        let res = ctx
            .designtime
            .borrow_mut()
            .input_transition(FSMEvent::MouseMove(ScreenspacePoint {
                x: args.mouse.x,
                y: args.mouse.y,
            }));
        log(&format!("input result: {:?}", res));
    }

    pub fn handle_mouse_up(&mut self, ctx: &NodeContext, args: ArgsMouseUp) {
        let res = ctx
            .designtime
            .borrow_mut()
            .input_transition(FSMEvent::MouseUp(ScreenspacePoint {
                x: args.mouse.x,
                y: args.mouse.y,
            }));
        log(&format!("input result: {:?}", res));
    }

    pub fn handle_key_down(&mut self, ctx: &NodeContext, args: ArgsKeyDown) {
        pax_lang::log("key down");
        //TODO: handle keydowns and pass into InputMapper
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
        }
    }
}

#[pax]
#[custom(Imports)]
pub struct ControlPoint {
    pub x: f64,
    pub y: f64,
}

#[pax]
#[custom(Imports)]
pub struct BoundingSegment {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}
