use std::cell::{RefCell};

use kurbo::{
    BezPath,
    Point,
    Vec2,
};
use piet::RenderContext;
use piet_web::WebRenderContext;

use crate::{Affine, Color, Error, Group, Size, PropertyExpression, PolymorphicValue, PropertyLiteral, Rectangle, SceneGraph, Stroke, StrokeStyle, Variable, PolymorphicType, PropertyTreeContext, Runtime, RenderNodePtr, RenderNodePtrList};




use std::collections::HashMap;
use std::rc::Rc;


// Public method for consumption by engine chassis, e.g. WebChassis
pub fn get_engine(logger: fn(&str), viewport_size: (f64, f64)) -> CarbonEngine {
    let engine = CarbonEngine::new(logger, viewport_size);
    engine
}

pub struct CarbonEngine {
    pub logger: fn(&str),
    pub frames_elapsed: u32,
    pub scene_graph: Rc<RefCell<SceneGraph>>,
    pub runtime: Rc<RefCell<Runtime>>,
    viewport_size: (f64, f64),
}


pub struct SceneGraphContext<'a>
{
    pub transform: &'a Affine,
    pub bounding_dimens: (f64, f64),
    pub runtime: Rc<RefCell<Runtime>>,
    pub parent: RenderNodePtr,
    pub node: RenderNodePtr,
}

pub struct StackFrame
{
    pub adoptees: Option<RenderNodePtrList>,
    //TODO: manage scope here for expressions, dynamic templating
}

