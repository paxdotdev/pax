use super::{Generic, Point2, Space, Vector2};
use std::{marker::PhantomData, ops::Mul};

//-----------------------------------------------------------
// Pax matrix/transform class heavily borrows from kurbos
// transform impl (copy/pasted initially with some modifications)
// curbo crate: https://www.michaelfbryan.com/arcs/kurbo/index.html
// original source code: https://www.michaelfbryan.com/arcs/src/kurbo/affine.rs.html#10
// Kurbo is distributed under an MIT license.
//-----------------------------------------------------------

pub struct Transform2<WFrom = Generic, WTo = WFrom> {
    m: [f64; 6],
    _panthom_from: PhantomData<WFrom>,
    _panthom_to: PhantomData<WTo>,
}

// Implement Clone, Copy, PartialEq, etc manually, as
// to not require the Space to implement these.

impl<F: Space, T: Space> Clone for Transform2<F, T> {
    fn clone(&self) -> Self {
        Self {
            m: self.m,
            _panthom_from: PhantomData,
            _panthom_to: PhantomData,
        }
    }
}

impl<F: Space, T: Space> std::fmt::Debug for Transform2<F, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} {} {}", self.m[0], self.m[2], self.m[4])?;
        write!(f, "{} {} {}", self.m[1], self.m[3], self.m[5])
    }
}

impl<F: Space, T: Space> PartialEq for Transform2<F, T> {
    fn eq(&self, other: &Self) -> bool {
        self.m == other.m
    }
}

impl<F: Space, T: Space> Copy for Transform2<F, T> {}

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

    pub fn parts(&self) -> Parts<WTo> {
        let [a, b, c, d, e, f] = self.m;
        let angle = f64::atan2(b, a);
        let denom = a.powi(2) + b.powi(2);
        let scale_x = f64::sqrt(denom);
        let scale_y = (a * d - c * b) / scale_x;
        let skew_x = f64::atan2(a * c + b * d, denom);

        Parts {
            origin: Vector2::new(e, f),
            vx: Vector2::new(a, b),
            vy: Vector2::new(c, d),
            scale: Vector2::new(scale_x, scale_y),
            skew: Vector2::new(skew_x, 0.0),
            rotation: angle,
        }
    }
}

pub struct Parts<W> {
    pub origin: Vector2<W>,
    pub vx: Vector2<W>,
    pub vy: Vector2<W>,
    pub scale: Vector2<W>,
    pub skew: Vector2<W>,
    pub rotation: f64,
}

impl<W: Space> std::fmt::Debug for Parts<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Parts")
            .field("origin", &self.origin)
            .field("vx", &self.vx)
            .field("vy", &self.vy)
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
