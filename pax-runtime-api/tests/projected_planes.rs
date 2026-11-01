use kurbo::{Affine, Point, Rect};
use pax_runtime_api::{
    camera::{Camera, CameraProjection, CameraSpace},
    math::{Generic, PlanarProjection, Point2, Point3, ProjectedPlane, Transform3, Vector3},
};

fn close(a: f64, b: f64) {
    assert!(
        (a - b).abs() < 1e-8 * a.abs().max(b.abs()).max(1.),
        "{a} != {b}"
    );
}

fn camera() -> Camera {
    Camera::new(
        Transform3::<Generic, CameraSpace>::identity(),
        CameraProjection::perspective((800., 600.), 1.1, 10., 1000.).unwrap(),
    )
    .unwrap()
}

#[test]
fn affine_projection_and_composition_preserve_exact_coordinates() {
    let affine = Affine::new([2., 0.25, -0.5, 3., 100., -70.]);
    let projection = PlanarProjection::from(affine);
    assert_eq!(projection.as_affine(), Some(affine));
    for point in [Point::ZERO, Point::new(12., -17.)] {
        assert_eq!(projection.project(point), Some(affine * point));
        let local = projection
            .inverse()
            .unwrap()
            .project(affine * point)
            .unwrap();
        close(local.x, point.x);
        close(local.y, point.y);
    }
    let offset = Affine::translate((41., -33.));
    assert_eq!(
        (PlanarProjection::from(offset) * projection).as_affine(),
        Some(offset * affine)
    );
    assert!(PlanarProjection::from(Affine::scale(0.))
        .inverse()
        .is_none());
}

#[test]
fn perspective_plane_matches_camera_projection_depth_and_picking() {
    let camera = camera();
    let pose = Transform3::<Generic>::translate(Vector3::new(-60., -30., -300.))
        * Transform3::<Generic>::rotate_y(0.6)
        * Transform3::<Generic>::rotate_x(-0.4);
    let plane = camera.project_plane(pose).unwrap();
    assert!(plane.mapping().as_affine().is_none());
    let depth = plane.depth_plane().unwrap();
    assert!(!depth.backface);
    for (x, y) in [(0., 0.), (100., 20.), (50., 80.)] {
        let local = Point::new(x, y);
        let world = pose.project_point(Point3::new(x, y, 0.)).unwrap();
        let expected = camera.project(world).unwrap();
        let screen = plane.mapping().project(local).unwrap();
        close(screen.x, expected.position.x);
        close(screen.y, expected.position.y);
        close(depth.depth_at(screen.x, screen.y), 1. - expected.depth);
        let inverse = plane.local_point(screen).unwrap();
        let hit = camera
            .pick_plane(Point2::new(screen.x, screen.y), pose)
            .unwrap();
        close(inverse.x, hit.local.x);
        close(inverse.y, hit.local.y);
    }
}

#[test]
fn near_and_eye_crossings_clip_edges_instead_of_discarding_whole_surfaces() {
    let camera = camera();
    let pose = Transform3::<Generic>::translate(Vector3::new(0., -20., -30.))
        * Transform3::<Generic>::rotate_y(-1.0);
    let plane = camera.project_plane(pose).unwrap();
    let local_bounds = Rect::new(0., 0., 100., 40.);
    let viewport = Rect::new(0., 0., 800., 600.);
    let local = plane.clip_polygon(
        vec![
            Point::new(0., 0.),
            Point::new(100., 0.),
            Point::new(100., 40.),
            Point::new(0., 40.),
        ],
        Some(viewport),
    );
    assert!(local.len() >= 3);
    assert!(local.iter().all(|p| plane.contains_local(*p)));
    assert!(local.iter().any(|p| p.x > 0. && p.x < 100.));
    let coverage = plane.coverage(local_bounds, Some(viewport)).unwrap();
    assert!(coverage.x0 >= -1e-8 && coverage.y0 >= -1e-8);
    assert!(coverage.x1 <= 800. + 1e-8 && coverage.y1 <= 600. + 1e-8);
    let clipped = plane.mapping().project(Point::new(30., 20.)).unwrap();
    assert!(plane.local_point(clipped).is_none());
}

#[test]
fn clipped_surfaces_do_not_leave_coverage_or_pickable_regions() {
    let camera = camera();
    for z in [0., 50., -5., -1001.] {
        let plane = camera
            .project_plane(Transform3::<Generic>::translate(Vector3::new(0., 0., z)))
            .unwrap();
        assert!(plane
            .coverage(Rect::new(0., 0., 100., 100.), None)
            .is_none());
        assert!(plane.local_point(Point::new(400., 300.)).is_none());
    }
    let edge = ProjectedPlane::affine(Affine::scale_non_uniform(0., 1.));
    assert!(edge.local_point(Point::ZERO).is_none());
    assert!(edge.depth_plane().is_none());
}

#[test]
fn magnification_bound_covers_the_entire_tilted_visible_surface() {
    let plane = camera()
        .project_plane(
            Transform3::<Generic>::translate(Vector3::new(-30., -30., -120.))
                * Transform3::<Generic>::rotate_y(-0.7),
        )
        .unwrap();
    let bounds = Rect::new(0., 0., 100., 100.);
    let viewport = Rect::new(0., 0., 800., 600.);
    let bound = plane.magnification_bound(bounds, Some(viewport)).unwrap();
    assert!(bound.is_finite() && bound > 0.);
    for x in 0..100 {
        for y in [0., 50., 99.] {
            let p = Point::new(x as f64, y);
            let screen = plane.mapping().project(p).unwrap();
            if !viewport.contains(screen) {
                continue;
            }
            for offset in [(0.001, 0.), (0., 0.001)] {
                let next = plane.mapping().project(p + offset).unwrap();
                assert!(screen.distance(next) / 0.001 <= bound * 1.00001);
            }
        }
    }
}

#[test]
fn rebasing_into_a_projected_owner_preserves_camera_clipping_and_input() {
    let camera = camera();
    let owner = camera
        .project_plane(
            Transform3::<Generic>::translate(Vector3::new(-100., -80., -400.))
                * Transform3::<Generic>::rotate_y(0.6),
        )
        .unwrap();
    let leaf = camera
        .project_plane(
            Transform3::<Generic>::translate(Vector3::new(0., -20., -30.))
                * Transform3::<Generic>::rotate_y(-1.),
        )
        .unwrap();
    let rebased = leaf.post_mapping(owner.mapping().inverse().unwrap());
    for x in [0., 10., 20., 30., 100.] {
        let p = Point::new(x, 20.);
        assert_eq!(leaf.contains_local(p), rebased.contains_local(p));
        if !leaf.contains_local(p) {
            continue;
        }
        let local = rebased.mapping().project(p).unwrap();
        let screen = owner.mapping().project(local).unwrap();
        let expected = leaf.mapping().project(p).unwrap();
        close(screen.x, expected.x);
        close(screen.y, expected.y);
        let hit = rebased.local_point(local).unwrap();
        close(hit.x, p.x);
        close(hit.y, p.y);
    }
    assert_ne!(rebased.mapping(), leaf.mapping());
    assert_eq!(rebased.camera_clip_rows(), leaf.camera_clip_rows());
}