impl CarbonEngine {
    fn new(logger: fn(&str), viewport_size: (f64, f64)) -> Self {
        CarbonEngine {
            logger,
            frames_elapsed: 0,
            runtime: Rc::new(RefCell::new(Runtime::new())),
            scene_graph: Rc::new(RefCell::new(SceneGraph {
                //TODO:  root is a Component (specifically: a Component definition — i.e. definition of a prefab,) not a Group
                //       - Components have locals/variables but Groups are just primitives
                //
                root: Rc::new(RefCell::new(Group {
                    id: String::from("root"),
                    align: (0.0, 0.0),
                    origin: (Size::Pixel(0.0), Size::Pixel(0.0),),
                    variables: vec![
                        Variable {
                            name: String::from("rotation"),
                            value: PolymorphicValue { float: 1.2 },
                        },
                    ],
                    transform: Affine::default(),
                    children: Rc::new(RefCell::new(vec![
                        Rc::new(RefCell::new(Group {
                            id: String::from("group_1"),
                            align: (0.0, 0.0),
                            origin: (Size::Pixel(0.0), Size::Pixel(0.0),),
                            transform: Affine::default(),
                            variables: Vec::new(),
                            children: Rc::new(RefCell::new(vec![
                                Rc::new(RefCell::new(Rectangle {
                                    id: String::from("rect_4"),
                                    align: (0.5, 0.5),
                                    origin: (Size::Percent(50.0), Size::Percent(50.0)),
                                    size: (
                                        Box::new(PropertyExpression {
                                            last_value: Size::Pixel(100.0),
                                            dependencies: vec![(String::from("engine.frames_elapsed"), PolymorphicType::Float)],
                                            evaluator: (|dep_values: HashMap<String, PolymorphicValue>| -> Size<f64>  {
                                                unsafe {
                                                    let frames_elapsed = dep_values.get("engine.frames_elapsed").unwrap().float;
                                                    return Size::Pixel((frames_elapsed / 100.).sin() * 500.)
                                                }
                                            })
                                        }),
                                        Box::new(PropertyExpression {
                                            last_value: Size::Pixel(500.0),
                                            dependencies: vec![(String::from("engine.frames_elapsed"), PolymorphicType::Float)],
                                            evaluator: (|dep_values: HashMap<String, PolymorphicValue>| {
                                                unsafe {
                                                    let frames_elapsed = dep_values.get("engine.frames_elapsed").unwrap().float;
                                                    return Size::Pixel((frames_elapsed / 100.).sin() * 500.)
                                                }
                                            })
                                        })
                                    ),
                                    fill: Box::new(
                                        PropertyExpression {
                                            last_value: Color::hlc(0.0,0.0,0.0),
                                            dependencies: vec![(String::from("engine.frames_elapsed"), PolymorphicType::Float)],
                                            evaluator: (|dep_values: HashMap<String, PolymorphicValue>| -> Color {
                                                unsafe {
                                                    let frames_elapsed = dep_values.get("engine.frames_elapsed").unwrap().float;
                                                    return Color::hlc((((frames_elapsed / 500.) * 360.) as i64 % 360) as f64, 75.0, 127.0);
                                                }
                                            })
                                        }
                                    ),
                                    transform: Affine::default(),
                                    stroke: Stroke {
                                        color: Color::hlc(280.0, 75.0, 127.0),
                                        width: 1.0,
                                        style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                                    },
                                })),

                                ///////////////////////

                                Rc::new(RefCell::new(Rectangle {
                                    id: String::from("rect_6"),
                                    align: (1.0, 0.5),
                                    origin: (Size::Percent(100.0), Size::Percent(50.0)),
                                    size: (
                                        Box::new(PropertyLiteral { value: Size::Pixel(250.0) }),
                                        Box::new(PropertyLiteral { value: Size::Percent(100.0) }),
                                    ),
                                    fill: Box::new(PropertyLiteral{value: Color::hlc(200.0, 75.0, 127.0)}),
                                    transform: Affine::default(),
                                    stroke: Stroke {
                                        color: Color::hlc(0.0, 75.0, 127.0),
                                        width: 1.0,
                                        style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                                    },
                                })),

                                ///////////////////////////

                                Rc::new(RefCell::new(Rectangle {
                                    id: String::from("rect_5"),
                                    align: (0.5, 0.5),
                                    origin: (Size::Percent(0.0), Size::Percent(0.0),),
                                    size: (
                                        Box::new(PropertyExpression {
                                            last_value: Size::Pixel(100.0),
                                            dependencies: vec![(String::from("engine.frames_elapsed"), PolymorphicType::Float)],
                                            evaluator: (|dep_values: HashMap<String, PolymorphicValue>| -> Size<f64>  {
                                                unsafe {
                                                    let frames_elapsed = dep_values.get("engine.frames_elapsed").unwrap().float;
                                                    return Size::Percent((frames_elapsed / 200.0).cos() * 100.0)
                                                }
                                            })
                                        }),
                                        Box::new(PropertyExpression {
                                            last_value: Size::Pixel(100.0),
                                            dependencies: vec![(String::from("engine.frames_elapsed"), PolymorphicType::Float)],
                                            evaluator: (|dep_values: HashMap<String, PolymorphicValue>| -> Size<f64>  {
                                                unsafe {
                                                    let frames_elapsed = dep_values.get("engine.frames_elapsed").unwrap().float;
                                                    return Size::Percent((frames_elapsed / 200.0).sin() * 100.0)
                                                }
                                            })
                                        })
                                    ),
                                    fill: Box::new(PropertyExpression {
                                        last_value: Color::hlc(0.0,0.0,0.0),
                                        dependencies: vec![(String::from("engine.frames_elapsed"), PolymorphicType::Float)],
                                        evaluator: (|dep_values: HashMap<String, PolymorphicValue>| -> Color {
                                            unsafe {
                                                let frames_elapsed = dep_values.get("engine.frames_elapsed").unwrap().float;
                                                return Color::hlc((((frames_elapsed / 250.) * 360.) as i64 % 360) as f64, 75.0, 127.0);
                                            }
                                        })
                                    }),
                                    transform: Affine::translate((0.0, 0.0)),
                                    stroke: Stroke {
                                        color: Color::hlc(0.0, 75.0, 127.0),
                                        width: 1.0,
                                        style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                                    },
                                })),
                            ])),
                        })),
                    ])),
                })),
            })),
            viewport_size,
        }
    }

    #[allow(dead_code)]
    fn log(&self, msg: &str) {
        (self.logger)(msg);
    }

    fn render_scene_graph(&self, rc: &mut WebRenderContext) {
        // hello world scene graph
        //           (root)
        //           /    \
        //       (rect)  (rect)

        // 1. find lowest node (last child of last node), accumulating transform along the way
        // 2. start rendering, from lowest node on-up

        // let mut call_stack = Vec::new();
        let mut scene_graph_context = SceneGraphContext {
            transform: &Affine::default(),
            bounding_dimens: self.viewport_size.clone(),
            runtime: self.runtime.clone(),
            node: Rc::clone(&self.scene_graph.borrow().root),
            parent: Rc::clone(&self.scene_graph.borrow().root),
        };
        self.recurse_render_scene_graph(&mut scene_graph_context, rc, Rc::clone(&self.scene_graph.borrow().root));
    }

