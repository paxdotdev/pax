use pax_engine::{layout::TransformAndBounds, NodeLocal, Property};
use pax_runtime_api::Size;
use pax_std::types::PathElement;

use crate::{math::coordinate_spaces::Glass, model::action::RaycastMode};

pub struct PathOutline {}

impl PathOutline {
    pub fn from_bounds(t_and_b: TransformAndBounds<NodeLocal, Glass>) -> Vec<PathElement> {
        let (o, u, v) = t_and_b.transform.decompose();
        let u = u * t_and_b.bounds.0;
        let v = v * t_and_b.bounds.1;
        let [p1, p4, p3, p2] = [o, o + v, o + u + v, o + u];
        vec![
            PathElement::point(Size::Pixels(p1.x.into()), Size::Pixels(p1.y.into())),
            PathElement::line(),
            PathElement::point(Size::Pixels(p2.x.into()), Size::Pixels(p2.y.into())),
            PathElement::line(),
            PathElement::point(Size::Pixels(p3.x.into()), Size::Pixels(p3.y.into())),
            PathElement::line(),
            PathElement::point(Size::Pixels(p4.x.into()), Size::Pixels(p4.y.into())),
            PathElement::close(),
        ]
    }
}
