use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use kurbo::{
    BezPath,
    Point,
    Vec2,
};
use piet::RenderContext;
use piet_web::WebRenderContext;

use crate::{Affine, Color, Transform, Component, Error, PolymorphicType, PolymorphicValue, PropertyExpression, PropertyLiteral, PropertyTreeContext, Rectangle, RenderNodePtr, RenderNodePtrList, RenderTree, Runtime, Size, Stroke, StrokeStyle, Variable, VariableAccessLevel, Evaluator, InjectionContext, RenderNode, SpreadProperties, RepeatProperties, SpreadCellProperties};
use crate::components::Spread;
use crate::primitives::{Frame, Placeholder};
use crate::primitives::group::Group;
use std::marker::PhantomData;
use std::any::Any;
use std::mem::ManuallyDrop;

// use crate::primitives::{Frame};

// Public method for consumption by engine chassis, e.g. WebChassis
pub fn get_engine(logger: fn(&str), viewport_size: (f64, f64)) -> CarbonEngine {
    CarbonEngine::new(logger, viewport_size)
}

pub struct CarbonEngine {
    pub frames_elapsed: u32,
    pub render_tree: Rc<RefCell<RenderTree>>,
    pub runtime: Rc<RefCell<Runtime>>,
    viewport_size: (f64, f64),
}

pub struct RenderTreeContext<'a>
{
    pub transform: &'a Affine,
    pub bounding_dimens: (f64, f64),
    pub runtime: Rc<RefCell<Runtime>>,
    pub parent: RenderNodePtr,
    pub node: RenderNodePtr,
}



//
//
// pub union StackUnion<D> {
//     pub repeat_properties: ManuallyDrop<Rc<RepeatProperties<D>>>,
//     pub main_component_properties: ManuallyDrop<Rc<MyMainComponentProperties>>,
//     pub spread: ManuallyDrop<Rc<SpreadProperties>>,
// }
//
// impl<D> Drop for StackUnion<D> {
//     fn drop(&mut self) {
//         unsafe {
//             match self {
//                 StackUnion { repeat_properties } => {
//                     ManuallyDrop::drop(&mut self.repeat_properties);
//                 },
//                 StackUnion { main_component_properties } => {
//                     ManuallyDrop::drop(&mut self.main_component_properties);
//                 }
//                 StackUnion { spread } => {
//                     ManuallyDrop::drop(&mut self.spread);
//                 }
//             }
//         }
//     }
// }

/// `Scope` attaches to stack frames to provide an evaluation context + relevant data access
/// for features like Expressions.
/// The stored values that are DI'ed into expressions are held in these scopes,
/// e.g. `index` and `datum` for `Repeat`.

//TODO:  Scopes need to play nicely with variadic expressions.  We need to be
//       able to access `self` (current component) and its `properties` <P>
pub struct Scope {
    pub properties: Rc<PropertyCoproduct>,
    // TODO: children, parent, etc.
}

pub enum PropertyCoproduct {
    RepeatItem(Rc<RepeatItem>),
    Spread(Rc<SpreadProperties>),
    Empty,
}

pub struct RepeatItem {
    pub i: usize,
    pub property_coproduct: Rc<RepeatPropertyCoproduct>
}

pub enum RepeatPropertyCoproduct {
    SpreadCell(Rc<SpreadCellProperties>)
}

pub struct StackFrame
{
    adoptees: RenderNodePtrList,
    adoptee_index: usize,
    scope: Rc<RefCell<Scope>>,
}

impl StackFrame {
    pub fn new(adoptees: RenderNodePtrList, scope: Rc<RefCell<Scope>>) -> Self {
        StackFrame {
            adoptees: Rc::clone(&adoptees),
            adoptee_index: 0,
            scope,
        }
    }

    pub fn get_adoptees(&self) -> RenderNodePtrList {
        Rc::clone(&self.adoptees)
    }

    pub fn get_scope(&self) -> Rc<RefCell<Scope>> {
        Rc::clone(&self.scope)
    }

}



/*****************************/
/* Codegen (macro) territory */

