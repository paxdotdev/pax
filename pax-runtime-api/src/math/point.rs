use std::{
    marker::PhantomData,
    ops::{Add, Sub},
};

use crate::Interpolatable;

use super::{vector::Vector2, Generic, Space};

impl<W: Space> Interpolatable for Point2<W> {}

/// A representation of a point in 2D space (float).
pub struct Point2<W = Generic> {
    /// Horizontal coordinate.
    pub x: f64,
    /// Vertical coordinate.
    pub y: f64,
    _phantom: PhantomData<W>,
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
            _phantom: PhantomData,
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
    /// Constructs a point from x and y coordinates.
    pub fn new(x: f64, y: f64) -> Self {
        Point2 {
            x,
            y,
            _phantom: PhantomData,
        }
    }

    /// Reinterprets this point as a vector from the origin.
    pub fn to_vector(self) -> Vector2<W> {
        Vector2::new(self.x, self.y)
    }

    /// Casts this point into another phantom coordinate space.
    pub fn cast_space<WNew: Space>(self) -> Point2<WNew> {
        Point2::new(self.x, self.y)
    }

    /// Returns the midpoint between this point and `other`.
    pub fn midpoint_towards(self, other: Self) -> Self {
        self.lerp_towards(other, 1.0 / 2.0)
    }

    /// Linearly interpolates toward `other` by `l`.
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
