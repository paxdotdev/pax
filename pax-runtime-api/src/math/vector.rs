use std::{
    f64::consts::PI,
    marker::PhantomData,
    ops::{Add, Div, Mul, Neg, Sub},
};

use crate::{Interpolatable, Numeric, Rotation};

use super::{Generic, Point2, Space};

/// A representation of a vector in 2D space.
pub struct Vector2<W = Generic> {
    /// Horizontal component.
    pub x: f64,
    /// Vertical component.
    pub y: f64,
    _phantom: PhantomData<W>,
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
            _phantom: PhantomData,
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
    /// Constructs a vector from x and y components.
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            _phantom: PhantomData,
        }
    }

    /// Unit vector along the x axis.
    pub fn x() -> Self {
        Self::new(1.0, 0.0)
    }

    /// Unit vector along the y axis.
    pub fn y() -> Self {
        Self::new(0.0, 1.0)
    }

    /// Returns the left-handed normal.
    pub fn normal(&self) -> Self {
        Self::new(-self.y, self.x)
    }

    /// Returns this vector scaled to unit length.
    pub fn normalize(self) -> Self {
        self / self.length()
    }

    /// Returns squared vector length.
    pub fn length_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    /// Returns vector length.
    pub fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }

    /// Returns a vector with absolute component values.
    pub fn coord_abs(&self) -> Self {
        Self::new(self.x.abs(), self.y.abs())
    }

    /// Projects this vector onto `axis`.
    pub fn project_onto(self, axis: Self) -> Self {
        let dot_product = self * axis;
        axis * dot_product / axis.length_squared()
    }

    // Projects this vector onto an axis-aligned frame described by `other`.
    pub fn project_axis_aligned(self, other: Self) -> Self {
        let v = self.coord_abs();
        let o = other.coord_abs().normalize();
        o.to_signums_of(self) * (v.x / o.x).max(v.y / o.y)
    }

    /// Returns the counter-clockwise angle from this vector to `other`.
    pub fn angle_to(self, other: Self) -> Rotation {
        let dot = self.x * other.x + self.y * other.y; //Dot product between [x1, y1] and [x2, y2]
        let det = self.x * other.y - self.y * other.x; //Determinant
        let angle = det.atan2(dot).rem_euclid(2.0 * PI); //atan2(y, x) or atan2(sin, cos)
        Rotation::Radians(Numeric::from(angle))
    }

    /// Returns the z component of the 3D cross product, assuming both vectors have z value 0.0.
    pub fn cross(self, other: Self) -> f64 {
        self.x * other.y - self.y * other.x
    }

    /// Copies this vector's magnitudes onto the component signs of `other`.
    pub fn to_signums_of(&self, other: Self) -> Self {
        Self::new(
            self.x.abs() * other.x.signum(),
            self.y.abs() * other.y.signum(),
        )
    }

    /// Reinterprets this vector as a point.
    pub fn to_point(&self) -> Point2<W> {
        Point2::new(self.x, self.y)
    }

    /// Casts this vector into another phantom coordinate space.
    pub fn cast_space<WNew: Space>(&self) -> Vector2<WNew> {
        Vector2::new(self.x, self.y)
    }

    /// Rotates this vector by `angle`.
    pub fn rotate(&self, angle: Rotation) -> Self {
        let (s, c) = angle.get_as_radians().sin_cos();
        let x = self.x * c - self.y * s;
        let y = self.x * s + self.y * c;
        Self::new(x, y)
    }

    /// Rotates this vector 90 degrees clockwise.
    pub fn rotate90(self) -> Self {
        Self::new(self.y, -self.x)
    }

    /// Multiplies component-wise by `other`.
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