struct MyManualMacroExpression<T> {
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

// TODO:  this LUT _has_ to happen in macro territory, because it's
//       inherently variadic.  We can't turn strings into logic outside of
//       the macro context, so we must *hand-write the InjectionContext -> Stream logic*
//       for the pre-macro Expressions v2 PoC
//       Node, an advantage of the LUT living in macro territory:  should avoid footprint bloat!
//       ** Note:  `match pattern {}` may be a better, Rustier approach than a HashMap "literal"
//
// struct InjectionMapperLUT {
//     function_map: HashMap<String, Fn(InjectionContext)>
// }
//
// impl InjectionMapperLUT {
//
// }
//

/* End codegen (macro) territory */
/*********************************/



pub struct MyMainComponentProperties {
    rotation: f64,
}

impl CarbonEngine {
    fn new(logger: fn(&str), viewport_size: (f64, f64)) -> Self {
        CarbonEngine {
            frames_elapsed: 0,
            runtime: Rc::new(RefCell::new(Runtime::new(logger))),
            render_tree: Rc::new(RefCell::new(RenderTree {
                root: Rc::new(RefCell::new(Component {
                    id: String::from("root"),
                    properties: Rc::new(
                        PropertyCoproduct::Empty
                        // StackUnion {main_component_properties: ManuallyDrop::new(Rc::new(MyMainComponentProperties { rotation: 0.44}))}
                    ),
                    transform: Transform::default(),
                    template: Rc::new(RefCell::new(vec![
                        Rc::new(RefCell::new(Frame {
                            id: String::from("frame_1"),
                            size: (Box::new(PropertyLiteral{value: Size::Pixel(550.0)}),Box::new(PropertyLiteral{value: Size::Pixel(400.0)})),
                            transform: Transform {
                                origin: (Box::new(PropertyLiteral{value: Size::Percent(50.0)}), Box::new(PropertyLiteral {value: Size::Percent(50.0)})),
                                align: (Box::new(PropertyLiteral { value: 0.5 }), Box::new(PropertyLiteral { value: 0.5 })),
                                ..Default::default()
                            },
                            children: Rc::new(RefCell::new(vec![
                                Rc::new(RefCell::new(
                                Rectangle {
                                    transform: Transform {
                                        origin: (Box::new(PropertyLiteral { value: Size::Percent(100.0)}), Box::new(PropertyLiteral {value: Size::Pixel(0.0) })),
                                        align: (Box::new(PropertyLiteral {value: 1.0}) , Box::new(PropertyLiteral { value: 0.0 })),
                                        ..Default::default()
                                    },

                                    fill: Box::new(
                                        PropertyExpression {
                                            cached_value: Color::hlc(0.0,0.0,0.0),
                                            dependencies: vec!["engine".to_string()],
                                            //TODO: expression!(|engine: &CarbonEngine| -> Color {})
                                            evaluator: MyManualMacroExpression{variadic_evaluator: |engine: &CarbonEngine| -> Color {
                                                Color::hlc((engine.frames_elapsed % 360) as f64, 75.0, 75.0)
                                            }}
                                        }
                                    ),
                                    stroke: Stroke {
                                        width: 4.0,
                                        style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                                        color: Color::rgba(1.0, 1.0, 0.0, 1.0)
                                    },
                                    id: String::from("frame_filler"),
                                    size: (Box::new(PropertyLiteral{value: Size::Percent(50.0)}),Box::new(PropertyLiteral{value: Size::Percent(100.0)})),
                                }
                            )),
                                Rc::new(RefCell::new(
                                    Spread::new(
                                        Rc::new(RefCell::new(vec![
                                            Rc::new(RefCell::new(
                                                Rectangle {
                                                    transform: Transform::default(),
                                                    fill: Box::new(
                                                        PropertyExpression {
                                                            cached_value: Color::hlc(0.0,0.0,0.0),
                                                            dependencies: vec!["engine".to_string()],
                                                            // expression!(|engine: &CarbonEngine| ->
                                                            evaluator: MyManualMacroExpression{variadic_evaluator: |engine: &CarbonEngine| -> Color {
                                                                Color::hlc((engine.frames_elapsed % 360) as f64, 75.0, 75.0)
                                                            }}
                                                        }
                                                    ),
                                                    stroke: Stroke {
                                                        width: 4.0,
                                                        style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                                                        color: Color::rgba(1.0, 1.0, 0.0, 1.0)
                                                    },
                                                    id: String::from("frame_filler"),
                                                    size: (Box::new(PropertyLiteral{value: Size::Percent(100.0)}),Box::new(PropertyLiteral{value: Size::Percent(100.0)})),
                                                }
                                            )),
                                            Rc::new(RefCell::new(
                                                Rectangle {
                                                    transform: Transform::default(),
                                                    fill:  Box::new(
                                                        PropertyLiteral {value: Color::rgba(1.0, 0.0, 0.0, 1.0) }
                                                    ),
                                                    stroke: Stroke {
                                                        width: 4.0,
                                                        style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                                                        color: Color::rgba(0.0, 1.0, 1.0, 1.0)
                                                    },
                                                    id: String::from("frame_filler"),
                                                    size: (Box::new(PropertyLiteral{value: Size::Percent(100.0)}),Box::new(PropertyLiteral{value: Size::Percent(100.0)})),
                                                }
                                            )),
                                        ])),
                                    String::from("my_first_spread"),
                                    (Box::new(PropertyLiteral{value: Size::Percent(100.0)}),Box::new(PropertyLiteral{value: Size::Percent(100.0)})),
                                    Default::default(),
                                    Rc::new(
                                        PropertyCoproduct::Spread(Rc::new(
                                            SpreadProperties {
                                                cell_size_spec: None,
                                                gutter: Size::Pixel(10.0),
                                            }
                                        ))
                                        // StackUnion { spread: ManuallyDrop::new(Rc::new(SpreadProperties {
                                        // gutter: Size::Pixel(10.0),
                                        // cell_size_spec: None,
                                        // }))}
                                    )

                                ))),

                                /////////

                                // Rc::new(RefCell::new(Rectangle {
                                //     id: String::from("rect_4"),
                                //     align: (0.5, 0.5),
                                //     origin: (Size::Percent(50.0), Size::Percent(50.0)),
                                //     size: (
                                //         Box::new(PropertyExpression {
                                //             last_value: Size::Pixel(100.0),
                                //             dependencies: vec![(String::from("engine.frames_elapsed"), PolymorphicType::Float)],
                                //             evaluator: (|dep_values: HashMap<String, PolymorphicValue>| -> Size<f64>  {
                                //                 unsafe {
                                //                     let frames_elapsed = dep_values.get("engine.frames_elapsed").unwrap().float;
                                //                     return Size::Pixel((frames_elapsed / 100.).sin() * 500.)
                                //                 }
                                //             })
                                //         }),
                                //         Box::new(PropertyExpression {
                                //             last_value: Size::Pixel(500.0),
                                //             dependencies: vec![(String::from("engine.frames_elapsed"), PolymorphicType::Float)],
                                //             evaluator: (|dep_values: HashMap<String, PolymorphicValue>| {
                                //                 unsafe {
                                //                     let frames_elapsed = dep_values.get("engine.frames_elapsed").unwrap().float;
                                //                     return Size::Pixel((frames_elapsed / 100.).sin() * 500.)
                                //                 }
                                //             })
                                //         })
                                //     ),
                                //     fill: Box::new(
                                //         PropertyExpression {
                                //             last_value: Color::hlc(0.0,0.0,0.0),
                                //             dependencies: vec![(String::from("engine.frames_elapsed"), PolymorphicType::Float)],
                                //             evaluator: (|dep_values: HashMap<String, PolymorphicValue>| -> Color {
                                //                 unsafe {
                                //                     let frames_elapsed = dep_values.get("engine.frames_elapsed").unwrap().float;
                                //                     return Color::hlc((((frames_elapsed / 500.) * 360.) as i64 % 360) as f64, 75.0, 127.0);
                                //                 }
                                //             })
                                //         }
                                //     ),
                                //     transform: Affine::default(),
                                //     stroke: Stroke {
                                //         color: Color::hlc(280.0, 75.0, 127.0),
                                //         width: 1.0,
                                //         style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                                //     },
                                // })),

                                ///////////////////////

                                // Rc::new(RefCell::new(Rectangle {
                                //     id: String::from("rect_6"),
                                //     align: (1.0, 0.5),
                                //     origin: (Size::Percent(100.0), Size::Percent(50.0)),
                                //     size: (
                                //         Box::new(PropertyLiteral { value: Size::Pixel(250.0) }),
                                //         Box::new(PropertyLiteral { value: Size::Percent(100.0) }),
                                //     ),
                                //     fill: Box::new(PropertyLiteral{value: Color::hlc(200.0, 75.0, 127.0)}),
                                //     transform: Affine::default(),
                                //     stroke: Stroke {
                                //         color: Color::hlc(0.0, 75.0, 127.0),
                                //         width: 1.0,
                                //         style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                                //     },
                                // })),

                                ///////////////////////////

                                // Rc::new(RefCell::new(Rectangle {
                                //     id: String::from("rect_5"),
                                //     align: (0.5, 0.5),
                                //     origin: (Size::Percent(0.0), Size::Percent(0.0),),
                                //     size: (
                                //         Box::new(PropertyExpression {
                                //             last_value: Size::Pixel(100.0),
                                //             dependencies: vec![(String::from("engine.frames_elapsed"), PolymorphicType::Float)],
                                //             evaluator: (|dep_values: HashMap<String, PolymorphicValue>| -> Size<f64>  {
                                //                 unsafe {
                                //                     let frames_elapsed = dep_values.get("engine.frames_elapsed").unwrap().float;
                                //                     return Size::Percent((frames_elapsed / 200.0).cos() * 100.0)
                                //                 }
                                //             })
                                //         }),
                                //         Box::new(PropertyExpression {
                                //             last_value: Size::Pixel(100.0),
                                //             dependencies: vec![(String::from("engine.frames_elapsed"), PolymorphicType::Float)],
                                //             evaluator: (|dep_values: HashMap<String, PolymorphicValue>| -> Size<f64>  {
                                //                 unsafe {
                                //                     let frames_elapsed = dep_values.get("engine.frames_elapsed").unwrap().float;
                                //                     return Size::Percent((frames_elapsed / 200.0).sin() * 100.0)
                                //                 }
                                //             })
                                //         })
                                //     ),
                                //     fill: Box::new(PropertyExpression {
                                //         last_value: Color::hlc(0.0,0.0,0.0),
                                //         dependencies: vec![(String::from("engine.frames_elapsed"), PolymorphicType::Float)],
                                //         evaluator: (|dep_values: HashMap<String, PolymorphicValue>| -> Color {
                                //             unsafe {
                                //                 let frames_elapsed = dep_values.get("engine.frames_elapsed").unwrap().float;
                                //                 return Color::hlc((((frames_elapsed / 250.) * 360.) as i64 % 360) as f64, 75.0, 127.0);
                                //             }
                                //         })
                                //     }),
                                //     transform: Affine::translate((0.0, 0.0)),
                                //     stroke: Stroke {
                                //         color: Color::hlc(0.0, 75.0, 127.0),
                                //         width: 1.0,
                                //         style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                                //     },
                                // })),

                                Rc::new(RefCell::new(
                                    Rectangle {
                                        transform: Transform::default(),
                                        fill:  Box::new(
                                            PropertyLiteral {value: Color::rgba(0.5, 0.5, 0.5, 1.0) }
                                        ),
                                        stroke: Stroke {
                                            width: 4.0,
                                            style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                                            color: Color::rgba(0.8, 0.8, 0.1, 1.0)
                                        },
                                        id: String::from("frame_filler"),
                                        size: (Box::new(PropertyLiteral{value: Size::Percent(100.0)}),Box::new(PropertyLiteral{value: Size::Percent(100.0)})),
                                    }
                                )),
                            ])),
                        })),
                    ])),
                })),
            })),
            viewport_size,
        }
    }

