//! Camera projection and clipped surface coverage for opaque convex meshes.
use super::{convex_hull, ConvexMesh, PosedConvexMesh};
use crate::{
    math::{Generic, PlanarProjection, Transform3},
    DepthPlane,
};
use kurbo::Rect;
use std::cell::OnceCell;
use std::rc::Rc;

type Point = [f64; 3];
type Triangle = [Point; 3];

/// Homogeneous solid projection. Camera clip equations stay in mesh coordinates
/// when a projected scroller rebases x/y/w into its own canvas chart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshProjection {
    matrix: [f64; 16],
    clip_rows: [[f64; 4]; 2],
    clip_depth: bool,
}

impl MeshProjection {
    /// Retains a finite column-major pixel-homogeneous matrix. Larger z/w is
    /// nearer; `clip_depth` limits the original camera depth to 0..=w.
    pub fn new(matrix: [f64; 16], clip_depth: bool) -> Option<Self> {
        matrix.iter().all(|v| v.is_finite()).then_some(Self {
            matrix,
            clip_rows: [row(matrix, 2), row(matrix, 3)],
            clip_depth,
        })
    }

    /// Mesh-local to destination-chart coordinates, before division by w.
    pub fn matrix(self) -> [f64; 16] {
        self.matrix
    }
    /// Original camera z and w equations in mesh-local coordinates.
    pub fn camera_clip_rows(self) -> [[f64; 4]; 2] {
        self.clip_rows
    }
    /// Whether the camera's near/far interval clips this surface.
    pub fn clips_depth(self) -> bool {
        self.clip_depth
    }

    /// Changes the destination chart without changing the camera's clip interval.
    pub fn post_mapping(self, mapping: PlanarProjection) -> Self {
        let mut matrix = self.matrix;
        for col in 0..4 {
            for (i, output) in [0, 1, 3].into_iter().enumerate() {
                matrix[col * 4 + output] = mapping.0[i] * self.matrix[col * 4]
                    + mapping.0[3 + i] * self.matrix[col * 4 + 1]
                    + mapping.0[6 + i] * self.matrix[col * 4 + 3];
            }
        }
        Self { matrix, ..self }
    }

    /// Composes mesh-local geometry before both projection and camera clipping.
    pub fn pre_transform(self, transform: [f64; 16]) -> Self {
        let matrix = (Transform3::<Generic>::new(self.matrix)
            * Transform3::<Generic>::new(transform))
        .coeffs();
        let clip_rows = self.clip_rows.map(|r| {
            std::array::from_fn(|col| (0..4).map(|i| r[i] * transform[col * 4 + i]).sum())
        });
        Self {
            matrix,
            clip_rows,
            ..self
        }
    }

    fn planes(self, viewport: Option<Rect>) -> Vec<[f64; 4]> {
        let w = row(self.matrix, 3);
        let mut planes = vec![w];
        if self.clip_depth {
            let [z, camera_w] = self.clip_rows;
            planes.extend([camera_w, z, subtract(camera_w, z)]);
        }
        if let Some(r) = viewport {
            let x = row(self.matrix, 0);
            let y = row(self.matrix, 1);
            planes.extend([
                subtract(x, w.map(|v| v * r.x0)),
                subtract(w.map(|v| v * r.x1), x),
                subtract(y, w.map(|v| v * r.y0)),
                subtract(w.map(|v| v * r.y1), y),
            ]);
        }
        planes
    }

    fn project(self, p: Point) -> Option<Point> {
        let w = evaluate(row(self.matrix, 3), p);
        if w <= 0. {
            return None;
        }
        let result = std::array::from_fn(|axis| evaluate(row(self.matrix, axis), p) / w);
        result.iter().all(|v| v.is_finite()).then_some(result)
    }
}

/// Camera-clipped surface triangles used by culling, picking and occlusion.
/// GPU topology remains shared and immutable; these are CPU coverage records.
#[derive(Debug)]
pub struct ProjectedConvexMesh {
    pub mesh: Rc<ConvexMesh>,
    pub projection: MeshProjection,
    viewport: Option<Rect>,
    vertices: Vec<Point>,
    clipped: Option<Vec<Triangle>>,
    unbounded: bool,
    silhouette: OnceCell<Vec<[f64; 2]>>,
}

