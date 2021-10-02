
use piet_web::{WebRenderContext};
use piet::{Color, StrokeStyle, RenderContext};
use kurbo::{Affine, BezPath, Point};

use crate::{Variable, Property, PropertyTreeContext};

pub struct SceneGraph {
    pub root: Box<dyn RenderNode>
}

pub trait RenderNode
{
    fn eval_properties_in_place(&mut self, ctx: &PropertyTreeContext);
    fn get_children(&self) -> Option<&Vec<Box<dyn RenderNode>>>;
    fn get_children_mut(&mut self) -> Option<&mut Vec<Box<dyn RenderNode>>>;
    fn get_dimensions(&self) -> (Option<f64>, Option<f64>);
    fn get_id(&self) -> &str;
    fn get_transform(&self) -> &Affine;
    fn render(&self, rc: &mut WebRenderContext, transform: &Affine, bounding_dimens: (f64, f64));
}

pub struct Group {
    pub children: Vec<Box<dyn RenderNode>>,
    pub height: Option<f64>,
    pub id: String,
    pub transform: Affine,
    pub variables: Vec<Variable>,
    pub width: Option<f64>,
}

impl RenderNode for Group {
    fn eval_properties_in_place(&mut self, ctx: &PropertyTreeContext) {
        //TODO: handle each of Group's `Expressable` properties
    }

    fn get_children(&self) -> Option<&Vec<Box<dyn RenderNode>>> {
        Some(&self.children)
    }
    fn get_children_mut(&mut self) -> Option<&mut Vec<Box<dyn RenderNode>>> { Some(&mut self.children) }
    fn get_dimensions(&self) -> (Option<f64>, Option<f64>) { (self.width, self.height) }
    fn get_id(&self) -> &str {
        &self.id.as_str()
    }
    fn get_transform(&self) -> &Affine {
        &self.transform
    }

    fn render(&self, _: &mut WebRenderContext, _: &Affine, bounding_dimens: (f64, f64)) {}
}

pub struct Stroke {
    pub color: Color,
    pub width: f64,
    pub style: StrokeStyle,
}

pub struct Rectangle {
    pub width: Box<dyn Property<Option<f64>>>,
    pub height: Box<dyn Property<Option<f64>>>,
    pub transform: Affine,
    pub stroke: Stroke,
    pub fill: Box<dyn Property<Color>>,
    pub id: String,
}





/*

TODO: back out the `None`-sizing logic
Remove concept of `None` sizing
Instead, support %, and make explicit width=100% the only way
    to fill a container.  Implications to consider:  how is the
    width

 */





impl RenderNode for Rectangle {
    fn get_children(&self) -> Option<&Vec<Box<dyn RenderNode>>> {
        None
    }
    fn get_children_mut(&mut self) -> Option<&mut Vec<Box<dyn RenderNode>>> {
        None
    }
    fn eval_properties_in_place(&mut self, ctx: &PropertyTreeContext) {
        self.width.eval_in_place(ctx);
        self.height.eval_in_place(ctx);
        self.fill.eval_in_place(ctx);
    }
    fn get_dimensions(&self) -> (Option<f64>, Option<f64>) { (*self.width.read(), *self.height.read()) }
    fn get_transform(&self) -> &Affine {
        &self.transform
    }
    fn get_id(&self) -> &str {
        &self.id.as_str()
    }
    fn render(&self, rc: &mut WebRenderContext, transform: &Affine, bounding_dimens: (f64, f64)) {
        //TODO:
        //  for each property that's used here (e.g. self.width and self.height)
        //  unbox the Value vs Expression and pack into a local for eval here

        let width: f64 = match *self.width.read() {
            Some(val) => val,
            None => bounding_dimens.0
        };
        let height: f64 = match *self.height.read() {
            Some(val) => val,
            None => bounding_dimens.1
        };
        let fill: &Color = &self.fill.read();

        let mut bez_path = BezPath::new();

        //TODO:  support dynamic Origin
        bez_path.move_to(Point::new(-width / 2., -height / 2.));
        bez_path.line_to(Point::new(width / 2., -height / 2.));
        bez_path.line_to(Point::new(width / 2., height / 2.));
        bez_path.line_to(Point::new(-width / 2., height / 2.));
        bez_path.line_to(Point::new(-width / 2., -height / 2.));
        bez_path.close_path();

        let transformed_bez_path = *transform * bez_path;
        let duplicate_transformed_bez_path = transformed_bez_path.clone();

        rc.fill(transformed_bez_path, fill);
        rc.stroke(duplicate_transformed_bez_path, &self.stroke.color, self.stroke.width);
    }
}