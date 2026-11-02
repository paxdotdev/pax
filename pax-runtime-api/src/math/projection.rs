//! Homogeneous flat-surface projection shared by rendering, native controls and input.

use super::{Generic, Space, Transform3};
use crate::DepthPlane;
use kurbo::{Affine, BezPath, PathEl, Point, Rect, Shape};

/// A column-major homography from local x/y to presentation x/y.
///
/// Coordinates are divided by the third homogeneous coordinate explicitly.
/// This describes a flat surface, not its layout or its camera clipping range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanarProjection(pub [f64; 9]);

impl Default for PlanarProjection {
    fn default() -> Self {
        Self([1., 0., 0., 0., 1., 0., 0., 0., 1.])
    }
}

impl From<Affine> for PlanarProjection {
    fn from(value: Affine) -> Self {
        let [a, b, c, d, e, f] = value.as_coeffs();
        Self([a, b, 0., c, d, 0., e, f, 1.])
    }
}

impl std::ops::Mul for PlanarProjection {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        let mut result = [0.; 9];
        for col in 0..3 {
            for row in 0..3 {
                result[col * 3 + row] = (0..3)
                    .map(|k| self.0[k * 3 + row] * rhs.0[col * 3 + k])
                    .sum();
            }
        }
        Self(result)
    }
}

impl PlanarProjection {
    /// Returns an affine only when this mapping is exactly affine.
    pub fn as_affine(self) -> Option<Affine> {
        let m = self.0;
        (m.iter().all(|v| v.is_finite()) && m[2] == 0. && m[5] == 0. && m[8] != 0.)
            .then(|| Affine::new([m[0], m[1], m[3], m[4], m[6], m[7]].map(|v| v / m[8])))
    }

    /// Multiplies a point without dividing or applying camera clipping.
    pub fn homogeneous(self, point: Point) -> [f64; 3] {
        std::array::from_fn(|row| {
            self.0[row] * point.x + self.0[3 + row] * point.y + self.0[6 + row]
        })
    }

    /// Projects a finite point on the positive-w side of the mapping.
    pub fn project(self, point: Point) -> Option<Point> {
        let [x, y, w] = self.homogeneous(point);
        if ![x, y, w].iter().all(|v| v.is_finite()) || w <= 0. {
            return None;
        }
        let point = Point::new(x / w, y / w);
        (point.x.is_finite() && point.y.is_finite()).then_some(point)
    }

    /// Inverts a nonsingular finite plane mapping, preserving homogeneous sign.
    pub fn inverse(self) -> Option<Self> {
        let m = self.0;
        let scale = m.iter().fold(0_f64, |a, v| a.max(v.abs()));
        if scale == 0. || !scale.is_finite() {
            return None;
        }
        let m = m.map(|v| v / scale);
        let cofactors = [
            m[4] * m[8] - m[7] * m[5],
            m[7] * m[2] - m[1] * m[8],
            m[1] * m[5] - m[4] * m[2],
            m[6] * m[5] - m[3] * m[8],
            m[0] * m[8] - m[6] * m[2],
            m[3] * m[2] - m[0] * m[5],
            m[3] * m[7] - m[6] * m[4],
            m[6] * m[1] - m[0] * m[7],
            m[0] * m[4] - m[3] * m[1],
        ];
        let determinant = m[0] * cofactors[0] + m[3] * cofactors[1] + m[6] * cofactors[2];
        if determinant == 0. || !determinant.is_finite() {
            return None;
        }
        let inverse = cofactors.map(|v| v / determinant / scale);
        inverse
            .iter()
            .all(|v| v.is_finite())
            .then_some(Self(inverse))
    }

