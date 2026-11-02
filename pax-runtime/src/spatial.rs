//! Retained scene geometry, before camera projection or native scrolling.

use pax_runtime_api::math::{Plane3, Transform2, Transform3};
use pax_runtime_api::{Interpolatable, Window};

use crate::{node_interface::NodeLocal, TransformAndBounds};

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq)]
enum RetainedTransform {
    Planar(Transform2<NodeLocal, Window>),
    Spatial(Transform3<NodeLocal, Window>),
}

/// A node's unprojected world transform and local layout dimensions.
///
/// World coordinates currently align with the unscrolled window. Composition
/// retains all three axes; projection is a separate operation. Existing 2D
/// trees retain the affine multiplication fast path. Bounds remain local and
/// do not change when a surface tilts or a camera moves.
///
/// Common spatial properties populate this retained geometry. The default
/// orthographic projection supplies exact affine footprints to native controls;
/// the GPU compositor separately retains plane depth and facing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpatialGeometry {
    transform: RetainedTransform,
    /// Untransformed width and height in logical pixels.
    pub bounds: (f64, f64),
}

impl Default for SpatialGeometry {
    fn default() -> Self {
        Self::from(TransformAndBounds::default())
    }
}

// Geometry is derived from authored pose values, never animated by interpolating
// matrix coefficients. This trait permits storage in the reactive graph.
impl Interpolatable for SpatialGeometry {}

impl From<TransformAndBounds<NodeLocal, Window>> for SpatialGeometry {
    fn from(value: TransformAndBounds<NodeLocal, Window>) -> Self {
        Self {
            transform: RetainedTransform::Planar(value.transform),
            bounds: value.bounds,
        }
    }
}

impl SpatialGeometry {
    /// World shading coordinates, independent of orthographic projection and tiles.
    /// Crossed tangents retain the surface's facing under shear and reflection;
    /// ordinary 2D reflections preserve the legacy +Z lighting normal.
    pub fn lighting_surface(self) -> Option<pax_runtime_api::LightingSurface> {
        let transform = self.world_transform();
        if !transform.is_affine() {
            return None;
        }
        let m = transform.coeffs();
        let normal = if self.is_spatial() {
            let n = self.plane()?.normal();
            [n.x as f32, n.y as f32, n.z as f32]
        } else {
            [0., 0., 1.]
        };
        let surface = pax_runtime_api::LightingSurface {
            origin: [m[12] as f32, m[13] as f32, m[14] as f32],
            tangent_x: [m[0] as f32, m[1] as f32, m[2] as f32],
            tangent_y: [m[4] as f32, m[5] as f32, m[6] as f32],
            normal,
        };
        surface
            .origin
            .iter()
            .chain(&surface.tangent_x)
            .chain(&surface.tangent_y)
            .chain(&surface.normal)
            .all(|v| v.is_finite())
            .then_some(surface)
    }

    /// Whether this value requires the spatial path rather than exact 2D affine arithmetic.
    pub fn is_spatial(self) -> bool {
        matches!(self.transform, RetainedTransform::Spatial(_))
    }

    /// Retains the depth equation while expressing x/y in a presentation surface.
    /// Returns `None` for the ordinary 2D fast path or an edge-on/invalid surface.
    pub fn depth_plane(
        self,
        surface: Transform2<NodeLocal, Window>,
    ) -> Option<pax_runtime_api::DepthPlane> {
        if matches!(self.transform, RetainedTransform::Planar(_)) {
            return None;
        }
        let inverse = surface.inverse();
        if !inverse.m.iter().all(|v| v.is_finite()) {
            return None;
        }
        let world = self.world_transform().coeffs();
        let coefficients = [
            world[2] * inverse.m[0] + world[6] * inverse.m[1],
            world[2] * inverse.m[2] + world[6] * inverse.m[3],
            world[2] * inverse.m[4] + world[6] * inverse.m[5] + world[14],
        ];
        coefficients
            .iter()
            .all(|v| v.is_finite())
            .then_some(pax_runtime_api::DepthPlane {
                presentation_layer: 0,
                coefficients,
                backface: self.plane()?.normal().z < 0.,
            })
    }
    /// Retains a spatial transform, recovering the 2D fast path only when exact.
    /// Invalid or projective matrices can be retained for inspection, but cannot
    /// produce an affine orthographic presentation or a world plane.
    pub fn new(transform: Transform3<NodeLocal, Window>, bounds: (f64, f64)) -> Self {
        Self {
            transform: transform
                .try_as_2d()
                .map(RetainedTransform::Planar)
                .unwrap_or(RetainedTransform::Spatial(transform)),
            bounds,
        }
    }

    /// Returns the complete transform, including depth and plane orientation.
    pub fn world_transform(self) -> Transform3<NodeLocal, Window> {
        match self.transform {
            RetainedTransform::Planar(transform) => transform.into(),
            RetainedTransform::Spatial(transform) => transform,
        }
    }

    /// Appends child-local geometry without projecting the parent first.
    pub fn append_local(
        self,
        transform: Transform3<NodeLocal, NodeLocal>,
        bounds: (f64, f64),
    ) -> Self {
        if let Some(transform) = transform.try_as_2d() {
            return self.append_local_2d(TransformAndBounds { transform, bounds });
        }
        Self::new(self.world_transform() * transform, bounds)
    }

    pub(crate) fn append_local_2d(self, local: TransformAndBounds<NodeLocal, NodeLocal>) -> Self {
        match self.transform {
            RetainedTransform::Planar(parent) => Self::from(TransformAndBounds {
                transform: parent * local.transform,
                bounds: local.bounds,
            }),
            RetainedTransform::Spatial(parent) => {
                Self::new(parent * Transform3::from(local.transform), local.bounds)
            }
        }
    }

    /// Projects the local z=0 plane onto the default orthographic x/y view.
    ///
    /// This preserves the footprint of any finite affine spatial transform,
    /// including an edge-on plane. It deliberately discards depth and must not
    /// be used to compose descendants or determine occlusion. Projective and
    /// non-finite spatial matrices return `None`. Existing planar matrices pass
    /// through unchanged, including the 2D pipeline's handling of invalid values.
    pub fn orthographic_projection(self) -> Option<TransformAndBounds<NodeLocal, Window>> {
        let transform = match self.transform {
            RetainedTransform::Planar(transform) => transform,
            RetainedTransform::Spatial(transform) => {
                if !transform.is_affine() {
                    return None;
                }
                let m = transform.coeffs();
                Transform2::new([m[0], m[1], m[4], m[5], m[12], m[13]])
            }
        };
        Some(TransformAndBounds {
            transform,
            bounds: self.bounds,
        })
    }

    /// Returns the oriented world plane, or `None` for invalid/collapsed x/y bases.
    /// A collapsed z basis does not collapse a flat surface.
    pub fn plane(self) -> Option<Plane3<Window>> {
        Plane3::from_transform(self.world_transform())
    }

    /// Tests geometric coplanarity, independently of front/back orientation.
    ///
    /// Tolerances are explicit: distance is in world pixels, angle is the sine
    /// between normals. `None` means at least one plane is undefined. This is
    /// only a geometric test; opacity, clipping, paint order and other isolation
    /// boundaries still govern whether two surfaces may be composited together.
    pub fn is_coplanar_with(
        self,
        other: Self,
        distance_tolerance: f64,
        angular_sine_tolerance: f64,
    ) -> Option<bool> {
        Some(
            self.plane()?
                .is_coplanar(&other.plane()?, distance_tolerance, angular_sine_tolerance),
        )
    }
}
