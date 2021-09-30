use std::rc::Rc;

use kurbo::{Affine, BezPath, Point, Vec2};
pub use piet::{Color, Error};
use piet::RenderContext;
pub use piet_web::WebRenderContext;

//
// pub struct Stroke {
//     width: f64,
//     solid: Color,
// }
//
// pub struct Fill {
//     solid: Color,
// }
//
// pub struct Group {
//     transform: Affine,
//     children: Vec<Box<dyn RenderNode>>,
// }





//Simplify:
// - Traverse a non-polymorphic tree (everything a Node)
// - Then make polymorphic

// base class for scene graph entities
// pub struct Rectangle {
//     width: f64,
//     height: f64,
//     stroke: Stroke,
//     fill: Fill,
//     transform: Affine,
// }

// trait RenderNode
// {
//     fn get_children(self) -> Vec<Box<dyn RenderNode>>;
//     fn get_transform(self) -> Affine;
// }
//
// impl RenderNode for Group
// {
//     fn get_children(self) -> Vec<Box<dyn RenderNode>> {
//         self.children
//     }
//     fn get_transform(self) -> Affine {
//         self.transform
//     }
// }
//
// impl RenderNode for Rectangle
// {
//     fn get_children(self) -> Vec<Box<dyn RenderNode>> {
//         Vec::new()
//     }
//     fn get_transform(self) -> Affine {
//         self.transform
//     }
// }
//
// //TODO:  organize alongside other nodes in fs, modules
// impl Rectangle {
//     fn new(width: f64, height: f64, stroke: Stroke, fill: Fill, transform: Affine) -> Self {
//         Rectangle {
//             width,
//             height,
//             stroke,
//             fill,
//             transform, //TODO:  this should probably be SugaryTransform
//         }
//     }
//     fn render(ctx: WebRenderContext) {
//     }
// }


struct Node {
    children: Vec<Node>,
    transform: Affine,
}

impl Node {
    fn get_children(&self) -> Option<&Vec<Node>> {
        Some(&self.children)
    }
    fn get_transform(&self) -> &Affine {
        &self.transform
    }
}

// Public method for consumption by engine chassis, e.g. WebChassis
pub fn get_engine() -> CarbonEngine {
    return CarbonEngine::new();
}

pub struct SceneGraph {
    root: Node
}

pub struct CarbonEngine {
    frames_elapsed: u32,
    scene_graph: SceneGraph,
}



impl CarbonEngine {
    fn new() -> Self {
        CarbonEngine {
            frames_elapsed: 0,
            scene_graph: SceneGraph {
                root: Node {
                    transform: Affine::rotate(1.5),
                    children: vec![
                        Node {
                            transform: Affine::translate(Vec2 { x: 300.0, y: 300.0 }) * Affine::rotate(1.2),
                            children: Vec::new(),
                        },
                    ],
                },
            },
        }
    }

    fn render_scene_graph(&self) -> Result<(), Error> {
        // hello world scene graph
        //           (root)
        //           /    \
        //       (rect)  (rect)

        // 1. find lowest node (last child of last node), accumulating transform along the way
        // 2. start rendering, from lowest node on-up
        self.recurse_render_scene_graph(&self.scene_graph.root, &Affine::default());
        Ok(())
    }

    fn recurse_render_scene_graph(&self, node: &Node, accumulated_transform: &Affine) -> Result<(), Error> {
        // Recurse:
        //  - iterate backwards over children (lowest first); recurse until there are no more descendants.  track transform matrix along the way.
        //  - we now have the back-most leaf node.  Render it.  Return.
        //  - we're now at the second back-most leaf node.  Render it.  Return ...
        //  - done

        let new_accumulated_transform = *accumulated_transform * *node.get_transform();



        match node.get_children() {
            Some(children) => {
                //keep recursing
                for i in (0..children.len() - 1).rev() {
                    //note that we're iterating starting from the last child
                    let child = children.get(i); //TODO: ?-syntax
                    match child {
                        None => { panic!() },
                        Some(child) => {
                            &self.recurse_render_scene_graph(child, &new_accumulated_transform);
                            return Ok(());
                        }
                    }
                }
            },
            None => {
                //this is a leaf node.  render it.
                //TODO:  some nodes (e.g. layouts) can have children AND require rendering.
                //       also: some nodes are virtual, e.g. $repeat
            }
        }


        Ok(())
    }


    pub fn tick_and_render(&mut self, rc: &mut WebRenderContext) -> Result<(), Error> {
        rc.clear(Color::rgb8(255, 255, 0));

        self.render_scene_graph();

        Ok(())
    }


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

