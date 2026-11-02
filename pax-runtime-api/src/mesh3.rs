//! Retained convex surface geometry for the opt-in spatial renderer.
//!
//! Mesh topology is immutable. Poses retain projected coverage independently of
//! GPU resources; clipping a convex volume against a plane never splits its mesh.

use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

use crate::DepthPlane;

pub mod projection;
pub use projection::{MeshProjection, ProjectedConvexMesh};

/// One local-space mesh vertex. Normals are outward-facing; UVs span the surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

/// An immutable, closed convex mesh. This type's coverage algorithms require
/// convexity; arbitrary concave triangle soups must not use this contract.
#[derive(Debug)]
pub struct ConvexMesh {
    vertices: Vec<MeshVertex>,
    indices: Vec<u32>,
    edges: Vec<[u32; 2]>,
}

impl ConvexMesh {
    /// Unit sphere centered at the origin. Ellipsoids scale this shared topology.
    pub fn sphere(longitudes: usize, latitudes: usize) -> Rc<Self> {
        let longitudes = longitudes.clamp(8, 256);
        let latitudes = latitudes.clamp(4, 128);
        let mut vertices = Vec::with_capacity((longitudes + 1) * (latitudes + 1));
        for y in 0..=latitudes {
            let v = y as f64 / latitudes as f64;
            let phi = v * std::f64::consts::PI;
            for x in 0..=longitudes {
                let u = x as f64 / longitudes as f64;
                let theta = u * std::f64::consts::TAU;
                let p = [
                    (phi.sin() * theta.cos()) as f32,
                    (-phi.cos()) as f32,
                    (phi.sin() * theta.sin()) as f32,
                ];
                vertices.push(MeshVertex {
                    position: p,
                    normal: p,
                    uv: [u as f32, v as f32],
                });
            }
        }
        let mut indices = Vec::with_capacity(longitudes * (latitudes - 1) * 6);
        for y in 0..latitudes {
            for x in 0..longitudes {
                let a = (y * (longitudes + 1) + x) as u32;
                let b = a + longitudes as u32 + 1;
                if y != 0 {
                    indices.extend([a, a + 1, b]);
                }
                if y + 1 != latitudes {
                    indices.extend([a + 1, b + 1, b]);
                }
            }
        }
        let mut edges = Vec::with_capacity(indices.len());
        for triangle in indices.chunks_exact(3) {
            for i in 0..3 {
                let (a, b) = (triangle[i], triangle[(i + 1) % 3]);
                edges.push([a.min(b), a.max(b)]);
            }
        }
        edges.sort_unstable();
        edges.dedup();
        Rc::new(Self {
            vertices,
            indices,
            edges,
        })
    }

    pub fn vertices(&self) -> &[MeshVertex] {
        &self.vertices
    }
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Retains a finite, nonsingular affine pose. Matrices are column-major.
    pub fn posed(self: &Rc<Self>, transform: [f64; 16]) -> Option<Rc<PosedConvexMesh>> {
        if !transform.iter().all(|v| v.is_finite())
            || transform[3] != 0.
            || transform[7] != 0.
            || transform[11] != 0.
            || transform[15] != 1.
        {
            return None;
        }
        let determinant = transform[0]
            * (transform[5] * transform[10] - transform[9] * transform[6])
            - transform[4] * (transform[1] * transform[10] - transform[9] * transform[2])
            + transform[8] * (transform[1] * transform[6] - transform[5] * transform[2]);
        if !determinant.is_finite() || determinant.abs() < 1e-12 {
            return None;
        }
        let vertices: Vec<_> = self
            .vertices
            .iter()
            .map(|v| {
                std::array::from_fn(|i| {
                    transform[i] * v.position[0] as f64
                        + transform[4 + i] * v.position[1] as f64
                        + transform[8 + i] * v.position[2] as f64
                        + transform[12 + i]
                })
            })
            .collect();
        if !vertices.iter().flatten().all(|v| v.is_finite()) {
            return None;
        }
        Some(Rc::new(PosedConvexMesh {
            mesh: self.clone(),
            transform,
            vertices,
            silhouette: OnceCell::new(),
            projections: RefCell::new(Vec::new()),
        }))
    }
}

