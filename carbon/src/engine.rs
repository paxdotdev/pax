use std::cell::RefCell;
use std::rc::Rc;

use kurbo::{
    BezPath,
    Point,
    Vec2,
};
use piet::RenderContext;
use piet_web::WebRenderContext;

use crate::{Affine, Color, Error, Evaluator, InjectionContext, PropertyValueExpression, PropertyValueLiteral, PropertyValueTimeline, RenderNodePtr, RenderTree, Size, SpreadDirection, SpreadProperties, Stroke, StrokeStyle, Transform, Text};
use crate::components::Spread;
use crate::primitives::component::Component;
use crate::rectangle::Rectangle;
use crate::rendering::Size2DFactory;
use crate::runtime::{PropertiesCoproduct, Runtime};
use crate::timeline::{EasingCurve, Timeline, TimelineSegment};
use std::collections::VecDeque;
use wasm_bindgen::JsValue;

// Public method for consumption by engine chassis, e.g. WebChassis
pub fn get_engine(logger: fn(&str), viewport_size: (f64, f64)) -> CarbonEngine {
    CarbonEngine::new(logger, viewport_size)
}

pub struct CarbonEngine {
    pub frames_elapsed: usize,
    pub render_tree: Rc<RefCell<RenderTree>>,
    pub runtime: Rc<RefCell<Runtime>>,
    viewport_size: (f64, f64),
}

#[derive(Clone)]
pub struct RenderTreeContext<'a>
{
    pub engine: &'a CarbonEngine,
    pub transform: Affine,
    pub bounds: (f64, f64),
    pub runtime: Rc<RefCell<Runtime>>,
    pub parent: RenderNodePtr,
    pub node: RenderNodePtr,
    pub timeline_playhead_position: usize,
}


pub struct HostPlatformContext<'a, 'b>
{
    pub drawing_context: &'a mut WebRenderContext<'b>,
    pub render_message_queue: Vec<JsValue>, //TODO: platform polyfill
    // pub serializer: Box<dyn serde::Serializer<>>,
}

pub struct DevAppRootProperties {
    //Here are the root app/component's "inputs" and properties
}

