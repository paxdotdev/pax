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
    surface: OnceCell<ProjectedSurface>,
    local_box: OnceCell<Option<Rect>>,
    depth: OnceCell<Option<[f64; 2]>>,
    silhouette: OnceCell<Vec<[f64; 2]>>,
    bounds: OnceCell<Option<(Point, Point)>>,
}

#[derive(Debug)]
struct ProjectedSurface {
    vertices: Vec<Point>,
    clipped: Option<Vec<Triangle>>,
    unbounded: bool,
}

impl ConvexMesh {
    /// Projects the actual mesh surface, clipping before division. Supplying a
    /// viewport also bounds a canvas chart whose projective horizon crosses it.
    pub fn projected(
        self: &Rc<Self>,
        projection: MeshProjection,
        viewport: Option<Rect>,
    ) -> Rc<ProjectedConvexMesh> {
        Rc::new(ProjectedConvexMesh {
            mesh: self.clone(),
            projection,
            viewport,
            surface: OnceCell::new(),
            local_box: OnceCell::new(),
            depth: OnceCell::new(),
            silhouette: OnceCell::new(),
            bounds: OnceCell::new(),
        })
    }

    fn project_surface(
        &self,
        projection: MeshProjection,
        viewport: Option<Rect>,
    ) -> ProjectedSurface {
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
        ProjectedSurface {
            vertices,
            clipped,
            unbounded,
        }
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
    fn surface(&self) -> &ProjectedSurface {
        self.surface
            .get_or_init(|| self.mesh.project_surface(self.projection, self.viewport))
    }

    /// Broad-phase bounds for culling and candidate selection. Exact clipped
    /// triangles are prepared on demand when the box touches a camera plane or
    /// projective horizon. An unbounded projection returns `None`.
    pub fn conservative_coverage_bounds(&self) -> Option<Rect> {
        match self.projected_local_box() {
            Some(bounds) => Some(self.viewport.map_or(bounds, |view| bounds.intersect(view))),
            None => self.coverage_bounds(),
        }
    }

    fn projected_local_box(&self) -> Option<Rect> {
        *self.local_box.get_or_init(|| self.compute_local_box())
    }

    fn compute_local_box(&self) -> Option<Rect> {
        // A linear fractional coordinate with positive w takes its extrema
        // at corners of a convex box. Both mesh constructors fit [-1, 1]^3.
        let mut bounds: Option<Rect> = None;
        for x in [-1., 1.] {
            for y in [-1., 1.] {
                for z in [-1., 1.] {
                    let p = [x, y, z];
                    if self.projection.clip_depth {
                        let [depth, w] = self.projection.clip_rows.map(|r| evaluate(r, p));
                        if !(depth >= 0. && depth <= w) {
                            return None;
                        }
                    }
                    let Some([x, y, _]) = self.projection.project(p) else {
                        return None;
                    };
                    let point = kurbo::Point::new(x, y);
                    bounds =
                        Some(bounds.map_or(Rect::from_points(point, point), |r| r.union_pt(point)));
                }
            }
        }
        bounds
    }

    fn surface_points(&self) -> impl Iterator<Item = Point> + '_ {
        let surface = self.surface();
        let indices = if surface.clipped.is_none() {
            self.mesh.surface_vertices.as_slice()
        } else {
            &[]
        };
        surface
            .clipped
            .iter()
            .flat_map(|triangles| triangles.iter().flatten().copied())
            .chain(indices.iter().map(|i| surface.vertices[*i as usize]))
    }

    fn bounds(&self) -> Option<(Point, Point)> {
        *self.bounds.get_or_init(|| {
            let mut points = self.surface_points();
            let first = points.next()?;
            Some(points.fold((first, first), |(min, max), p| {
                (
                    std::array::from_fn(|i| min[i].min(p[i])),
                    std::array::from_fn(|i| max[i].max(p[i])),
                )
            }))
        })
    }

