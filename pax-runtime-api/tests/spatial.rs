use pax_runtime_api::{
    camera::{Camera, CameraProjection, CameraSpace},
    math::{Plane3, Point2, Point3, Space, Transform2, Transform3, Vector2, Vector3},
};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

struct Local;
struct World;
impl Space for Local {}
impl Space for World {}

fn close(a: f64, b: f64) {
    assert!(
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.),
        "{a} != {b}"
    );
}

fn same_point<W: Space>(a: Point3<W>, b: Point3<W>) {
    close(a.x, b.x);
    close(a.y, b.y);
    close(a.z, b.z);
}

fn ortho() -> Camera<World> {
    Camera::new(
        Transform3::identity(),
        CameraProjection::orthographic((800., 600.), 1000., -1000.).unwrap(),
    )
    .unwrap()
}

#[test]
fn identity_and_affine_embedding_preserve_existing_2d_geometry() {
    let transforms: [Transform2<Local, World>; 5] = [
        Transform2::identity(),
        Transform2::translate(Vector2::new(123., -45.)),
        Transform2::rotate(1.23),
        Transform2::new([1.2, -0.4, 0.7, -2., 36., 78.]),
        Transform2::scale(0.),
    ];
    assert_eq!(Transform3::<World>::default(), Transform3::identity());
    for affine in transforms {
        let spatial = Transform3::from(affine);
        assert_eq!(spatial.try_as_2d(), Some(affine));
        for (x, y, z) in [(0., 0., 0.), (0.5, 0.25, 0.), (-4., 70., 30.)] {
            let old = affine * Point2::new(x, y);
            let world = spatial.project_point(Point3::new(x, y, z)).unwrap();
            same_point(world, Point3::new(old.x, old.y, z));
            let projected = ortho().project(world).unwrap();
            close(projected.position.x, old.x);
            close(projected.position.y, old.y);
        }
    }
    // CSS matrix3d and the shared representation preserve the z basis.
    assert_eq!(
        Transform3::from(transforms[3]).coeffs(),
        [1.2, -0.4, 0., 0., 0.7, -2., 0., 0., 0., 0., 1., 0., 36., 78., 0., 1.]
    );
}

#[test]
fn composition_and_z_rotation_match_2d_conventions() {
    let a: Transform2<World> = Transform2::translate(Vector2::new(300., 120.));
    let b: Transform2<Local, World> = Transform2::rotate(FRAC_PI_4);
    assert_eq!(
        Transform3::from(a * b),
        Transform3::from(a) * Transform3::from(b)
    );
    assert_eq!(Transform3::from(b), Transform3::rotate_z(FRAC_PI_4));
    let point = Point3::new(1., 0., 0.);
    same_point(
        Transform3::<World>::rotate_z(FRAC_PI_2)
            .project_point(point)
            .unwrap(),
        Point3::new(0., 1., 0.),
    );
    same_point(
        Transform3::<World>::rotate_x(FRAC_PI_2)
            .project_point(Point3::new(0., 1., 0.))
            .unwrap(),
        Point3::new(0., 0., 1.),
    );
    same_point(
        Transform3::<World>::rotate_y(FRAC_PI_2)
            .project_point(point)
            .unwrap(),
        Point3::new(0., 0., -1.),
    );
    assert!(Transform3::<World>::translate(Vector3::new(0., 0., 1e-20))
        .try_as_2d()
        .is_none());
    assert!(Transform3::<World>::rotate_y(1e-20).try_as_2d().is_none());
}

#[test]
fn inverse_roundtrips_nested_spatial_transforms_and_tiny_scales() {
    for scale in [1., -2., 1e-8, 1e-150] {
        let transform: Transform3<Local, World> = Transform3::<World>::rotate_y(0.7)
            * Transform3::<World>::rotate_x(-0.2)
            * Transform3::scale(Vector3::new(scale, 2. * scale, scale));
        let inverse = transform.try_inverse().unwrap();
        let point = Point3::new(3., -4., 5.);
        same_point(
            inverse
                .project_point(transform.project_point(point).unwrap())
                .unwrap(),
            point,
        );
        assert!(Plane3::from_transform(transform).is_some());
    }
    let transform: Transform3<Local, World> =
        Transform3::<World>::translate(Vector3::new(50., 60., 70.))
            * Transform3::<World>::rotate_z(0.4)
            * Transform3::rotate_y(0.8);
    let point = Point3::new(30., 20., 10.);
    same_point(
        transform
            .try_inverse()
            .unwrap()
            .project_point(transform.project_point(point).unwrap())
            .unwrap(),
        point,
    );
}

