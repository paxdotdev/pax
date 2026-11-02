#![cfg(feature = "3d")]
use kurbo::{Affine, BezPath, Point, Rect, Shape};
use pax_runtime_api::{
    camera::{Camera, CameraProjection, CameraSpace},
    math::{Generic, Point3, Transform3, Vector3},
    mesh3::{ConvexMesh, MeshProjection},
    DepthPlane,
};

type Matrix = Transform3<Generic>;
fn view(near: f64, far: f64) -> Matrix {
    Camera::new(
        Transform3::<Generic, CameraSpace>::identity(),
        CameraProjection::perspective((128., 128.), 1.2, near, far).unwrap(),
    )
    .unwrap()
    .project_plane(Matrix::identity())
    .unwrap()
    .matrix()
}

// Independent mesh-local ray/triangle oracle; no projected triangles or silhouettes.
fn ray_depth(mesh: &ConvexMesh, projection: MeshProjection, x: f64, y: f64) -> Option<f64> {
    let inverse = Matrix::new(projection.matrix()).try_inverse().unwrap();
    let point = |z| inverse.project_point(Point3::new(x, y, z)).unwrap();
    let a = point(0.123);
    let b = point(0.321);
    let origin = a;
    let direction = b - a;
    mesh.indices()
        .chunks_exact(3)
        .filter_map(|indices| {
            let v: Vec<_> = indices
                .iter()
                .map(|i| {
                    let p = mesh.vertices()[*i as usize].position;
                    Point3::<Generic>::new(p[0] as f64, p[1] as f64, p[2] as f64)
                })
                .collect();
            let e1 = v[1] - v[0];
            let e2 = v[2] - v[0];
            let h = direction.cross(e2);
            let det = e1.dot(h);
            if det.abs() < 1e-12 {
                return None;
            }
            let s = origin - v[0];
            let u = s.dot(h) / det;
            let q = s.cross(e1);
            let v = direction.dot(q) / det;
            if u < -1e-9 || v < -1e-9 || u + v > 1. + 1e-9 {
                return None;
            }
            let p = origin + direction * (e2.dot(q) / det);
            let local = [p.x, p.y, p.z, 1.];
            let [z, w] = projection
                .camera_clip_rows()
                .map(|r| r.into_iter().zip(local).map(|(a, b)| a * b).sum::<f64>());
            if projection.clips_depth() && (w <= 0. || z < -1e-9 || z > w + 1e-9) {
                return None;
            }
            let m = projection.matrix();
            let w: f64 = (0..4).map(|i| m[i * 4 + 3] * local[i]).sum();
            (w > 0.).then(|| (0..4).map(|i| m[i * 4 + 2] * local[i]).sum::<f64>() / w)
        })
        .max_by(f64::total_cmp)
}

#[test]
fn camera_clipped_mesh_depth_and_occlusion_match_local_triangle_rays() {
    let mesh = ConvexMesh::sphere(24, 12);
    for (z, near, far) in [
        (-4., 0.1, 20.),
        (-1.3, 1., 20.),
        (-0.3, 0.5, 20.),
        (-5., 0.1, 5.2),
        (2., 0.1, 20.),
        (-50., 0.1, 20.),
    ] {
        let pose = Matrix::translate(Vector3::new(0., 0., z))
            * Matrix::rotate_y(0.6)
            * Matrix::scale(Vector3::new(1.4, 0.9, 1.));
        let projection = MeshProjection::new((view(near, far) * pose).coeffs(), true).unwrap();
        let projected = mesh.projected(projection, Some(Rect::new(0., 0., 128., 128.)));
        let plane = DepthPlane {
            presentation_layer: 0,
            coefficients: [0.0002, 0., 0.45],
            backface: false,
        };
        let mut path = BezPath::new();
        for p in projected.front_contours(plane) {
            path.move_to((p[0][0], p[0][1]));
            for p in &p[1..] {
                path.line_to((p[0], p[1]));
            }
            path.close_path();
        }
        for y in (3..128).step_by(7) {
            for x in (3..128).step_by(7) {
                let (x, y) = (x as f64 + 0.37, y as f64 + 0.29);
                let expected = ray_depth(&mesh, projection, x, y);
                let actual = projected.depth_at(x, y);
                assert_eq!(actual.is_some(), expected.is_some(), "z={z} at {x},{y}");
                if let (Some(a), Some(b)) = (actual, expected) {
                    assert!((a - b).abs() < 1e-8, "{a} != {b}");
                }
                assert_eq!(
                    path.contains(Point::new(x, y)),
                    expected.is_some_and(|z| z > plane.depth_at(x, y) + 1e-7),
                    "coverage z={z} at {x},{y}"
                );
            }
        }
    }
}

#[test]
fn clipping_does_not_invent_an_occluding_cap() {
    let mesh = ConvexMesh::sphere(32, 16);
    // The near plane removes the front half. The surviving back is behind the
    // queried panel, so its center stays interactive through the opening.
    let projection = MeshProjection::new(
        (view(2., 20.) * Matrix::translate(Vector3::new(0., 0., -2.))).coeffs(),
        true,
    )
    .unwrap();
    let projected = mesh.projected(projection, None);
    let mut path = BezPath::new();
    for p in projected.front_contours(DepthPlane {
        presentation_layer: 0,
        coefficients: [0., 0., 0.9],
        backface: false,
    }) {
        path.move_to((p[0][0], p[0][1]));
        for p in &p[1..] {
            path.line_to((p[0], p[1]));
        }
        path.close_path();
    }
    assert!(!path.contains(Point::new(64., 64.)));
    assert!((0..64).any(|x| path.contains(Point::new(x as f64, 64.))));
}

#[test]
fn canvas_chart_rebasing_preserves_camera_clip_equations() {
    let mesh = ConvexMesh::sphere(24, 12);
    let projection = MeshProjection::new(
        (view(1., 20.) * Matrix::translate(Vector3::new(0., 0., -1.2))).coeffs(),
        true,
    )
    .unwrap();
    let mapping =
        pax_runtime_api::math::PlanarProjection([1., 0., 0.002, 0., 1., 0., 20., -10., 1.]);
    let rebased = projection.post_mapping(mapping);
    assert_eq!(projection.camera_clip_rows(), rebased.camera_clip_rows());
    let a = mesh.projected(projection, None);
    let b = mesh.projected(rebased, None);
    for y in (10..120).step_by(11) {
        for x in (10..120).step_by(11) {
            let point = Point::new(x as f64, y as f64);
            let p = mapping.project(point).unwrap();
            let w = mapping.homogeneous(point)[2];
            let expected = a.depth_at(point.x, point.y).map(|z| z / w);
            let actual = b.depth_at(p.x, p.y);
            assert_eq!(actual.is_some(), expected.is_some());
            if let (Some(a), Some(b)) = (actual, expected) {
                assert!((a - b).abs() < 1e-8);
            }
        }
    }
    let affine = MeshProjection::new(Matrix::identity().coeffs(), false)
        .unwrap()
        .post_mapping(Affine::new([2., 0., 0., 3., 50., 60.]).into());
    assert!(mesh.projected(affine, None).depth_at(50., 60.).unwrap() > 0.99);
}
