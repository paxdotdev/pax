
use piet_web::{WebRenderContext};
use piet::{Color, StrokeStyle, RenderContext};
use kurbo::{Affine, BezPath};

use crate::{Variable, Property, PropertyTreeContext};

pub struct SceneGraph {
    pub root: Box<dyn RenderNode>
}

pub trait RenderNode
{
    fn eval_properties_in_place(&mut self, ctx: &PropertyTreeContext);
    fn get_align(&self) -> (f64, f64);
    fn get_children(&self) -> Option<&Vec<Box<dyn RenderNode>>>;
    fn get_children_mut(&mut self) -> Option<&mut Vec<Box<dyn RenderNode>>>;
    fn get_size(&self) -> Option<(Size<f64>, Size<f64>)>;

    /// Returns the size of this node in pixels, requiring
    /// parent bounds for calculation of `Percent` values
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64);

    fn get_id(&self) -> &str;
    fn get_origin(&self) -> (Size<f64>, Size<f64>);
    fn get_transform(&self) -> &Affine;
    fn pre_render(&self);
    fn render(&self, rc: &mut WebRenderContext, transform: &Affine, bounding_dimens: (f64, f64));
}

pub struct Group {
    pub children: Vec<Box<dyn RenderNode>>,
    pub id: String,
    pub align: (f64, f64),
    pub origin: (Size<f64>, Size<f64>),
    pub transform: Affine,
    pub variables: Vec<Variable>,
}

impl RenderNode for Group {
    fn eval_properties_in_place(&mut self, _: &PropertyTreeContext) {
        //TODO: handle each of Group's `Expressable` properties
    }

    fn get_align(&self) -> (f64, f64) { self.align }
    fn get_children(&self) -> Option<&Vec<Box<dyn RenderNode>>> {
        Some(&self.children)
    }
    fn get_children_mut(&mut self) -> Option<&mut Vec<Box<dyn RenderNode>>> { Some(&mut self.children) }
    fn get_size(&self) -> Option<(Size<f64>, Size<f64>)> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_id(&self) -> &str {
        &self.id.as_str()
    }
    fn get_origin(&self) -> (Size<f64>, Size<f64>) { self.origin }
    fn get_transform(&self) -> &Affine {
        &self.transform
    }
    fn pre_render(&self) {}
    fn render(&self, _: &mut WebRenderContext, _: &Affine, _: (f64, f64)) {}
}

pub struct Stroke {
    pub color: Color,
    pub width: f64,
    pub style: StrokeStyle,
}


#[derive(Copy, Clone)]
pub enum Size<T> {
    Pixel(T),
    Percent(T),
}

pub struct Rectangle {
    pub align: (f64, f64),
    pub size: (
        Box<dyn Property<Size<f64>>>,
        Box<dyn Property<Size<f64>>>,
    ),
    pub origin: (Size<f64>, Size<f64>),
    pub transform: Affine,
    pub stroke: Stroke,
    pub fill: Box<dyn Property<Color>>,
    pub id: String,
}


impl RenderNode for Rectangle {
    fn get_align(&self) -> (f64, f64) { self.align }
    fn get_children(&self) -> Option<&Vec<Box<dyn RenderNode>>> {
        None
    }
    fn get_children_mut(&mut self) -> Option<&mut Vec<Box<dyn RenderNode>>> {
        None
    }
    fn eval_properties_in_place(&mut self, ctx: &PropertyTreeContext) {
        self.size.0.eval_in_place(ctx);
        self.size.1.eval_in_place(ctx);
        self.fill.eval_in_place(ctx);
    }
    fn get_origin(&self) -> (Size<f64>, Size<f64>) { self.origin }
    fn get_size(&self) -> Option<(Size<f64>, Size<f64>)> { Some((*self.size.0.read(), *self.size.1.read())) }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) {
        let size_raw = self.get_size().unwrap();
        return (
            match size_raw.0 {
                Size::Pixel(width) => {
                    width
                },
                Size::Percent(width) => {
                    bounds.0 * (width / 100.0)
                }
            },
            match size_raw.1 {
                Size::Pixel(height) => {
                    height
                },
                Size::Percent(height) => {
                    bounds.1 * (height / 100.0)
                }
            }
        )
    }
    fn get_transform(&self) -> &Affine {
        &self.transform
    }
    fn get_id(&self) -> &str {
        &self.id.as_str()
    }
    fn pre_render(&self) {}
    fn render(&self, rc: &mut WebRenderContext, transform: &Affine, bounding_dimens: (f64, f64)) {

        let width: f64 =  bounding_dimens.0;
        let height: f64 =  bounding_dimens.1;

        let fill: &Color = &self.fill.read();

        let mut bez_path = BezPath::new();
        bez_path.move_to((0.0, 0.0));
        bez_path.line_to((width , 0.0));
        bez_path.line_to((width , height ));
        bez_path.line_to((0.0, height));
        bez_path.line_to((0.0,0.0));
        bez_path.close_path();

        let transformed_bez_path = *transform * bez_path;
        let duplicate_transformed_bez_path = transformed_bez_path.clone();

        rc.fill(transformed_bez_path, fill);
        rc.stroke(duplicate_transformed_bez_path, &self.stroke.color, self.stroke.width);
    }
}