#[test]
fn invalid_transforms_fail_without_nan_geometry() {
    assert!(Transform3::<World>::scale(Vector3::new(0., 1., 1.))
        .try_inverse()
        .is_none());
    assert!(
        Transform3::<World>::translate(Vector3::new(f64::NAN, 0., 0.))
            .try_inverse()
            .is_none()
    );
    assert!(
        Transform3::<World>::scale(Vector3::new(f64::INFINITY, 1., 1.))
            .try_as_2d()
            .is_none()
    );
    assert!(Transform3::<World>::new([0.; 16])
        .project_point(Point3::new(1., 2., 3.))
        .is_none());
    assert!(Vector3::<World>::new(0., 0., 0.).try_normalize().is_none());
    let normalized = Vector3::<World>::new(1e300, -1e300, 1e300)
        .try_normalize()
        .unwrap();
    close(normalized.dot(normalized), 1.);
}

#[test]
fn coplanarity_follows_world_planes_and_distinguishes_face_orientation() {
    let group: Transform3<World> = Transform3::<World>::translate(Vector3::new(100., 200., 30.))
        * Transform3::<World>::rotate_y(FRAC_PI_4)
        * Transform3::rotate_x(0.25);
    let parent = Plane3::from_transform(group).unwrap();
    let coplanar: Transform3<Local, World> = group
        * Transform3::<Local, World>::translate(Vector3::new(15., 30., 0.))
        * Transform3::<Local>::rotate_z(0.4)
        * Transform3::scale(Vector3::new(-2., 3., 1.));
    let reversed = Plane3::from_transform(coplanar).unwrap();
    assert!(parent.is_coplanar(&reversed, 1e-9, 1e-12));
    assert!(parent.normal().dot(reversed.normal()) < 0.);
    let flipped = Plane3::from_transform(group * Transform3::<Local, World>::rotate_x(PI)).unwrap();
    assert!(parent.is_coplanar(&flipped, 1e-9, 1e-12));
    let tilted =
        Plane3::from_transform(group * Transform3::<Local, World>::rotate_y(FRAC_PI_4)).unwrap();
    assert!(!parent.is_coplanar(&tilted, 1e-9, 1e-12));
    let lifted = Plane3::from_transform(
        group * Transform3::<Local, World>::translate(Vector3::new(0., 0., 0.01)),
    )
    .unwrap();
    assert!(!parent.is_coplanar(&lifted, 1e-9, 1e-12));
    assert!(!parent.is_coplanar(&reversed, f64::NAN, 1e-12));
    assert!(!parent.is_coplanar(&reversed, 1e-9, f64::INFINITY));
}

#[test]
fn equal_rotation_angles_with_different_pivots_need_not_share_a_plane() {
    let origin_rotation: Transform3<World> = Transform3::rotate_y(FRAC_PI_4);
    let pivot_rotation = Transform3::translate(Vector3::new(100., 0., 0.))
        * origin_rotation
        * Transform3::<World>::translate(Vector3::new(-100., 0., 0.));
    assert!(!Plane3::from_transform(origin_rotation)
        .unwrap()
        .is_coplanar(
            &Plane3::from_transform(pivot_rotation).unwrap(),
            1e-9,
            1e-12
        ));
    assert!(Plane3::from_transform(Transform3::<World>::scale(Vector3::new(0., 1., 1.))).is_none());
    assert!(Plane3::from_transform(Transform3::<World>::scale(Vector3::new(1., 1., 0.))).is_some());
}