    fn render_render_tree(&self, rc: &mut WebRenderContext) {
        // Broadly:
        // 1. find lowest node (last child of last node), accumulating transform along the way
        // 2. start rendering, from lowest node on-up

        let mut rtc = RenderTreeContext {
            transform: &Affine::default(),
            bounding_dimens: self.viewport_size.clone(),
            runtime: self.runtime.clone(),
            node: Rc::clone(&self.render_tree.borrow().root),
            parent: Rc::clone(&self.render_tree.borrow().root),
        };
        self.recurse_render_render_tree(&mut rtc, rc, Rc::clone(&self.render_tree.borrow().root));
    }

    fn recurse_render_render_tree(&self, rtc: &mut RenderTreeContext, rc: &mut WebRenderContext, node: RenderNodePtr)  {
        // Recurse:
        //  - iterate backwards over children (lowest first); recurse until there are no more descendants.  track transform matrix & bounding dimensions along the way.
        //  - we now have the back-most leaf node.  Render it.  Return.
        //  - we're now at the second back-most leaf node.  Render it.  Return ...
        //  - done with this frame

        //populate a pointer to this (current) `RenderNode` onto `rtc`
        rtc.node = Rc::clone(&node);

        let accumulated_transform = rtc.transform;
        let accumulated_bounds = rtc.bounding_dimens;

        //Note: this cloning transform-fetching logic could certainly be written more efficiently

        let node_computed_transform = {
            let mut node_borrowed = rtc.node.borrow_mut();
            let node_size = node_borrowed.get_size_calc(accumulated_bounds);
            node_borrowed.get_transform_mut()
            .compute_transform_in_place(
                node_size,
                accumulated_bounds
            ).clone()
        };

        let new_accumulated_transform = *accumulated_transform * node_computed_transform;

        //get the size of this node (calc'd or otherwise) and use
        //it as the new accumulated bounds: both for this nodes children (their parent container bounds)
        //and for this node itself (e.g. for specifying the size of a Rectangle node)
        let new_accumulated_bounds = node.borrow().get_size_calc(accumulated_bounds);

        let mut new_rtc = RenderTreeContext {
            bounding_dimens: new_accumulated_bounds,
            transform: &new_accumulated_transform,
            runtime: Rc::clone(&rtc.runtime),
            parent: Rc::clone(&node),
            node: Rc::clone(&node),
        };

        //lifecycle: pre_render happens before traversing this node's children
        //           this is useful for pre-computation or for in-place mutations,
        //           e.g. `Placeholder`'s children/adoptee-switching logic
        //           and `Spread`'s layout-computing logic
        node.borrow_mut().pre_render(&mut new_rtc, rc);

        {
            let children = node.borrow().get_children();

            if children.borrow().len() > 0 {
                //keep recursing
                for i in (0..children.borrow().len()).rev() {
                    //note that we're iterating starting from the last child, for z-index
                    let children_borrowed = children.borrow();
                    let child = children_borrowed.get(i); //TODO: ?-syntax
                    match child {
                        None => { return },
                        Some(child) => {
                            &self.recurse_render_render_tree(&mut new_rtc, rc, Rc::clone(child));
                        }
                    }
                }
            }
        }

        // `render` lifecycle event:
        // this is this node's time to do its own rendering, aside
        // from its children.  Its children have already been rendered.
        node.borrow().render(&mut new_rtc, rc);

        //Lifecycle event: post_render can be used for cleanup, e.g. for
        //components to pop a stack frame
        node.borrow().post_render(&mut new_rtc, rc);
    }

