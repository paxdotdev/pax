//! Camera math shared by spatial presentation and picking.
//!
//! These are Rust geometry contracts, not template camera elements or an active
//! renderer camera. The current renderer and native-control protocol remain 2D.
//! Orthographic projection preserves window x/y in logical pixels. Perspective
//! views look along -z, with x right and y down; clip depth is 0 at the near/front
//! plane and 1 at the far/back plane. View matrices move the world into camera
//! space without modifying local layout or scene identity.
//!
//! ```
//! use pax_runtime_api::{
//!     camera::{Camera, CameraProjection},
//!     math::{Point3, Transform3},
//! };
//!
//! let projection = CameraProjection::orthographic((800., 600.), 1000., -1000.).unwrap();
//! let camera: Camera = Camera::new(Transform3::identity(), projection).unwrap();
//! let screen = camera.project(Point3::new(120., 80., 0.)).unwrap();
//! assert!((screen.position.x - 120.).abs() < 1e-9);
//! assert!((screen.position.y - 80.).abs() < 1e-9);
//! assert_eq!(screen.depth, 0.5);
//! ```

use crate::math::{Generic, Plane3, Point2, Point3, Space, Transform3};
use crate::Window;

/// Camera coordinates: x right, y down, looking toward -z.
pub struct CameraSpace;
impl Space for CameraSpace {}

/// Homogeneous GPU clip coordinates, with visible depth in 0..=w.
pub struct ClipSpace;
impl Space for ClipSpace {}

/// A validated projection and its logical-pixel viewport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraProjection {
    matrix: Transform3<CameraSpace, ClipSpace>,
    viewport: (f64, f64),
}

impl CameraProjection {
    /// Creates the compatibility orthographic projection.
    ///
    /// Camera-space (0,0) maps to the window's top-left; width/height map to its
    /// bottom-right. `front_z > back_z` are finite camera-space z coordinates,
    /// not positive distances. Choose a range containing zero for existing 2D
    /// content. This utility does not impose a renderer's default depth range.
    pub fn orthographic(viewport: (f64, f64), front_z: f64, back_z: f64) -> Option<Self> {
        if !valid_viewport(viewport)
            || !front_z.is_finite()
            || !back_z.is_finite()
            || front_z <= back_z
        {
            return None;
        }
        let span = front_z - back_z;
        Self::checked(
            viewport,
            Transform3::new([
                2. / viewport.0,
                0.,
                0.,
                0.,
                0.,
                -2. / viewport.1,
                0.,
                0.,
                0.,
                0.,
                -1. / span,
                0.,
                -1.,
                1.,
                front_z / span,
                1.,
            ]),
        )
    }

    /// Creates a perspective projection looking along camera-space -z.
    ///
    /// `vertical_fov` is in radians and must be between 0 and pi; near/far are
    /// finite positive distances with far > near. The optical axis projects to
    /// the center of the viewport. No automatic camera pose or layout is added.
    pub fn perspective(
        viewport: (f64, f64),
        vertical_fov: f64,
        near: f64,
        far: f64,
    ) -> Option<Self> {
        if !valid_viewport(viewport)
            || !vertical_fov.is_finite()
            || vertical_fov <= 0.
            || vertical_fov >= std::f64::consts::PI
            || !near.is_finite()
            || !far.is_finite()
            || near <= 0.
            || far <= near
        {
            return None;
        }
        let f = (vertical_fov * 0.5).tan().recip();
        let depth_scale = far / (near - far);
        Self::checked(
            viewport,
            Transform3::new([
                f * viewport.1 / viewport.0,
                0.,
                0.,
                0.,
                0.,
                -f,
                0.,
                0.,
                0.,
                0.,
                depth_scale,
                -1.,
                0.,
                0.,
                depth_scale * near,
                0.,
            ]),
        )
    }

    fn checked(viewport: (f64, f64), matrix: Transform3<CameraSpace, ClipSpace>) -> Option<Self> {
        matrix.try_inverse()?;
        Some(Self { matrix, viewport })
    }

    /// Returns the camera-to-clip matrix in the shared spatial representation.
    pub fn matrix(self) -> Transform3<CameraSpace, ClipSpace> {
        self.matrix
    }

    /// Returns the viewport width and height in logical pixels.
    pub fn viewport(self) -> (f64, f64) {
        self.viewport
    }
}

fn valid_viewport((w, h): (f64, f64)) -> bool {
    w.is_finite() && h.is_finite() && w > 0. && h > 0.
}

/// Window position and normalized GPU depth of a projected point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectedPoint {
    /// Window coordinates in logical pixels; may lie outside the viewport.
    pub position: Point2<Window>,
    /// Near/front = 0, far/back = 1. Smaller values are nearer the camera.
    pub depth: f64,
}

