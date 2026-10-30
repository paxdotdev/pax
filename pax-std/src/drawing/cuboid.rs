use crate::*;
use pax_engine::api::{Depth, Fill, LocalProperty, Property, Store};
use pax_engine::pax;
use pax_runtime::api::NodeContext;

/// A rectangular solid whose six surfaces participate in spatial composition.
///
/// `width` and `height` use ordinary local layout; `depth` is an absolute length
/// centered around local Z=0. The default depth is 100 logical pixels. All faces
/// receive `fill`; add [`CuboidFace`] children to place live content on individual
/// faces. Face content keeps its mounted identity when the solid rotates.
///
/// Uses the experimental GPU spatial compositor, including its native-layer
/// restrictions. The current view is orthographic; materials retain the current
/// renderer's material model with the position and oriented normal of each face.
#[pax]
#[engine_import_path("pax_engine")]
#[file("drawing/cuboid.pax")]
#[custom(Default)]
pub struct Cuboid {
    /// Solid extent along local Z, in logical pixels. Negative/non-finite values
    /// collapse to zero. Percentages are not accepted.
    pub depth: Property<Depth>,
    /// Paint shared by all six faces. Empty paints leave only authored face content.
    pub fill: Property<Vec<Fill>>,
    // Reactive face geometry, shared with the authored face containers.
    pub _faces: Property<Vec<CuboidSurface>>,
}

impl Default for Cuboid {
    fn default() -> Self {
        Self {
            depth: Property::new(Depth::from(100.0)),
            fill: Property::new(vec![Fill::default()]),
            _faces: Default::default(),
        }
    }
}

/// Outward-facing sides of a [`Cuboid`], before its own transforms are applied.
#[pax]
#[engine_import_path("pax_engine")]
pub enum CuboidSide {
    /// Positive Z; faces the default viewer.
    #[default]
    Front,
    /// Negative Z.
    Back,
    /// Positive X.
    Right,
    /// Negative X.
    Left,
    /// Negative Y.
    Top,
    /// Positive Y.
    Bottom,
}

/// Places an ordinary live subtree on one face of the nearest owning [`Cuboid`].
///
/// Front/back content is width × height, left/right is depth × height, and
/// top/bottom is width × depth. Content is upright when viewed from outside its
/// face. A projected rectangular aperture clips it to that face. Normal layout,
/// events, native controls, and further spatial transforms work inside the face.
/// Changing `side` moves the same subtree; it does not recreate its children.
/// Several face containers may contribute to one side in ordinary paint order.
#[pax]
#[engine_import_path("pax_engine")]
#[file("drawing/cuboid_face.pax")]
pub struct CuboidFace {
    /// The face receiving this subtree; defaults to Front.
    pub side: Property<CuboidSide>,
    // Face placement relative to the cuboid's local layout rectangle.
    pub _surface: Property<CuboidSurface>,
}

// Public for generated cartridge descriptors; not a separate authoring API.
#[pax]
#[engine_import_path("pax_engine")]
pub struct CuboidSurface {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub width: f64,
    pub height: f64,
    pub rotate_x: f64,
    pub rotate_y: f64,
}

#[derive(Clone)]
struct CuboidGeometry {
    bounds: LocalProperty<(f64, f64)>,
    depth: LocalProperty<Depth>,
}

impl Store for CuboidGeometry {}

impl Cuboid {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let geometry = CuboidGeometry {
            bounds: ctx.bounds_self.clone(),
            depth: self.depth.local(),
        };
        ctx.provide_store(geometry.clone())
            .expect("Cuboid geometry is provided during mount");
        let deps = [geometry.bounds.untyped(), geometry.depth.untyped()];
        self._faces.replace_with(LocalProperty::computed(
            move || {
                [
                    CuboidSide::Front,
                    CuboidSide::Back,
                    CuboidSide::Right,
                    CuboidSide::Left,
                    CuboidSide::Top,
                    CuboidSide::Bottom,
                ]
                .into_iter()
                .map(|side| surface(geometry.bounds.get(), geometry.depth.get(), side))
                .collect()
            },
            &deps,
        ));
    }
}

impl CuboidFace {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let geometry = ctx
            .with_store(|geometry: &mut CuboidGeometry| geometry.clone())
            .expect("CuboidFace requires an owning Cuboid");
        let side = self.side.local();
        let deps = [
            geometry.bounds.untyped(),
            geometry.depth.untyped(),
            side.untyped(),
        ];
        self._surface.replace_with(LocalProperty::computed(
            move || surface(geometry.bounds.get(), geometry.depth.get(), side.get()),
            &deps,
        ));
    }
}

