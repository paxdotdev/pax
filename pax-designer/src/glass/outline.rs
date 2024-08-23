use pax_engine::api::Size;
use pax_engine::{node_layout::TransformAndBounds, NodeLocal, Property};
use pax_std::drawing::path::PathElement;

use crate::{math::coordinate_spaces::Glass, model::action::RaycastMode};

pub struct PathOutline {}

impl PathOutline {
    pub fn from_bounds(t_and_b: TransformAndBounds<NodeLocal, Glass>) -> Vec<PathElement> {
        let [p1, p4, p3, p2] = t_and_b.corners();
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