    /// Transforms a path whose vertices lie in the finite positive-w chart.
    /// Curves are flattened using a caller-supplied local-space tolerance.
    pub fn project_path(self, path: &BezPath, tolerance: f64) -> BezPath {
        if let Some(affine) = self.as_affine() {
            return affine * path.clone();
        }
        let mut output = BezPath::new();
        kurbo::flatten(path.iter(), tolerance, |element| match element {
            PathEl::MoveTo(p) => {
                if let Some(p) = self.project(p) {
                    output.move_to(p);
                }
            }
            PathEl::LineTo(p) => {
                if let Some(p) = self.project(p) {
                    output.line_to(p);
                }
            }
            PathEl::ClosePath => output.close_path(),
            _ => unreachable!("flatten emits only line segments"),
        });
        output
    }

    /// Embeds the plane in a CSS/Apple column-major 4x4 matrix.
    /// Depth ordering is supplied separately by the shared compositor.
    pub fn matrix3d(self) -> [f64; 16] {
        let m = self.0;
        [
            m[0], m[1], 0., m[2], m[3], m[4], 0., m[5], 0., 0., 1., 0., m[6], m[7], 0., m[8],
        ]
    }
}

/// A projected local z=0 surface, including camera depth clipping.
///
/// The matrix outputs logical-pixel x/y multiplied by w. In perspective mode,
/// depth is 0 at the far plane and w at the near plane, so larger depth remains
/// nearer in Pax's compositor. Orthographic surfaces retain unbounded depth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectedPlane {
    matrix: [f64; 16],
    clip_depth: bool,
    clip_rows: [[f64; 3]; 2],
}

impl ProjectedPlane {
    /// Retains a finite projection, including collapsed or edge-on planes.
    pub fn new<F: Space, T: Space>(matrix: Transform3<F, T>, clip_depth: bool) -> Option<Self> {
        let matrix = matrix.coeffs();
        matrix.iter().all(|v| v.is_finite()).then_some(Self {
            matrix,
            clip_depth,
            clip_rows: [
                [matrix[2], matrix[6], matrix[14]],
                [matrix[3], matrix[7], matrix[15]],
            ],
        })
    }

    /// Creates an unbounded-depth affine plane.
    pub fn affine(transform: Affine) -> Self {
        let [a, b, c, d, e, f] = transform.as_coeffs();
        Self {
            matrix: [a, b, 0., 0., c, d, 0., 0., 0., 0., 1., 0., e, f, 0., 1.],
            clip_depth: false,
            clip_rows: [[0.; 3], [0., 0., 1.]],
        }
    }

    /// Complete homogeneous matrix, retaining near/far clipping coordinates.
    pub fn matrix(self) -> Transform3<Generic> {
        Transform3::new(self.matrix)
    }

    /// Composes an affine change of destination coordinates, preserving depth and w.
    pub fn post_affine(self, transform: Affine) -> Self {
        let affine = Self::affine(transform).matrix();
        Self {
            matrix: (affine * self.matrix()).coeffs(),
            ..self
        }
    }

    /// Composes local geometry before projection, including local scroll offsets.
    pub fn pre_affine(self, transform: Affine) -> Self {
        let affine = Self::affine(transform).matrix();
        let matrix = (self.matrix() * affine).coeffs();
        let transform = PlanarProjection::from(transform).0;
        let clip_rows = self.clip_rows.map(|row| {
            std::array::from_fn(|col| (0..3).map(|i| row[i] * transform[col * 3 + i]).sum())
        });
        Self {
            matrix,
            clip_rows,
            ..self
        }
    }

    /// Expresses this surface in another projected plane's coordinate chart.
    /// Camera clip equations remain in the original local coordinates.
    pub fn post_mapping(self, mapping: PlanarProjection) -> Self {
        let old = self.matrix;
        let mut matrix = old;
        for col in 0..4 {
            for (output, row) in [0, 1, 3].into_iter().enumerate() {
                matrix[col * 4 + row] = mapping.0[output] * old[col * 4]
                    + mapping.0[3 + output] * old[col * 4 + 1]
                    + mapping.0[6 + output] * old[col * 4 + 3];
            }
        }
        Self { matrix, ..self }
    }

    /// Camera reverse-depth and w equations, retained across surface rebasing.
    pub fn camera_clip_rows(self) -> [[f64; 3]; 2] {
        self.clip_rows
    }

