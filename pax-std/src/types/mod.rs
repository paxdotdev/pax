pub mod text;

use crate::primitives::Path;
pub use kurbo::RoundedRectRadii;
use pax_engine::api::Numeric;
pub use pax_engine::api::Size;
use pax_engine::*;

#[pax]
pub struct StackerCell {
    pub x_px: f64,
    pub y_px: f64,
    pub width_px: f64,
    pub height_px: f64,
}

#[pax]
pub enum StackerDirection {
    #[default]
    Vertical,
    Horizontal,
}

#[pax]
pub enum SidebarDirection {
    Left,
    #[default]
    Right,
}

#[pax]
pub enum PathElement {
    #[default]
    Empty,
    Point(Size, Size),
    Line,
    Curve(Size, Size),
    Close,
}

impl PathElement {
    pub fn line() -> Self {
        Self::Line
    }
    pub fn close() -> Self {
        Self::Close
    }
    pub fn point(x: Size, y: Size) -> Self {
        Self::Point(x, y)
    }
    pub fn curve(x: Size, y: Size) -> Self {
        Self::Curve(x, y)
    }
}

#[pax]
pub struct LineSegmentData {
    pub start: Point,
    pub end: Point,
}

impl LineSegmentData {
    pub fn new(p1: Point, p2: Point) -> Self {
        Self { start: p1, end: p2 }
    }
}

#[pax]
pub struct CurveSegmentData {
    pub start: Point,
    pub handle: Point,
    pub end: Point,
}

#[pax]
#[derive(Copy)]
pub struct Point {
    pub x: Size,
    pub y: Size,
}

impl Point {
    pub fn new(x: Size, y: Size) -> Self {
        Self { x, y }
    }

    pub fn to_kurbo_point(self, bounds: (f64, f64)) -> kurbo::Point {
        let x = self.x.evaluate(bounds, api::Axis::X);
        let y = self.y.evaluate(bounds, api::Axis::Y);
        kurbo::Point { x, y }
    }
}

impl Path {
    pub fn start(x: Size, y: Size) -> Vec<PathElement> {
        let mut start: Vec<PathElement> = Vec::new();
        start.push(PathElement::Point(x, y));
        start
    }
    pub fn line_to(mut path: Vec<PathElement>, x: Size, y: Size) -> Vec<PathElement> {
        path.push(PathElement::Line);
        path.push(PathElement::Point(x, y));
        path
    }

    pub fn curve_to(
        mut path: Vec<PathElement>,
        h_x: Size,
        h_y: Size,
        x: Size,
        y: Size,
    ) -> Vec<PathElement> {
        path.push(PathElement::Curve(h_x, h_y));
        path.push(PathElement::Point(x, y));
        path
    }
}

#[pax]
pub struct RectangleCornerRadii {
    pub top_left: Property<Numeric>,
    pub top_right: Property<Numeric>,
    pub bottom_right: Property<Numeric>,
    pub bottom_left: Property<Numeric>,
}

impl Into<RoundedRectRadii> for &RectangleCornerRadii {
    fn into(self) -> RoundedRectRadii {
        RoundedRectRadii::new(
            self.top_left.get().to_float(),
            self.top_right.get().to_float(),
            self.bottom_right.get().to_float(),
            self.bottom_left.get().to_float(),
        )
    }
}

impl RectangleCornerRadii {
    pub fn radii(
        top_left: Numeric,
        top_right: Numeric,
        bottom_right: Numeric,
        bottom_left: Numeric,
    ) -> Self {
        RectangleCornerRadii {
            top_left: Property::new(top_left),
            top_right: Property::new(top_right),
            bottom_right: Property::new(bottom_right),
            bottom_left: Property::new(bottom_left),
        }
    }
}