fn surface((width, height): (f64, f64), depth: Depth, side: CuboidSide) -> CuboidSurface {
    let depth = depth.to_float();
    let depth = if depth.is_finite() {
        depth.max(0.0)
    } else {
        0.0
    };
    let mut face = CuboidSurface {
        x: width / 2.0,
        y: height / 2.0,
        z: 0.0,
        width,
        height,
        rotate_x: 0.0,
        rotate_y: 0.0,
    };
    match side {
        CuboidSide::Front => face.z = depth / 2.0,
        CuboidSide::Back => {
            face.z = -depth / 2.0;
            face.rotate_y = 180.0;
        }
        CuboidSide::Right => {
            face.x = width;
            face.width = depth;
            face.rotate_y = 90.0;
        }
        CuboidSide::Left => {
            face.x = 0.0;
            face.width = depth;
            face.rotate_y = -90.0;
        }
        CuboidSide::Top => {
            face.y = 0.0;
            face.height = depth;
            face.rotate_x = 90.0;
        }
        CuboidSide::Bottom => {
            face.y = height;
            face.height = depth;
            face.rotate_x = -90.0;
        }
    }
    face
}

#[cfg(test)]
mod tests {
    use super::*;
    use pax_engine::api::math::{Generic, Plane3, Point3, Transform3, Vector3};

    fn transform(face: &CuboidSurface) -> Transform3 {
        Transform3::<Generic>::translate(Vector3::new(face.x, face.y, face.z))
            * Transform3::<Generic>::rotate_y(face.rotate_y.to_radians())
            * Transform3::<Generic>::rotate_x(face.rotate_x.to_radians())
            * Transform3::<Generic>::translate(Vector3::new(
                -face.width / 2.0,
                -face.height / 2.0,
                0.0,
            ))
    }

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-8, "{a} != {b}");
    }

    #[test]
    fn six_faces_close_at_eight_shared_corners_and_point_outward() {
        let sides = [
            CuboidSide::Front,
            CuboidSide::Back,
            CuboidSide::Right,
            CuboidSide::Left,
            CuboidSide::Top,
            CuboidSide::Bottom,
        ];
        let mut corners = std::collections::BTreeMap::new();
        for side in sides {
            let face = surface((320.0, 180.0), Depth::from(140.0), side);
            let transform = transform(&face);
            let plane = Plane3::from_transform(transform).unwrap();
            let outward = Vector3::new(face.x - 160.0, face.y - 90.0, face.z);
            assert!(plane.normal().dot(outward) > 0.0);
            for (x, y) in [
                (0.0, 0.0),
                (face.width, 0.0),
                (face.width, face.height),
                (0.0, face.height),
            ] {
                let point = transform.project_point(Point3::new(x, y, 0.0)).unwrap();
                let key = (
                    point.x.round() as i32,
                    point.y.round() as i32,
                    point.z.round() as i32,
                );
                close(point.x, key.0 as f64);
                close(point.y, key.1 as f64);
                close(point.z, key.2 as f64);
                assert!([0, 320].contains(&key.0));
                assert!([0, 180].contains(&key.1));
                assert!([-70, 70].contains(&key.2));
                *corners.entry(key).or_insert(0) += 1;
            }
        }
        assert_eq!(corners.len(), 8);
        assert!(corners.values().all(|count| *count == 3));
    }

    #[test]
    fn adjacent_route_faces_fill_a_rectangular_viewport_after_quarter_turns() {
        for (side, rx, ry, depth) in [
            (CuboidSide::Right, 0_f64, -90_f64, 960.0),
            (CuboidSide::Left, 0.0, 90.0, 960.0),
            (CuboidSide::Top, -90.0, 0.0, 540.0),
            (CuboidSide::Bottom, 90.0, 0.0, 540.0),
        ] {
            let face = surface((960.0, 540.0), Depth::from(depth), side);
            let rotation = Transform3::<Generic>::translate(Vector3::new(480.0, 270.0, 0.0))
                * Transform3::<Generic>::rotate_y(ry.to_radians())
                * Transform3::<Generic>::rotate_x(rx.to_radians())
                * Transform3::<Generic>::translate(Vector3::new(-480.0, -270.0, 0.0));
            let placement = rotation * transform(&face);
            let first = placement.project_point(Point3::new(0.0, 0.0, 0.0)).unwrap();
            let last = placement
                .project_point(Point3::new(face.width, face.height, 0.0))
                .unwrap();
            close(first.x, 0.0);
            close(first.y, 0.0);
            close(last.x, 960.0);
            close(last.y, 540.0);
            close(first.z, depth / 2.0);
            close(last.z, depth / 2.0);
        }
    }

    #[test]
    fn invalid_depth_collapses_without_poisoning_face_transforms() {
        for depth in [-100.0, f64::NAN, f64::INFINITY] {
            let front = surface((100.0, 80.0), Depth::from(depth), CuboidSide::Front);
            let side = surface((100.0, 80.0), Depth::from(depth), CuboidSide::Right);
            assert_eq!(front.z, 0.0);
            assert_eq!(side.width, 0.0);
            assert!(transform(&front).is_affine());
            assert!(transform(&side).is_affine());
        }
    }
}
