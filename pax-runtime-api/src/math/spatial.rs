//! Spatial coordinates, homogeneous transforms, and camera-independent planes.
//! Types are also reexported by [`super`] alongside the existing 2D math.

use super::{Generic, Point2, Space, Transform2};
use std::{
    marker::PhantomData,
    ops::{Add, Mul, Sub},
};

/// A point in a phantom 3D coordinate space.
/// Local/world positions use logical pixels; projected clip coordinates are unitless.
pub struct Point3<W = Generic> {
    /// Horizontal coordinate.
    pub x: f64,
    /// Vertical coordinate (positive down in the default view).
    pub y: f64,
    /// Depth coordinate (positive toward the default viewer).
    pub z: f64,
    space: PhantomData<W>,
}

/// A geometric vector in a phantom 3D coordinate space.
///
/// Unlike the authoring [`crate::drawing::Vector3`] lighting value, this type
/// carries a coordinate-space marker and does not participate in Pax values.
pub struct Vector3<W = Generic> {
    /// Horizontal component.
    pub x: f64,
    /// Vertical component.
    pub y: f64,
    /// Depth component.
    pub z: f64,
    space: PhantomData<W>,
}

// Space markers need not implement Clone, Copy, Debug, or PartialEq.
macro_rules! spatial_coordinates {
    ($name:ident) => {
        impl<W: Space> $name<W> {
            /// Constructs coordinates in the indicated space.
            pub fn new(x: f64, y: f64, z: f64) -> Self {
                Self {
                    x,
                    y,
                    z,
                    space: PhantomData,
                }
            }

            /// Returns whether every coordinate is finite.
            pub fn is_finite(self) -> bool {
                self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
            }
        }
        impl<W: Space> Clone for $name<W> {
            fn clone(&self) -> Self {
                *self
            }
        }
        impl<W: Space> Copy for $name<W> {}
        impl<W: Space> PartialEq for $name<W> {
            fn eq(&self, other: &Self) -> bool {
                self.x == other.x && self.y == other.y && self.z == other.z
            }
        }
        impl<W: Space> std::fmt::Debug for $name<W> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple(stringify!($name))
                    .field(&self.x)
                    .field(&self.y)
                    .field(&self.z)
                    .finish()
            }
        }
    };
}
spatial_coordinates!(Point3);
spatial_coordinates!(Vector3);

impl<W: Space> Point3<W> {
    /// Drops z after a caller has chosen the plane or projection.
    pub fn xy(self) -> Point2<W> {
        Point2::new(self.x, self.y)
    }
}

impl<W: Space> Vector3<W> {
    /// Scalar product in the shared coordinate space.
    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Component cross product; the local x/y basis produces the +z normal.
    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    /// Returns a unit vector, or `None` for a zero or non-finite vector.
    pub fn try_normalize(self) -> Option<Self> {
        if !self.is_finite() {
            return None;
        }
        // Rescale before squaring so very small, valid entrance scales survive.
        let scale = self.x.abs().max(self.y.abs()).max(self.z.abs());
        if scale == 0.0 {
            return None;
        }
        let v = Self::new(self.x / scale, self.y / scale, self.z / scale);
        Some(v * v.dot(v).sqrt().recip())
    }
}

impl<W: Space> Sub for Point3<W> {
    type Output = Vector3<W>;
    fn sub(self, rhs: Self) -> Self::Output {
        Vector3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}
impl<W: Space> Add<Vector3<W>> for Point3<W> {
    type Output = Self;
    fn add(self, rhs: Vector3<W>) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}
impl<W: Space> Mul<f64> for Vector3<W> {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

/// A column-major homogeneous 4×4 transform between phantom coordinate spaces.
///
/// `parent * child` applies the child first, as with [`Transform2`]. Defaults
/// are identity. [`Self::from`] embeds an affine 2D transform without changing
/// x/y or flattening z. Raw coefficients use CSS `matrix3d`/WGSL column order.
/// Perspective division is explicit and fallible through [`Self::project_point`].
/// No coefficient interpolation is provided: interpolate authored pose values.
pub struct Transform3<F = Generic, T = F> {
    m: [f64; 16],
    spaces: PhantomData<(F, T)>,
}

impl<F, T> Clone for Transform3<F, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<F, T> Copy for Transform3<F, T> {}
impl<F, T> PartialEq for Transform3<F, T> {
    fn eq(&self, other: &Self) -> bool {
        self.m == other.m
    }
}
impl<F, T> std::fmt::Debug for Transform3<F, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Transform3").field(&self.m).finish()
    }
}
impl<F: Space, T: Space> Default for Transform3<F, T> {
    fn default() -> Self {
        Self::identity()
    }
}

