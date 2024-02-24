use std::{
    f64::consts::PI,
    ops::{Add, Mul},
};

use kurbo::Affine;

mod point;
mod transform;
mod vector;

pub use point::Point2;
pub use transform::Transform2;
pub use vector::Vector2;

pub trait Space {}

pub struct Generic;

impl Space for Generic {}

#[derive(Clone, Copy)]
pub struct Angle {
    radians: f64,
}

impl Angle {
    pub fn from_radians(radians: f64) -> Self {
        Self { radians }
    }

    pub fn from_degrees(deg: f64) -> Self {
        Self {
            radians: deg * PI / 180.0,
        }
    }

    pub fn radians(&self) -> f64 {
        self.radians
    }

    pub fn degrees(&self) -> f64 {
        self.radians * 180.0 / PI
    }
}

impl Add for Angle {
    type Output = Angle;

    fn add(self, rhs: Self) -> Self::Output {
        Angle::from_radians(self.radians() + rhs.radians())
    }
}

// TODO remove after Affine not used
impl<W: Space> Mul<Point2<W>> for Affine {
    type Output = Point2<W>;

    #[inline]
    fn mul(self, other: Point2<W>) -> Point2<W> {
        let coeffs = self.as_coeffs();
        Self::Output::new(
            coeffs[0] * other.x + coeffs[2] * other.y + coeffs[4],
            coeffs[1] * other.x + coeffs[3] * other.y + coeffs[5],
        )
    }
}