#[test]
fn orthographic_projection_preserves_xy_and_orders_depth() {
    let camera = ortho();
    for (z, depth) in [(1000., 0.), (200., 0.4), (0., 0.5), (-1000., 1.)] {
        let world = Point3::new(123., 456., z);
        let projected = camera.project(world).unwrap();
        close(projected.position.x, 123.);
        close(projected.position.y, 456.);
        close(projected.depth, depth);
        same_point(
            camera
                .unproject(projected.position, projected.depth)
                .unwrap(),
            world,
        );
    }
    assert!(camera.project(Point3::new(0., 0., 1001.)).is_none());
    assert!(camera.project(Point3::new(0., 0., -1001.)).is_none());
    assert!(camera.project(Point3::new(-100., 900., 0.)).is_some());
}

#[test]
fn perspective_has_correct_axes_depth_and_inverse() {
    let projection = CameraProjection::perspective((400., 200.), FRAC_PI_2, 1., 1000.).unwrap();
    let camera: Camera<World> = Camera::new(Transform3::identity(), projection).unwrap();
    for (point, x, y) in [
        (Point3::new(0., 0., -100.), 200., 100.),
        (Point3::new(100., 50., -100.), 300., 150.),
        (Point3::new(100., 50., -200.), 250., 125.),
    ] {
        let screen = camera.project(point).unwrap();
        close(screen.position.x, x);
        close(screen.position.y, y);
        same_point(
            camera.unproject(screen.position, screen.depth).unwrap(),
            point,
        );
    }
    close(camera.project(Point3::new(0., 0., -1.)).unwrap().depth, 0.);
    close(
        camera.project(Point3::new(0., 0., -1000.)).unwrap().depth,
        1.,
    );
    for z in [0., 10., -0.5, -1001.] {
        assert!(camera.project(Point3::new(0., 0., z)).is_none());
    }
    assert!(Plane3::from_transform(projection.matrix()).is_none());
}

#[test]
fn picking_matches_projection_for_tilted_surfaces_in_both_camera_types() {
    let cameras = [
        ortho(),
        Camera::new(
            Transform3::identity(),
            CameraProjection::perspective((800., 600.), 1., 1., 2000.).unwrap(),
        )
        .unwrap(),
    ];
    let surface: Transform3<Local, World> =
        Transform3::<World>::translate(Vector3::new(200., 100., -400.))
            * Transform3::<World>::rotate_y(0.7)
            * Transform3::rotate_x(0.2);
    for camera in cameras {
        for local in [
            Point3::new(0., 0., 0.),
            Point3::new(100., 50., 0.),
            Point3::new(-50., 200., 0.),
        ] {
            let world = surface.project_point(local).unwrap();
            let screen = camera.project(world).unwrap();
            let hit = camera.pick_plane(screen.position, surface).unwrap();
            close(hit.local.x, local.x);
            close(hit.local.y, local.y);
            same_point(hit.world, world);
            close(hit.depth, screen.depth);
            assert!(hit.front_facing);
        }
    }
}

#[test]
fn picking_rejects_edge_on_collapsed_and_out_of_depth_surfaces() {
    let camera = ortho();
    for transform in [
        Transform3::<Local, World>::rotate_y(FRAC_PI_2),
        Transform3::scale(Vector3::new(0., 1., 1.)),
        Transform3::translate(Vector3::new(0., 0., 1100.)),
        Transform3::translate(Vector3::new(0., 0., -1100.)),
    ] {
        assert!(camera
            .pick_plane(Point2::new(10., 20.), transform)
            .is_none());
    }
    let backwards: Transform3<Local, World> = Transform3::rotate_y(PI);
    assert!(
        !camera
            .pick_plane(Point2::new(10., 20.), backwards)
            .unwrap()
            .front_facing
    );
    let tiny: Transform3<Local, World> = Transform3::scale(Vector3::new(1e-8, 1e-8, 1.));
    let hit = camera.pick_plane(Point2::new(100., 200.), tiny).unwrap();
    close(hit.local.x, 1e10);
    close(hit.local.y, 2e10);
    let flat: Transform3<Local, World> = Transform3::scale(Vector3::new(2., 3., 0.));
    let hit = camera.pick_plane(Point2::new(100., 150.), flat).unwrap();
    close(hit.local.x, 50.);
    close(hit.local.y, 50.);
}