impl CarbonEngine {
    fn new(logger: fn(&str), viewport_size: (f64, f64)) -> Self {
        CarbonEngine {
            frames_elapsed: 0,
            runtime: Rc::new(RefCell::new(Runtime::new(logger))),
            render_tree: Rc::new(RefCell::new(RenderTree {
                root: Rc::new(RefCell::new(Component {
                    adoptees: Rc::new(RefCell::new(vec![])),//TODO: accept from outside application, e.g. from a React app or iOS app
                    properties: Rc::new(RefCell::new(
                        PropertiesCoproduct::DevAppRoot(Rc::new(RefCell::new(DevAppRootProperties{})))
                    )),
                    transform: Rc::new(RefCell::new(Transform::default())),
                    timeline: Some(Rc::new(RefCell::new(
                        Timeline {
                            frame_count: 300,
                            playhead_position: 0,
                            is_playing: true,
                        }
                    ))),
                    template: Rc::new(RefCell::new(vec![
                        Rc::new(RefCell::new(

                            //top spread

                            Spread::new(
                                Rc::new(RefCell::new(
                                    SpreadProperties {
                                        cell_count: Box::new(PropertyValueLiteral {value: 4}),
                                        gutter_width: Box::new(PropertyValueLiteral {value: Size::Pixel(15.0)}),
                                        ..Default::default()
                                    }
                                )),
                                Rc::new(RefCell::new(vec![
                                    //rainbow
                                    Rc::new(RefCell::new(
                                        Rectangle {
                                            transform: Rc::new(RefCell::new(Transform::default())),
                                            fill: Box::new(
                                                PropertyValueExpression {
                                                    cached_value: Color::hlc(0.0,0.0,0.0),
                                                    // expression!(|engine: &CarbonEngine| ->
                                                    evaluator: MyManualMacroExpression{variadic_evaluator: |engine: &CarbonEngine| -> Color {
                                                        Color::hlc((engine.frames_elapsed % 360) as f64, 75.0, 75.0)
                                                    }}
                                                }
                                            ),
                                            stroke: Stroke {
                                                width: 4.0,
                                                style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                                                color: Color::rgba(0.0, 0.0, 1.0, 1.0)
                                            },
                                            size: Size2DFactory::literal(Size::Percent(100.0), Size::Percent(100.0)),
                                        }
                                    )),
                                    //green
                                    Rc::new(RefCell::new(
                                        Rectangle {
                                            transform: Rc::new(RefCell::new(
                                                Transform {
                                                    translate: (
                                                        Box::new(
                                                           PropertyValueLiteral {value: 0.0}
                                                        ),
                                                        Box::new(
                                                            PropertyValueTimeline {
                                                                starting_value: Box::new(
                                                                    PropertyValueLiteral {value: 0.0}
                                                                ),
                                                                cached_evaluated_value: 0.0,
                                                                timeline_segments: vec![
                                                                    TimelineSegment {
                                                                        curve_in: EasingCurve::in_out_back(),
                                                                        ending_frame_inclusive: 74,
                                                                        ending_value: Box::new(
                                                                           PropertyValueLiteral {value: 500.0}
                                                                        ),
                                                                    },
                                                                    TimelineSegment {
                                                                        curve_in: EasingCurve::in_out_back(),
                                                                        ending_frame_inclusive: 149,
                                                                        ending_value: Box::new(
                                                                           PropertyValueLiteral {value: 0.0}
                                                                        ),
                                                                    },
                                                                    TimelineSegment {
                                                                        curve_in: EasingCurve::in_out_back(),
                                                                        ending_frame_inclusive: 224,
                                                                        ending_value: Box::new(
                                                                           PropertyValueLiteral {value: -500.0}
                                                                        ),
                                                                    },
                                                                    TimelineSegment {
                                                                        curve_in: EasingCurve::in_out_back(),
                                                                        ending_frame_inclusive: 299,
                                                                        ending_value: Box::new(
                                                                           PropertyValueLiteral {value: -0.0}
                                                                        ),
                                                                    },
                                                                ],
                                                           }
                                                       ),
                                                    ),
                                                    ..Default::default()
                                                }
                                            )),
                                            fill:  Box::new(
                                                PropertyValueLiteral {value: Color::rgba(0.0, 1.0, 0.0, 1.0) }
                                            ),
                                            stroke: Stroke {
                                                width: 4.0,
                                                style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                                                color: Color::rgba(0.0, 1.0, 1.0, 1.0)
                                            },
                                            size: Size2DFactory::literal(Size::Percent(100.0), Size::Percent(100.0)),
                                        }
                                    )),
                                    //text
                                    Rc::new(RefCell::new(
                                        Text {
                                            id: "a-thing-of-beauty".into(),
                                            content: Box::new(PropertyValueLiteral {value: "A thing of beauty...".to_string()}),
                                            transform: Rc::new(RefCell::new(Transform {
                                                // translate:
                                                // align: (
                                                //     Box::new(),
                                                //     Box::new(PropertyValueLiteral {value: 0.5}),
                                                // ),
                                                // origin: (                                                      Box::new(PropertyValueLiteral {value: 0.5}),
                                                //     Box::new(PropertyValueLiteral {value: Size::Percent(50.0)}),
                                                //
                                                //     Box::new()
                                                // ),
                                                ..Default::default()
                                            })),
                                            size: Size2DFactory::literal(Size::Percent(100.0), Size::Percent(100.0)),
                                        }
                                    )),

                                    // vertical spread

                                    Rc::new(RefCell::new(
                                        Spread::new(
                                            Rc::new(RefCell::new(
                                                SpreadProperties {
                                                    cell_count: Box::new(PropertyValueLiteral {value: 3}),
                                                    direction: SpreadDirection::Vertical,
                                                    gutter_width: Box::new(PropertyValueLiteral {value: Size::Pixel(15.0)}),
                                                    ..Default::default()
                                                }
                                            )),
                                            Rc::new(RefCell::new(vec![
                                                //rainbow
                                                Rc::new(RefCell::new(
                                                    Rectangle {
                                                        transform: Rc::new(RefCell::new(Transform::default())),
                                                        fill: Box::new(
                                                            PropertyValueExpression {
                                                                cached_value: Color::hlc(0.0,0.0,0.0),
                                                                // expression!(|engine: &CarbonEngine| ->
                                                                evaluator: MyManualMacroExpression{variadic_evaluator: |engine: &CarbonEngine| -> Color {
                                                                    Color::hlc(((engine.frames_elapsed + 180) % 360) as f64, 75.0, 75.0)
                                                                }}
                                                            }
                                                        ),
                                                        stroke: Stroke {
                                                            width: 4.0,
                                                            style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                                                            color: Color::rgba(0.0, 0.0, 1.0, 1.0)
                                                        },
                                                        size: Size2DFactory::literal(Size::Percent(100.0), Size::Percent(100.0)),
                                                    }
                                                )),
                                                //green
                                                Rc::new(RefCell::new(
                                                    Rectangle {
                                                        transform: Rc::new(RefCell::new(Transform::default())),
                                                        fill:  Box::new(
                                                            PropertyValueLiteral {value: Color::rgba(0.0, 1.0, 0.0, 1.0) }
                                                        ),
                                                        stroke: Stroke {
                                                            width: 4.0,
                                                            style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                                                            color: Color::rgba(0.0, 1.0, 1.0, 1.0)
                                                        },
                                                        size: Size2DFactory::literal(Size::Percent(100.0), Size::Percent(100.0)),
                                                    }
                                                )),
                                                //text
                                                Rc::new(RefCell::new(
                                                    Text {
                                                        id: "a-joy-forever".into(),
                                                        content: Box::new(PropertyValueLiteral {value: "...is a joy forever".to_string()}),
                                                        transform: Rc::new(RefCell::new(Transform { ..Default::default() })),
                                                        size: Size2DFactory::literal(Size::Percent(100.0), Size::Percent(100.0)),
                                                    }
                                                )),

                                            ])),


                                        )
                                    ))
                                ])),


                            )
                        )),

                        // // Our background fill
                        //i
                        // Rc::new(RefCell::new(
                        //     Rectangle {
                        //         transform: Rc::new(RefCell::new(Transform::default())),
                        //         fill:  Box::new(
                        //             PropertyLiteral {value: Color::rgba(0.5, 0.5, 0.5, 0.25) }
                        //         ),
                        //         stroke: Stroke {
                        //             width: 2.0,
                        //             style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                        //             color: Color::rgba(0.8, 0.8, 0.1, 1.0)
                        //         },
                        //         size: Size2DFactory::Literal(Size::Percent(100.0), Size::Percent(100.0)),
                        //     }
                        // )),


                    ])),

                })),
            })),
            viewport_size,
        }
    }

