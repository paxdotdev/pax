use std::{
    marker::PhantomData,
    ops::{Add, Div, Mul, Sub},
};

use super::{Generic, Point2, Space};

#[derive(Copy, Clone, Default, Debug)]
pub struct Vector2<W = Generic> {
    pub x: f64,
    pub y: f64,
    _panthom: PhantomData<W>,
}

impl<W: Space> Vector2<W> {
    pub fn new(x: f64, y: f64) -> Self {
        Vector2 {
            x,
            y,
            _panthom: PhantomData,
        }
    }
    pub fn normal(&self) -> Self {
        Self::new(-self.y, self.x)
    }

    pub fn length_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    pub fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }

    pub fn project_onto(self, axis: Vector2<W>) -> f64 {
        let dot_product = self * axis;
        dot_product / axis.length_squared()
    }

    pub fn to_point(self) -> Point2<W> {
        Point2::new(self.x, self.y)
    }

    pub fn to_world<WNew: Space>(self) -> Point2<WNew> {
        Point2::new(self.x, self.y)
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

impl<W: Space> Sub for Vector2<W> {
    type Output = Vector2<W>;
    fn sub(self, rhs: Vector2<W>) -> Self::Output {
        Self::Output::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<W: Space> Div<f64> for Vector2<W> {
    type Output = Vector2<W>;
    fn div(self, rhs: f64) -> Self::Output {
        Self::Output::new(self.x / rhs, self.y / rhs)
    }
}
