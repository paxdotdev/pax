
use piet_web::{WebRenderContext};
use piet::{Color, StrokeStyle, RenderContext};
use kurbo::{Affine, BezPath, Point};

use crate::{Variable};

pub struct SceneGraph {
    pub root: Box<dyn RenderNode>
}

pub trait RenderNode
{
    fn get_children(&self) -> Option<&Vec<Box<dyn RenderNode>>>;
    fn get_transform(&self) -> &Affine;
    fn get_id(&self) -> &str;
    fn render(&self, rc: &mut WebRenderContext, transform: &Affine);
}


pub struct Group {
    pub children: Vec<Box<dyn RenderNode>>,
    pub transform: Affine,
    pub variables: Vec<Variable>,
    pub id: String,
}

impl RenderNode for Group {
    fn get_children(&self) -> Option<&Vec<Box<dyn RenderNode>>> {
        Some(&self.children)
    }
    fn get_transform(&self) -> &Affine {
        &self.transform
    }
    fn get_id(&self) -> &str {
        &self.id.as_str()
    }
    fn render(&self, _: &mut WebRenderContext, _: &Affine) {}
}

pub struct Stroke {
    pub color: Color,
    pub width: f64,
    pub style: StrokeStyle,
}

pub struct Rectangle {
    pub width: f64,
    pub height: f64,
    pub transform: Affine,
    pub stroke: Stroke,
    pub fill: Color,
    pub id: String,
}

impl RenderNode for Rectangle {
    fn get_children(&self) -> Option<&Vec<Box<dyn RenderNode>>> {
        None
    }
    fn get_transform(&self) -> &Affine {
        &self.transform
    }
    fn get_id(&self) -> &str {
        &self.id.as_str()
    }
    fn render(&self, rc: &mut WebRenderContext, transform: &Affine) {
        let bp_width: f64 = self.width;
        let bp_height: f64 = self.height;
        let mut bez_path = BezPath::new();

        //TODO:  support dynamic Origin
        bez_path.move_to(Point::new(-bp_width / 2., -bp_height / 2.));
        bez_path.line_to(Point::new(bp_width / 2., -bp_height / 2.));
        bez_path.line_to(Point::new(bp_width / 2., bp_height / 2.));
        bez_path.line_to(Point::new(-bp_width / 2., bp_height / 2.));
        bez_path.line_to(Point::new(-bp_width / 2., -bp_height / 2.));
        bez_path.close_path();

        let transformed_bez_path = *transform * bez_path;
        let duplicate_transformed_bez_path = transformed_bez_path.clone();

        rc.fill(transformed_bez_path, &self.fill);
        rc.stroke(duplicate_transformed_bez_path, &self.stroke.color, self.stroke.width);
    }
}