    /// Whether the camera imposes a finite near/far depth interval.
    pub fn clips_depth(self) -> bool {
        self.clip_depth
    }

    /// Plane homography used for drawing and inverse input mapping.
    pub fn mapping(self) -> PlanarProjection {
        let m = self.matrix;
        PlanarProjection([m[0], m[1], m[3], m[4], m[5], m[7], m[12], m[13], m[15]])
    }

    fn row(self, row: usize) -> [f64; 3] {
        [
            self.matrix[row],
            self.matrix[4 + row],
            self.matrix[12 + row],
        ]
    }

    /// Tests the camera clip interval before division. Layout bounds are separate.
    pub fn contains_local(self, point: Point) -> bool {
        let w = evaluate(self.clip_rows[1], point);
        let z = evaluate(self.clip_rows[0], point);
        w > 0.
            && w.is_finite()
            && z.is_finite()
            && (!self.clip_depth
                || (z >= -64. * f64::EPSILON * w && z <= (1. + 64. * f64::EPSILON) * w))
    }

    /// Inverse projection and camera clipping agree, including near-plane crossings.
    pub fn local_point(self, point: Point) -> Option<Point> {
        let local = self.mapping().inverse()?.project(point)?;
        self.contains_local(local).then_some(local)
    }

    /// Clips a local convex polygon in homogeneous coordinates before division.
    /// Viewport clipping bounds exposure even when a surface crosses the eye plane.
    pub fn clip_polygon(self, mut polygon: Vec<Point>, viewport: Option<Rect>) -> Vec<Point> {
        let w = self.row(3);
        let mut planes = vec![w];
        if self.clip_depth {
            let [z, camera_w] = self.clip_rows;
            planes.extend([camera_w, z, subtract(camera_w, z)]);
        }
        if let Some(viewport) = viewport {
            let x = self.row(0);
            let y = self.row(1);
            planes.extend([
                subtract(x, w.map(|v| v * viewport.x0)),
                subtract(w.map(|v| v * viewport.x1), x),
                subtract(y, w.map(|v| v * viewport.y0)),
                subtract(w.map(|v| v * viewport.y1), y),
            ]);
        }
        for plane in planes {
            polygon = clip_half_plane(&polygon, plane);
            if polygon.len() < 3 {
                return Vec::new();
            }
        }
        polygon
    }

    /// Visible convex footprint in presentation coordinates.
    pub fn footprint(self, bounds: Rect, viewport: Option<Rect>) -> Vec<Point> {
        let polygon = self.clip_polygon(rect_polygon(bounds), viewport);
        // A vertex on w=0 cannot be divided. For a valid perspective camera,
        // near clipping keeps every nondegenerate visible vertex away from it.
        polygon
            .into_iter()
            .filter_map(|p| self.mapping().project(p))
            .collect()
    }

    /// Projects filled contours after camera/viewport clipping. Curves use a
    /// conservative magnification bound to keep flattening error within a
    /// quarter presentation pixel, including near-plane intersections.
    pub fn project_path(self, path: &BezPath, viewport: Option<Rect>) -> BezPath {
        if !self.clip_depth {
            if let Some(affine) = self.mapping().as_affine() {
                if viewport.is_none() {
                    return affine * path.clone();
                }
            }
        }
        let Some(magnification) = self.magnification_bound(path.bounding_box(), viewport) else {
            return BezPath::new();
        };
        let tolerance = 0.25 / magnification.max(f64::MIN_POSITIVE);
        let mut output = BezPath::new();
        let mut contour = Vec::new();
        let mut flush = |contour: &mut Vec<Point>| {
            let clipped = self.clip_polygon(std::mem::take(contour), viewport);
            let mut points = clipped
                .into_iter()
                .filter_map(|p| self.mapping().project(p));
            if let Some(first) = points.next() {
                output.move_to(first);
                for p in points {
                    output.line_to(p);
                }
                output.close_path();
            }
        };
        kurbo::flatten(path.iter(), tolerance, |element| match element {
            PathEl::MoveTo(p) => {
                flush(&mut contour);
                contour.push(p);
            }
            PathEl::LineTo(p) => contour.push(p),
            PathEl::ClosePath => flush(&mut contour),
            _ => unreachable!("flatten emits only line segments"),
        });
        flush(&mut contour);
        output
    }