    fn traverse_render_tree(&self, rc: &mut WebRenderContext) -> Vec<JsValue> {
        // Broadly:
        // 1. compute properties
        // 2. find lowest node (last child of last node), accumulating transform along the way
        // 3. start rendering, from lowest node on-up

        let mut rtc = RenderTreeContext {
            engine: &self,
            transform: Affine::default(),
            bounds: self.viewport_size,
            runtime: self.runtime.clone(),
            node: Rc::clone(&self.render_tree.borrow().root),
            parent: Rc::clone(&self.render_tree.borrow().root),//TODO: refactor to Option<> ?
            timeline_playhead_position: self.frames_elapsed,
        };


        let mut hpc = HostPlatformContext {
            drawing_context: rc,
            render_message_queue: Vec::new(),
        };

        &self.recurse_traverse_render_tree(&mut rtc, &mut hpc, Rc::clone(&self.render_tree.borrow().root));
        // self.runtime.borrow_mut().log(&format!("{}",hpc.))
        hpc.render_message_queue
    }

    fn recurse_traverse_render_tree(&self, rtc: &mut RenderTreeContext, hpc: &mut HostPlatformContext, node: RenderNodePtr)  {
        // Recurse:
        //  - compute properties for this node
        //  - iterate backwards over children (lowest first); recurse until there are no more descendants.  track transform matrix & bounding dimensions along the way.
        //  - we now have the back-most leaf node.  Render it.  Return.
        //  - we're now at the second back-most leaf node.  Render it.  Return ...
        //  - done with this frame

        //populate a pointer to this (current) `RenderNode` onto `rtc`
        rtc.node = Rc::clone(&node);

        //peek at the current stack frame and set a scoped playhead position as needed
        match rtc.runtime.borrow_mut().peek_stack_frame() {
            Some(stack_frame) => {
                rtc.timeline_playhead_position = stack_frame.borrow().get_timeline_playhead_position();
            },
            None => ()
        }

        //lifecycle: init_and_calc happens before anything else and
        //           calculates
        //
        node.borrow_mut().compute_properties(rtc);
        let accumulated_transform = rtc.transform;
        let accumulated_bounds = rtc.bounds;

        //get the size of this node (calc'd or otherwise) and use
        //it as the new accumulated bounds: both for this nodes children (their parent container bounds)
        //and for this node itself (e.g. for specifying the size of a Rectangle node)
        let new_accumulated_bounds = node.borrow().get_size_calc(accumulated_bounds);

        let node_computed_transform = {
            let mut node_borrowed = rtc.node.borrow_mut();
            let node_size = node_borrowed.get_size_calc(accumulated_bounds);
            node_borrowed.get_transform().borrow_mut()
            .compute_matrix_in_place(
                node_size,
                accumulated_bounds,
            ).clone()
        };

        let new_accumulated_transform = accumulated_transform * node_computed_transform;

        rtc.bounds = new_accumulated_bounds;
        rtc.transform = new_accumulated_transform;

        //lifecycle: pre_render
        node.borrow_mut().pre_render(rtc, hpc);

        let children = node.borrow().get_rendering_children();

        //keep recursing through children
        children.borrow().iter().rev().for_each(|child| {
            //note that we're iterating starting from the last child, for z-index (.rev())
            let mut new_rtc = rtc.clone();
            &self.recurse_traverse_render_tree(&mut new_rtc, hpc, Rc::clone(child));
            //TODO: for dependency management, return computed values from subtree above
        });

        // lifecycle: `render`
        // this is this node's time to do its own rendering, aside
        // from its children.  Its children have already been rendered.
        node.borrow().render(rtc, hpc);

        // lifecycle: post_render
        node.borrow_mut().post_render(rtc, hpc);
    }