    pub fn update_property_tree(&self) {
        // - traverse render tree
        // - update cache (current, `last_known_value`) for each property
        // - done

        //TODO:
        // - be smarter about updates, think "spreadsheet"
        //      - don't update values that don't need updating
        //      - traverse dependency graph, "distal"-inward
        //      - disallow circular deps
        // - make this and all `property` logic part of `Runtime`?
        let ctx = PropertyTreeContext {
            engine: &self,
            runtime: Rc::clone(&self.runtime),
        };

        &self.recurse_update_property_tree(&ctx,&mut self.render_tree.borrow_mut().root);
    }

    fn recurse_update_property_tree(&self, ctx: &PropertyTreeContext, node: &mut RenderNodePtr)  {
        // Recurse:
        //  - evaluate in a pre-order traversal, ensuring ancestors have been evaluated first
        //  - for each property, call eval_in_place(), which updates cache (read elsewhere in rendering logic)
        //  - done

        let mut node_borrowed = node.borrow_mut();
        let rnpl = node_borrowed.get_children(); //gnarly unboxing step to appease borrow-checker.  can we improve ergonomics here?
        let mut children_borrowed = rnpl.borrow_mut();

        node_borrowed.eval_properties_in_place(ctx);

        {

            //keep recursing as long as we have children
            for i in 0..children_borrowed.len() {
                //note that we're iterating starting from the last child
                let child = children_borrowed.get_mut(i); //TODO: ?-syntax
                match child {
                    None => { return },
                    Some(child) => {
                        &self.recurse_update_property_tree(ctx, child);
                    }
                }
            }
        }

        node_borrowed.post_eval_properties_in_place(ctx);
    }

    pub fn set_viewport_size(&mut self, new_viewport_size: (f64, f64)) {
        self.viewport_size = new_viewport_size;
    }

    pub fn tick(&mut self, rc: &mut WebRenderContext) {
        rc.clear(Color::rgb8(0, 0, 0));

        self.update_property_tree();
        self.render_render_tree(rc);
        self.frames_elapsed = self.frames_elapsed + 1;

        // Logging example:
        // self.log(format!("Frame: {}", self.frames_elapsed).as_str());

        // Draw a red box around viewport:
        // let mut outer_bounds = kurbo::Rect::new(0.0,0.0,self.viewport_size.0, self.viewport_size.1);
        // rc.stroke(outer_bounds, &piet::Color::rgba(1.0, 0.0, 0.0, 1.0), 5.0);
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