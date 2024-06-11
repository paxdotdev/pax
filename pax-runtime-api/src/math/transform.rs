use crate::Interpolatable;

use super::{Generic, Point2, Space, Vector2};
use std::{marker::PhantomData, ops::Mul};

//-----------------------------------------------------------
// Pax matrix/transform class heavily borrows from kurbos
// transform impl (copy/pasted initially with some modifications)
// curbo crate: https://www.michaelfbryan.com/arcs/kurbo/index.html
// original source code: https://www.michaelfbryan.com/arcs/src/kurbo/affine.rs.html#10
// Kurbo is distributed under an MIT license.
//-----------------------------------------------------------

impl<W: Space, T: Space> Interpolatable for Transform2<W, T> {}

pub struct Transform2<WFrom = Generic, WTo = WFrom> {
    m: [f64; 6],
    _panthom_from: PhantomData<WFrom>,
    _panthom_to: PhantomData<WTo>,
}

// Implement Clone, Copy, PartialEq, etc manually, as
// to not require the Space to implement these.

impl<F, T> Clone for Transform2<F, T> {
    fn clone(&self) -> Self {
        Self {
            m: self.m,
            _panthom_from: PhantomData,
            _panthom_to: PhantomData,
        }
    }
}

impl<F, T> std::fmt::Debug for Transform2<F, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} {} {}", self.m[0], self.m[2], self.m[4])?;
        write!(f, "{} {} {}", self.m[1], self.m[3], self.m[5])
    }
}

impl<F, T> PartialEq for Transform2<F, T> {
    fn eq(&self, other: &Self) -> bool {
        self.m == other.m
    }
}

impl<F, T> Copy for Transform2<F, T> {}

impl<F: Space, T: Space> Default for Transform2<F, T> {
    fn default() -> Self {
        Self::identity()
    }
}

impl<WFrom: Space, WTo: Space> Transform2<WFrom, WTo> {
    pub fn new(m: [f64; 6]) -> Self {
        Self {
            m,
            _panthom_from: PhantomData,
            _panthom_to: PhantomData,
        }
    }

    pub fn identity() -> Self {
        Self::new([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
    }

    pub fn scale(s: f64) -> Self {
        Self::scale_sep(Vector2::new(s, s))
    }

    pub fn scale_sep(s: Vector2<WTo>) -> Self {
        Self::new([s.x, 0.0, 0.0, s.y, 0.0, 0.0])
    }

    pub fn skew(k: Vector2<WTo>) -> Self {
        Self::new([1.0, k.y, k.x, 1.0, 0.0, 0.0])
    }

    pub fn rotate(th: f64) -> Self {
        let (s, c) = th.sin_cos();
        Self::new([c, s, -s, c, 0.0, 0.0])
    }

    pub fn translate(p: Vector2<WTo>) -> Self {
        Self::new([1.0, 0.0, 0.0, 1.0, p.x, p.y])
    }

    pub fn determinant(self) -> f64 {
        self.m[0] * self.m[3] - self.m[1] * self.m[2]
    }

    pub fn coeffs(&self) -> [f64; 6] {
        self.m
    }

    pub fn get_translation(self) -> Vector2<WFrom> {
        (self * Point2::<WFrom>::default()).cast_space().to_vector()
    }

    pub fn get_scale(self) -> Vector2<WTo> {
        self * Vector2::<WFrom>::new(1.0, 1.0)
    }

    pub fn cast_spaces<W: Space, T: Space>(self) -> Transform2<W, T> {
        Transform2::new(self.m)
    }

    /// Produces NaN values when the determinant is zero.
    pub fn inverse(self) -> Transform2<WTo, WFrom> {
        let inv_det = self.determinant().recip();
        Transform2::<WTo, WFrom>::new([
            inv_det * self.m[3],
            -inv_det * self.m[1],
            -inv_det * self.m[2],
            inv_det * self.m[0],
            inv_det * (self.m[2] * self.m[5] - self.m[3] * self.m[4]),
            inv_det * (self.m[1] * self.m[4] - self.m[0] * self.m[5]),
        ])
    }

    pub fn compose(p: Point2<WTo>, vx: Vector2<WTo>, vy: Vector2<WTo>) -> Self {
        Self::new([vx.x, vx.y, vy.x, vy.y, p.x, p.y])
    }

    // Decomposes the transform into translation point + unit vector transforms
    // (ie. where (0, 1) and (1, 0) end up)
    pub fn decompose(&self) -> (Point2<WTo>, Vector2<WTo>, Vector2<WTo>) {
        let [v1x, v1y, v2x, v2y, px, py] = self.m;
        (
            Point2::new(px, py),
            Vector2::new(v1x, v1y),
            Vector2::new(v2x, v2y),
        )
    }
}

#[derive(PartialEq, Clone)]
pub struct Parts {
    pub origin: Vector2,
    pub scale: Vector2,
    pub skew: Vector2,
    pub rotation: f64,
}

impl<F: Space, W: Space> Into<Transform2<F, W>> for Parts {
    fn into(self) -> Transform2<F, W> {
        (Transform2::<Generic>::translate(self.origin)
            * Transform2::rotate(self.rotation)
            * Transform2::<Generic>::scale_sep(self.scale)
            * Transform2::<Generic>::skew(self.skew))
        .cast_spaces()
    }
}

/// NOTE: the returned parts.skew.y will always be equal to 0,
impl<F: Space, T: Space> Into<Parts> for Transform2<F, T> {
    fn into(self) -> Parts {
        let [a, b, c, d, e, f] = self.m;
        let angle = f64::atan2(b, a);
        let denom = a.powi(2) + b.powi(2);
        let scale_x = f64::sqrt(denom);
        let scale_y = (a * d - c * b) / scale_x;
        let skew_x = (a * c + b * d) / denom;

        Parts {
            origin: Vector2::new(e, f),
            scale: Vector2::new(scale_x, scale_y),
            skew: Vector2::new(skew_x, 0.0),
            rotation: angle,
        }
    }
}

impl std::fmt::Debug for Parts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Parts")
            .field("origin", &self.origin)
            .field("scale", &self.scale)
            .field("skew", &self.skew)
            .field("rotation", &self.rotation)
            .finish()
    }
}

