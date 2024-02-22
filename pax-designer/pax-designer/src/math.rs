use pax_engine::math::{Point2, Space, Vector2};

use crate::model::math::coordinate_spaces::Glass;

// min (-1.0, -1.0) for top left
// max (1.0, 1.0) for bottom right
pub struct BoxPoint;

impl Space for BoxPoint {}

pub struct AxisAlignedBox<W = Glass> {
    min: Point2<W>,
    max: Point2<W>,
}

impl<W: Space> std::fmt::Debug for AxisAlignedBox<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AxisAlignedBox")
            .field("top_left", &self.min)
            .field("bottom_right", &self.max)
            .finish()
    }
}

impl<W: Space> AxisAlignedBox<W> {
    pub fn new(p1: Point2<W>, p2: Point2<W>) -> Self {
        let min = Point2::new(p1.x.min(p2.x), p1.y.min(p2.y));
        let max = Point2::new(p1.x.max(p2.x), p1.y.max(p2.y));
        Self { min, max }
    }

    pub fn top_left(&self) -> Point2<W> {
        self.min
    }

    pub fn bottom_right(&self) -> Point2<W> {
        self.max
    }

    pub fn width(&self) -> f64 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f64 {
        self.max.y - self.min.y
    }

    pub fn bounding_points(&self) -> [Point2<W>; 4] {
        [
            self.min,
            Point2::new(self.min.x, self.max.y),
            self.max,
            Point2::new(self.max.x, self.min.y),
        ]
    }

    pub fn lerp(&self, point: Point2<BoxPoint>) -> Point2<W> {
        debug_assert!(point.x >= -1.0 && point.x <= 1.0);
        debug_assert!(point.y >= -1.0 && point.y <= 1.0);
        let lerp_range = (point - Point2::new(-1.0, -1.0)) / 2.0;
        let x_pos = self.min.x + (self.max.x - self.min.x) * lerp_range.x;
        let y_pos = self.min.y + (self.max.y - self.min.y) * lerp_range.y;
        Point2::new(x_pos, y_pos)
    }
}
