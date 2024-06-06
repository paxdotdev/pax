use std::{
    marker::PhantomData,
    ops::{Add, Div, Mul, Neg, Sub},
};

use crate::{Interpolatable, Numeric, Rotation};

use super::{Generic, Point2, Space};

pub struct Vector2<W = Generic> {
    pub x: f64,
    pub y: f64,
    _panthom: PhantomData<W>,
}

// Implement Clone, Copy, PartialEq, etc manually, as
// to not require the Space to implement these.
impl<W: Space> Interpolatable for Vector2<W> {}

impl<W: Space> std::fmt::Debug for Vector2<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{} {}>", self.x, self.y)
    }
}

impl<W: Space> Clone for Vector2<W> {
    fn clone(&self) -> Self {
        Self {
            x: self.x,
            y: self.y,
            _panthom: PhantomData,
        }
    }
}

impl<W: Space> Copy for Vector2<W> {}

impl<W: Space> PartialEq for Vector2<W> {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl<W: Space> Default for Vector2<W> {
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}

impl<W: Space> Vector2<W> {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            _panthom: PhantomData,
        }
    }
    pub fn normal(&self) -> Self {
        Self::new(-self.y, self.x)
    }

    pub fn normalize(self) -> Self {
        self / self.length()
    }

    pub fn length_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    pub fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }

    pub fn coord_abs(&self) -> Self {
        Self::new(self.x.abs(), self.y.abs())
    }

    pub fn project_onto(self, axis: Self) -> Self {
        let dot_product = self * axis;
        axis * dot_product / axis.length_squared()
    }

    pub fn project_axis_aligned(self, other: Self) -> Self {
        let v = self.coord_abs();
        let o = other.coord_abs().normalize();
        o.to_signums_of(self) * (v.x / o.x).max(v.y / o.y)
    }

    /// Returns the angle walking from self to other counter clockwise
    pub fn angle_to(self, other: Self) -> Rotation {
        let dot = (self.normalize() * other.normalize()).clamp(0.0, 1.0);
        let s = self.cross(other).signum();
        Rotation::Radians(Numeric::from(s * dot.acos()))
    }

    /// Returns the magnitude of the cross product as if both vectors had z value 0.0
    pub fn cross(self, other: Self) -> f64 {
        self.x * other.y - self.y * other.x
    }

    pub fn to_signums_of(&self, other: Self) -> Self {
        Self::new(
            self.x.abs() * other.x.signum(),
            self.y.abs() * other.y.signum(),
        )
    }

    pub fn to_point(&self) -> Point2<W> {
        Point2::new(self.x, self.y)
    }

    pub fn cast_space<WNew: Space>(&self) -> Vector2<WNew> {
        Vector2::new(self.x, self.y)
    }

    pub fn mult(&self, other: Self) -> Vector2<W> {
        Vector2::new(self.x * other.x, self.y * other.y)
    }
}

impl<W: Space> Mul for Vector2<W> {
    type Output = f64;

    fn mul(self, rhs: Vector2<W>) -> Self::Output {
        self.x * rhs.x + self.y * rhs.y
    }
}

impl<W: Space> Mul<f64> for Vector2<W> {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Vector2::new(self.x * rhs, self.y * rhs)
    }
}
impl<W: Space> Mul<Vector2<W>> for f64 {
    type Output = Vector2<W>;

    fn mul(self, rhs: Vector2<W>) -> Self::Output {
        Vector2::new(rhs.x * self, rhs.y * self)
    }
}

impl<W: Space> Add for Vector2<W> {
    type Output = Vector2<W>;

    fn add(self, rhs: Vector2<W>) -> Self::Output {
        Self::Output::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<W: Space> Neg for Vector2<W> {
    type Output = Vector2<W>;

    fn neg(self) -> Self::Output {
        Self::Output::new(-self.x, -self.y)
    }
}

impl<W: Space> Sub for Vector2<W> {
    type Output = Vector2<W>;
    fn sub(self, rhs: Vector2<W>) -> Self::Output {
        Self::Output::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<W: Space> Sub<f64> for Vector2<W> {
    type Output = Vector2<W>;
    fn sub(self, rhs: f64) -> Self::Output {
        Self::Output::new(self.x - rhs, self.y - rhs)
    }
}

impl<W: Space> Add<f64> for Vector2<W> {
    type Output = Vector2<W>;
    fn add(self, rhs: f64) -> Self::Output {
        Self::Output::new(self.x + rhs, self.y + rhs)
    }
}

impl<W: Space> Div<f64> for Vector2<W> {
    type Output = Vector2<W>;
    fn div(self, rhs: f64) -> Self::Output {
        Self::Output::new(self.x / rhs, self.y / rhs)
    }
}

impl<W: Space> Div for Vector2<W> {
    type Output = Vector2<W>;
    fn div(self, rhs: Vector2<W>) -> Self::Output {
        Self::Output::new(self.x / rhs.x, self.y / rhs.y)
    }
}