    fn recurse_render_scene_graph(&self, sc: &mut SceneGraphContext, rc: &mut WebRenderContext, node: RenderNodePtr)  {

        //populate a pointer to this (current) `RenderNode` onto `sc`
        sc.node = Rc::clone(&node);

        let accumulated_transform = sc.transform;
        let accumulated_bounds = sc.bounding_dimens;
        // Recurse:
        //  - iterate backwards over children (lowest first); recurse until there are no more descendants.  track transform matrix & bounding dimensions along the way.
        //  - we now have the back-most leaf node.  Render it.  Return.
        //  - we're now at the second back-most leaf node.  Render it.  Return ...
        //  - done

        //lifecycle: pre_render happens before anything else for this node
        //           this is useful for pre-calculation, e.g. for layout
        node.borrow_mut().pre_render(sc);

        let node_size_calc = node.borrow().get_size_calc(accumulated_bounds);
        let origin_transform = Affine::translate(
        (
                match node.borrow().get_origin().0 {
                    Size::Pixel(x) => { -x },
                    Size::Percent(x) => {
                        -node_size_calc.0 * (x / 100.0)
                    },
                },
                match node.borrow().get_origin().1 {
                    Size::Pixel(y) => { -y },
                    Size::Percent(y) => {
                        -node_size_calc.1 * (y / 100.0)
                    },
                }
            )
        );

        let align_transform = Affine::translate((node.borrow().get_align().0 * accumulated_bounds.0, node.borrow().get_align().1 * accumulated_bounds.1));
        let new_accumulated_transform = *accumulated_transform * align_transform * origin_transform * *node.borrow().get_transform();

        //default to our parent-provided bounding dimensions
        let new_accumulated_bounds = node.borrow().get_size_calc(accumulated_bounds);

        match node.borrow().get_children() {
            Some(children) => {
                //keep recursing
                for i in (0..children.borrow().len()).rev() {
                    //note that we're iterating starting from the last child, for z-index
                    let children_borrowed = children.borrow();
                    let child = children_borrowed.get(i); //TODO: ?-syntax
                    match child {
                        None => { return },
                        Some(child) => {
                            let mut new_scene_graph_context = SceneGraphContext {
                                transform: &new_accumulated_transform,
                                bounding_dimens: new_accumulated_bounds,
                                runtime: Rc::clone(&sc.runtime),
                                parent: Rc::clone(&node),
                                node: Rc::clone(&node),
                            };
                            &self.recurse_render_scene_graph(&mut new_scene_graph_context, rc, Rc::clone(child));
                        }
                    }
                }
            },
            None => ()  // this is a leaf node — no special treatment required
                        // since it will render itself later in this method
        }
        let mut new_scene_graph_context = SceneGraphContext {
            bounding_dimens: new_accumulated_bounds,
            transform: &new_accumulated_transform,
            runtime: Rc::clone(&sc.runtime),
            parent: Rc::clone(&node),
            node: Rc::clone(&node),
        };
        node.borrow().render(&mut new_scene_graph_context, rc);

        //Lifecycle event: post_render can be used for cleanup, e.g. for
        //components to pop a stack frame
        node.borrow().post_render(&mut new_scene_graph_context);
    }

    pub fn update_property_tree(&self) {
        // - traverse scene graph
        // - update cache (current, `last_known_value`) for each property
        // - done

        //TODO:
        // - be smarter about updates, think "spreadsheet"
        //      - don't update values that don't need updating
        //      - traverse dependency graph, "distal"-inward
        //      - disallow circular deps
        let ctx = PropertyTreeContext {
            engine: &self,
        };

        &self.recurse_update_property_tree(&ctx,&mut self.scene_graph.borrow_mut().root);
    }

    fn recurse_update_property_tree(&self, ctx: &PropertyTreeContext, node: &mut RenderNodePtr)  {
        // Recurse:
        //  - iterate in a pre-order traversal, ensuring ancestors have been evaluated first
        //  - for each property, call eval_in_place(), which updates cache (read elsewhere in rendering logic)
        //  - done

        node.borrow_mut().eval_properties_in_place(ctx);

        match &mut node.borrow_mut().get_children() {
            Some(children) => {
                //keep recursing
                for i in 0..children.borrow().len() {
                    //note that we're iterating starting from the last child
                    let mut children_borrowed = children.borrow_mut();
                    let child = children_borrowed.get_mut(i); //TODO: ?-syntax
                    match child {
                        None => { return },
                        Some(child) => {
                            &self.recurse_update_property_tree(ctx, child);
                        }
                    }
                }
            },
            None => {
                //this is a leaf node.  we're done with this branch.
            }
        }
    }

    pub fn set_viewport_size(&mut self, new_viewport_size: (f64, f64)) {
        self.viewport_size = new_viewport_size;
    }

    pub fn tick(&mut self, rc: &mut WebRenderContext) {
        rc.clear(Color::rgb8(0, 0, 0));

        self.update_property_tree();

        self.render_scene_graph(rc);
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