impl<W1: Space, W2: Space, W3: Space> Mul<Transform2<W1, W2>> for Transform2<W2, W3> {
    type Output = Transform2<W1, W3>;

    fn mul(self, rhs: Transform2<W1, W2>) -> Self::Output {
        Self::Output::new([
            self.m[0] * rhs.m[0] + self.m[2] * rhs.m[1],
            self.m[1] * rhs.m[0] + self.m[3] * rhs.m[1],
            self.m[0] * rhs.m[2] + self.m[2] * rhs.m[3],
            self.m[1] * rhs.m[2] + self.m[3] * rhs.m[3],
            self.m[0] * rhs.m[4] + self.m[2] * rhs.m[5] + self.m[4],
            self.m[1] * rhs.m[4] + self.m[3] * rhs.m[5] + self.m[5],
        ])
    }
}

impl<F: Space, T: Space> Mul<Point2<F>> for Transform2<F, T> {
    type Output = Point2<T>;

    fn mul(self, other: Point2<F>) -> Self::Output {
        Self::Output::new(
            self.m[0] * other.x + self.m[2] * other.y + self.m[4],
            self.m[1] * other.x + self.m[3] * other.y + self.m[5],
        )
    }
}

impl<F: Space, T: Space> Mul<Vector2<F>> for Transform2<F, T> {
    type Output = Vector2<T>;

    fn mul(self, other: Vector2<F>) -> Self::Output {
        Self::Output::new(
            self.m[0] * other.x + self.m[2] * other.y,
            self.m[1] * other.x + self.m[3] * other.y,
        )
    }
}

impl<T: Space, F: Space> From<Transform2<T, F>> for kurbo::Affine {
    fn from(value: Transform2<T, F>) -> Self {
        Self::new(value.m)
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use crate::math::{Generic, Vector2};

    use super::{Parts, Transform2};

    #[test]
    fn from_to_parts() {
        // any values
        for origin in [
            Vector2::new(0.0, 0.0),
            Vector2::new(11.2, 10.5),
            Vector2::new(-9.2, 0.98),
            Vector2::new(14.2, -3.2),
            Vector2::new(-5.0, -5.1),
        ] {
            // always > 0
            for scale in [
                Vector2::new(1.0, 1.0),
                Vector2::new(1.2, 1.5),
                Vector2::new(0.2, 0.4395),
            ] {
                // any values
                for rotation in [0.0, 1.0, 1.2940, -0.495, 5.0, -40.1] {
                    // skew x any value, skew_y = 0.0
                    for skew in [
                        Vector2::new(0.0, 0.0),
                        Vector2::new(1.0, 0.0),
                        Vector2::new(0.13904, 0.0),
                        Vector2::new(-0.55, 0.0),
                        Vector2::new(100.0, 0.0),
                    ] {
                        let parts = Parts {
                            origin,
                            scale,
                            rotation,
                            skew,
                        };
                        let transform: Transform2 = parts.clone().into();
                        let new_parts: Parts = transform.into();
                        assert!((parts.skew - new_parts.skew).length() < 1e-3);
                        assert!((parts.origin - new_parts.origin).length() < 1e-3);
                        assert!((parts.scale - new_parts.scale).length() < 1e-3);
                        // rotation should be similar up to multiples of 2 pi
                        assert!(
                            ((parts.rotation - new_parts.rotation).rem_euclid(2.0 * PI)).abs()
                                < 1e-3
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_scale_and_resize() {
        let transform = Transform2::<Generic>::new([1.0, 0.4, 0.8, 0.2, 0.1, 0.5]);
        let mut parts: Parts = transform.into();
        parts.scale.x = 2.6 * parts.scale.x;
        parts.scale.y = 1.8 * parts.scale.y;
        let res_transform: Transform2<Generic> = parts.into();
        let res2_transform = transform * Transform2::<Generic>::scale_sep(Vector2::new(2.6, 1.8));
        assert!(
            !(res_transform
                .coeffs()
                .into_iter()
                .zip(res2_transform.coeffs())
                .map(|(a, b)| (a - b).abs())
                .sum::<f64>()
                < 1e-3)
        );
    }
}
