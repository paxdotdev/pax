use std::{
    marker::PhantomData,
    ops::{Add, Sub},
};

use crate::Interpolatable;

use super::{vector::Vector2, Generic, Space};

impl<W: Space> Interpolatable for Point2<W> {}

pub struct Point2<W = Generic> {
    pub x: f64,
    pub y: f64,
    _panthom: PhantomData<W>,
}

// Implement Clone, Copy, PartialEq, etc manually, as
// to not require the Space to implement these.

impl<W: Space> std::fmt::Debug for Point2<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({} {})", self.x, self.y)
    }
}

impl<W: Space> Clone for Point2<W> {
    fn clone(&self) -> Self {
        Self {
            x: self.x,
            y: self.y,
            _panthom: PhantomData,
        }
    }
}

impl<W: Space> Copy for Point2<W> {}

impl<W: Space> PartialEq for Point2<W> {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl<W: Space> Default for Point2<W> {
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
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

    pub fn cast_space<WNew: Space>(self) -> Point2<WNew> {
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

impl<W: Space> Sub<Vector2<W>> for Point2<W> {
    type Output = Point2<W>;
    fn sub(self, rhs: Vector2<W>) -> Self::Output {
        Self::Output::new(self.x - rhs.x, self.y - rhs.y)
    }
}
