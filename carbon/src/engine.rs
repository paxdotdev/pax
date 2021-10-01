use std::cell::{RefCell};

use kurbo::{
    BezPath,
    Point,
    Vec2,
};
use piet::RenderContext;
use piet_web::WebRenderContext;

use crate::{Affine, Color, Error, Group, PolymorphicValue, PropertyExpression, PropertyLiteral, Rectangle, RenderNode, SceneGraph, Stroke, StrokeStyle, Variable, PolymorphicType, PropertyTreeContext};




use std::collections::HashMap;



// Public method for consumption by engine chassis, e.g. WebChassis
pub fn get_engine(logger: fn(&str)) -> CarbonEngine {
    return CarbonEngine::new(logger);
}

pub struct CarbonEngine {
    pub logger: fn(&str),
    pub frames_elapsed: u32,
    pub scene_graph: RefCell<SceneGraph>,
}

impl CarbonEngine {
    fn new(logger: fn(&str)) -> Self {
        CarbonEngine {
            logger,
            frames_elapsed: 0,
            scene_graph: RefCell::new(SceneGraph {
                root: Box::new(Group {
                    id: String::from("root"),
                    variables: vec![
                        Variable {
                            name: String::from("rotation"),
                            value: PolymorphicValue { float: 1.2 },
                        },
                    ],
                    transform: Affine::default(),
                    children: vec![
                        Box::new(Group {
                            id: String::from("group_1"),
                            variables: vec![],
                            transform: Affine::translate(Vec2 { x: 800.0, y: -200.0 }),
                            children: vec![
                                Box::new(Rectangle {
                                    id: String::from("rect_4"),
                                    width: Box::new(PropertyExpression {
                                        last_value: 100.0,
                                        dependencies: vec![(String::from("engine.frames_elapsed"), PolymorphicType::Float)],
                                        evaluator: (|dep_values: HashMap<String, PolymorphicValue>| -> f64 {
                                            unsafe {
                                                let frames_elapsed = dep_values.get("engine.frames_elapsed").unwrap().float;
                                                return (frames_elapsed / 100.).sin() * 500.
                                            }
                                        })
                                    }),
                                    height: 50.0,
                                    fill: Color::hlc(120.0, 75.0, 127.0),
                                    transform: Affine::translate(Vec2 { x: 550.0, y: 550.0 }),
                                    stroke: Stroke {
                                        color: Color::hlc(280.0, 75.0, 127.0),
                                        width: 5.0,
                                        style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                                    },
                                }),
                                Box::new(Rectangle {
                                    id: String::from("rect_5"),
                                    width: Box::new(PropertyLiteral { value: 100.0 }),
                                    height: 100.0,
                                    fill: Color::hlc(160.0, 75.0, 127.0),
                                    transform: Affine::translate(Vec2 { x: 350.0, y: 350.0 }),
                                    stroke: Stroke {
                                        color: Color::hlc(320.0, 75.0, 127.0),
                                        width: 5.0,
                                        style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                                    },
                                }),
                                Box::new(Rectangle {
                                    id: String::from("rect_6"),
                                    width: Box::new(PropertyLiteral { value: 250.0 }),
                                    height: 250.0,
                                    fill: Color::hlc(200.0, 75.0, 127.0),
                                    transform: Affine::translate(Vec2 { x: 750.0, y: 750.0 }),
                                    stroke: Stroke {
                                        color: Color::hlc(0.0, 75.0, 127.0),
                                        width: 5.0,
                                        style: StrokeStyle { line_cap: None, dash: None, line_join: None, miter_limit: None },
                                    },
                                }),
                            ],
                        }),
                    ],
                }),
            }),
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
        self.recurse_render_scene_graph(rc, &self.scene_graph.borrow().root, &Affine::default());
    }

    fn recurse_render_scene_graph(&self, rc: &mut WebRenderContext, node: &Box<dyn RenderNode>, accumulated_transform: &Affine)  {
        // Recurse:
        //  - iterate backwards over children (lowest first); recurse until there are no more descendants.  track transform matrix along the way.
        //  - we now have the back-most leaf node.  Render it.  Return.
        //  - we're now at the second back-most leaf node.  Render it.  Return ...
        //  - done

        let new_accumulated_transform = *accumulated_transform * *node.get_transform();

        // TODO:
        //  - evaluate Expressions and update (or alias) relevant properties
        //  -

        match node.get_children() {
            Some(children) => {
                //keep recursing
                for i in (0..children.len()).rev() {
                    //note that we're iterating starting from the last child
                    let child = children.get(i); //TODO: ?-syntax
                    match child {
                        None => { return },
                        Some(child) => {
                            &self.recurse_render_scene_graph(rc, child, &new_accumulated_transform);
                        }
                    }
                }
            },
            None => {
                //this is a leaf node.  render it.
                let theta = (self.frames_elapsed as f64 / 50.0).sin() * 5.0 + (self.frames_elapsed as f64 / 50.0);
                let frame_rotated_transform = new_accumulated_transform * Affine::rotate(theta);
                node.render(rc, &frame_rotated_transform);
            }
        }

        //TODO: Now that children have been rendered, if there's rendering to be done at this node,
        //      (e.g. for layouts, perhaps virtual nodes like $repeat), do that rendering here

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

    fn recurse_update_property_tree(&self, ctx: &PropertyTreeContext, node: &mut Box<dyn RenderNode>)  {
        // Recurse:
        //  - iterate in a pre-order traversal, ensuring ancestors have been evaluated first
        //  - for each property, call eval_in_place(), which updates cache (read elsewhere in rendering logic)
        //  - done

        node.eval_properties_in_place(ctx);

        match &mut node.get_children_mut() {
            Some(children) => {
                //keep recursing
                for i in 0..children.len() {
                    //note that we're iterating starting from the last child
                    let child = children.get_mut(i); //TODO: ?-syntax
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

    pub fn set_viewport_size(&mut self, dimens: (f64, f64)) {
        // this affects the size of root
        //OR:  root can specify a concrete size (a la Flash stage)
        //     or dynamic size, e.g. fn(viewport size)
    }

    pub fn tick(&mut self, rc: &mut WebRenderContext) -> Result<(), Error> {
        rc.clear(Color::rgb8(0, 0, 0));

        self.update_property_tree();

        self.render_scene_graph(rc);
        self.frames_elapsed = self.frames_elapsed + 1;

        // Logging example:
        // self.log(format!("Frame: {}", self.frames_elapsed).as_str());

        Ok(())
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