#[test]
fn camera_changes_leave_world_plane_and_local_geometry_unchanged() {
    let projection = CameraProjection::orthographic((800., 600.), 1000., -1000.).unwrap();
    let before: Camera<World> = Camera::new(Transform3::identity(), projection).unwrap();
    let after = Camera::new(
        Transform3::<World, CameraSpace>::translate(Vector3::new(-80., 40., -20.)),
        projection,
    )
    .unwrap();
    let surface: Transform3<Local, World> =
        Transform3::<World>::translate(Vector3::new(300., 200., 30.)) * Transform3::rotate_y(0.4);
    let world = surface.project_point(Point3::new(10., 20., 0.)).unwrap();
    let a = before.project(world).unwrap();
    let b = after.project(world).unwrap();
    close(b.position.x, a.position.x - 80.);
    close(b.position.y, a.position.y + 40.);
    for camera in [before, after] {
        let hit = camera
            .pick_plane(camera.project(world).unwrap().position, surface)
            .unwrap();
        close(hit.local.x, 10.);
        close(hit.local.y, 20.);
    }
}

#[test]
fn invalid_camera_configuration_and_nonfinite_input_are_rejected() {
    for viewport in [(0., 100.), (-1., 100.), (100., f64::INFINITY)] {
        assert!(CameraProjection::orthographic(viewport, 100., -100.).is_none());
        assert!(CameraProjection::perspective(viewport, 1., 1., 100.).is_none());
    }
    for (front, back) in [(0., 0.), (-1., 1.), (f64::NAN, 0.), (f64::MAX, -f64::MAX)] {
        assert!(CameraProjection::orthographic((800., 600.), front, back).is_none());
    }
    for (fov, near, far) in [
        (0., 1., 10.),
        (PI, 1., 10.),
        (1., 0., 10.),
        (1., 10., 1.),
        (1., 1., f64::INFINITY),
    ] {
        assert!(CameraProjection::perspective((800., 600.), fov, near, far).is_none());
    }
    let projection = CameraProjection::orthographic((800., 600.), 100., -100.).unwrap();
    assert!(
        Camera::<World>::new(Transform3::scale(Vector3::new(0., 1., 1.)), projection).is_none()
    );
    assert!(ortho().unproject(Point2::new(0., 0.), f64::NAN).is_none());
    assert!(ortho()
        .project(Point3::new(f64::INFINITY, 0., 0.))
        .is_none());
    assert!(ortho()
        .pick_plane(
            Point2::new(f64::NAN, 0.),
            Transform3::<Local, World>::identity()
        )
        .is_none());
}

#[test]
fn picking_and_projection_agree_at_clipping_planes() {
    let cameras: [(Camera<World>, [f64; 2]); 2] = [
        (ortho(), [1000., -1000.]),
        (
            Camera::new(
                Transform3::identity(),
                CameraProjection::perspective((800., 600.), 1., 1., 2000.).unwrap(),
            )
            .unwrap(),
            [-1., -2000.],
        ),
    ];
    for (camera, zs) in cameras {
        for z in zs {
            let surface: Transform3<Local, World> = Transform3::translate(Vector3::new(0., 0., z));
            let world = Point3::new(20., 30., z);
            let projected = camera.project(world).unwrap();
            let hit = camera.pick_plane(projected.position, surface).unwrap();
            close(hit.local.x, 20.);
            close(hit.local.y, 30.);
            close(hit.depth, projected.depth);
        }
    }
}

#[test]
fn depth_order_is_camera_relative_and_can_reverse() {
    let projection = CameraProjection::orthographic((800., 600.), 1000., -1000.).unwrap();
    let front: Camera<World> = Camera::new(Transform3::identity(), projection).unwrap();
    let back = Camera::new(Transform3::<World, CameraSpace>::rotate_y(PI), projection).unwrap();
    let a = Point3::new(0., 0., 100.);
    let b = Point3::new(0., 0., -100.);
    assert!(front.project(a).unwrap().depth < front.project(b).unwrap().depth);
    assert!(back.project(a).unwrap().depth > back.project(b).unwrap().depth);
    let surface: Transform3<Local, World> = Transform3::identity();
    let center = back.project(Point3::new(0., 0., 0.)).unwrap().position;
    assert!(!back.pick_plane(center, surface).unwrap().front_facing);
}