impl<F: Space, T: Space> Transform3<F, T> {
    /// Constructs a matrix in column-major order; validation occurs at use sites.
    pub fn new(m: [f64; 16]) -> Self {
        Self {
            m,
            spaces: PhantomData,
        }
    }

    /// Returns the identity matrix.
    pub fn identity() -> Self {
        Self::new([
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
        ])
    }

    /// Returns column-major coefficients for GPU/native presentation.
    pub fn coeffs(self) -> [f64; 16] {
        self.m
    }

    /// Constructs a translation, in logical pixels.
    pub fn translate(v: Vector3<T>) -> Self {
        let mut result = Self::identity();
        result.m[12] = v.x;
        result.m[13] = v.y;
        result.m[14] = v.z;
        result
    }

    /// Constructs independent x/y/z scale factors.
    pub fn scale(v: Vector3<T>) -> Self {
        Self::new([
            v.x, 0., 0., 0., 0., v.y, 0., 0., 0., 0., v.z, 0., 0., 0., 0., 1.,
        ])
    }

    /// Rotates around x in radians; positive angles turn +y toward +z.
    pub fn rotate_x(radians: f64) -> Self {
        let (s, c) = radians.sin_cos();
        Self::new([1., 0., 0., 0., 0., c, s, 0., 0., -s, c, 0., 0., 0., 0., 1.])
    }

    /// Rotates around y in radians; positive angles turn +z toward +x.
    pub fn rotate_y(radians: f64) -> Self {
        let (s, c) = radians.sin_cos();
        Self::new([c, 0., -s, 0., 0., 1., 0., 0., s, 0., c, 0., 0., 0., 0., 1.])
    }

    /// Rotates around z in radians, matching [`Transform2::rotate`].
    pub fn rotate_z(radians: f64) -> Self {
        Self::from(Transform2::rotate(radians))
    }

    /// Returns whether the matrix is finite and has an affine last row.
    pub fn is_affine(self) -> bool {
        self.m.iter().all(|n| n.is_finite())
            && self.m[3] == 0.
            && self.m[7] == 0.
            && self.m[11] == 0.
            && self.m[15] == 1.
    }

    /// Recovers the exact 2D fast path only if this matrix leaves z unchanged.
    /// No epsilon is used: small, authored spatial changes must not disappear.
    pub fn try_as_2d(self) -> Option<Transform2<F, T>> {
        (self.is_affine()
            && self.m[2] == 0.
            && self.m[6] == 0.
            && self.m[8] == 0.
            && self.m[9] == 0.
            && self.m[10] == 1.
            && self.m[14] == 0.)
            .then(|| {
                Transform2::new([
                    self.m[0], self.m[1], self.m[4], self.m[5], self.m[12], self.m[13],
                ])
            })
    }

    /// Transforms a homogeneous coordinate without dividing or clipping.
    pub fn transform_homogeneous(self, point: [f64; 4]) -> [f64; 4] {
        std::array::from_fn(|row| (0..4).map(|col| self.m[col * 4 + row] * point[col]).sum())
    }

    /// Transforms and divides by w, rejecting zero w and non-finite results.
    /// This does not clip; camera visibility also requires positive w and depth.
    pub fn project_point(self, point: Point3<F>) -> Option<Point3<T>> {
        let [x, y, z, w] = self.transform_homogeneous([point.x, point.y, point.z, 1.]);
        if w == 0. || !w.is_finite() {
            return None;
        }
        let result = Point3::new(x / w, y / w, z / w);
        result.is_finite().then_some(result)
    }