/// One affine instance of a convex mesh, in a chosen world/presentation space.
#[derive(Debug)]
pub struct PosedConvexMesh {
    pub mesh: Rc<ConvexMesh>,
    pub transform: [f64; 16],
    vertices: Vec<[f64; 3]>,
    silhouette: OnceCell<Vec<[f64; 2]>>,
    // Native siblings, physical tiles, and fade views often ask about the same
    // plane. Bound residency even when a stationary solid meets animated planes.
    // Unscrolled, window, and canvas charts share this mounted world pose.
    projections: RefCell<Vec<Rc<ProjectedConvexMesh>>>,
}

impl PosedConvexMesh {
    pub fn vertices(&self) -> &[[f64; 3]] {
        &self.vertices
    }

    /// Projected silhouette of the tessellated solid in these x/y coordinates.
    pub fn silhouette(&self) -> Vec<[f64; 2]> {
        self.silhouette
            .get_or_init(|| convex_hull(self.vertices.iter().map(|p| [p[0], p[1]]).collect()))
            .clone()
    }

    /// Projects the portion closer than a planar UI surface. Edge/plane crossings
    /// are included, so a panel passing through the volume gets the intersection
    /// contour, not merely a subset of the solid's existing silhouette vertices.
    /// This unprojected helper is uncached. Camera-aware consumers use
    /// [`crate::visibility::Relations`] with the actual clipped projection.
    pub fn coverage_in_front(&self, plane: DepthPlane) -> Vec<[f64; 2]> {
        self.compute_front_coverage(plane)
    }

    fn compute_front_coverage(&self, plane: DepthPlane) -> Vec<[f64; 2]> {
        let distances: Vec<_> = self
            .vertices
            .iter()
            .map(|p| p[2] - plane.depth_at(p[0], p[1]))
            .collect();
        if distances.iter().all(|d| *d > 1e-7) {
            return self.silhouette();
        }
        if distances.iter().all(|d| *d <= 1e-7) {
            return Vec::new();
        }
        let mut points: Vec<_> = self
            .vertices
            .iter()
            .zip(&distances)
            .filter(|(_, d)| **d > 1e-7)
            .map(|(p, _)| [p[0], p[1]])
            .collect();
        for &[a, b] in &self.mesh.edges {
            let (a, b) = (a as usize, b as usize);
            let (da, db) = (distances[a], distances[b]);
            if (da > 1e-7) != (db > 1e-7) {
                let t = da / (da - db);
                let (a, b) = (self.vertices[a], self.vertices[b]);
                points.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
            }
        }
        convex_hull(points)
    }

    /// Nearest surface depth under orthographic viewing, matching the rendered
    /// triangles rather than a different analytic approximation of the solid.
    pub fn depth_at(&self, x: f64, y: f64) -> Option<f64> {
        let mut depth: Option<f64> = None;
        for triangle in self.mesh.indices.chunks_exact(3) {
            let [a, b, c] = [
                self.vertices[triangle[0] as usize],
                self.vertices[triangle[1] as usize],
                self.vertices[triangle[2] as usize],
            ];
            let det = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            if det.abs() < 1e-12 {
                continue;
            }
            let u = ((x - a[0]) * (c[1] - a[1]) - (y - a[1]) * (c[0] - a[0])) / det;
            let v = ((b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])) / det;
            if u >= -1e-9 && v >= -1e-9 && u + v <= 1. + 1e-9 {
                let z = a[2] + u * (b[2] - a[2]) + v * (c[2] - a[2]);
                depth = Some(depth.map_or(z, |old| old.max(z)));
            }
        }
        depth
    }
}

