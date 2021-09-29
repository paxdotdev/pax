pub use piet_web::WebRenderContext;
pub use piet::{Error, Color};
use piet::RenderContext;

use kurbo::{Rect, Line, Point, Size, Affine};

pub struct CarbonEngine {
    // tick_and_render: fn(&mut Context) -> Result<(), Error>
    frames_elapsed: u32
}

pub struct Transform {
    translate: (f64, f64),
    scale: (f64, f64),
    rotate: f64 //z only for 2D
}

pub struct Stroke {
    width: f64,
    solid: Color,
}

pub struct Fill {
    solid: Color,
}

pub struct Node {
    children: Vec<Rectangle>
}

pub struct SceneGraph {
    root: Node
}

// base class for scene graph entities
pub struct Rectangle {
    width: f64,
    height: f64,
    stroke: Stroke,
    fill: Fill,
    transform: Transform
}

//TODO:  decorate with renderable trait
//TODO:  organize alongside other nodes in fs, modules
impl Rectangle {
    fn new(width: f64, height: f64, stroke: Stroke, fill: Fill, transform: Transform) -> Self {
        Rectangle {
            width,
            height,
            stroke,
            fill,
            transform,
        }
    }
    fn render(ctx: WebRenderContext) {
        // ctx.d
        // let
    }
}

pub fn get_engine() -> CarbonEngine {
    return CarbonEngine::new();
}

impl CarbonEngine {
    fn new() -> Self {
        let scene_graph = SceneGraph {
            root: Node {
                children: vec![
                    Rectangle {
                        width: 200.0,
                        height: 200.0,
                        stroke: Stroke {
                            width: 1.0,
                            solid: Color::rgb8(255, 0, 0),
                        },
                        fill: Fill {
                          solid: Color::rgb8(0,255,0),
                        },
                        transform: Transform {
                            scale: (1.0, 1.0),
                            translate: (0., 0.),
                            rotate: 0.,
                        }
                    },
                    Rectangle {
                        width: 100.0,
                        height: 100.0,
                        stroke: Stroke {
                            width: 1.0,
                            solid: Color::rgb8(255, 0, 255),
                        },
                        fill: Fill {
                            solid: Color::rgb8(255,255,0),
                        },
                        transform: Transform {
                            scale: (1.0, 1.0),
                            translate: (250., 250.),
                            rotate: 0.,
                        }
                    }
                ]
            }};
        CarbonEngine {
            frames_elapsed: 0
        }
    }

    fn render_scene_graph() -> Result<(), Error> {
        // hello world scene graph
        //           (root)
        //           /    \
        //       (rect)  (rect)


        Ok(())
    }

    pub fn tick_and_render (&mut self, rc: &mut WebRenderContext) -> Result<(), Error> {

        const speed : f64 = 4.0;

        let hue = (((self.frames_elapsed + 1) as f64 * speed) as i64 % 360) as f64;
        let current_color = Color::hlc(hue, 75.0, 127.0);
        rc.clear(current_color);

        let bounce = ((self.frames_elapsed as f64) / 50.).sin() * 200.;
        let rect = Rect::new(250., 250., 250. + bounce, 250. + bounce);

        // let transform = Affine::rotate(1.2)

        //TODO:  can we make a filled square with 4 cubic beziers?
        //       this would allow the use of kurbo::Affine for transforms

        let phased_hue = ((hue + 180.) as i64 % 360) as f64;
        let phased_color = Color::hlc(phased_hue, 75., 127.);
        rc.fill(rect, &phased_color);

        self.frames_elapsed = self.frames_elapsed + 1;
        Ok(())
    }
}