/// An intersection with the infinite local z=0 plane.
///
/// The caller still applies authored bounds, apertures, and paint/depth ordering.
pub struct PlaneHit<L = Generic, W = Generic> {
    /// Intersection in the surface's unscaled local x/y coordinates.
    pub local: Point2<L>,
    /// Intersection in world coordinates.
    pub world: Point3<W>,
    /// Normalized depth, comparable with other hits through this camera.
    pub depth: f64,
    /// Whether the transformed x/y basis faces the camera ray.
    pub front_facing: bool,
}

/// An immutable view/projection pair with a cached inverse for picking.
///
/// Changing cameras constructs a new pair; it does not change the scene or native
/// ownership. This type does not select the runtime's active camera.
pub struct Camera<W = Generic> {
    world_to_clip: Transform3<W, ClipSpace>,
    clip_to_world: Transform3<ClipSpace, W>,
    viewport: (f64, f64),
}

impl<W: Space> Camera<W> {
    /// Combines an affine world-to-camera view with a projection.
    /// Rejects singular, non-finite, or projective view transforms.
    pub fn new(
        world_to_view: Transform3<W, CameraSpace>,
        projection: CameraProjection,
    ) -> Option<Self> {
        if !world_to_view.is_affine() {
            return None;
        }
        let world_to_clip = projection.matrix * world_to_view;
        Some(Self {
            world_to_clip,
            clip_to_world: world_to_clip.try_inverse()?,
            viewport: projection.viewport,
        })
    }

    /// Returns the exact matrix used by projection and picking.
    pub fn world_to_clip(&self) -> Transform3<W, ClipSpace> {
        self.world_to_clip
    }

    /// Projects a world point to logical window pixels and normalized depth.
    /// Rejects points behind the eye or outside the front/back clipping planes.
    /// Window x/y are not clipped, so callers can compute conservative coverage.
    pub fn project(&self, point: Point3<W>) -> Option<ProjectedPoint> {
        let [x, y, z, w] = self
            .world_to_clip
            .transform_homogeneous([point.x, point.y, point.z, 1.]);
        if w <= 0. || ![x, y, z, w].iter().all(|v| v.is_finite()) {
            return None;
        }
        let depth = z / w;
        // Accept arithmetic noise at clip boundaries, not a world-space slab.
        let epsilon = 64. * f64::EPSILON;
        if !(-epsilon..=1. + epsilon).contains(&depth) {
            return None;
        }
        let position = Point2::new(
            (x / w + 1.) * 0.5 * self.viewport.0,
            (1. - y / w) * 0.5 * self.viewport.1,
        );
        if !position.x.is_finite() || !position.y.is_finite() {
            return None;
        }
        Some(ProjectedPoint {
            position,
            depth: depth.clamp(0., 1.),
        })
    }

    /// Maps a window point at normalized depth 0..=1 back into world space.
    /// Depths 0 and 1 give the endpoints of the visible picking ray segment.
    pub fn unproject(&self, position: Point2<Window>, depth: f64) -> Option<Point3<W>> {
        if !(0.0..=1.0).contains(&depth) {
            return None;
        }
        self.clip_to_world.project_point(Point3::new(
            position.x / self.viewport.0 * 2. - 1.,
            1. - position.y / self.viewport.1 * 2.,
            depth,
        ))
    }

    /// Intersects the visible camera ray with a surface's local z=0 plane.
    ///
    /// Returns `None` for collapsed planes/non-affine transforms, numerically parallel
    /// rays, or intersections outside the camera's depth range. Both faces are
    /// pickable here; the caller owns back-face visibility and logical bounds.
    pub fn pick_plane<L: Space>(
        &self,
        position: Point2<Window>,
        local_to_world: Transform3<L, W>,
    ) -> Option<PlaneHit<L, W>> {
        let plane = Plane3::from_transform(local_to_world)?;
        // Picking needs the inverse of the x/y surface, not of its unused z
        // axis. Complete the basis with the plane normal so scale_z=0 (or a
        // z axis lying in the plane) does not make a valid flat surface unpickable.
        let mut basis = local_to_world.coeffs();
        let normal = plane.normal();
        basis[8] = normal.x;
        basis[9] = normal.y;
        basis[10] = normal.z;
        let world_to_local = Transform3::<L, W>::new(basis).try_inverse()?;
        let near = self.unproject(position, 0.)?;
        // Use an interior point to construct the ray: perspective unprojection
        // near the far plane loses precision as the near/far ratio grows.
        // The shared project() depth check below clips the actual intersection.
        let interior = self.unproject(position, 0.5)?;
        let delta = interior - near;
        let direction = delta.try_normalize()?;
        let facing = direction.dot(plane.normal());
        if facing.abs() <= 64. * f64::EPSILON {
            return None;
        }
        let t = (plane.origin() - near).dot(plane.normal()) / delta.dot(plane.normal());
        if !t.is_finite() {
            return None;
        }
        let world = near + delta * t;
        let local = world_to_local.project_point(world)?.xy();
        let depth = self.project(world)?.depth;
        Some(PlaneHit {
            local,
            world,
            depth,
            front_facing: facing < 0.,
        })
    }
}
