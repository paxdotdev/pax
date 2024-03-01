use std::ops::Mul;

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