    /// Conservative finite coverage after camera and optional viewport clipping.
    pub fn coverage(self, bounds: Rect, viewport: Option<Rect>) -> Option<Rect> {
        let points = self.footprint(bounds, viewport);
        if points.len() < 3 {
            return None;
        }
        Some(
            points
                .iter()
                .skip(1)
                .fold(Rect::from_points(points[0], points[0]), |r, p| {
                    r.union_pt(*p)
                }),
        )
    }

    /// Screen-space depth equation for a noncollapsed projected plane.
    pub fn depth_plane(self) -> Option<DepthPlane> {
        let inverse = self.mapping().inverse()?.0;
        let z = self.row(2);
        let coefficients =
            std::array::from_fn(|col| (0..3).map(|row| z[row] * inverse[col * 3 + row]).sum());
        let m = self.mapping().0;
        let determinant = m[0] * (m[4] * m[8] - m[5] * m[7]) - m[3] * (m[1] * m[8] - m[2] * m[7])
            + m[6] * (m[1] * m[5] - m[2] * m[4]);
        coefficients
            .iter()
            .all(|v: &f64| v.is_finite())
            .then_some(DepthPlane {
                presentation_layer: 0,
                coefficients,
                backface: determinant < 0.,
            })
    }

    /// Conservative upper bound on local magnification over the visible polygon.
    /// The denominator is bounded over the whole clipped region, not sampled at
    /// its center. Hosts still apply their normal backing-density and pixel caps.
    pub fn magnification_bound(self, bounds: Rect, viewport: Option<Rect>) -> Option<f64> {
        let polygon = self.clip_polygon(rect_polygon(bounds), viewport);
        if polygon.len() < 3 {
            return None;
        }
        let m = self.mapping().0;
        let mut min_w = f64::INFINITY;
        let mut numerators = [0_f64; 4];
        for point in polygon {
            let [x, y, w] = self.mapping().homogeneous(point);
            min_w = min_w.min(w);
            for (maximum, value) in numerators.iter_mut().zip([
                m[0] * w - m[2] * x,
                m[3] * w - m[5] * x,
                m[1] * w - m[2] * y,
                m[4] * w - m[5] * y,
            ]) {
                *maximum = maximum.max(value.abs());
            }
        }
        if min_w <= 0. {
            return None;
        }
        let bound = numerators
            .iter()
            .map(|v| (v / min_w / min_w).powi(2))
            .sum::<f64>()
            .sqrt();
        bound.is_finite().then_some(bound)
    }
}

fn evaluate(plane: [f64; 3], p: Point) -> f64 {
    plane[0] * p.x + plane[1] * p.y + plane[2]
}
fn subtract(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn rect_polygon(r: Rect) -> Vec<Point> {
    vec![
        Point::new(r.x0, r.y0),
        Point::new(r.x1, r.y0),
        Point::new(r.x1, r.y1),
        Point::new(r.x0, r.y1),
    ]
}
fn clip_half_plane(input: &[Point], plane: [f64; 3]) -> Vec<Point> {
    let mut result = Vec::new();
    if input.is_empty() {
        return result;
    }
    let mut previous = *input.last().unwrap();
    let mut previous_distance = evaluate(plane, previous);
    for &current in input {
        let distance = evaluate(plane, current);
        if (distance >= 0.) != (previous_distance >= 0.) {
            let t = previous_distance / (previous_distance - distance);
            result.push(previous.lerp(current, t));
        }
        if distance >= 0. {
            result.push(current);
        }
        previous = current;
        previous_distance = distance;
    }
    result
}
