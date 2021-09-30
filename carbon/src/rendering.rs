
use piet_web::{WebRenderContext};
use piet::{Color, StrokeStyle, RenderContext};
use kurbo::{Affine, BezPath, Point};

use crate::RenderNode;

pub struct Group {
    pub children: Vec<Box<dyn RenderNode>>,
    pub transform: Affine,
}

impl RenderNode for Group {
    fn get_children(&self) -> Option<&Vec<Box<dyn RenderNode>>> {
        Some(&self.children)
    }
    fn get_transform(&self) -> &Affine {
        &self.transform
    }
    fn render(&self, rc: &mut WebRenderContext, transform: &Affine) {}
}

pub struct Stroke {
    pub color: Color,
    pub style: StrokeStyle,
}

pub struct Rectangle {
    pub width: f64,
    pub height: f64,
    pub transform: Affine,
    pub stroke: Stroke,
}

impl RenderNode for Rectangle {
    fn get_children(&self) -> Option<&Vec<Box<dyn RenderNode>>> {
        None
    }
    fn get_transform(&self) -> &Affine {
        &self.transform
    }
    fn render(&self, rc: &mut WebRenderContext, transform: &Affine) {
        let bp_width: f64 = self.width;
        let bp_height: f64 = self.height;
        let mut bez_path = BezPath::new();
        bez_path.move_to(Point::new(-bp_width / 2., -bp_height / 2.));
        bez_path.line_to(Point::new(bp_width / 2., -bp_height / 2.));
        bez_path.line_to(Point::new(bp_width / 2., bp_height / 2.));
        bez_path.line_to(Point::new(-bp_width / 2., bp_height / 2.));
        bez_path.line_to(Point::new(-bp_width / 2., -bp_height / 2.));
        bez_path.close_path();

        let transformed_bez_path = *transform * bez_path;
        let duplicate_transformed_bez_path = transformed_bez_path.clone();

        let phased_color = Color::rgba(227., 225., 27., 0.25);
        rc.fill(transformed_bez_path, &phased_color);

        rc.stroke(duplicate_transformed_bez_path, &self.stroke.color, 3.0);
    }
}