    /// Inverts a finite matrix, rejecting singular or unrepresentable results.
    pub fn try_inverse(self) -> Option<Transform3<T, F>> {
        if !self.m.iter().all(|v| v.is_finite()) {
            return None;
        }
        let mut rows = [[0.; 8]; 4];
        for r in 0..4 {
            for c in 0..4 {
                rows[r][c] = self.m[c * 4 + r];
            }
            rows[r][r + 4] = 1.;
        }
        // Partial pivoting avoids a determinant-size cutoff: tiny invertible
        // transforms must remain usable during scale animations.
        for c in 0..4 {
            let pivot = (c..4).max_by(|&a, &b| rows[a][c].abs().total_cmp(&rows[b][c].abs()))?;
            rows.swap(c, pivot);
            let divisor = rows[c][c];
            if divisor == 0. || !divisor.is_finite() {
                return None;
            }
            for value in &mut rows[c] {
                *value /= divisor;
            }
            for r in 0..4 {
                if r != c {
                    let factor = rows[r][c];
                    for k in 0..8 {
                        rows[r][k] -= factor * rows[c][k];
                    }
                }
            }
        }
        let inverse = std::array::from_fn(|i| rows[i % 4][4 + i / 4]);
        inverse
            .iter()
            .all(|v| v.is_finite())
            .then(|| Transform3::new(inverse))
    }
}

impl<F: Space, T: Space> From<Transform2<F, T>> for Transform3<F, T> {
    fn from(value: Transform2<F, T>) -> Self {
        let [a, b, c, d, e, f] = value.coeffs();
        Self::new([a, b, 0., 0., c, d, 0., 0., 0., 0., 1., 0., e, f, 0., 1.])
    }
}

impl<A: Space, B: Space, C: Space> Mul<Transform3<A, B>> for Transform3<B, C> {
    type Output = Transform3<A, C>;
    fn mul(self, rhs: Transform3<A, B>) -> Self::Output {
        Transform3::new(std::array::from_fn(|i| {
            (0..4)
                .map(|k| self.m[k * 4 + i % 4] * rhs.m[i / 4 * 4 + k])
                .sum()
        }))
    }
}

/// An oriented geometric plane, independent of camera and compositing scope.
pub struct Plane3<W = Generic> {
    origin: Point3<W>,
    normal: Vector3<W>,
}

impl<W: Space> Plane3<W> {
    /// Returns a point on the plane, in logical pixels.
    pub fn origin(&self) -> Point3<W> {
        self.origin
    }

    /// Returns its unit normal; reversing this changes face, not coplanarity.
    pub fn normal(&self) -> Vector3<W> {
        self.normal
    }

    /// Derives the plane of local z=0 from an affine transform's x/y basis.
    /// Returns `None` for a collapsed plane or non-affine transform.
    /// Collapsing only the z axis does not collapse a zero-thickness surface.
    pub fn from_transform<L: Space>(transform: Transform3<L, W>) -> Option<Self> {
        if !transform.is_affine() {
            return None;
        }
        let m = transform.coeffs();
        let x = Vector3::new(m[0], m[1], m[2]).try_normalize()?;
        let y = Vector3::new(m[4], m[5], m[6]).try_normalize()?;
        Some(Self {
            origin: Point3::new(m[12], m[13], m[14]),
            normal: x.cross(y).try_normalize()?,
        })
    }

    /// Compares geometric planes, accepting opposite faces.
    ///
    /// Distance tolerance is in logical pixels; angular tolerance is the sine
    /// of the allowed angle between normals (0..=1). Both must be finite and
    /// nonnegative. This pairwise numerical predicate is not a surface cache key
    /// or permission to flatten content across opacity/clipping boundaries.
    pub fn is_coplanar(
        &self,
        other: &Self,
        distance_tolerance: f64,
        angular_tolerance: f64,
    ) -> bool {
        if !distance_tolerance.is_finite()
            || distance_tolerance < 0.
            || !(0.0..=1.0).contains(&angular_tolerance)
        {
            return false;
        }
        let cross = self.normal.cross(other.normal);
        let delta = other.origin - self.origin;
        cross.dot(cross) <= angular_tolerance * angular_tolerance
            && delta.dot(self.normal).abs() <= distance_tolerance
            && delta.dot(other.normal).abs() <= distance_tolerance
    }
}