    pub fn set_viewport_size(&mut self, new_viewport_size: (f64, f64)) {
        self.viewport_size = new_viewport_size;
    }

    pub fn tick(&mut self, rc: &mut WebRenderContext) -> Vec<JsValue> {
        rc.clear(Color::rgb8(0, 0, 0));
        let render_queue = self.traverse_render_tree(rc);
        self.frames_elapsed = self.frames_elapsed + 1;
        render_queue
    }

    //keeping until this can be done via scene graph
    pub fn tick_and_render_disco_taps(&mut self, rc: &mut WebRenderContext) -> Result<(), Error> {
        let hue = (((self.frames_elapsed + 1) as f64 * 2.0) as i64 % 360) as f64;
        let current_color = Color::hlc(hue, 75.0, 127.0);
        rc.clear(current_color);

        for x in 0..20 {
            for y in 0..12 {
                let bp_width: f64 = 100.;
                let bp_height: f64 = 100.;
                let mut bez_path = BezPath::new();
                bez_path.move_to(Point::new(-bp_width / 2., -bp_height / 2.));
                bez_path.line_to(Point::new(bp_width / 2., -bp_height / 2.));
                bez_path.line_to(Point::new(bp_width / 2., bp_height / 2.));
                bez_path.line_to(Point::new(-bp_width / 2., bp_height / 2.));
                bez_path.line_to(Point::new(-bp_width / 2., -bp_height / 2.));
                bez_path.close_path();

                let theta = self.frames_elapsed as f64 * (0.04 + (x as f64 + y as f64 + 10.) / 64.) / 10.;
                let transform =
                    Affine::translate(Vec2::new(x as f64 * bp_width, y as f64 * bp_height)) *
                        Affine::rotate(theta) *
                        Affine::scale(theta.sin() * 1.2)
                    ;

                let transformed_bez_path = transform * bez_path;

                let phased_hue = ((hue + 180.) as i64 % 360) as f64;
                let phased_color = Color::hlc(phased_hue, 75., 127.);
                rc.fill(transformed_bez_path, &phased_color);
            }
        }

        self.frames_elapsed = self.frames_elapsed + 1;
        Ok(())
    }
}


/*****************************/
/* Codegen (macro) territory */

//OR: revisit this approach, without variadics.

pub struct MyManualMacroExpression<T> {
    pub variadic_evaluator: fn(engine: &CarbonEngine) -> T,
}

//TODO:  should this hard-code the return type
impl<T> MyManualMacroExpression<T> {

}

impl<T> Evaluator<T> for MyManualMacroExpression<T> {
    fn inject_and_evaluate(&self, ic: &InjectionContext) -> T {
        //TODO:CODEGEN
        //       pull necessary data from `ic`,
        //       map into the variadic args of self.variadic_evaluator()
        //       Perhaps this is a LUT of `String => (Fn(InjectionContext) -> V)` for any variadic type (injection stream) V
        let engine = ic.engine;
        (self.variadic_evaluator)(engine)
    }
}


/* End codegen (macro) territory */
/*********************************/