impl ConvexMesh {
    /// Projects the actual mesh surface, clipping before division. Supplying a
    /// viewport also bounds a canvas chart whose projective horizon crosses it.
    pub fn projected(
        self: &Rc<Self>,
        projection: MeshProjection,
        viewport: Option<Rect>,
    ) -> Rc<ProjectedConvexMesh> {
        let local: Vec<Point> = self
            .vertices
            .iter()
            .map(|v| v.position.map(f64::from))
            .collect();
        let planes = projection.planes(viewport);
        let cut = local.iter().any(|p| {
            evaluate(planes[0], *p) <= 0. || planes.iter().any(|plane| evaluate(*plane, *p) < 0.)
        });
        let mut unbounded = false;
        let vertices = local
            .iter()
            .map(|p| projection.project(*p).unwrap_or([0.; 3]))
            .collect();
        let clipped = cut.then(|| {
            let mut triangles = Vec::new();
            for indices in self.indices.chunks_exact(3) {
                let mut polygon: Vec<Point> = indices.iter().map(|i| local[*i as usize]).collect();
                for plane in &planes {
                    polygon = clip(&polygon, *plane);
                    if polygon.len() < 3 {
                        break;
                    }
                }
                if polygon.len() < 3 {
                    continue;
                }
                let projected: Option<Vec<Point>> =
                    polygon.into_iter().map(|p| projection.project(p)).collect();
                let Some(polygon) = projected else {
                    unbounded = true;
                    continue;
                };
                for i in 1..polygon.len() - 1 {
                    triangles.push([polygon[0], polygon[i], polygon[i + 1]]);
                }
            }
            triangles
        });
        Rc::new(ProjectedConvexMesh {
            mesh: self.clone(),
            projection,
            viewport,
            vertices,
            clipped,
            unbounded,
            silhouette: OnceCell::new(),
        })
    }
}

impl PosedConvexMesh {
    /// Retains up to four camera/chart projections of this world pose. The
    /// supplied projection includes the world transform and camera clip planes;
    /// coverage is not additionally clipped to a viewport.
    pub fn projected(&self, projection: MeshProjection) -> Rc<ProjectedConvexMesh> {
        let mut cache = self.projections.borrow_mut();
        if let Some(index) = cache.iter().position(|p| p.projection == projection) {
            let result = cache.remove(index);
            cache.push(result.clone());
            return result;
        }
        let result = self.mesh.projected(projection, None);
        if cache.len() == 4 {
            cache.remove(0);
        }
        cache.push(result.clone());
        result
    }
}

impl ProjectedConvexMesh {
    /// Computational clipping domain used when this surface was projected.
    /// Equal matrices alone do not imply equal clipped coverage.
    pub fn projection_viewport(&self) -> Option<Rect> {
        self.viewport
    }
    /// Actual surviving surface triangles in destination coordinates.
    pub fn triangles(&self) -> impl Iterator<Item = Triangle> + '_ {
        let count = self
            .clipped
            .as_ref()
            .map_or(self.mesh.indices.len() / 3, Vec::len);
        (0..count).map(|i| match &self.clipped {
            Some(triangles) => triangles[i],
            None => std::array::from_fn(|j| self.vertices[self.mesh.indices[i * 3 + j] as usize]),
        })
    }

    /// Conservative convex envelope. Use `front_contours` for exact occlusion
    /// because a clipped surface can leave openings within this envelope.
    pub fn silhouette(&self) -> Vec<[f64; 2]> {
        self.silhouette
            .get_or_init(|| convex_hull(self.triangles().flatten().map(|p| [p[0], p[1]]).collect()))
            .clone()
    }

    /// `None` is conservatively unbounded; an empty clipped mesh has zero coverage.
    pub fn coverage_bounds(&self) -> Option<Rect> {
        if self.unbounded {
            return None;
        }
        let mut points = self.triangles().flatten();
        let Some(p) = points.next() else {
            return Some(Rect::ZERO);
        };
        Some(points.fold(Rect::new(p[0], p[1], p[0], p[1]), |r, p| {
            r.union_pt((p[0], p[1]))
        }))
    }

    /// Closest rendered triangle depth; camera-clipped caps are not invented.
    pub fn depth_at(&self, x: f64, y: f64) -> Option<f64> {
        self.triangles()
            .filter_map(|[a, b, c]| {
                let det = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
                if det.abs() < 1e-12 {
                    return None;
                }
                let u = ((x - a[0]) * (c[1] - a[1]) - (y - a[1]) * (c[0] - a[0])) / det;
                let v = ((b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])) / det;
                (u >= -1e-9 && v >= -1e-9 && u + v <= 1. + 1e-9)
                    .then(|| a[2] + u * (b[2] - a[2]) + v * (c[2] - a[2]))
            })
            .max_by(f64::total_cmp)
    }

    /// Positive-winding contours whose nonzero union covers rendered surfaces
    /// closer than a plane. Clipped solids can have holes; never convexify them.
    /// This pure operation does not cache; retained consumers share the bounded
    /// [`crate::visibility::Relations`] service instead of per-pose history.
    pub fn front_contours(&self, plane: DepthPlane) -> Vec<Vec<[f64; 2]>> {
        self.compute_front_contours(plane)
    }

    /// Computes exact front contours without retaining a per-mesh cache entry.
    /// Runtime visibility uses this when it owns a shared, byte-bounded cache.
    pub fn compute_front_contours(&self, plane: DepthPlane) -> Vec<Vec<[f64; 2]>> {
        let [a, b, c] = plane.coefficients;
        let front = [-a, -b, 1., -c - 1e-7];
        if self.clipped.is_none() {
            if self.vertices.iter().all(|p| evaluate(front, *p) > 0.) {
                let hull = self.silhouette();
                return if hull.len() < 3 { vec![] } else { vec![hull] };
            }
            if self.vertices.iter().all(|p| evaluate(front, *p) < 0.) {
                return vec![];
            }
            let mut points = Vec::new();
            for triangle in self.triangles() {
                points.extend(clip(&triangle, front).into_iter().map(|p| [p[0], p[1]]));
            }
            let hull = convex_hull(points);
            return if hull.len() < 3 { vec![] } else { vec![hull] };
        }
        self.triangles()
            .filter_map(|triangle| {
                let mut polygon: Vec<_> = clip(&triangle, front)
                    .into_iter()
                    .map(|p| [p[0], p[1]])
                    .collect();
                if polygon.len() < 3 {
                    return None;
                }
                let area: f64 = (0..polygon.len())
                    .map(|i| {
                        let p = polygon[i];
                        let q = polygon[(i + 1) % polygon.len()];
                        p[0] * q[1] - q[0] * p[1]
                    })
                    .sum();
                if area.abs() < 1e-12 {
                    return None;
                }
                if area < 0. {
                    polygon.reverse();
                }
                Some(polygon)
            })
            .collect()
    }
}