    /// Computational clipping domain used when this surface was projected.
    /// Equal matrices alone do not imply equal clipped coverage.
    pub fn projection_viewport(&self) -> Option<Rect> {
        self.viewport
    }
    /// Actual surviving surface triangles in destination coordinates.
    pub fn triangles(&self) -> impl Iterator<Item = Triangle> + '_ {
        let surface = self.surface();
        let count = surface
            .clipped
            .as_ref()
            .map_or(self.mesh.indices.len() / 3, Vec::len);
        (0..count).map(|i| match &surface.clipped {
            Some(triangles) => triangles[i],
            None => {
                std::array::from_fn(|j| surface.vertices[self.mesh.indices[i * 3 + j] as usize])
            }
        })
    }

    /// Conservative convex envelope. Use `front_contours` for exact occlusion
    /// because a clipped surface can leave openings within this envelope.
    pub fn silhouette(&self) -> Vec<[f64; 2]> {
        self.silhouette
            .get_or_init(|| convex_hull(self.surface_points().map(|p| [p[0], p[1]]).collect()))
            .clone()
    }

    /// `None` is conservatively unbounded; an empty clipped mesh has zero coverage.
    pub fn coverage_bounds(&self) -> Option<Rect> {
        if self.surface().unbounded {
            return None;
        }
        Some(self.bounds().map_or(Rect::ZERO, |(min, max)| {
            Rect::new(min[0], min[1], max[0], max[1])
        }))
    }

    /// Retained minimum and maximum z/w of the surviving surface vertices.
    /// Empty surfaces return `None`; clipped surfaces exclude discarded vertices.
    pub fn depth_bounds(&self) -> Option<[f64; 2]> {
        *self.depth.get_or_init(|| {
            if self.surface.get().is_none()
                && self.projected_local_box().is_some_and(|bounds| {
                    self.viewport
                        .is_none_or(|viewport| viewport.contains_rect(bounds))
                })
            {
                // The entire mesh survived clipping. Depth normalization needs
                // only the z/w of indexed vertices, not projected x/y or hulls.
                let z = row(self.projection.matrix, 2);
                let w = row(self.projection.matrix, 3);
                let mut range = [f64::INFINITY, f64::NEG_INFINITY];
                for &index in &self.mesh.surface_vertices {
                    let p = self.mesh.vertices[index as usize].position.map(f64::from);
                    let depth = evaluate(z, p) / evaluate(w, p);
                    range[0] = range[0].min(depth);
                    range[1] = range[1].max(depth);
                }
                return Some(range);
            }
            self.bounds().map(|(min, max)| [min[2], max[2]])
        })
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
        if self.surface().clipped.is_none() {
            if self
                .surface()
                .vertices
                .iter()
                .all(|p| evaluate(front, *p) > 0.)
            {
                let hull = self.silhouette();
                return if hull.len() < 3 { vec![] } else { vec![hull] };
            }
            if self
                .surface()
                .vertices
                .iter()
                .all(|p| evaluate(front, *p) < 0.)
            {
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
    fn broad_phase_and_unclipped_depth_leave_exact_surface_lazy() {
        let mesh = ConvexMesh::sphere(24, 12);
        let transform = [
            2., 0.2, 0.5, 0., 0.4, 3., -0.3, 0., 0.8, 0.1, 4., 0., 20., -7., 3., 1.,
        ];
        let posed = mesh.posed(transform).unwrap();
        let projected = posed.projected(MeshProjection::new(transform, false).unwrap());
        let broad = projected.conservative_coverage_bounds().unwrap();
        let depth = projected.depth_bounds().unwrap();
        assert!(posed.vertices.get().is_none());
        assert!(projected.surface.get().is_none());
        let exact = projected.coverage_bounds().unwrap();
        assert!(broad.contains_rect(exact));
        let (lo, hi) = projected.bounds().unwrap();
        assert_eq!(depth, [lo[2], hi[2]]);
        assert!(projected.depth_at(20., -7.).is_some());
    }

    #[test]
    fn retained_bounds_and_hulls_match_surviving_triangles() {
        let mut mesh = ConvexMesh::sphere(24, 12);
        // Bounds must describe indexed surface geometry, even if a vertex is
        // unused. This also guards the UV seam/pole vertices of sphere meshes.
        let unused = super::super::MeshVertex {
            position: [100., 100., 100.],
            normal: [0., 0., 1.],
            uv: [0., 0.],
        };
        Rc::get_mut(&mut mesh).unwrap().vertices.push(unused);
        let identity = Transform3::<Generic>::identity().coeffs();
        let affine = [
            2., 0.2, 0.5, 0., 0.4, 3., -0.3, 0., 0.8, 0.1, 4., 0., 20., -7., 3., 1.,
        ];
        let perspective = [
            2., 0., 0., 0., 0., 2., 0., 0., 0., 0., 1., 0.5, 0., 0., 0.5, 1.,
        ];
        for (matrix, camera_clip, viewport) in [
            (identity, false, None),
            (affine, false, None),
            (identity, true, None),
            (perspective, true, None),
            (identity, false, Some(Rect::new(-0.25, -0.4, 0.7, 0.6))),
            (identity, false, Some(Rect::new(4., 4., 5., 5.))),
        ] {
            let projected =
                mesh.projected(MeshProjection::new(matrix, camera_clip).unwrap(), viewport);
            let points: Vec<_> = projected.triangles().flatten().collect();
            let expected_hull = convex_hull(points.iter().map(|p| [p[0], p[1]]).collect());
            let expected_depth = points.first().map(|first| {
                points.iter().fold([first[2], first[2]], |[lo, hi], p| {
                    [lo.min(p[2]), hi.max(p[2])]
                })
            });
            let expected_bounds = (!projected.surface().unbounded).then(|| {
                points.first().map_or(Rect::ZERO, |first| {
                    points
                        .iter()
                        .fold(Rect::new(first[0], first[1], first[0], first[1]), |r, p| {
                            r.union_pt((p[0], p[1]))
                        })
                })
            });
            for _ in 0..2 {
                assert_eq!(projected.silhouette(), expected_hull);
                assert_eq!(projected.depth_bounds(), expected_depth);
                assert_eq!(projected.coverage_bounds(), expected_bounds);
            }
        }
    }

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
        assert!(b.surface().clipped.is_some());
        assert!(b.triangles().flatten().all(|p| p[2] >= 0.));
        for projected in [&a, &b] {
            for i in 0..64 {
                let plane = DepthPlane {
                    presentation_layer: 0,
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