/// Counterclockwise projected hull. Collinear/duplicate vertices are removed.
pub fn convex_hull(mut points: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    points.retain(|p| p.iter().all(|v| v.is_finite()));
    points.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    points.dedup();
    if points.len() < 3 {
        return Vec::new();
    }
    let half = |points: Vec<[f64; 2]>| {
        let mut hull: Vec<[f64; 2]> = Vec::new();
        for p in points {
            while hull.len() >= 2 {
                let a = hull[hull.len() - 2];
                let b = hull[hull.len() - 1];
                if (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]) > 1e-12 {
                    break;
                }
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop();
        hull
    };
    let mut hull = half(points.clone());
    points.reverse();
    hull.extend(half(points));
    if hull.len() < 3 {
        Vec::new()
    } else {
        hull
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn identity() -> [f64; 16] {
        [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
        ]
    }

    #[test]
    fn unprojected_coverage_matches_kernel_without_retaining_the_pose() {
        let mesh = ConvexMesh::sphere(64, 32);
        let pose = mesh.posed(identity()).unwrap();
        for i in 0..96 {
            let plane = DepthPlane {
                coefficients: [0.2, -0.1, i as f64 / 40. - 1.2],
                backface: false,
            };
            let expected = pose.compute_front_coverage(plane);
            assert_eq!(pose.coverage_in_front(plane), expected);
            assert_eq!(pose.coverage_in_front(plane), expected);
        }
        let mut transform = identity();
        transform[14] = 2.;
        let moved = mesh.posed(transform).unwrap();
        let plane = DepthPlane {
            coefficients: [0., 0., 0.8],
            backface: false,
        };
        assert_ne!(
            moved.coverage_in_front(plane),
            pose.coverage_in_front(plane)
        );
        let weak = Rc::downgrade(&pose);
        drop(pose);
        assert!(
            weak.upgrade().is_none(),
            "coverage queries do not retain removed poses"
        );
    }

    #[test]
    fn clipped_convex_coverage_matches_triangle_depth() {
        let mesh = ConvexMesh::sphere(40, 20);
        let mut m = identity();
        m[4] = 0.3;
        m[8] = -0.4;
        m[10] = 1.7;
        let pose = mesh.posed(m).unwrap();
        for plane in [
            DepthPlane::default(),
            DepthPlane {
                coefficients: [0.4, -0.2, 0.7],
                backface: false,
            },
            DepthPlane {
                coefficients: [0., 0., 3.],
                backface: false,
            },
        ] {
            let hull = pose.coverage_in_front(plane);
            for y in -20..=20 {
                for x in -20..=20 {
                    let (x, y) = (x as f64 * 0.06 + 0.001, y as f64 * 0.06 + 0.003);
                    let actual = hull.len() >= 3
                        && (0..hull.len()).all(|i| {
                            let a = hull[i];
                            let b = hull[(i + 1) % hull.len()];
                            (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0]) >= -1e-9
                        });
                    let expected = pose
                        .depth_at(x, y)
                        .is_some_and(|z| z > plane.depth_at(x, y) + 1e-7);
                    assert_eq!(actual, expected, "at {x},{y}, plane={plane:?}");
                }
            }
        }
        assert_eq!(mesh.indices().len(), 40 * 19 * 6);
        assert!(mesh.posed([0.; 16]).is_none());
    }
}

/// Settled opaque mesh draw, retaining shared topology across replay and motion.
#[derive(Clone, Debug)]
pub struct OpaqueMeshDraw {
    pub mesh: Rc<ConvexMesh>,
    /// Local mesh to unscrolled world, used for lighting.
    pub world: [f64; 16],
    /// Local mesh to homogeneous canvas-content coordinates, with camera clipping.
    pub surface: MeshProjection,
    pub color: crate::Color,
    pub material: crate::ResolvedMaterial,
}