fn row(m: [f64; 16], axis: usize) -> [f64; 4] {
    std::array::from_fn(|i| m[i * 4 + axis])
}
fn evaluate(row: [f64; 4], p: Point) -> f64 {
    row[0] * p[0] + row[1] * p[1] + row[2] * p[2] + row[3]
}
fn subtract(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn clip(points: &[Point], plane: [f64; 4]) -> Vec<Point> {
    let Some(&last) = points.last() else {
        return vec![];
    };
    let mut previous = last;
    let mut before = evaluate(plane, previous);
    let mut result = Vec::new();
    for &point in points {
        let after = evaluate(plane, point);
        if (before >= 0.) != (after >= 0.) {
            let t = before / (before - after);
            result.push(std::array::from_fn(|i| {
                previous[i] + t * (point[i] - previous[i])
            }));
        }
        if after >= 0. {
            result.push(point);
        }
        previous = point;
        before = after;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_projections_and_contours_are_bounded_and_follow_camera_clipping() {
        let mesh = ConvexMesh::sphere(24, 12);
        let identity = Transform3::<Generic>::identity().coeffs();
        let pose = mesh.posed(identity).unwrap();
        let full = MeshProjection::new(identity, false).unwrap();
        let cut = MeshProjection::new(identity, true).unwrap();
        let a = pose.projected(full);
        assert!(Rc::ptr_eq(&a, &pose.projected(full)));
        let b = pose.projected(cut);
        assert!(!Rc::ptr_eq(&a, &b));
        assert!(b.clipped.is_some());
        assert!(b.triangles().flatten().all(|p| p[2] >= 0.));
        for projected in [&a, &b] {
            for i in 0..64 {
                let plane = DepthPlane {
                    coefficients: [0.2, -0.1, i as f64 / 30. - 1.],
                    backface: false,
                };
                let expected = projected.compute_front_contours(plane);
                assert_eq!(projected.front_contours(plane), expected);
                assert_eq!(projected.front_contours(plane), expected);
            }
        }
        let weak = Rc::downgrade(&a);
        drop(a);
        for x in 1..16 {
            let translated = full.post_mapping(kurbo::Affine::translate((x as f64, 0.)).into());
            pose.projected(translated);
            assert!(pose.projections.borrow().len() <= 4);
        }
        assert!(
            weak.upgrade().is_none(),
            "evicted camera records must retire"
        );
        let last = pose.projected(full);
        let weak = Rc::downgrade(&last);
        drop(last);
        drop(pose);
        assert!(
            weak.upgrade().is_none(),
            "unmounted poses must retire their projections"
        );
    }
}
