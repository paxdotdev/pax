use std::{
    marker::PhantomData,
    ops::{Add, Sub},
};

use super::{vector::Vector2, Generic, Space};

#[derive(Copy, Clone, Default, Debug)]
pub struct Point2<W = Generic> {
    pub x: f64,
    pub y: f64,
    _panthom: PhantomData<W>,
}

impl<W: Space> Point2<W> {
    pub fn new(x: f64, y: f64) -> Self {
        Point2 {
            x,
            y,
            _panthom: PhantomData,
        }
    }

    pub fn to_vector(self) -> Vector2<W> {
        Vector2::new(self.x, self.y)
    }

    pub fn to_world<WNew: Space>(self) -> Point2<WNew> {
        Point2::new(self.x, self.y)
    }

    pub fn midpoint_towards(self, other: Self) -> Self {
        self.lerp_towards(other, 1.0 / 2.0)
    }

    pub fn lerp_towards(self, other: Self, l: f64) -> Self {
        let v = other - self;
        self + l * v
    }
}

impl<W: Space> Sub for Point2<W> {
    type Output = Vector2<W>;
    fn sub(self, rhs: Point2<W>) -> Self::Output {
        Self::Output::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<W: Space> Add<Vector2<W>> for Point2<W> {
    type Output = Point2<W>;
    fn add(self, rhs: Vector2<W>) -> Self::Output {
        Self::Output::new(self.x + rhs.x, self.y + rhs.y)
    }
}
impl<W: Space> Add<Point2<W>> for Vector2<W> {
    type Output = Point2<W>;
    fn add(self, rhs: Point2<W>) -> Self::Output {
        Self::Output::new(self.x + rhs.x, self.y + rhs.y)
    }
}
