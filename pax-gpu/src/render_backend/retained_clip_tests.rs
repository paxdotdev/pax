//! Hardware regression coverage for retained image/vector clip ordering.
//! Run with `cargo test -p pax-gpu retained_clip_pixels -- --ignored` on macOS.

use super::*;
use crate::{point, Angle, Color, Fill, Path, Vector2D, WgpuRenderer};
use objc2::{
    msg_send,
    runtime::{AnyClass, AnyObject},
};

#[link(name = "QuartzCore", kind = "framework")]
extern "C" {}

struct MetalLayer(*mut AnyObject);

#[test]
#[ignore = "requires a macOS Metal device"]
fn attachment_retirement_source_resize_capture_eviction_and_shutdown() {
    let layer = MetalLayer::new();
    let mut backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 32, 32, [1., 1.]),
        )
    })
    .unwrap();
    let device = backend.device.clone();
    let context = backend.shared_context();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let key = capture::CaptureKey::Alpha(1);
    // Different feather domains replace the source stencil and capture scratch
    // attachments while earlier clear/copy commands remain unsubmitted.
    for margin in [2., 12., 0., 8.] {
        assert!(backend.begin_source_domain(margin));
        backend.capture_is_cached(key, 0);
        backend.begin_capture();
        backend.end_capture_rect(key, 0, (0, 0, 16, 16));
        backend.end_source_domain();
    }
    backend.retain_captures(&HashSet::new());
    let batch = backend.take_pending_command_buffers();
    assert!(!batch.is_empty());
    // Shutdown must not destroy the detached batch's retired attachments early.
    drop(backend);
    context.submit_command_buffers(batch);
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn attachment_retirement_resize_discards_unsubmitted_source_work() {
    let layer = MetalLayer::new();
    let mut backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 32, 32, [1., 1.]),
        )
    })
    .unwrap();
    let device = backend.device.clone();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    assert!(backend.begin_source_domain(4.));
    backend.end_source_domain();
    backend.resize_surface(64, 48);
    assert!(backend.take_pending_command_buffers().is_empty());
    assert!(backend.begin_source_domain(8.));
    backend.end_source_domain();
    backend.submit_pending_commands();
    drop(backend);
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn order_metadata_preserves_geometry_and_opacity_captures_but_reorders_pixels() {
    use pax_runtime_api::OpacityScope;
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 32, 32, [1., 1.]),
        )
    })
    .unwrap();
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    for (id, color) in [
        (1, Color::rgba(1., 0., 0., 1.)),
        (2, Color::rgba(0., 0., 1., 1.)),
    ] {
        renderer.begin_node(id, id as i32, 0);
        renderer.set_node_opacity_scopes(
            id,
            &[OpacityScope {
                node_id: 10,
                opacity: 0.5,
            }],
        );
        renderer.fill_path(rect(0., 0., 32., 32.), Fill::Solid(color));
        renderer.end_node(id);
    }
    renderer.request_screenshot_capture(0);
    renderer.flush();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    assert_pixel(
        &renderer.take_screenshot_capture(0).unwrap(),
        8,
        8,
        [0, 0, 128, 128],
    );
    renderer.take_resource_churn_stats();
    assert!(!renderer.update_node_orders(&[(1, 101), (2, 102)]));
    renderer.request_screenshot_capture(1);
    renderer.flush();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    assert_pixel(
        &renderer.take_screenshot_capture(1).unwrap(),
        8,
        8,
        [0, 0, 128, 128],
    );
    let stats = renderer.take_resource_churn_stats();
    assert_eq!(stats.opacity_group_renders, 0);
    assert_eq!(stats.vector_geometry_rebuilds, 0);
    assert_eq!(stats.vector_resource_creates, 0);
    assert!(stats.opacity_group_cache_hits > 0);
    assert!(renderer.update_node_orders(&[(1, 103)]));
    renderer.request_screenshot_capture(2);
    renderer.flush();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    assert_pixel(
        &renderer.take_screenshot_capture(2).unwrap(),
        8,
        8,
        [128, 0, 0, 128],
    );
    assert_eq!(
        renderer.take_resource_churn_stats().opacity_group_renders,
        1
    );
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn order_metadata_keeps_mask_sources_private_and_cached() {
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 32, 32, [1., 1.]),
        )
    })
    .unwrap();
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    renderer.begin_node(10, 10, 0);
    renderer.save();
    renderer.begin_alpha_source(Transform2D::identity(), 0.0);
    for id in [11, 12] {
        renderer.begin_node(id, 0, 0);
        renderer.fill_path(
            rect(0., 0., 16., 32.),
            Fill::Solid(Color::rgba(1., 1., 1., 1.)),
        );
        renderer.end_node(id);
    }
    renderer.end_alpha_source();
    renderer.fill_path(
        rect(0., 0., 32., 32.),
        Fill::Solid(Color::rgba(0., 1., 0., 1.)),
    );
    renderer.restore();
    renderer.end_node(10);
    renderer.flush();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    renderer.take_resource_churn_stats();
    assert!(!renderer.update_node_orders(&[(10, 20), (11, 500)]));
    renderer.request_screenshot_capture(1);
    renderer.flush();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let pixels = renderer.take_screenshot_capture(1).unwrap();
    assert_pixel(&pixels, 8, 8, [0, 255, 0, 255]);
    assert_pixel(&pixels, 24, 8, [0; 4]);
    let stats = renderer.take_resource_churn_stats();
    assert_eq!(stats.alpha_source_renders, 0);
    assert!(stats.alpha_source_cache_hits > 0);
    assert_eq!(stats.vector_geometry_rebuilds, 0);
}

impl MetalLayer {
    fn new() -> Self {
        // The owned layer outlives its wgpu surface; no AppKit window is needed.
        Self(unsafe { msg_send![AnyClass::get(c"CAMetalLayer").unwrap(), new] })
    }
}

impl Drop for MetalLayer {
    fn drop(&mut self) {
        unsafe {
            let _: () = msg_send![self.0, release];
        }
    }
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Path {
    let mut builder = Path::builder();
    builder.begin(point(x, y));
    builder.line_to(point(x + width, y));
    builder.line_to(point(x + width, y + height));
    builder.line_to(point(x, y + height));
    builder.end(true);
    builder.build()
}

fn assert_pixel(frame: &CapturedFrame, x: usize, y: usize, expected: [u8; 4]) {
    let offset = (y * frame.width as usize + x) * 4;
    assert_eq!(
        &frame.rgba[offset..offset + 4],
        &expected,
        "pixel ({x}, {y})"
    );
}

#[cfg(feature = "3d")]
#[test]
#[ignore = "requires a macOS Metal device"]
fn perspective_vectors_images_and_stencil_share_clipped_projection() {
    use pax_runtime_api::{
        camera::{Camera, CameraProjection, CameraSpace},
        math::{Generic, Point3, Transform3, Vector3},
    };
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 128, 96, [1., 1.]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    let camera: Camera = Camera::new(
        Transform3::<Generic, CameraSpace>::identity(),
        CameraProjection::perspective((128., 96.), std::f64::consts::FRAC_PI_2, 10., 1000.)
            .unwrap(),
    )
    .unwrap();
    let image = Image {
        rgba: vec![0, 255, 0, 255],
        pixel_width: 1,
        pixel_height: 1,
    };
    let poses = [
        Transform3::<Generic>::translate(Vector3::new(-40., -30., -80.))
            * Transform3::<Generic>::rotate_y(0.8),
        Transform3::<Generic>::translate(Vector3::new(0., -10., -30.))
            * Transform3::<Generic>::rotate_y(-1.),
        Transform3::<Generic>::translate(Vector3::new(-40., -30., -80.))
            * Transform3::<Generic>::rotate_y(0.8),
        Transform3::<Generic>::translate(Vector3::new(0., -10., -30.))
            * Transform3::<Generic>::rotate_y(-1.),
    ];
    for (index, pose) in poses.into_iter().enumerate() {
        let plane = camera.project_plane(pose).unwrap();
        let plane = if index == 3 {
            plane.post_mapping(pax_runtime_api::math::PlanarProjection([
                1., 0., 0.002, 0., 1., 0., 0., 0., 1.,
            ]))
        } else {
            plane
        };
        renderer.begin_node(0, 0, 0);
        renderer.fill_path(
            rect(0., 0., 128., 96.),
            Fill::Solid(Color::rgba(0., 0., 0., 1.)),
        );
        renderer.end_node(0);
        renderer.begin_node(1, 1, 0);
        renderer.save();
        renderer.project(plane);
        renderer.fill_path(
            rect(0., 0., 80., 60.),
            Fill::Solid(Color::rgba(1., 0., 0., 1.)),
        );
        renderer.restore();
        renderer.end_node(1);
        renderer.begin_node(2, 2, 0);
        renderer.save();
        renderer.project(plane);
        renderer.clip(rect(10., 10., 60., 40.));
        renderer.clip_alpha(
            vec![crate::AlphaMaskPaint {
                path: rect(12., 0., 56., 60.),
                transform: Transform2D::identity(),
                fill: Fill::Solid(Color::rgba(1., 1., 1., 1.)),
                opacity: 1.,
                composition: None,
            }],
            0.,
        );
        renderer.draw_image(
            "perspective-green",
            1,
            &image,
            Box2D::new(point(0., 0.), point(80., 60.)),
        );
        renderer.restore();
        renderer.end_node(2);
        renderer.request_screenshot_capture(index as u32);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let captured = renderer
            .take_screenshot_capture(index as u32)
            .expect("captured frame");
        // Pick several interior points through exactly the same matrix used by
        // native/input projection; include the partially near-clipped frame.
        for (x, y, color) in [(5., 20., [255, 0, 0, 255]), (15., 20., [0, 255, 0, 255])] {
            let world = pose.project_point(Point3::new(x, y, 0.)).unwrap();
            let _ = camera
                .project(world)
                .expect("sample lies inside the camera clip range");
            let screen = plane.mapping().project(kurbo::Point::new(x, y)).unwrap();
            assert_pixel(
                &captured,
                screen.x.floor() as usize,
                screen.y.floor() as usize,
                color,
            );
        }
    }
}

#[cfg(all(feature = "3d", feature = "shaders"))]
#[test]
#[ignore = "requires a macOS Metal device"]
fn perspective_effect_and_detached_source_follow_camera_and_tile_charts() {
    use pax_runtime_api::{
        camera::{Camera, CameraProjection, CameraSpace},
        math::{Generic, PlanarProjection, Transform3, Vector3},
        ShaderEffect, ShaderInput, ShaderPass, ShaderProgram,
    };
    static PROGRAM: ShaderProgram = ShaderProgram {
        label: "projected local sampling",
        passes: &[ShaderPass {
            label: "projected-local.wgsl",
            source: "fn effect(p: vec2<f32>) -> vec4<f32> { let a = sample_input0(p - vec2<f32>(5.0,0.0)).a; return select(vec4<f32>(a,0.0,0.0,a),vec4<f32>(0.0,a,0.0,a),p.x >= 35.0); }",
            parameters: &[], inputs: [ShaderInput::Subtree; 2], downsample: 1,
        }],
    };
    let camera: Camera = Camera::new(
        Transform3::<Generic, CameraSpace>::identity(),
        CameraProjection::perspective((128., 96.), std::f64::consts::FRAC_PI_2, 10., 1000.)
            .unwrap(),
    )
    .unwrap();
    let poses = [
        Transform3::<Generic>::translate(Vector3::new(-40., -30., -80.))
            * Transform3::<Generic>::rotate_y(0.8),
        Transform3::<Generic>::translate(Vector3::new(-40., -30., -60.))
            * Transform3::<Generic>::rotate_y(0.5),
        Transform3::<Generic>::translate(Vector3::new(-20., -15., -30.))
            * Transform3::<Generic>::rotate_y(-0.9),
    ];
    for (width, tile_x) in [(128., 0.), (64., 0.), (64., 64.)] {
        let layer = MetalLayer::new();
        let backend = pollster::block_on(unsafe {
            RenderBackend::to_core_animation_layer(
                layer.0.cast(),
                RenderConfig::new(true, width as u32, 96, [1., 1.]),
            )
        })
        .unwrap();
        let device = backend.device.clone();
        let mut renderer = WgpuRenderer::new(backend);
        renderer.set_surface_transform(Transform2D::translation(-tile_x, 0.));
        for frame in 0..6 {
            let plane = camera
                .project_plane(poses[frame / 2])
                .unwrap()
                .post_mapping(PlanarProjection([1., 0., 0.001, 0., 1., 0., 0., 0., 1.]));
            let effect = ShaderEffect {
                program: &PROGRAM,
                parameters: vec![vec![]],
                outset: 6.,
            };
            renderer.begin_projected_shader_effect(1, 1, &effect, plane, (80., 60.), &[], &[2, 4]);
            renderer.begin_node(2, 1, 0);
            renderer.save();
            renderer.begin_alpha_source(Transform2D::identity(), 0.);
            renderer.begin_node(3, 1, 0);
            renderer.save();
            renderer.project(plane);
            renderer.fill_path(
                rect(10., 0., 60., 60.),
                Fill::Solid(Color::rgba(1., 1., 1., 1.)),
            );
            renderer.restore();
            renderer.end_node(3);
            renderer.end_alpha_source();
            renderer.end_node(2);
            renderer.begin_node(4, 2, 0);
            renderer.save();
            renderer.project(plane);
            renderer.fill_path(
                rect(0., 0., 80., 60.),
                Fill::Solid(Color::rgba(1., 1., 1., 1.)),
            );
            renderer.restore();
            renderer.end_node(4);
            renderer.restore();
            renderer.end_shader_effect();
            renderer.request_screenshot_capture(frame as u32);
            renderer.flush();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let pixels = renderer.take_screenshot_capture(frame as u32).unwrap();
            for (x, expected) in [
                (6., [0, 0, 0, 0]),
                (22., [255, 0, 0, 255]),
                (48., [0, 255, 0, 255]),
            ] {
                let local = kurbo::Point::new(x, 25.);
                if !plane.contains_local(local) {
                    continue;
                }
                let point = plane.mapping().project(local).unwrap();
                let x = point.x - tile_x as f64;
                if x < 2. || x >= width as f64 - 2. || point.y < 2. || point.y >= 94. {
                    continue;
                }
                assert_pixel(
                    &pixels,
                    x.floor() as usize,
                    point.y.floor() as usize,
                    expected,
                );
            }
            let stats = renderer.take_resource_churn_stats();
            if frame % 2 == 1 {
                assert_eq!(
                    stats.effect_source_renders, 0,
                    "unchanged camera retains source capture"
                );
                assert_eq!(
                    stats.effect_passes, 0,
                    "unchanged camera retains filtered pixels"
                );
            }
        }
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn mesh_paint_retained_pixels_blends_and_masks() {
    use crate::{AlphaMaskPaint, Vector2D};
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 96, 64, [1.0, 1.0]),
        )
    })
    .unwrap();
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    let mesh = |color: Color| Fill::Mesh {
        rows: (0..2)
            .map(|y| (0..2).map(|x| ([x as f32, y as f32], color)).collect())
            .collect(),
        pos: point(8.0, 8.0),
        main_axis: Vector2D::new(80.0, 0.0),
        off_axis: Vector2D::new(0.0, 48.0),
    };
    for frame in 0..3 {
        renderer.begin_node(0, 0, 0);
        renderer.fill_path(
            rect(0.0, 0.0, 96.0, 64.0),
            Fill::Solid(Color::rgba(0.0, 0.0, 0.0, 1.0)),
        );
        renderer.end_node(0);
        renderer.begin_node(1, 1, 0);
        renderer.save();
        if frame == 2 {
            renderer.clip_alpha(
                vec![AlphaMaskPaint {
                    path: rect(8.0, 8.0, 80.0, 48.0),
                    transform: Transform2D::identity(),
                    fill: mesh(Color::rgba(1.0, 1.0, 1.0, 0.5)),
                    opacity: 1.0,
                    composition: None,
                }],
                0.0,
            );
        }
        let red = mesh(Color::rgba(1.0, 0.0, 0.0, 0.5));
        let blue = mesh(Color::rgba(0.0, 0.0, 1.0, 0.5));
        renderer.fill_path(
            rect(8.0, 8.0, 80.0, 48.0),
            Fill::Blend(vec![
                (red, if frame == 0 { 0.25 } else { 0.5 }),
                (blue, if frame == 0 { 0.75 } else { 0.5 }),
            ]),
        );
        renderer.restore();
        renderer.end_node(1);
        renderer.request_screenshot_capture(frame);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let result = renderer.take_screenshot_capture(frame).unwrap();
        let expected: [u8; 4] = match frame {
            0 => [32, 0, 96, 255],
            1 => [64, 0, 64, 255],
            _ => [32, 0, 32, 255],
        };
        let pixel = &result.rgba[(32 * 96 + 48) * 4..][..4];
        for c in 0..4 {
            assert!(
                pixel[c].abs_diff(expected[c]) <= 1,
                "frame {frame}: {pixel:?} expected {expected:?}"
            );
        }
        assert_pixel(&result, 2, 2, [0, 0, 0, 255]);
        let stats = renderer.take_resource_churn_stats();
        if frame == 1 {
            assert_eq!(stats.mesh_texture_allocations, 0);
            assert_eq!(
                stats.mesh_raster_passes, 0,
                "changing blend weights reuses endpoint fields"
            );
            assert_eq!(stats.vector_geometry_rebuilds, 0);
        }
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn mesh_retained_pages_cull_share_capture_and_fail_closed() {
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 96, 64, [1.0, 1.0]),
        )
    })
    .unwrap();
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    let mesh = |alpha: f32, x: f32, invalid: bool| Fill::Mesh {
        rows: if invalid {
            vec![]
        } else {
            (0..2)
                .map(|y| {
                    (0..2)
                        .map(|x| ([x as f32, y as f32], Color::rgba(1.0, 0.0, 0.0, alpha)))
                        .collect()
                })
                .collect()
        },
        pos: point(x, 0.0),
        main_axis: Vector2D::new(12.0, 0.0),
        off_axis: Vector2D::new(0.0, 64.0),
    };
    for frame in 0..3 {
        // More than the concatenating fast-path threshold: each ordered draw must bind its page.
        for id in 0..72 {
            let x = if id < 8 { id as f32 * 12.0 } else { 200.0 };
            renderer.begin_node(id, id as i32, 0);
            renderer.fill_path(rect(x, 0.0, 12.0, 64.0), mesh(0.5, x, frame == 2));
            renderer.end_node(id);
        }
        // The same field is also sampled during detached alpha capture, then group opacity.
        if frame == 1 {
            renderer.begin_node(80, 80, 0);
            renderer.save();
            renderer.begin_alpha_source(Transform2D::identity(), 0.0);
            renderer.begin_node(81, 0, 0);
            renderer.fill_path(rect(0.0, 0.0, 12.0, 64.0), mesh(0.5, 0.0, false));
            renderer.end_node(81);
            renderer.end_alpha_source();
            renderer.fill_path(
                rect(0.0, 0.0, 12.0, 64.0),
                Fill::Solid(Color::rgba(0.0, 0.0, 1.0, 1.0)),
            );
            renderer.restore();
            renderer.end_node(80);
        } else {
            renderer.remove_node(80);
        }
        renderer.request_screenshot_capture(frame);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let result = renderer.take_screenshot_capture(frame).unwrap();
        if frame < 2 {
            assert_pixel(&result, 20, 32, [128, 0, 0, 128]);
        } else {
            assert_pixel(&result, 20, 32, [0; 4]);
        }
        let stats = renderer.take_resource_churn_stats();
        if frame == 0 {
            assert_eq!(
                stats.mesh_texture_allocations, 1,
                "one live shared page, no offscreen allocations"
            );
        }
        if frame == 1 {
            assert_eq!(
                stats.mesh_texture_allocations, 0,
                "source capture shares the field"
            );
            assert_eq!(stats.mesh_raster_passes, 0);
            let pixel = &result.rgba[(32 * 96 + 6) * 4..][..4];
            assert!(
                pixel[2] > pixel[0] && pixel[3] > 128,
                "capture contributes blue over red: {pixel:?}"
            );
        }
    }
    renderer.reset_retained_scene();
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn retained_clip_pixels() {
    for mirror in [false, true] {
        for deferred in [false, true] {
            let layer = MetalLayer::new();
            let mut backend = pollster::block_on(unsafe {
                RenderBackend::to_core_animation_layer(
                    layer.0.cast(),
                    RenderConfig::new(true, 128, 64, [1.0, 1.0]),
                )
            })
            .expect("Metal backend");
            let device = backend.device.clone();
            if mirror {
                // Exercise the web-style screenshot mirror even when Metal can copy the surface.
                backend.surface_config.usage.remove(TextureUsages::COPY_SRC);
            } else {
                assert!(backend.surface_supports_copy_src());
            }
            let mut renderer = WgpuRenderer::new(backend);
            for frame_id in 0..2 {
                eprintln!("mirror={mirror}, deferred={deferred}, frame={frame_id}");
                renderer.begin_node(0, 0, 0);
                renderer.fill_path(
                    rect(0.0, 0.0, 128.0, 64.0),
                    Fill::Solid(Color::rgba(0.0, 0.0, 1.0, 1.0)),
                );
                renderer.end_node(0);

                // Two distinct masks at the same depth: saving only the depth cannot identify
                // the mask that each image must see. Rotate again on the second retained frame.
                for (id, x, rgba) in [(1, 24.0, [255, 0, 0, 255]), (2, 64.0, [0, 255, 0, 255])] {
                    renderer.begin_node(id, id as i32, 0);
                    renderer.save();
                    renderer.transform(
                        Transform2D::rotation(Angle::degrees(30.0 + frame_id as f32 * 15.0))
                            .then_translate(crate::Vector2D::new(x, 32.0)),
                    );
                    renderer.clip(rect(-12.0, -12.0, 24.0, 24.0));
                    // A larger image makes a missing or wrong clip observable outside the mask.
                    renderer.draw_image(
                        &format!("image-{id}"),
                        0,
                        &Image {
                            rgba: rgba.to_vec(),
                            pixel_width: 1,
                            pixel_height: 1,
                        },
                        Box2D::new(point(-24.0, -24.0), point(24.0, 24.0)),
                    );
                    renderer.restore();
                    renderer.end_node(id);
                }

                // Nested stencil push/pop followed by a scissor-only draw. This also exercises
                // vectors in the mixed image path and returning to stencil depth zero.
                renderer.begin_node(3, 3, 0);
                renderer.save();
                renderer.transform(
                    Transform2D::rotation(Angle::degrees(45.0))
                        .then_translate(crate::Vector2D::new(104.0, 32.0)),
                );
                renderer.clip(rect(-16.0, -16.0, 32.0, 32.0));
                renderer.clip(rect(-8.0, -8.0, 16.0, 16.0));
                renderer.fill_path(
                    rect(-24.0, -24.0, 48.0, 48.0),
                    Fill::Solid(Color::rgba(1.0, 1.0, 0.0, 1.0)),
                );
                renderer.restore();
                renderer.end_node(3);

                renderer.begin_node(4, 4, 0);
                renderer.save();
                renderer.clip(rect(120.0, 0.0, 8.0, 8.0));
                renderer.fill_path(
                    rect(0.0, 0.0, 128.0, 64.0),
                    Fill::Solid(Color::rgba(1.0, 0.0, 1.0, 1.0)),
                );
                renderer.restore();
                renderer.end_node(4);
                renderer.request_screenshot_capture(frame_id);
                if deferred {
                    renderer.flush_deferred();
                    let commands = renderer.take_pending_command_buffers();
                    renderer.submit_command_buffers(commands);
                    renderer.complete_submitted_work();
                    renderer.present_deferred_frame();
                } else {
                    renderer.flush();
                }
                device
                    .poll(wgpu::PollType::wait_indefinitely())
                    .expect("GPU readback");
                let frame = renderer
                    .take_screenshot_capture(frame_id)
                    .expect("captured frame");
                assert_pixel(&frame, 24, 32, [255, 0, 0, 255]);
                assert_pixel(&frame, 64, 32, [0, 255, 0, 255]);
                assert_pixel(&frame, 104, 32, [255, 255, 0, 255]);
                assert_pixel(&frame, 124, 4, [255, 0, 255, 255]);
                for x in [24, 64, 104] {
                    assert_pixel(&frame, x, 12, [0, 0, 255, 255]);
                }
            }
        }
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn retained_image_opacity_pixels() {
    for mirror in [false, true] {
        let layer = MetalLayer::new();
        let mut backend = pollster::block_on(unsafe {
            RenderBackend::to_core_animation_layer(
                layer.0.cast(),
                RenderConfig::new(true, 64, 64, [1.0, 1.0]),
            )
        })
        .expect("Metal backend");
        let device = backend.device.clone();
        if mirror {
            backend.surface_config.usage.remove(TextureUsages::COPY_SRC);
        }
        let mut renderer = WgpuRenderer::new(backend);
        // Transparent clear makes alpha directly measurable without color-space rounding.
        // Revisit full opacity after zero to catch stale retained draw data as well.
        for (frame_id, opacity) in [1.0, 0.5, 0.0, 1.0].into_iter().enumerate() {
            for (id, x, source_alpha) in [(1, 0.0, 255), (2, 32.0, 128)] {
                renderer.begin_node(id, id as i32, 0);
                renderer.save();
                renderer.clip(rect(x + 4.0, 4.0, 24.0, 56.0));
                renderer.draw_image_with_opacity(
                    &format!("opacity-{id}"),
                    0,
                    &Image {
                        rgba: vec![255, 255, 255, source_alpha],
                        pixel_width: 1,
                        pixel_height: 1,
                    },
                    Box2D::new(point(x, 0.0), point(x + 32.0, 64.0)),
                    opacity,
                );
                renderer.restore();
                renderer.end_node(id);
            }
            renderer.request_screenshot_capture(frame_id as u32);
            renderer.flush();
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("GPU readback");
            let frame = renderer
                .take_screenshot_capture(frame_id as u32)
                .expect("captured frame");
            for (x, source_alpha) in [(16, 255.0), (48, 128.0)] {
                let alpha = frame.rgba[(32 * frame.width as usize + x) * 4 + 3];
                let expected = (source_alpha * opacity).round() as u8;
                assert!(
                    alpha.abs_diff(expected) <= 1,
                    "mirror={mirror}, opacity={opacity}: {alpha} != {expected}"
                );
            }
            assert_pixel(&frame, 1, 32, [0, 0, 0, 0]);
            let stats = renderer.take_resource_churn_stats();
            assert_eq!(stats.texture_creates, if frame_id == 0 { 2 } else { 0 });
            assert_eq!(
                stats.texture_upload_bytes,
                if frame_id == 0 { 8 } else { 0 }
            );
        }
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn paint_blend_pixels_preserve_alpha_geometry_and_retained_updates() {
    use crate::{GradientStop, GradientType, Vector2D};
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 64, 64, [1.0, 1.0]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    let gradient = |reverse: bool, color0: Color, color1: Color| Fill::Gradient {
        gradient_type: GradientType::Linear,
        pos: point(if reverse { 64.0 } else { 0.0 }, 0.0),
        main_axis: Vector2D::new(if reverse { -64.0 } else { 64.0 }, 0.0),
        off_axis: Vector2D::zero(),
        stops: vec![
            GradientStop {
                color: color0,
                stop: 0.0,
            },
            GradientStop {
                color: color1,
                stop: 64.0,
            },
        ],
    };
    for (frame_id, opacity) in [1.0, 0.5, 0.25].into_iter().enumerate() {
        renderer.begin_node(1, 0, 0);
        renderer.fill_path(
            rect(0.0, 0.0, 64.0, 32.0),
            Fill::Blend(vec![
                (
                    gradient(
                        false,
                        Color::rgba(1.0, 0.0, 0.0, 0.0),
                        Color::rgba(1.0, 0.0, 0.0, 0.0),
                    ),
                    0.5,
                ),
                (Fill::Solid(Color::rgba(0.0, 0.0, 1.0, opacity)), 0.5),
            ]),
        );
        renderer.end_node(1);
        renderer.begin_node(2, 1, 0);
        // More than a uniform batch's former 64-gradient ceiling. This is one
        // paint, not 160 source-over draws; reversing the axes cancels the ramp.
        let black = Color::rgba(0.0, 0.0, 0.0, 1.0);
        let white = Color::rgba(1.0, 1.0, 1.0, 1.0);
        renderer.fill_path(
            rect(0.0, 32.0, 64.0, 32.0),
            Fill::Blend(
                (0..160)
                    .map(|i| (gradient(i % 2 == 0, black, white), 1.0 / 160.0))
                    .collect(),
            ),
        );
        renderer.end_node(2);
        renderer.request_screenshot_capture(frame_id as u32);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let frame = renderer.take_screenshot_capture(frame_id as u32).unwrap();
        for x in [4, 32, 60] {
            let pixel = &frame.rgba[(16 * 64 + x) * 4..(16 * 64 + x) * 4 + 4];
            assert_eq!(
                pixel[0], 0,
                "transparent red must not contaminate the mixture"
            );
            assert!(
                pixel[2].abs_diff(pixel[3]) <= 1,
                "premultiplied blue: {pixel:?}"
            );
            assert!(
                pixel[3].abs_diff((opacity * 0.5 * 255.0).round() as u8) <= 1,
                "{pixel:?}"
            );
            let gray = &frame.rgba[(48 * 64 + x) * 4..(48 * 64 + x) * 4 + 4];
            for channel in &gray[..3] {
                assert!(channel.abs_diff(128) <= 1, "{gray:?}");
            }
            assert_eq!(gray[3], 255);
        }
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn radial_pixels_cover_focus_transforms_masks_and_crossfades() {
    use crate::{AlphaMaskPaint, GradientStop, GradientType, Vector2D};
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 128, 128, [1.0, 1.0]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    // Expected positions follow circles at known t: center=(1-t)*focus, radius=t.
    // Negative t below denotes the unpainted region outside the radial cone.
    let cases = [
        (
            point(0.0, 0.0),
            Vector2D::new(24.0, 0.0),
            Vector2D::new(0.0, 24.0),
            vec![
                (64, 64, 0.0),
                (76, 64, 0.5),
                (64, 76, 0.5),
                (52, 64, 0.5),
                (40, 64, 1.0),
            ],
        ),
        (
            point(0.5, 0.0),
            Vector2D::new(24.0, 0.0),
            Vector2D::new(0.0, 24.0),
            vec![
                (76, 64, 0.0),
                (64, 64, 1.0 / 3.0),
                (70, 76, 0.5),
                (82, 64, 0.5),
                (40, 64, 1.0),
            ],
        ),
        // Reflection + shear + nonuniform scale must preserve the signed inverse basis.
        (
            point(0.5, 0.0),
            Vector2D::new(-32.0, 0.0),
            Vector2D::new(16.0, 16.0),
            vec![
                (48, 64, 0.0),
                (64, 72, 0.5),
                (40, 64, 0.5),
                (72, 64, 0.5),
                (80, 80, 1.0),
            ],
        ),
        // Focus on the outer circle uses the linear limit of the quadratic.
        (
            point(1.0, 0.0),
            Vector2D::new(24.0, 0.0),
            Vector2D::new(0.0, 24.0),
            vec![(64, 64, 0.5), (76, 76, 0.5), (40, 64, 1.0), (100, 64, -1.0)],
        ),
        (
            point(2.0, 0.0),
            Vector2D::new(24.0, 0.0),
            Vector2D::new(0.0, 24.0),
            vec![
                (64, 64, 2.0),
                (88, 64, 1.0),
                (40, 64, 3.0),
                (100, 64, 0.5),
                (112, 64, -1.0),
                (112, 88, -1.0),
            ],
        ),
        // A collapsed transform paints nothing, rather than a spuriously huge circle.
        (
            point(0.0, 0.0),
            Vector2D::new(24.0, 0.0),
            Vector2D::new(12.0, 0.0),
            vec![(64, 64, -1.0), (76, 64, -1.0), (64, 76, -1.0)],
        ),
    ];
    let mut capture = 0;
    for dpr in [[1.0, 1.0], [2.0, 3.0]] {
        renderer.resize_surface(128.0 * dpr[0], 128.0 * dpr[1]);
        renderer.set_viewport(128.0, 128.0, dpr);
        for (focus, axis, off, samples) in cases.clone() {
            let radial = Fill::Gradient {
                gradient_type: GradientType::Radial { focal_point: focus },
                pos: point(64.0 + 0.5 / dpr[0], 64.0 + 0.5 / dpr[1]),
                main_axis: axis,
                off_axis: off,
                stops: vec![
                    GradientStop {
                        color: Color::rgba(1.0, 0.0, 0.0, 1.0),
                        stop: 0.0,
                    },
                    GradientStop {
                        color: Color::rgba(1.0, 0.0, 0.0, 0.0),
                        stop: 1.0,
                    },
                ],
            };
            let linear = Fill::Gradient {
                gradient_type: GradientType::Linear,
                pos: point(0.0, 0.0),
                main_axis: Vector2D::new(128.0, 0.0),
                off_axis: Vector2D::zero(),
                stops: vec![
                    GradientStop {
                        color: Color::rgba(0.0, 0.0, 1.0, 0.5),
                        stop: 0.0,
                    },
                    GradientStop {
                        color: Color::rgba(0.0, 0.0, 1.0, 0.5),
                        stop: 128.0,
                    },
                ],
            };
            // Reuse the same retained node as geometry, focus, weights and masking change.
            // The reversal visits an earlier displayed mixture, then returns to radial.
            for weight in [1.0, 0.375, 0.75, 1.0] {
                let paint = if weight == 1.0 {
                    radial.clone()
                } else {
                    Fill::Blend(vec![
                        (radial.clone(), weight),
                        (linear.clone(), 1.0 - weight),
                    ])
                };
                for masked in [false, true] {
                    renderer.begin_node(1, 0, 0);
                    renderer.save();
                    if masked {
                        renderer.clip_alpha(
                            vec![AlphaMaskPaint {
                                composition: None,
                                path: rect(0.0, 0.0, 128.0, 128.0),
                                transform: Transform2D::identity(),
                                fill: paint.clone(),
                                opacity: 1.0,
                            }],
                            0.0,
                        );
                    }
                    renderer.fill_path(
                        rect(0.0, 0.0, 128.0, 128.0),
                        if masked {
                            Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0))
                        } else {
                            paint.clone()
                        },
                    );
                    renderer.restore();
                    renderer.end_node(1);
                    renderer.request_screenshot_capture(capture);
                    renderer.flush();
                    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                    let frame = renderer.take_screenshot_capture(capture).unwrap();
                    capture += 1;
                    for &(x, y, t) in &samples {
                        let red = if t < 0.0 {
                            0.0
                        } else {
                            (1.0_f32 - t).clamp(0.0, 1.0) * weight
                        };
                        let blue = 0.5 * (1.0 - weight);
                        let alpha = red + blue;
                        let expected = if masked {
                            [alpha; 4]
                        } else {
                            [red, 0.0, blue, alpha]
                        };
                        let offset =
                            (y * dpr[1] as usize * frame.width as usize + x * dpr[0] as usize) * 4;
                        let pixel = &frame.rgba[offset..offset + 4];
                        for (actual, expected) in pixel.iter().zip(expected) {
                            assert!(actual.abs_diff((expected*255.0).round() as u8)<=2,
                            "focus={focus:?} axis={axis:?} off={off:?} weight={weight} mask={masked} ({x},{y}) t={t}: {pixel:?}");
                        }
                    }
                }
            }
        }
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn retained_subtree_opacity_pixels_and_reuse() {
    use pax_runtime_api::OpacityScope;
    for mirror in [false, true] {
        let layer = MetalLayer::new();
        let mut backend = pollster::block_on(unsafe {
            RenderBackend::to_core_animation_layer(
                layer.0.cast(),
                RenderConfig::new(true, 96, 64, [1.0, 1.0]),
            )
        })
        .expect("Metal backend");
        let device = backend.device.clone();
        if mirror {
            backend.surface_config.usage.remove(TextureUsages::COPY_SRC);
        }
        let mut renderer = WgpuRenderer::new(backend);
        for (frame_id, opacity) in [0.75, 1.0, 0.5, 0.25, 0.0, 1.0].into_iter().enumerate() {
            for (id, x) in [(1, 0.0), (2, 16.0)] {
                renderer.begin_node(id, id as i32, 0);
                renderer.set_node_opacity_scopes(
                    id,
                    &[OpacityScope {
                        node_id: 100,
                        opacity,
                    }],
                );
                renderer.save();
                renderer.clip(rect(4.0, 4.0, 52.0, 56.0));
                renderer.fill_path(
                    rect(x, 0.0, 40.0, 64.0),
                    Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
                );
                renderer.restore();
                renderer.end_node(id);
            }
            // A nested group containing an image and an overlapping vector. Half source alpha
            // tests transparent-on-transparent composition: .5 over .5 = .75, then two fades.
            for id in [3, 4] {
                renderer.begin_node(id, id as i32, 0);
                renderer.set_node_opacity_scopes(
                    id,
                    &[
                        OpacityScope {
                            node_id: 200,
                            opacity,
                        },
                        OpacityScope {
                            node_id: 201,
                            opacity: 0.5,
                        },
                    ],
                );
                if id == 3 {
                    renderer.draw_image(
                        "translucent",
                        0,
                        &Image {
                            rgba: vec![255, 255, 255, 128],
                            pixel_width: 1,
                            pixel_height: 1,
                        },
                        Box2D::new(point(64.0, 0.0), point(96.0, 64.0)),
                    );
                } else {
                    renderer.fill_path(
                        rect(64.0, 0.0, 32.0, 64.0),
                        Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 0.5)),
                    );
                }
                renderer.end_node(id);
            }
            renderer.request_screenshot_capture(frame_id as u32);
            renderer.flush();
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("readback");
            let frame = renderer
                .take_screenshot_capture(frame_id as u32)
                .expect("capture");
            let expected = (255.0 * opacity).round() as u8;
            for x in [8, 24, 48] {
                let actual = frame.rgba[(32 * frame.width as usize + x) * 4 + 3];
                assert!(actual.abs_diff(expected) <= 1, "mirror={mirror}, opacity={opacity}, x={x}, alpha={actual}, expected={expected}");
            }
            let nested = frame.rgba[(32 * frame.width as usize + 80) * 4 + 3];
            let expected_nested =
                (255.0 * (0.5 + (128.0 / 255.0) * 0.5) * 0.5 * opacity).round() as u8;
            assert!(
                nested.abs_diff(expected_nested) <= 2,
                "nested alpha {nested} != {expected_nested}"
            );
            assert_pixel(&frame, 1, 32, [0, 0, 0, 0]);
            let stats = renderer.take_resource_churn_stats();
            assert_eq!(
                stats.opacity_group_renders,
                if frame_id == 0 { 3 } else { 0 }
            );
            assert_eq!(stats.texture_creates, if frame_id == 0 { 1 } else { 0 });
            if frame_id > 0 {
                assert_eq!(stats.vector_buffer_rebuilds, 0);
                assert_eq!(stats.texture_upload_bytes, 0);
                assert_eq!(stats.image_draw_creates, 0);
                assert_eq!(
                    stats.retained_draws, 0,
                    "cached fades must not replay their contents"
                );
            }
        }
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn retained_subtree_content_invalidation_and_color() {
    use pax_runtime_api::OpacityScope;
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 128, 64, [1.0, 1.0]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    for frame_id in 0..5 {
        let color = if frame_id < 2 {
            Color::rgba(0.2, 0.7, 0.4, 1.0)
        } else {
            Color::rgba(0.8, 0.3, 0.1, 1.0)
        };
        let shift = if frame_id >= 3 { 8.0 } else { 0.0 };
        for id in [1, 2, 3] {
            renderer.begin_node(id, id as i32, 0);
            if id < 3 {
                renderer.set_node_opacity_scopes(
                    id,
                    &[OpacityScope {
                        node_id: 100,
                        opacity: 0.5,
                    }],
                );
            }
            renderer.save();
            let x = if id == 3 { 64.0 } else { 0.0 };
            renderer.transform(Transform2D::translation(x + 32.0, 32.0));
            renderer.transform(Transform2D::rotation(Angle::degrees(20.0)));
            // Nonrectangular stencil, changing independently of the node's paint transform.
            renderer.clip(rect(-20.0 + shift, -20.0, 32.0, 32.0));
            renderer.fill_path_with_opacity(
                rect(-32.0, -32.0, 64.0, 64.0),
                Fill::Solid(color),
                if id == 3 { 0.5 } else { 1.0 },
            );
            renderer.restore();
            renderer.end_node(id);
        }
        if frame_id == 4 {
            renderer.remove_node(1);
        }
        renderer.request_screenshot_capture(frame_id);
        renderer.flush();
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("readback");
        let frame = renderer.take_screenshot_capture(frame_id).expect("capture");
        for y in 4..60 {
            for x in 4..60 {
                let a = (y * 128 + x) * 4;
                let b = (y * 128 + x + 64) * 4;
                // Antialiased overlapping edges accumulate coverage before the group fade;
                // compare only the fully covered interior and exterior to the single paint.
                let alpha = frame.rgba[b + 3];
                if alpha == 128 || alpha == 0 {
                    for channel in 0..4 {
                        assert!(
                            frame.rgba[a + channel].abs_diff(frame.rgba[b + channel]) <= 2,
                            "frame={frame_id} pixel=({x},{y}) grouped={:?} reference={:?}",
                            &frame.rgba[a..a + 4],
                            &frame.rgba[b..b + 4]
                        );
                    }
                }
            }
        }
        let stats = renderer.take_resource_churn_stats();
        assert_eq!(
            stats.opacity_group_renders,
            u64::from(frame_id != 1),
            "content changes must invalidate the cache"
        );
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn retained_subtree_shared_image_refresh_and_final_removal() {
    use pax_runtime_api::OpacityScope;
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 64, 32, [1.0, 1.0]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    for frame_id in 0..5 {
        // Only the outside consumer replays when new shared artwork arrives.
        // The group must notice the new texture version despite retaining its draw.
        for id in [1, 2] {
            if id == 1 && frame_id > 0 {
                continue;
            }
            if frame_id >= 3 {
                continue;
            }
            renderer.begin_node(id, id as i32, 0);
            if id == 1 {
                renderer.set_node_opacity_scopes(
                    id,
                    &[OpacityScope {
                        node_id: 100,
                        opacity: 0.5,
                    }],
                );
            }
            let changed = frame_id >= 1;
            renderer.draw_image(
                "shared",
                u64::from(changed),
                &Image {
                    rgba: if changed {
                        vec![0, 255, 0, 255]
                    } else {
                        vec![255, 0, 0, 255]
                    },
                    pixel_width: 1,
                    pixel_height: 1,
                },
                Box2D::new(
                    point((id - 1) as f32 * 32.0, 0.0),
                    point(id as f32 * 32.0, 32.0),
                ),
            );
            renderer.end_node(id);
        }
        if frame_id == 3 {
            renderer.remove_node(1);
            renderer.remove_node(2);
        }
        renderer.request_screenshot_capture(frame_id);
        renderer.flush();
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("readback");
        let frame = renderer.take_screenshot_capture(frame_id).expect("capture");
        if frame_id < 3 {
            let pixel = &frame.rgba[(16 * 64 + 16) * 4..(16 * 64 + 16) * 4 + 4];
            let channel = usize::from(frame_id > 0);
            assert!(
                pixel[channel] > 100,
                "shared image color at frame {frame_id}: {pixel:?}"
            );
            assert!(pixel[1 - channel] <= 1);
            assert!(pixel[3].abs_diff(128) <= 1);
        } else {
            assert_pixel(&frame, 16, 16, [0, 0, 0, 0]);
            assert_pixel(&frame, 48, 16, [0, 0, 0, 0]);
        }
        let stats = renderer.take_resource_churn_stats();
        assert_eq!(stats.opacity_group_renders, u64::from(frame_id <= 1));
        assert_eq!(stats.texture_creates, u64::from(frame_id <= 1));
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn retained_subtree_gradient_image_seam_at_fractional_translation() {
    use crate::{GradientStop, GradientType, Vector2D};
    use pax_runtime_api::OpacityScope;
    for dpr in [1.0, 1.25, 2.0, 3.0] {
        let layer = MetalLayer::new();
        let backend = pollster::block_on(unsafe {
            RenderBackend::to_core_animation_layer(
                layer.0.cast(),
                RenderConfig::new(true, (64.0 * dpr) as u32, (80.0 * dpr) as u32, [dpr, dpr]),
            )
        })
        .expect("Metal backend");
        let device = backend.device.clone();
        let mut renderer = WgpuRenderer::new(backend);
        for step in 0..256 {
            let y = 8.0 + (step % 64) as f32 / 64.0;
            // Start opaque to cover the uncached path before introducing a group surface.
            let opacity = [1.0, 0.25, 0.5, 0.75][step as usize / 64];
            for id in [1, 2, 3] {
                renderer.begin_node(id, id as i32, 0);
                renderer.set_node_opacity_scopes(
                    id,
                    &[OpacityScope {
                        node_id: 100,
                        opacity,
                    }],
                );
                renderer.save();
                renderer.transform(Transform2D::translation(8.0, y));
                match id {
                    1 => renderer.fill_path(
                        rect(0.0, 0.0, 48.0, 64.0),
                        Fill::Solid(Color::rgba(0.0, 0.0, 0.0, 1.0)),
                    ),
                    2 => {
                        // ImageFit::Fill draws a larger image through the element's clip.
                        renderer.clip(rect(0.0, 0.0, 48.0, 40.0));
                        renderer.draw_image(
                            "bright",
                            0,
                            &Image {
                                rgba: vec![255, 255, 255, 255],
                                pixel_width: 1,
                                pixel_height: 1,
                            },
                            Box2D::new(point(0.0, -4.0), point(48.0, 44.0)),
                        );
                    }
                    _ => renderer.fill_path(
                        rect(0.0, 0.0, 48.0, 40.0),
                        Fill::Gradient {
                            gradient_type: GradientType::Linear,
                            pos: point(8.0, y),
                            main_axis: Vector2D::new(0.0, 40.0),
                            off_axis: Vector2D::new(1.0, 0.0),
                            stops: vec![
                                GradientStop {
                                    stop: 0.0,
                                    color: Color::rgba(0.0, 0.0, 0.0, 0.0),
                                },
                                GradientStop {
                                    stop: 40.0,
                                    color: Color::rgba(0.0, 0.0, 0.0, 1.0),
                                },
                            ],
                        },
                    ),
                }
                renderer.restore();
                renderer.end_node(id);
            }
            renderer.request_screenshot_capture(step);
            renderer.flush();
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("readback");
            let frame = renderer.take_screenshot_capture(step).expect("capture");
            let seam_y = ((y + 40.0) * dpr).floor() as usize;
            for py in seam_y.saturating_sub(1)..=seam_y + 1 {
                let offset = (py * frame.width as usize + (32.0 * dpr) as usize) * 4;
                let pixel = &frame.rgba[offset..offset + 4];
                // At most one pixel above the edge, the gradient has already almost
                // reached opaque black. An exposed white-image fringe is a regression.
                assert!(
                    pixel[0] < 40,
                    "dpr={dpr} opacity={opacity} step={step} y={py} seam={seam_y}: {pixel:?}"
                );
            }
        }
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn paint_blend_updates_invalidate_composed_group_pixels() {
    use pax_runtime_api::OpacityScope;
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 32, 32, [1.0, 1.0]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    for (frame_id, weight) in [0.25, 0.75, 0.25, 0.25].into_iter().enumerate() {
        renderer.begin_node(1, 0, 0);
        renderer.set_node_opacity_scopes(
            1,
            &[OpacityScope {
                node_id: 100,
                opacity: 0.5,
            }],
        );
        renderer.fill_path(
            rect(0.0, 0.0, 32.0, 32.0),
            Fill::Blend(vec![
                (Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)), weight),
                (Fill::Solid(Color::rgba(0.0, 0.0, 1.0, 1.0)), 1.0 - weight),
            ]),
        );
        renderer.end_node(1);
        renderer.request_screenshot_capture(frame_id as u32);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let frame = renderer.take_screenshot_capture(frame_id as u32).unwrap();
        let pixel = &frame.rgba[(16 * 32 + 16) * 4..(16 * 32 + 16) * 4 + 4];
        for (actual, expected) in pixel
            .iter()
            .zip([weight * 0.5, 0.0, (1.0 - weight) * 0.5, 0.5])
        {
            assert!(
                actual.abs_diff((expected * 255.0).round() as u8) <= 1,
                "{pixel:?}"
            );
        }
        assert_eq!(
            renderer.take_resource_churn_stats().opacity_group_renders,
            u64::from(frame_id != 3)
        );
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn layered_gradient_strokes_match_fill_and_reuse_geometry() {
    use crate::{
        DrawRange, GradientStop, GradientType, Material, Stroke, StrokeCap, StrokeJoin, Vector2D,
    };
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 128, 80, [1.0, 1.0]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    let mut builder = Path::builder();
    builder.begin(point(16.0, 45.0));
    builder.line_to(point(112.0, 45.0));
    builder.end(false);
    let path = builder.build();
    for frame_id in 0..4 {
        let paint = Fill::Gradient {
            gradient_type: GradientType::Linear,
            pos: point(16.0, 0.0),
            main_axis: Vector2D::new(96.0, 0.0),
            off_axis: Vector2D::new(0.0, 1.0),
            stops: vec![
                GradientStop {
                    color: Color::rgba(1.0, 0.0, 0.0, 1.0),
                    stop: 0.0,
                },
                GradientStop {
                    color: Color::rgba(0.0, (frame_id % 2) as f32, 1.0, 1.0),
                    stop: 96.0,
                },
            ],
        };
        renderer.begin_node(1, 0, 0);
        renderer.fill_path(rect(16.0, 5.0, 96.0, 16.0), paint.clone());
        // Back-to-front submission: wide gradient, then narrow white highlight.
        for (width, fill) in [
            (12.0, paint),
            (2.0, Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0))),
        ] {
            renderer.stroke_path_with_draw_range_and_material_and_opacity(
                path.clone(),
                Stroke {
                    fill,
                    weight: width,
                    cap: StrokeCap::Round,
                    join: StrokeJoin::Round,
                },
                Material::default(),
                1.0,
                DrawRange::enabled(0.0, if frame_id % 2 == 0 { 0.5 } else { 0.9 }),
            );
        }
        renderer.end_node(1);
        renderer.request_screenshot_capture(frame_id);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let frame = renderer.take_screenshot_capture(frame_id).unwrap();
        let pixel = |x: usize, y: usize| &frame.rgba[(y * 128 + x) * 4..(y * 128 + x) * 4 + 4];
        assert_eq!(
            pixel(40, 10),
            pixel(40, 49),
            "fill/stroke paint coordinates disagree"
        );
        assert_eq!(
            pixel(40, 45),
            &[255; 4],
            "top stroke must cover lower paint"
        );
        let stats = renderer.take_resource_churn_stats();
        if frame_id > 0 {
            assert_eq!(
                stats.tessellated_vertices, 0,
                "paint and reveal must reuse geometry"
            );
            assert_eq!(stats.vector_geometry_cache_misses, 0);
        }
    }
    renderer.remove_node(1);
    renderer.flush();
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn layered_alpha_paint_updates_reuse_mask_tessellation() {
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 32, 32, [1.0, 1.0]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    for (frame_id, alpha) in [0.5, 1.0, 0.5].into_iter().enumerate() {
        renderer.begin_node(1, 0, 0);
        renderer.save();
        renderer.clip_alpha(
            (0..2)
                .map(|_| crate::AlphaMaskPaint {
                    path: rect(0.0, 0.0, 32.0, 32.0),
                    transform: Transform2D::identity(),
                    fill: Fill::Solid(Color::rgba(1.0, 1.0, 1.0, alpha)),
                    opacity: 1.0,
                    composition: Some((20, 0.5)),
                })
                .collect(),
            0.0,
        );
        renderer.fill_path(
            rect(0.0, 0.0, 32.0, 32.0),
            Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
        );
        renderer.restore();
        renderer.end_node(1);
        renderer.request_screenshot_capture(frame_id as u32);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let frame = renderer.take_screenshot_capture(frame_id as u32).unwrap();
        let expected = ((1.0 - (1.0 - alpha).powi(2)) * 0.5 * 255.0).round() as u8;
        assert!(frame.rgba[(16 * 32 + 16) * 4 + 3].abs_diff(expected) <= 1);
        let stats = renderer.take_resource_churn_stats();
        if frame_id > 0 {
            assert_eq!(stats.tessellated_vertices, 0);
        }
    }
    renderer.remove_node(1);
    renderer.flush();
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn captured_alpha_subtree_opacity_clips_reveal_and_retirement() {
    use crate::{DrawRange, Material, Stroke, StrokeCap, StrokeJoin};
    use pax_runtime_api::OpacityScope;
    for dpr in [[1.0, 1.0], [2.0, 2.0]] {
        let layer = MetalLayer::new();
        let backend = pollster::block_on(unsafe {
            RenderBackend::to_core_animation_layer(
                layer.0.cast(),
                RenderConfig::new(true, (128.0 * dpr[0]) as u32, (64.0 * dpr[1]) as u32, dpr),
            )
        })
        .unwrap();
        let device = backend.device.clone();
        let mut renderer = WgpuRenderer::new(backend);
        renderer.set_viewport(128.0, 64.0, dpr);
        let mut path = Path::builder();
        path.begin(point(16.0, 48.0));
        path.line_to(point(112.0, 48.0));
        path.end(false);
        let path = path.build();
        for (frame_id, reveal) in [0.5, 0.9, 0.9, 0.25].into_iter().enumerate() {
            renderer.begin_node(10, 0, 0);
            renderer.save();
            renderer.begin_alpha_source(Transform2D::identity(), 0.0);
            renderer.begin_node(11, 0, 0);
            renderer.save();
            renderer.clip(rect(8.0, 8.0, 104.0, 48.0));
            renderer.end_node(11);
            for (id, x) in [(12, 0.0), (13, 32.0)] {
                renderer.begin_node(id, 0, 0);
                renderer.set_node_opacity_scopes(
                    id,
                    &[OpacityScope {
                        node_id: 100,
                        opacity: 0.5,
                    }],
                );
                // Black source RGB must produce the same mask as white.
                renderer.fill_path(
                    rect(x, 0.0, 64.0, 28.0),
                    Fill::Solid(Color::rgba(0.0, 0.0, 0.0, 1.0)),
                );
                renderer.end_node(id);
            }
            renderer.begin_node(14, 0, 0);
            renderer.stroke_path_with_draw_range_and_material_and_opacity(
                path.clone(),
                Stroke {
                    fill: Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
                    weight: 8.0,
                    cap: StrokeCap::Round,
                    join: StrokeJoin::Round,
                },
                Material::default(),
                1.0,
                DrawRange::enabled(0.0, reveal),
            );
            renderer.end_node(14);
            renderer.restore();
            renderer.end_alpha_source();
            renderer.fill_path(
                rect(0.0, 0.0, 128.0, 64.0),
                Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
            );
            renderer.restore();
            renderer.end_node(10);
            renderer.request_screenshot_capture(frame_id as u32);
            renderer.flush();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let frame = renderer.take_screenshot_capture(frame_id as u32).unwrap();
            let alpha = |x: usize, y: usize| {
                frame.rgba
                    [((y * dpr[1] as usize) * frame.width as usize + x * dpr[0] as usize) * 4 + 3]
            };
            for x in [16, 48, 80] {
                assert!(
                    alpha(x, 16).abs_diff(128) <= 1,
                    "source group opacity at {x}: {}",
                    alpha(x, 16)
                );
            }
            assert_eq!(alpha(4, 16), 0, "source-side clip");
            assert_eq!(
                alpha(100, 16),
                0,
                "source must not appear in visible sequence"
            );
            assert_eq!(alpha(24, 48), 255);
            assert_eq!(alpha(90, 48), if reveal > 0.8 { 255 } else { 0 });
            let stats = renderer.take_resource_churn_stats();
            if frame_id > 0 {
                assert_eq!(
                    stats.tessellated_vertices, 0,
                    "source reveals reuse stroke geometry"
                );
            }
            assert_eq!(stats.alpha_source_renders, u64::from(frame_id != 2));
            assert_eq!(renderer.visible_node_count(), 1);
        }
        renderer.remove_node(10);
        renderer.flush();
        assert_eq!(renderer.visible_node_count(), 0);
        assert_eq!(renderer.alpha_source_resource_counts(), (0, 0, 0));
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn captured_alpha_feather_reads_beyond_tile_and_nested_sources() {
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 64, 64, [1.0, 1.0]),
        )
    })
    .unwrap();
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    for frame_id in 0..2 {
        renderer.begin_node(1, 0, 0);
        renderer.save();
        // This parent mask applies to the consuming content, not to source capture or its blur.
        renderer.clip(rect(0.0, 8.0, 64.0, 48.0));
        renderer.begin_alpha_source(Transform2D::identity(), 4.0);
        renderer.begin_node(2, 0, 0);
        renderer.save();
        renderer.begin_alpha_source(Transform2D::identity(), 0.0);
        renderer.begin_node(3, 0, 0);
        renderer.fill_path(
            rect(-24.0, -24.0, 112.0, 112.0),
            Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 0.5)),
        );
        renderer.end_node(3);
        renderer.end_alpha_source();
        renderer.fill_path(
            rect(-24.0, -24.0, 112.0, 112.0),
            Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)),
        );
        renderer.restore();
        renderer.end_node(2);
        renderer.end_alpha_source();
        renderer.fill_path(
            rect(0.0, 0.0, 64.0, 64.0),
            Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
        );
        renderer.restore();
        renderer.end_node(1);
        renderer.request_screenshot_capture(frame_id);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let frame = renderer.take_screenshot_capture(frame_id).unwrap();
        for (x, y) in [(0, 32), (1, 8), (32, 8), (63, 32), (32, 55)] {
            let alpha = frame.rgba[(y * 64 + x) * 4 + 3];
            assert!(
                alpha.abs_diff(128) <= 1,
                "feather/source domain at {x},{y}: {alpha}"
            );
        }
        assert_pixel(&frame, 32, 4, [0, 0, 0, 0]);
        let stats = renderer.take_resource_churn_stats();
        assert_eq!(
            stats.alpha_source_renders,
            if frame_id == 0 { 2 } else { 0 }
        );
    }
    renderer.remove_node(1);
    renderer.flush();
    assert_eq!(renderer.alpha_source_resource_counts(), (0, 0, 0));
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn captured_alpha_matches_transformed_layered_gradient_strokes() {
    use crate::{
        DrawRange, GradientStop, GradientType, Material, Stroke, StrokeCap, StrokeJoin, Vector2D,
    };
    use pax_runtime_api::OpacityScope;
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 192, 96, [1.0, 1.0]),
        )
    })
    .unwrap();
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    let mut builder = Path::builder();
    builder.begin(point(8.0, 20.0));
    builder.line_to(point(65.0, 20.0));
    builder.end(false);
    let path = builder.build();
    for frame_id in 0..3 {
        let paint = Fill::Gradient {
            gradient_type: if frame_id == 0 {
                GradientType::Linear
            } else {
                GradientType::Radial {
                    focal_point: point(0.15, 0.0),
                }
            },
            pos: point(8.0, 20.0),
            main_axis: Vector2D::new(57.0, 0.0),
            off_axis: Vector2D::new(0.0, 57.0),
            stops: vec![
                GradientStop {
                    color: Color::rgba(1.0, 0.0, 0.0, 0.15),
                    stop: 0.0,
                },
                GradientStop {
                    color: Color::rgba(0.0, 1.0, 1.0, 0.85),
                    stop: 57.0,
                },
            ],
        };
        let paint = if frame_id == 2 {
            Fill::Blend(vec![
                (paint, 0.7),
                (Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 0.4)), 0.3),
            ])
        } else {
            paint
        };
        for masked in [false, true] {
            let offset = if masked { 96.0 } else { 0.0 };
            fn shifted(fill: &Fill, x: f32) -> Fill {
                match fill {
                    Fill::Gradient {
                        gradient_type,
                        pos,
                        main_axis,
                        off_axis,
                        stops,
                    } => Fill::Gradient {
                        gradient_type: gradient_type.clone(),
                        pos: point(pos.x + x, pos.y),
                        main_axis: *main_axis,
                        off_axis: *off_axis,
                        stops: stops.clone(),
                    },
                    Fill::Blend(terms) => Fill::Blend(
                        terms
                            .iter()
                            .map(|(fill, weight)| (shifted(fill, x), *weight))
                            .collect(),
                    ),
                    _ => fill.clone(),
                }
            }
            // Low-level GPU paints are already resolved in surface coordinates by the runtime.
            let paint = shifted(&paint, offset);
            let id = if masked { 10 } else { 1 };
            renderer.begin_node(id, id as i32, 0);
            renderer.save();
            if masked {
                renderer.begin_alpha_source(Transform2D::identity(), 0.0);
                renderer.begin_node(11, 0, 0);
            }
            let draw_id = if masked { 11 } else { 1 };
            renderer.set_node_opacity_scopes(
                draw_id,
                &[OpacityScope {
                    node_id: draw_id + 100,
                    opacity: 0.6,
                }],
            );
            renderer.save();
            // Shear and nonuniform scaling keep paint and geometry in the same local frame.
            renderer.transform(Transform2D::from_array([
                1.0,
                0.25,
                0.15,
                1.3,
                offset + 2.0,
                10.0,
            ]));
            renderer.fill_path(rect(8.0, 16.0, 57.0, 8.0), paint.clone());
            for (width, fill) in [
                (12.0, paint.clone()),
                (2.0, Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 0.3))),
            ] {
                renderer.stroke_path_with_draw_range_and_material_and_opacity(
                    path.clone(),
                    Stroke {
                        fill,
                        weight: width,
                        cap: StrokeCap::Round,
                        join: StrokeJoin::Round,
                    },
                    Material::default(),
                    0.8,
                    DrawRange::enabled(0.0, 0.8),
                );
            }
            renderer.restore();
            if masked {
                renderer.end_node(11);
                renderer.end_alpha_source();
                renderer.fill_path(
                    rect(96.0, 0.0, 96.0, 96.0),
                    Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
                );
            }
            renderer.restore();
            renderer.end_node(id);
        }
        renderer.request_screenshot_capture(frame_id);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let frame = renderer.take_screenshot_capture(frame_id).unwrap();
        for y in 0..96 {
            for x in 0..96 {
                let a = frame.rgba[(y * 192 + x) * 4 + 3];
                let b = frame.rgba[(y * 192 + x + 96) * 4 + 3];
                assert!(
                    a.abs_diff(b) <= 2,
                    "gradient frame {frame_id} at {x},{y}: visible={a}, mask={b}"
                );
            }
        }
        if frame_id > 0 {
            assert_eq!(renderer.take_resource_churn_stats().tessellated_vertices, 0);
        } else {
            renderer.take_resource_churn_stats();
        }
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn captured_empty_and_oversized_sources_fail_closed_independently() {
    let layer = MetalLayer::new();
    let mut backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 64, 32, [1.0, 1.0]),
        )
    })
    .unwrap();
    let device = backend.device.clone();
    // A small artificial limit exercises the failure path without allocating huge textures.
    backend.max_surface_dimension = 96;
    let mut renderer = WgpuRenderer::new(backend);
    for frame_id in 0..2 {
        for (owner, x) in [(1, 0.0), (10, 20.0), (20, 40.0)] {
            renderer.begin_node(owner, owner as i32, 0);
            renderer.save();
            renderer.begin_alpha_source(
                Transform2D::identity(),
                if owner == 10 && frame_id == 0 {
                    10.0
                } else {
                    0.0
                },
            );
            if owner != 1 {
                renderer.begin_node(owner + 1, 0, 0);
                renderer.fill_path(
                    rect(x, 0.0, 20.0, 32.0),
                    Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
                );
                renderer.end_node(owner + 1);
            }
            renderer.end_alpha_source();
            renderer.fill_path(
                rect(x, 0.0, 20.0, 32.0),
                Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
            );
            renderer.restore();
            renderer.end_node(owner);
        }
        renderer.request_screenshot_capture(frame_id);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let frame = renderer.take_screenshot_capture(frame_id).unwrap();
        assert_pixel(&frame, 10, 16, [0; 4]);
        assert_pixel(
            &frame,
            30,
            16,
            if frame_id == 0 { [0; 4] } else { [255; 4] },
        );
        assert_pixel(&frame, 50, 16, [255; 4]);
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn unchanged_lighting_does_not_redraw_retained_scene() {
    for grouped in [false, true] {
        let layer = MetalLayer::new();
        let backend = pollster::block_on(unsafe {
            RenderBackend::to_core_animation_layer(
                layer.0.cast(),
                RenderConfig::new(true, 32, 32, [1.0, 1.0]),
            )
        })
        .expect("Metal backend");
        let mut renderer = WgpuRenderer::new(backend);
        renderer.begin_node(0, 0, 0);
        if grouped {
            renderer.set_node_opacity_scopes(
                0,
                &[pax_runtime_api::OpacityScope {
                    node_id: 100,
                    opacity: 0.5,
                }],
            );
        }
        renderer.fill_path(
            rect(0.0, 0.0, 32.0, 32.0),
            Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
        );
        renderer.end_node(0);
        let mut lighting = crate::SceneLighting::default();
        renderer.set_scene_lighting(lighting.clone());
        renderer.flush();
        let stats = renderer.take_resource_churn_stats();
        assert_eq!(stats.retained_nodes_considered, 1);
        assert_eq!(stats.opacity_group_renders, u64::from(grouped));

        for _ in 0..10 {
            renderer.set_scene_lighting(lighting.clone());
            renderer.flush_deferred();
            assert!(renderer.take_pending_command_buffers().is_empty());
        }
        let stats = renderer.take_resource_churn_stats();
        assert_eq!(stats.retained_nodes_considered, 0);
        assert_eq!(stats.opacity_group_renders, 0);

        lighting.active = true;
        lighting.ambient_is_authored = true;
        lighting.ambient_intensity = 0.5;
        renderer.set_scene_lighting(lighting);
        renderer.flush();
        let stats = renderer.take_resource_churn_stats();
        assert_eq!(stats.retained_nodes_considered, 1);
        assert_eq!(stats.opacity_group_renders, u64::from(grouped));
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn retained_transform_pages_pixels() {
    for (low_limits, mixed) in [(false, false), (true, false), (true, true)] {
        let layer = MetalLayer::new();
        let mut config = RenderConfig::new(true, 360, 256, [1.0, 1.0]);
        if low_limits {
            config.required_limits = Some(wgpu::Limits {
                max_storage_buffers_per_shader_stage: 1,
                max_storage_buffer_binding_size: 128 * 1024 * 1024,
                ..wgpu::Limits::downlevel_webgl2_defaults()
            });
        }
        let mut backend = pollster::block_on(unsafe {
            RenderBackend::to_core_animation_layer(layer.0.cast(), config)
        })
        .expect("Metal backend");
        let device = backend.device.clone();
        // Cover the browser-style mirror and deferred submission together.
        backend.surface_config.usage.remove(TextureUsages::COPY_SRC);
        let mut renderer = WgpuRenderer::new(backend);
        if mixed {
            // One image selects the retained mixed-resource path instead of CPU batching.
            renderer.begin_node(1001, 1001, 0);
            renderer.draw_image(
                "blue",
                0,
                &Image {
                    rgba: vec![0, 0, 255, 255],
                    pixel_width: 1,
                    pixel_height: 1,
                },
                Box2D::new(point(350.0, 245.0), point(360.0, 256.0)),
            );
            renderer.end_node(1001);
        }
        for frame_id in 0..4 {
            eprintln!("limits={low_limits}, mixed={mixed}, frame={frame_id}");
            renderer.begin_node(1000, -1, 0);
            renderer.fill_path(
                rect(0.0, 0.0, 360.0, 256.0),
                Fill::Solid(Color::rgba(0.0, 0.0, 1.0, 1.0)),
            );
            renderer.end_node(1000);
            if frame_id == 2 {
                for id in 0..600 {
                    assert!(renderer.remove_node(id));
                }
            }
            for id in 0..600 {
                let x = (id % 30) as f32 * 12.0 + 1.0 + (frame_id % 2) as f32;
                let y = (id / 30) as f32 * 12.0 + 1.0;
                renderer.begin_node(id, id as i32, 0);
                renderer.save();
                renderer.transform(Transform2D::from_array([1.0, 0.1, 0.0, 1.0, x, y]));
                renderer.clip(rect(1.0, 1.0, 6.0, 6.0));
                renderer.fill_path_with_opacity(
                    rect(0.0, 0.0, 10.0, 10.0),
                    Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)),
                    if id % 2 == 0 { 1.0 } else { 0.5 },
                );
                renderer.restore();
                renderer.end_node(id);
            }
            renderer.request_screenshot_capture(frame_id);
            renderer.flush_deferred();
            let commands = renderer.take_pending_command_buffers();
            assert!(
                commands.len() <= 4,
                "clip count must not multiply command buffers"
            );
            renderer.submit_command_buffers(commands);
            renderer.complete_submitted_work();
            renderer.present_deferred_frame();
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("GPU readback");
            let capture = renderer
                .take_screenshot_capture(frame_id)
                .expect("captured frame");
            for id in 0..600 {
                let x = (id % 30) as usize * 12 + 4 + (frame_id % 2) as usize;
                let y = (id / 30) as usize * 12 + 4;
                let expected = if id % 2 == 0 {
                    [255, 0, 0, 255]
                } else {
                    [128, 0, 127, 255]
                };
                let offset = (y * capture.width as usize + x) * 4;
                for (actual, expected) in capture.rgba[offset..offset + 4].iter().zip(expected) {
                    assert!(
                        (*actual as i32 - expected).abs() <= 1,
                        "limits={low_limits}, frame={frame_id}, node={id}, actual={:?}",
                        &capture.rgba[offset..offset + 4]
                    );
                }
                assert_pixel(&capture, x + 5, y, [0, 0, 255, 255]);
            }
            assert_pixel(&capture, 359, 255, [0, 0, 255, 255]);
            let stats = renderer.take_resource_churn_stats();
            if frame_id == 1 || frame_id == 3 {
                assert_eq!(stats.vector_geometry_rebuilds, 0);
                assert_eq!(stats.vector_resource_creates, 0);
                assert_eq!(stats.vector_resource_update_bytes, 0);
            }
        }
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn retained_resource_page_spans_pixels() {
    let layer = MetalLayer::new();
    let mut config = RenderConfig::new(true, 360, 256, [1.0, 1.0]);
    config.required_limits = Some(wgpu::Limits {
        max_storage_buffers_per_shader_stage: 1,
        max_storage_buffer_binding_size: 128 * 1024 * 1024,
        ..wgpu::Limits::downlevel_webgl2_defaults()
    });
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(layer.0.cast(), config)
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    for frame_id in 0..4 {
        // Grow past the page boundary after resources have already been retained.
        let groups = if frame_id == 0 { 1 } else { 2 };
        for group in 0..groups {
            renderer.begin_node(group, group as i32, 0);
            for local in 0..300 {
                let id = group * 300 + local;
                renderer.save();
                renderer.transform(Transform2D::translation(
                    (id % 30) as f32 * 12.0 + 1.0 + (frame_id % 2) as f32,
                    (id / 30) as f32 * 12.0 + 1.0,
                ));
                renderer.fill_path_with_opacity(
                    rect(0.0, 0.0, 6.0, 6.0),
                    Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)),
                    if id % 2 == 0 { 1.0 } else { 0.5 },
                );
                renderer.restore();
            }
            // Identity appears after page-spanning geometry and must not use that page's slot 0.
            renderer.fill_path(
                rect(0.0, 244.0 + group as f32 * 6.0, 6.0, 4.0),
                Fill::Solid(Color::rgba(0.0, 1.0, 0.0, 1.0)),
            );
            renderer.end_node(group);
        }
        renderer.begin_node(1000, -1, 0);
        renderer.fill_path(
            rect(0.0, 0.0, 360.0, 256.0),
            Fill::Solid(Color::rgba(0.0, 0.0, 1.0, 1.0)),
        );
        renderer.end_node(1000);
        renderer.request_screenshot_capture(frame_id);
        renderer.flush();
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("GPU readback");
        let capture = renderer
            .take_screenshot_capture(frame_id)
            .expect("captured frame");
        for id in 0..groups * 300 {
            let x = (id % 30) as usize * 12 + 4 + (frame_id % 2) as usize;
            let y = (id / 30) as usize * 12 + 4;
            let expected = if id % 2 == 0 {
                [255, 0, 0, 255]
            } else {
                [128, 0, 127, 255]
            };
            let offset = (y * capture.width as usize + x) * 4;
            for (actual, expected) in capture.rgba[offset..offset + 4].iter().zip(expected) {
                assert!(
                    (*actual as i32 - expected).abs() <= 1,
                    "frame={frame_id}, node={id}"
                );
            }
        }
        for group in 0..groups {
            assert_pixel(&capture, 2, 246 + group as usize * 6, [0, 255, 0, 255]);
        }
        let stats = renderer.take_resource_churn_stats();
        if frame_id >= 2 {
            assert_eq!(stats.vector_geometry_rebuilds, 0);
            assert_eq!(stats.vector_resource_creates, 0);
            assert_eq!(stats.vector_resource_update_bytes, 0);
        }
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn retained_shared_surfaces_keep_transform_pages_independent() {
    let layers = [MetalLayer::new(), MetalLayer::new()];
    let mut context = None;
    let mut renderers: Vec<WgpuRenderer<'static>> = Vec::new();
    for layer in &layers {
        let mut config = RenderConfig::new(true, 360, 240, [1.0, 1.0]);
        config.required_limits = Some(wgpu::Limits {
            max_storage_buffers_per_shader_stage: 1,
            max_storage_buffer_binding_size: 128 * 1024 * 1024,
            ..wgpu::Limits::downlevel_webgl2_defaults()
        });
        let (backend, shared) = pollster::block_on(unsafe {
            RenderBackend::to_core_animation_layer_with_context(
                layer.0.cast(),
                config,
                context.clone(),
            )
        })
        .expect("Metal surface");
        context = Some(shared);
        let mut renderer = WgpuRenderer::new(backend);
        if let Some(first) = renderers.first() {
            renderer.share_vector_caches_from(first);
        }
        renderers.push(renderer);
    }
    for frame_id in 0..3 {
        let mut commands = CommandBatch::default();
        for (surface, renderer) in renderers.iter_mut().enumerate() {
            for group in 0..2 {
                renderer.begin_node(group, group as i32, 0);
                for local in 0..300 {
                    let id = group * 300 + local;
                    renderer.save();
                    renderer.transform(Transform2D::translation(
                        (id % 30) as f32 * 12.0 + 1.0 + surface as f32 * 6.0 + frame_id as f32,
                        (id / 30) as f32 * 12.0 + 1.0,
                    ));
                    renderer.fill_path(
                        rect(0.0, 0.0, 3.0, 3.0),
                        Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)),
                    );
                    renderer.restore();
                }
                renderer.end_node(group);
            }
            renderer.request_screenshot_capture(frame_id);
            renderer.flush_deferred();
            commands.append(&mut renderer.take_pending_command_buffers());
            let stats = renderer.take_resource_churn_stats();
            if surface == 1 && frame_id == 0 {
                assert_eq!(stats.vector_resource_creates, 0);
                assert_eq!(stats.vector_resource_cache_hits, 2);
            }
            if frame_id > 0 {
                assert_eq!(stats.vector_resource_creates, 0);
                assert_eq!(stats.vector_resource_update_bytes, 0);
            }
        }
        context.as_ref().unwrap().submit_command_buffers(commands);
        context
            .as_ref()
            .unwrap()
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("GPU readback");
        for (surface, renderer) in renderers.iter_mut().enumerate() {
            renderer.complete_submitted_work();
            renderer.present_deferred_frame();
            let frame = renderer
                .take_screenshot_capture(frame_id)
                .expect("captured surface");
            for id in 0..600 {
                let x = id % 30 * 12 + 2 + surface * 6 + frame_id as usize;
                let y = id / 30 * 12 + 2;
                assert_pixel(&frame, x, y, [255, 0, 0, 255]);
            }
        }
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn retained_mask_source_clip_pages_pixels() {
    let layer = MetalLayer::new();
    let mut config = RenderConfig::new(true, 360, 240, [1.0, 1.0]);
    config.required_limits = Some(wgpu::Limits {
        max_storage_buffers_per_shader_stage: 1,
        max_storage_buffer_binding_size: 128 * 1024 * 1024,
        ..wgpu::Limits::downlevel_webgl2_defaults()
    });
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(layer.0.cast(), config)
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    for frame_id in 0..3 {
        // Grow the clip arena after the source stencil exists, then move its clips.
        let count = if frame_id == 0 { 300 } else { 600 };
        let shift = if frame_id == 2 { 1.0 } else { 0.0 };
        renderer.begin_node(1000, 0, 0);
        renderer.save();
        renderer.begin_alpha_source(Transform2D::identity(), 0.0);
        for id in 0..count {
            let x = (id % 30) as f32 * 12.0 + 1.0 + shift;
            let y = (id / 30) as f32 * 12.0 + 1.0;
            renderer.begin_node(id, id as i32, 0);
            renderer.save();
            // Shear prevents the clip from taking the axis-aligned scissor shortcut.
            renderer.transform(Transform2D::from_array([1.0, 0.1, 0.0, 1.0, x, y]));
            renderer.clip(rect(1.0, 1.0, 6.0, 6.0));
            renderer.fill_path(
                rect(0.0, 0.0, 10.0, 10.0),
                Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
            );
            renderer.restore();
            renderer.end_node(id);
        }
        renderer.end_alpha_source();
        renderer.fill_path(
            rect(0.0, 0.0, 360.0, 240.0),
            Fill::Mesh {
                rows: (0..2)
                    .map(|y| {
                        (0..2)
                            .map(|x| ([x as f32, y as f32], Color::rgba(1.0, 0.0, 0.0, 1.0)))
                            .collect()
                    })
                    .collect(),
                pos: point(0.0, 0.0),
                main_axis: Vector2D::new(360.0, 0.0),
                off_axis: Vector2D::new(0.0, 240.0),
            },
        );
        renderer.restore();
        renderer.end_node(1000);
        renderer.request_screenshot_capture(frame_id);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let frame = renderer.take_screenshot_capture(frame_id).unwrap();
        for id in 0..600 {
            let x = (id % 30) as usize * 12 + 4 + shift as usize;
            let y = (id / 30) as usize * 12 + 4;
            let expected = if id < count {
                [255, 0, 0, 255]
            } else {
                [0, 0, 0, 0]
            };
            assert_pixel(&frame, x, y, expected);
            assert_pixel(&frame, x + 5, y, [0, 0, 0, 0]);
        }
    }
}

#[cfg(feature = "shaders")]
#[test]
#[ignore = "requires a macOS Metal device"]
fn shader_effect_pixels_clip_cache_and_tile_halo() {
    use pax_runtime_api::{
        OpacityScope, ShaderEffect, ShaderInput, ShaderParameter, ShaderParameterType, ShaderPass,
        ShaderProgram, ShaderValue,
    };
    static PROGRAM: ShaderProgram = ShaderProgram { label: "test displacement", passes: &[ShaderPass {
        label: "translate.wgsl", source: "fn effect(p: vec2<f32>) -> vec4<f32> { return sample_input0(p - vec2<f32>(parameters.shift,0.0)); }",
        parameters: &[ShaderParameter {name:"shift",ty:ShaderParameterType::Float}], inputs:[ShaderInput::Subtree;2],downsample:1,
    }] };
    for dpr in [1.0, 2.0, 3.0] {
        let layer = MetalLayer::new();
        let backend = pollster::block_on(unsafe {
            RenderBackend::to_core_animation_layer(
                layer.0.cast(),
                RenderConfig::new(true, (64.0 * dpr) as u32, (32.0 * dpr) as u32, [dpr, dpr]),
            )
        })
        .unwrap();
        let device = backend.device.clone();
        let mut renderer = WgpuRenderer::new(backend);
        renderer.set_viewport(64., 32., [dpr, dpr]);
        for frame in 0..3 {
            renderer.begin_node(9, 9, 0);
            renderer.save();
            renderer.clip(rect(2., 0., 60., 32.));
            renderer.end_node(9);
            let effect = ShaderEffect {
                program: &PROGRAM,
                parameters: vec![vec![ShaderValue::Float(if frame == 0 { 0. } else { 24. })]],
                outset: 28.,
            };
            renderer.begin_shader_effect(
                1,
                1,
                &effect,
                Transform2D::identity(),
                (64., 32.),
                &[],
                &[2],
            );
            if frame == 0 {
                renderer.begin_node(2, 0, 0);
                renderer.set_node_opacity_scopes(
                    2,
                    &[OpacityScope {
                        node_id: 2,
                        opacity: 0.5,
                    }],
                );
                renderer.fill_path(
                    rect(-16., 8., 8., 16.),
                    Fill::Solid(Color::rgba(1., 0., 0., 1.)),
                );
                renderer.end_node(2);
            }
            renderer.end_shader_effect();
            renderer.restore();
            renderer.request_screenshot_capture(frame);
            renderer.flush();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let result = renderer.take_screenshot_capture(frame).unwrap();
            assert_pixel(
                &result,
                (12. * dpr) as usize,
                (16. * dpr) as usize,
                if frame == 0 {
                    [0, 0, 0, 0]
                } else {
                    [128, 0, 0, 128]
                },
            );
            assert_pixel(&result, 0, (16. * dpr) as usize, [0, 0, 0, 0]);
            assert_eq!(
                renderer.effect_source_domain_entries(),
                1,
                "cached sources, including uniform-only effect changes, need no capture clear"
            );
        }
        let stats = renderer.take_resource_churn_stats();
        assert_eq!(stats.effect_source_renders, 1);
        assert_eq!(stats.effect_source_cache_hits, 2);
        assert_eq!(stats.effect_pipeline_compilations, 1);
        assert_eq!(stats.effect_texture_allocations, 1);
        assert_eq!(stats.effect_passes, 2);
        assert_eq!(stats.effect_cache_hits, 1);
    }
}

#[cfg(feature = "shaders")]
#[test]
#[ignore = "requires a macOS Metal device"]
fn shader_effect_off_tile_and_cached_sources_skip_capture_setup() {
    use pax_runtime_api::{ShaderEffect, ShaderInput, ShaderPass, ShaderProgram};
    static PROGRAM: ShaderProgram = ShaderProgram {
        label: "off-tile identity",
        passes: &[ShaderPass {
            label: "off-tile-identity.wgsl",
            source: "fn effect(p: vec2<f32>) -> vec4<f32> { return sample_input0(p); }",
            parameters: &[],
            inputs: [ShaderInput::Subtree; 2],
            downsample: 1,
        }],
    };
    for dpr in [1., 2.] {
        let layer = MetalLayer::new();
        let backend = pollster::block_on(unsafe {
            RenderBackend::to_core_animation_layer(
                layer.0.cast(),
                RenderConfig::new(true, (64. * dpr) as u32, (32. * dpr) as u32, [dpr, dpr]),
            )
        })
        .unwrap();
        let device = backend.device.clone();
        let mut renderer = WgpuRenderer::new(backend);
        let effect = ShaderEffect {
            program: &PROGRAM,
            parameters: vec![vec![]],
            outset: 4.,
        };
        renderer.set_viewport(64., 32., [dpr, dpr]);
        let mut previous_offset = None;
        for (frame, (offset, entries)) in
            [(500., 0), (500., 0), (0., 1), (0., 1), (500., 1), (0., 2)]
                .into_iter()
                .enumerate()
        {
            let transform = Transform2D::translation(offset, 0.);
            renderer.begin_shader_effect(1, 1, &effect, transform, (16., 16.), &[], &[2]);
            renderer.begin_shader_effect(2, 2, &effect, transform, (16., 16.), &[], &[3]);
            if previous_offset != Some(offset) {
                renderer.begin_node(3, 3, 0);
                renderer.fill_path(
                    rect(offset, 0., 16., 16.),
                    Fill::Solid(Color::rgba(1., 0., 0., 1.)),
                );
                renderer.end_node(3);
            }
            previous_offset = Some(offset);
            renderer.end_shader_effect();
            renderer.end_shader_effect();
            renderer.request_screenshot_capture(frame as u32);
            renderer.flush();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let pixels = renderer.take_screenshot_capture(frame as u32).unwrap();
            assert_pixel(
                &pixels,
                (8. * dpr) as usize,
                (8. * dpr) as usize,
                if offset == 0. {
                    [255, 0, 0, 255]
                } else {
                    [0, 0, 0, 0]
                },
            );
            assert_eq!(renderer.effect_source_domain_entries(), entries);
            if offset != 0. {
                assert!(!renderer.alpha_source_has_working_storage());
            }
        }
    }
}

#[cfg(feature = "shaders")]
#[test]
#[ignore = "requires a macOS Metal device"]
fn shader_effect_movement_reuses_textures_and_updates_coordinates() {
    use pax_runtime_api::{OpacityScope, ShaderEffect, ShaderInput, ShaderPass, ShaderProgram};
    static PROGRAM: ShaderProgram = ShaderProgram {
        label: "local coordinates",
        passes: &[ShaderPass {
            label: "local-colors.wgsl",
            source: "fn effect(p: vec2<f32>) -> vec4<f32> { let a = sample_input0(p).a; return select(vec4<f32>(a,0.0,0.0,a),vec4<f32>(0.0,a,0.0,a),p.x >= 8.0); }",
            parameters: &[],
            inputs: [ShaderInput::Subtree; 2],
            downsample: 1,
        }],
    };
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 64, 32, [1., 1.]),
        )
    })
    .unwrap();
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    let effect = ShaderEffect {
        program: &PROGRAM,
        parameters: vec![vec![]],
        outset: 0.,
    };
    let transforms = [
        Transform2D::translation(8., 8.),
        Transform2D::translation(24., 8.),
        Transform2D::from_array([0., 1., -1., 0., 40., 8.]),
    ];
    for (frame, transform) in transforms.into_iter().enumerate() {
        renderer.begin_shader_effect(
            1,
            1,
            &effect,
            transform,
            (16., 16.),
            &[OpacityScope {
                node_id: 1,
                opacity: 0.5,
            }],
            &[2],
        );
        renderer.begin_node(2, 0, 0);
        renderer.save();
        renderer.transform(transform);
        renderer.fill_path(
            rect(0., 0., 16., 16.),
            Fill::Solid(Color::rgba(1., 1., 1., 1.)),
        );
        renderer.restore();
        renderer.end_node(2);
        renderer.end_shader_effect();
        renderer.request_screenshot_capture(frame as u32);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let pixels = renderer.take_screenshot_capture(frame as u32).unwrap();
        let (red, green) = match frame {
            0 => ((12, 12), (20, 12)),
            1 => ((28, 12), (36, 12)),
            _ => ((36, 12), (36, 20)),
        };
        assert_pixel(&pixels, red.0, red.1, [128, 0, 0, 128]);
        assert_pixel(&pixels, green.0, green.1, [0, 128, 0, 128]);
        assert_pixel(&pixels, if frame == 0 { 28 } else { 12 }, 12, [0, 0, 0, 0]);
        let stats = renderer.take_resource_churn_stats();
        assert_eq!(stats.effect_texture_allocations, u64::from(frame == 0));
        assert_eq!(stats.effect_pipeline_compilations, u64::from(frame == 0));
        assert_eq!(stats.effect_passes, 1);
    }
}

#[cfg(feature = "shaders")]
#[test]
#[ignore = "requires a macOS Metal device"]
fn shader_effect_animated_bounds_reuse_storage_without_stretching_or_leaking() {
    use pax_runtime_api::{ShaderEffect, ShaderInput, ShaderPass, ShaderProgram};
    static PROGRAM: ShaderProgram = ShaderProgram {
        label: "animated capture bounds",
        passes: &[
            ShaderPass {
                label: "coarse-source.wgsl",
                source: "fn effect(p: vec2<f32>) -> vec4<f32> { return sample_input0(p); }",
                parameters: &[],
                inputs: [ShaderInput::Subtree; 2],
                downsample: 2,
            },
            ShaderPass {
                label: "active-source-and-procedural-bounds.wgsl",
                // The blue band is procedural: spare storage must not expand it.
                // Below it, sample both resolutions and verify local pixel size.
                source: "fn effect(p: vec2<f32>) -> vec4<f32> { if p.y < 4.0 { return vec4<f32>(0.0,0.0,1.0,1.0); } let coarse = sample_input0(p); let full = sample_input1(p); let red = select(0.0, coarse.r, abs(input0_pixel_size().x - parameters.pixel_size) < 0.001); return vec4<f32>(red,full.g,0.0,full.a); }",
                parameters: &[pax_runtime_api::ShaderParameter { name: "pixel_size", ty: pax_runtime_api::ShaderParameterType::Float }],
                inputs: [ShaderInput::Pass(0), ShaderInput::Subtree],
                downsample: 1,
            },
        ],
    };
    for dpr in [1., 2., 3.] {
        let layer = MetalLayer::new();
        let backend = pollster::block_on(unsafe {
            RenderBackend::to_core_animation_layer(
                layer.0.cast(),
                RenderConfig::new(true, (80. * dpr) as u32, (40. * dpr) as u32, [dpr, dpr]),
            )
        })
        .unwrap();
        let device = backend.device.clone();
        let mut renderer = WgpuRenderer::new(backend);
        renderer.set_viewport(80., 40., [dpr, dpr]);
        let effect = ShaderEffect {
            program: &PROGRAM,
            parameters: vec![vec![], vec![pax_runtime_api::ShaderValue::Float(2. / dpr)]],
            outset: 0.,
        };
        for (frame, width) in [40., 42., 38., 44., 40.].into_iter().enumerate() {
            renderer.begin_shader_effect(
                1,
                1,
                &effect,
                Transform2D::translation(8., 8.),
                (16., 16.),
                &[],
                &[2],
            );
            renderer.begin_node(2, 0, 0);
            renderer.fill_path(
                rect(8., 8., width, 16.),
                Fill::Solid(Color::rgba(1., 1., 1., 1.)),
            );
            renderer.end_node(2);
            renderer.end_shader_effect();
            renderer.request_screenshot_capture(frame as u32);
            renderer.flush();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let pixels = renderer.take_screenshot_capture(frame as u32).unwrap();
            for (x, y, color) in [
                (12., 10., [0, 0, 255, 255]),
                (8. + width - 1., 10., [0, 0, 255, 255]),
                (8. + width - 1., 20., [255, 255, 0, 255]),
                (8. + width + 1., 10., [0, 0, 0, 0]),
                (8. + width + 1., 20., [0, 0, 0, 0]),
                (12., 26., [0, 0, 0, 0]),
            ] {
                assert_pixel(&pixels, (x * dpr) as usize, (y * dpr) as usize, color);
            }
            let stats = renderer.take_resource_churn_stats();
            assert_eq!(
                stats.effect_pipeline_compilations,
                if frame == 0 { 2 } else { 0 }
            );
            assert_eq!(
                stats.effect_texture_allocations,
                // The 44px frame at DPR 3 grows past the initial 128px capacity.
                if frame == 0 || (frame == 3 && dpr == 3.) {
                    2
                } else {
                    0
                }
            );
            assert_eq!(stats.effect_source_renders, 1);
            assert_eq!(stats.effect_passes, 2);
        }
    }
}

#[cfg(feature = "shaders")]
#[test]
#[ignore = "requires a macOS Metal device"]
fn shader_effect_nested_pixels_and_relative_opacity() {
    use pax_runtime_api::{
        OpacityScope, ShaderEffect, ShaderInput, ShaderParameter, ShaderParameterType, ShaderPass,
        ShaderProgram, ShaderValue,
    };
    static PROGRAM:ShaderProgram=ShaderProgram {label:"user gain",passes:&[ShaderPass {label:"gain.wgsl",source:"fn effect(p: vec2<f32>) -> vec4<f32> { return sample_input0(p) * parameters.gain; }",parameters:&[ShaderParameter {name:"gain",ty:ShaderParameterType::Float}],inputs:[ShaderInput::Subtree;2],downsample:1}]};
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 64, 32, [1., 1.]),
        )
    })
    .unwrap();
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    for frame in 0..3 {
        let outer = ShaderEffect {
            program: &PROGRAM,
            parameters: vec![vec![ShaderValue::Float(1.)]],
            outset: 0.,
        };
        let inner = ShaderEffect {
            program: &PROGRAM,
            parameters: vec![vec![ShaderValue::Float(if frame == 0 { 1. } else { 0.5 })]],
            outset: 0.,
        };
        renderer.begin_shader_effect(
            1,
            3,
            &outer,
            Transform2D::identity(),
            (64., 32.),
            &[OpacityScope {
                node_id: 1,
                opacity: 0.5,
            }],
            &[2],
        );
        renderer.begin_shader_effect(2, 2, &inner, Transform2D::identity(), (64., 32.), &[], &[3]);
        if frame != 1 {
            renderer.begin_node(3, 1, 0);
            renderer.set_node_opacity_scopes(
                3,
                &[OpacityScope {
                    node_id: 3,
                    opacity: 0.5,
                }],
            );
            renderer.draw_image(
                "source",
                frame as u64,
                &Image {
                    rgba: vec![255, 0, 0, if frame == 0 { 255 } else { 128 }],
                    pixel_width: 1,
                    pixel_height: 1,
                },
                Box2D::new(point(8., 8.), point(40., 24.)),
            );
            renderer.end_node(3);
        }
        renderer.end_shader_effect();
        renderer.end_shader_effect();
        renderer.request_screenshot_capture(frame);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let result = renderer.take_screenshot_capture(frame).unwrap();
        let expected = [64, 32, 16][frame as usize];
        let offset = (16 * result.width as usize + 16) * 4;
        for channel in [0, 3] {
            assert!(
                (result.rgba[offset + channel] as i32 - expected).abs() <= 1,
                "frame {frame}: {:?}",
                &result.rgba[offset..offset + 4]
            );
        }
    }
    let stats = renderer.take_resource_churn_stats();
    assert_eq!(
        stats.effect_source_renders, 5,
        "inner source changes only with image version; outer source also changes with inner uniforms"
    );
    assert_eq!(stats.effect_texture_allocations, 2);
    assert_eq!(stats.effect_pipeline_compilations, 1);
    assert_eq!(stats.effect_passes, 6);
}

#[cfg(feature = "shaders")]
#[test]
#[ignore = "requires a macOS Metal device"]
fn shader_effect_gaussian_matches_across_tiles() {
    use pax_runtime_api::{
        ShaderEffect, ShaderInput, ShaderParameter, ShaderParameterType as T, ShaderPass,
        ShaderProgram, ShaderValue as V,
    };
    const PARAMETERS: &[ShaderParameter] = &[
        ShaderParameter {
            name: "sigma",
            ty: T::Float,
        },
        ShaderParameter {
            name: "axis",
            ty: T::Vec2,
        },
        ShaderParameter {
            name: "spacing",
            ty: T::Float,
        },
        ShaderParameter {
            name: "enabled",
            ty: T::Uint,
        },
    ];
    const GAUSSIAN: &str = r#"fn effect(p: vec2<f32>) -> vec4<f32> {
    if parameters.enabled == 0u { return vec4<f32>(0.0); }
    let sigma = parameters.sigma;
    if sigma < 0.001 { return sample_input0(p); }
    let step_size = dot(input0_pixel_size(), parameters.axis) * parameters.spacing;
    let n = i32(ceil(3.0 * sigma / step_size));
    var sum = vec4<f32>(0.0);
    var weight_sum = 0.0;
    for (var i = -n; i <= n; i += 1) {
        let offset = f32(i) * step_size;
        let weight = exp(-0.5 * offset * offset / (sigma * sigma));
        sum += sample_input0(p + parameters.axis * offset) * weight;
        weight_sum += weight;
    }
    return sum / weight_sum;
}
"#;
    static PROGRAM: ShaderProgram = ShaderProgram {
        label: "reference blur",
        passes: &[
            ShaderPass {
                label: "gaussian-x",
                source: GAUSSIAN,
                parameters: PARAMETERS,
                inputs: [ShaderInput::Subtree; 2],
                downsample: 1,
            },
            ShaderPass {
                label: "gaussian-y",
                source: GAUSSIAN,
                parameters: PARAMETERS,
                inputs: [ShaderInput::Pass(0); 2],
                downsample: 1,
            },
        ],
    };
    static COARSE: ShaderProgram = ShaderProgram {
        label: "coarse blur",
        passes: &[
            ShaderPass {
                label: "gaussian-x",
                source: GAUSSIAN,
                parameters: PARAMETERS,
                inputs: [ShaderInput::Subtree; 2],
                downsample: 4,
            },
            ShaderPass {
                label: "gaussian-y",
                source: GAUSSIAN,
                parameters: PARAMETERS,
                inputs: [ShaderInput::Pass(0); 2],
                downsample: 4,
            },
        ],
    };
    for (program, spacing) in [(&PROGRAM, 1.0), (&COARSE, 4.0)] {
        for dpr in [1., 2., 3.] {
            for sigma in [0., 2., 8., 48.] {
                let mut frames = Vec::new();
                for (width, origin) in [(62., 0.), (31., 0.), (31., 31.)] {
                    let layer = MetalLayer::new();
                    let backend = pollster::block_on(unsafe {
                        RenderBackend::to_core_animation_layer(
                            layer.0.cast(),
                            RenderConfig::new(
                                true,
                                (width * dpr) as u32,
                                (32. * dpr) as u32,
                                [dpr, dpr],
                            ),
                        )
                    })
                    .unwrap();
                    let device = backend.device.clone();
                    let mut renderer = WgpuRenderer::new(backend);
                    renderer.set_viewport(width, 32., [dpr, dpr]);
                    renderer.transform(Transform2D::translation(-origin, 0.));
                    let effect = ShaderEffect {
                        program,
                        parameters: vec![
                            vec![
                                V::Float(sigma),
                                V::Vec2([1., 0.]),
                                V::Float(spacing),
                                V::Uint(1),
                            ],
                            vec![V::Float(sigma), V::Vec2([0., 1.]), V::Float(1.), V::Uint(1)],
                        ],
                        outset: 145.,
                    };
                    renderer.begin_shader_effect(
                        1,
                        2,
                        &effect,
                        Transform2D::identity(),
                        (62., 32.),
                        &[],
                        &[2],
                    );
                    renderer.begin_node(2, 1, 0);
                    renderer.fill_path(
                        rect(28., 8., 8., 16.),
                        Fill::Solid(Color::rgba(1., 1., 1., 1.)),
                    );
                    renderer.end_node(2);
                    renderer.end_shader_effect();
                    renderer.request_screenshot_capture(1);
                    renderer.flush();
                    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                    frames.push(renderer.take_screenshot_capture(1).unwrap());
                }
                if program.label == "reference blur" {
                    let response = |pixel: i32, low: f32, high: f32| {
                        if sigma == 0.0 {
                            return if pixel >= (low * dpr) as i32 && pixel < (high * dpr) as i32 {
                                1.0
                            } else {
                                0.0
                            };
                        }
                        let n = (3.0 * sigma * dpr).ceil() as i32;
                        let mut total = 0.0f64;
                        let mut covered = 0.0f64;
                        for i in -n..=n {
                            let w = (-0.5 * (i as f64 / (sigma * dpr) as f64).powi(2)).exp();
                            total += w;
                            if pixel + i >= (low * dpr) as i32 && pixel + i < (high * dpr) as i32 {
                                covered += w;
                            }
                        }
                        covered / total
                    };
                    for x in [26., 30., 34., 38.] {
                        for y in [8., 16., 24.] {
                            let px = (x * dpr) as usize;
                            let py = (y * dpr) as usize;
                            let expected =
                                response(px as i32, 28., 36.) * response(py as i32, 8., 24.) * 255.;
                            let actual =
                                frames[0].rgba[(py * frames[0].width as usize + px) * 4 + 3] as f64;
                            assert!((actual-expected).abs()<=2.0,"Gaussian CPU reference: sigma={sigma} dpr={dpr}, ({x},{y}): {actual} vs {expected}");
                        }
                    }
                }
                for tile in 0..2 {
                    for y in 0..frames[tile + 1].height as usize {
                        for x in 0..frames[tile + 1].width as usize {
                            let a = (y * frames[0].width as usize
                                + x
                                + tile * frames[tile + 1].width as usize)
                                * 4;
                            let b = (y * frames[tile + 1].width as usize + x) * 4;
                            for channel in 0..4 {
                                assert!(
                                    (frames[0].rgba[a + channel] as i32
                                        - frames[tile + 1].rgba[b + channel] as i32)
                                        .abs()
                                        <= 1,
                                    "program={} sigma={sigma} dpr={dpr} tile={tile} pixel={x},{y}: {} vs {}", program.label,frames[0].rgba[a+channel],frames[tile+1].rgba[b+channel]
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(feature = "shaders")]
#[test]
#[ignore = "requires a macOS Metal device"]
fn shader_effect_output_obeys_stencil_and_alpha() {
    use pax_runtime_api::{ShaderEffect, ShaderInput, ShaderPass, ShaderProgram};
    static PROGRAM: ShaderProgram = ShaderProgram {
        label: "identity",
        passes: &[ShaderPass {
            label: "identity.wgsl",
            source: "fn effect(p: vec2<f32>) -> vec4<f32> {return sample_input0(p);}",
            parameters: &[],
            inputs: [ShaderInput::Subtree; 2],
            downsample: 1,
        }],
    };
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 64, 32, [1., 1.]),
        )
    })
    .unwrap();
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    renderer.begin_node(9, 9, 0);
    renderer.save();
    let mut path = Path::builder();
    path.begin(point(0., 0.));
    path.line_to(point(64., 0.));
    path.line_to(point(0., 32.));
    path.end(true);
    renderer.clip(path.build());
    renderer.clip_alpha(
        vec![crate::AlphaMaskPaint {
            path: rect(0., 0., 64., 32.),
            transform: Transform2D::identity(),
            fill: Fill::Solid(Color::rgba(1., 1., 1., 0.5)),
            opacity: 1.,
            composition: None,
        }],
        0.,
    );
    renderer.end_node(9);
    renderer.begin_shader_effect(
        1,
        2,
        &ShaderEffect {
            program: &PROGRAM,
            parameters: vec![vec![]],
            outset: 0.,
        },
        Transform2D::identity(),
        (64., 32.),
        &[],
        &[2],
    );
    renderer.begin_node(2, 1, 0);
    renderer.fill_path(
        rect(0., 0., 64., 32.),
        Fill::Solid(Color::rgba(1., 0., 0., 1.)),
    );
    renderer.end_node(2);
    renderer.end_shader_effect();
    renderer.restore();
    renderer.request_screenshot_capture(1);
    renderer.flush();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let frame = renderer.take_screenshot_capture(1).unwrap();
    assert_pixel(&frame, 8, 8, [128, 0, 0, 128]);
    assert_pixel(&frame, 48, 24, [0, 0, 0, 0]);
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn lighting_changes_preserve_unrelated_captures_and_geometry() {
    use crate::{LightShape, Material, SceneLight, SceneLighting};
    use pax_runtime_api::OpacityScope;
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 64, 32, [1.0, 1.0]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    let mut lighting = SceneLighting {
        active: true,
        ambient_intensity: 0.0,
        lights: vec![SceneLight {
            shape: LightShape::Directional,
            position: [0.0, 0.0, 10.0],
            direction: [0.0, 0.0, -1.0],
            color: Color::rgba(1.0, 0.0, 0.0, 1.0),
            intensity: 1.0,
            radius: 100.0,
        }],
        ..Default::default()
    };
    assert!(!renderer.set_scene_lighting(lighting.clone()), "empty tile");
    for (id, mask, material) in [
        (
            1,
            1,
            Material::custom(0.0, 1.0, 0.0, 0.5, Color::rgba(0.0, 0.0, 0.0, 1.0), 0.0),
        ),
        (2, 0, Material::default()),
        (3, 1, Material::unlit()),
    ] {
        renderer.begin_node(id, id as i32, mask);
        renderer.set_node_opacity_scopes(
            id,
            &[OpacityScope {
                node_id: 100 + id,
                opacity: 0.5,
            }],
        );
        renderer.fill_path_with_material_and_opacity(
            rect((id - 1) as f32 * 16.0, 0.0, 16.0, 32.0),
            Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
            material,
            1.0,
        );
        renderer.end_node(id);
    }
    // An unrelated detached mask source must keep its composed pixels, too.
    renderer.begin_node(10, 10, 0);
    renderer.save();
    renderer.begin_alpha_source(Transform2D::identity(), 0.0);
    renderer.begin_node(11, 0, 0);
    renderer.fill_path(
        rect(48.0, 0.0, 16.0, 32.0),
        Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
    );
    renderer.end_node(11);
    renderer.end_alpha_source();
    renderer.fill_path(
        rect(48.0, 0.0, 16.0, 32.0),
        Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
    );
    renderer.restore();
    renderer.end_node(10);
    renderer.request_screenshot_capture(0);
    renderer.flush();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let first = renderer.take_screenshot_capture(0).unwrap();
    let stats = renderer.take_resource_churn_stats();
    assert_eq!(stats.opacity_group_renders, 3);
    assert_eq!(stats.alpha_source_renders, 1);

    lighting.lights[0].color = Color::rgba(0.0, 0.0, 1.0, 1.0);
    assert!(renderer.set_scene_lighting(lighting.clone()));
    renderer.request_screenshot_capture(1);
    renderer.flush();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let second = renderer.take_screenshot_capture(1).unwrap();
    assert_ne!(
        &first.rgba[(16 * 64 + 8) * 4..(16 * 64 + 8) * 4 + 4],
        &second.rgba[(16 * 64 + 8) * 4..(16 * 64 + 8) * 4 + 4]
    );
    for x in [24, 40, 56] {
        assert_eq!(
            &first.rgba[(16 * 64 + x) * 4..(16 * 64 + x) * 4 + 4],
            &second.rgba[(16 * 64 + x) * 4..(16 * 64 + x) * 4 + 4]
        );
    }
    let stats = renderer.take_resource_churn_stats();
    assert_eq!(stats.opacity_group_renders, 1);
    assert!(stats.opacity_group_cache_hits >= 2);
    assert_eq!(stats.alpha_source_renders, 0);
    assert_eq!(stats.vector_geometry_rebuilds, 0);
    assert_eq!(stats.vector_resource_updates, 0);

    renderer.remove_node(1);
    renderer.flush();
    renderer.take_resource_churn_stats();
    lighting.lights[0].intensity = 0.25;
    assert!(
        !renderer.set_scene_lighting(lighting.clone()),
        "unlit and excluded draws remain"
    );
    renderer.flush_deferred();
    assert!(renderer.take_pending_command_buffers().is_empty());

    // A detached source is a dependency even when the visible consumer is unlit.
    renderer.begin_node(11, 0, 1);
    renderer.fill_path(
        rect(48.0, 0.0, 16.0, 32.0),
        Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
    );
    renderer.end_node(11);
    renderer.flush();
    renderer.take_resource_churn_stats();
    lighting.lights[0].intensity = 0.5;
    assert!(renderer.set_scene_lighting(lighting));
    renderer.flush();
    assert_eq!(renderer.take_resource_churn_stats().alpha_source_renders, 1);
}

#[cfg(feature = "shaders")]
#[test]
#[ignore = "requires a macOS Metal device"]
fn lighting_changes_invalidate_only_affected_shader_sources() {
    use crate::{LightShape, Material, SceneLight, SceneLighting};
    use pax_runtime_api::{ShaderEffect, ShaderInput, ShaderPass, ShaderProgram};
    static PROGRAM: ShaderProgram = ShaderProgram {
        label: "lighting identity",
        passes: &[ShaderPass {
            label: "lighting-identity.wgsl",
            source: "fn effect(p: vec2<f32>) -> vec4<f32> { return sample_input0(p); }",
            parameters: &[],
            inputs: [ShaderInput::Subtree; 2],
            downsample: 1,
        }],
    };
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 48, 32, [1.0, 1.0]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    let mut lighting = SceneLighting {
        active: true,
        ambient_intensity: 0.0,
        lights: vec![SceneLight {
            shape: LightShape::Directional,
            position: [0.0, 0.0, 10.0],
            direction: [0.0, 0.0, -1.0],
            color: Color::rgba(1.0, 0.0, 0.0, 1.0),
            intensity: 1.0,
            radius: 100.0,
        }],
        ..Default::default()
    };
    renderer.set_scene_lighting(lighting.clone());
    for (id, mask, material) in [
        (
            1,
            1,
            Material::custom(0.0, 1.0, 0.0, 0.5, Color::rgba(0.0, 0.0, 0.0, 1.0), 0.0),
        ),
        (2, 0, Material::default()),
        (3, 1, Material::unlit()),
    ] {
        renderer.begin_shader_effect(
            100 + id,
            id as i32,
            &ShaderEffect {
                program: &PROGRAM,
                parameters: vec![vec![]],
                outset: 0.0,
            },
            Transform2D::identity(),
            (48.0, 32.0),
            &[],
            &[id],
        );
        renderer.begin_node(id, id as i32, mask);
        renderer.fill_path_with_material_and_opacity(
            rect((id - 1) as f32 * 16.0, 0.0, 16.0, 32.0),
            Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
            material,
            1.0,
        );
        renderer.end_node(id);
        renderer.end_shader_effect();
    }
    renderer.request_screenshot_capture(0);
    renderer.flush();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let before = renderer.take_screenshot_capture(0).unwrap();
    assert_eq!(
        renderer.take_resource_churn_stats().effect_source_renders,
        3
    );
    lighting.lights[0].color = Color::rgba(0.0, 0.0, 1.0, 1.0);
    assert!(renderer.set_scene_lighting(lighting));
    renderer.request_screenshot_capture(1);
    renderer.flush();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let after = renderer.take_screenshot_capture(1).unwrap();
    assert_pixel(&before, 8, 16, [255, 0, 0, 255]);
    assert_pixel(&after, 8, 16, [0, 0, 255, 255]);
    for x in [24, 40] {
        assert_pixel(&before, x, 16, [255, 255, 255, 255]);
        assert_pixel(&after, x, 16, [255, 255, 255, 255]);
    }
    let stats = renderer.take_resource_churn_stats();
    assert_eq!(stats.effect_source_renders, 1);
    assert_eq!(stats.effect_source_cache_hits, 2);
    assert_eq!(stats.effect_passes, 1);
    assert_eq!(stats.effect_pipeline_compilations, 0);
    assert_eq!(stats.effect_texture_allocations, 0);
    assert_eq!(stats.vector_geometry_rebuilds, 0);
    assert_eq!(stats.vector_resource_updates, 0);
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn startup_alpha_source_empty_and_first_capture_pixels() {
    use crate::{DrawRange, Material, Stroke, StrokeCap, StrokeJoin};
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 32, 32, [1.0, 1.0]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    renderer.set_viewport(32.0, 32.0, [1.0, 1.0]);
    for (frame_id, mode) in [0, 1, 2, 0].into_iter().enumerate() {
        renderer.begin_node(10, 0, 0);
        renderer.save();
        renderer.begin_alpha_source(Transform2D::identity(), 0.0);
        if mode != 0 {
            renderer.begin_node(11, 0, 0);
            let mut path = Path::builder();
            path.begin(point(4.0, 16.0));
            path.line_to(point(28.0, 16.0));
            path.end(false);
            renderer.stroke_path_with_draw_range_and_material_and_opacity(
                path.build(),
                Stroke {
                    fill: Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
                    weight: 8.0,
                    cap: StrokeCap::Butt,
                    join: StrokeJoin::Round,
                },
                Material::default(),
                1.0,
                DrawRange::enabled(0.0, if mode == 1 { 0.0 } else { 1.0 }),
            );
            renderer.end_node(11);
        }
        renderer.end_alpha_source();
        renderer.end_node(10);
        renderer.begin_node(20, 1, 0);
        renderer.fill_path(
            rect(0.0, 0.0, 32.0, 32.0),
            Fill::Mesh {
                rows: (0..2)
                    .map(|y| {
                        (0..2)
                            .map(|x| ([x as f32, y as f32], Color::rgba(1.0, 0.0, 0.0, 1.0)))
                            .collect()
                    })
                    .collect(),
                pos: point(0.0, 0.0),
                main_axis: Vector2D::new(32.0, 0.0),
                off_axis: Vector2D::new(0.0, 32.0),
            },
        );
        renderer.end_node(20);
        renderer.begin_node(10, 0, 0);
        renderer.restore();
        renderer.end_node(10);
        renderer.request_screenshot_capture(frame_id as u32);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let frame = renderer.take_screenshot_capture(frame_id as u32).unwrap();
        assert_pixel(
            &frame,
            16,
            16,
            if mode == 2 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 0, 0]
            },
        );
        assert_pixel(&frame, 16, 2, [0, 0, 0, 0]);
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn empty_alpha_sources_skip_working_storage_and_reenter_padded_domain() {
    for scale in [1.0, 2.0] {
        let layer = MetalLayer::new();
        let backend = pollster::block_on(unsafe {
            RenderBackend::to_core_animation_layer(
                layer.0.cast(),
                RenderConfig::new(
                    true,
                    (64.0 * scale) as u32,
                    (32.0 * scale) as u32,
                    [scale; 2],
                ),
            )
        })
        .unwrap();
        let device = backend.device.clone();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut renderer = WgpuRenderer::new(backend);
        renderer.set_viewport(64.0, 32.0, [scale; 2]);
        for (frame_id, x) in [-80.0, -80.0, 20.0, 20.0, -80.0, -8.0, -80.0]
            .into_iter()
            .enumerate()
        {
            renderer.begin_node(1, 0, 0);
            renderer.save();
            renderer.begin_alpha_source(Transform2D::translation(x, 0.0), 4.0);
            renderer.begin_node(2, 0, 0);
            renderer.save();
            renderer.clip(rect(-1.0, 0.0, 8.0, 32.0));
            renderer.fill_path(
                rect(0.0, 4.0, 6.0, 24.0),
                Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
            );
            renderer.restore();
            renderer.end_node(2);
            renderer.end_alpha_source();
            renderer.fill_path(
                rect(0.0, 0.0, 64.0, 32.0),
                Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)),
            );
            renderer.restore();
            renderer.end_node(1);
            renderer.request_screenshot_capture(frame_id as u32);
            renderer.flush();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let frame = renderer.take_screenshot_capture(frame_id as u32).unwrap();
            let stats = renderer.take_resource_churn_stats();
            if x == -80.0 {
                assert_eq!(renderer.alpha_source_resource_counts().2, 0);
                assert!(!renderer.alpha_source_has_working_storage());
                assert_eq!(stats.alpha_source_renders, 0);
                assert!(frame.rgba.chunks_exact(4).all(|p| p == [0, 0, 0, 0]));
            } else {
                assert!(renderer.alpha_source_has_working_storage());
                let pixel_x = if x < 0.0 { 0 } else { (23.0 * scale) as usize };
                let alpha =
                    frame.rgba[((16.0 * scale) as usize * frame.width as usize + pixel_x) * 4 + 3];
                assert!(
                    alpha > 5,
                    "source at {x} must contribute at DPR {scale}: {alpha}"
                );
                if frame_id == 3 {
                    assert_eq!(stats.alpha_source_renders, 0);
                    assert!(stats.alpha_source_cache_hits > 0);
                }
            }
        }
        assert!(pollster::block_on(scope.pop()).is_none());
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn empty_alpha_source_at_surface_limit_fails_closed_without_capture() {
    let layer = MetalLayer::new();
    let mut backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 32, 32, [1.0; 2]),
        )
    })
    .unwrap();
    // Model a surface at the device limit without allocating a maximum-size texture.
    backend.max_surface_dimension = 32;
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    renderer.begin_node(1, 0, 0);
    renderer.save();
    renderer.begin_alpha_source(Transform2D::identity(), 0.0);
    renderer.end_alpha_source();
    renderer.fill_path(
        rect(0.0, 0.0, 32.0, 32.0),
        Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
    );
    renderer.restore();
    renderer.end_node(1);
    renderer.request_screenshot_capture(0);
    renderer.flush();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let frame = renderer.take_screenshot_capture(0).unwrap();
    assert!(frame.rgba.chunks_exact(4).all(|p| p == [0, 0, 0, 0]));
    assert!(!renderer.alpha_source_has_working_storage());
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn empty_parent_alpha_masks_remain_transparent_to_nested_sources() {
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 32, 32, [1.0; 2]),
        )
    })
    .unwrap();
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    renderer.set_viewport(32.0, 32.0, [1.0; 2]);
    for (frame_id, visible) in [false, true, false, true].into_iter().enumerate() {
        renderer.begin_node(1, 0, 0);
        renderer.save();
        renderer.begin_alpha_source(Transform2D::identity(), 0.0);
        if visible {
            renderer.begin_node(2, 0, 0);
            renderer.fill_path(
                rect(0.0, 0.0, 32.0, 32.0),
                Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
            );
            renderer.end_node(2);
        }
        renderer.end_alpha_source();
        renderer.end_node(1);
        renderer.begin_node(10, 1, 0);
        renderer.save();
        renderer.begin_alpha_source(Transform2D::identity(), 0.0);
        renderer.begin_node(11, 0, 0);
        renderer.fill_path(
            rect(8.0, 8.0, 16.0, 16.0),
            Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
        );
        renderer.end_node(11);
        renderer.end_alpha_source();
        renderer.fill_path(
            rect(0.0, 0.0, 32.0, 32.0),
            Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)),
        );
        renderer.restore();
        renderer.end_node(10);
        renderer.begin_node(1, 0, 0);
        renderer.restore();
        renderer.end_node(1);
        renderer.request_screenshot_capture(frame_id as u32);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let frame = renderer.take_screenshot_capture(frame_id as u32).unwrap();
        assert_pixel(
            &frame,
            16,
            16,
            if visible { [255, 0, 0, 255] } else { [0; 4] },
        );
        assert_pixel(&frame, 2, 2, [0; 4]);
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
#[cfg(feature = "3d")]
fn intersecting_spatial_planes_preserve_transparency_clips_and_retained_updates() {
    check_intersecting_spatial_planes(false);
}

#[test]
#[ignore = "requires a macOS Metal device"]
#[cfg(feature = "3d")]
fn spatial_screenshot_mirror_preserves_intersection_clip_order() {
    check_intersecting_spatial_planes(true);
}

#[cfg(feature = "3d")]
fn check_intersecting_spatial_planes(mirror: bool) {
    use pax_runtime_api::DepthPlane;
    let layer = MetalLayer::new();
    let mut backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 64, 32, [1.0, 1.0]),
        )
    })
    .expect("Metal backend");
    if mirror {
        backend.surface_config.usage.remove(TextureUsages::COPY_SRC);
    }
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    renderer.set_viewport(64.0, 32.0, [1.0, 1.0]);
    for (frame, coefficients) in [[1.0, 0.0, -32.0], [-1.0, 0.0, 32.0], [0.0, 0.0, 100.0]]
        .into_iter()
        .enumerate()
    {
        // Frame-like clip owners end before the children draw. On subsequent frames
        // only the moving child is recorded; the other child keeps its retained clip.
        renderer.begin_node(100, 0, 0);
        renderer.save();
        let mut clip = Path::builder();
        if frame == 2 {
            renderer.transform(Transform2D::new(-1.0, 0.0, 0.0, 1.0, 64.0, 0.0));
        }
        clip.begin(point(0.0, 4.0));
        clip.line_to(point(64.0, 4.0));
        clip.line_to(point(60.0, 28.0));
        clip.line_to(point(4.0, 28.0));
        clip.close();
        renderer.clip(clip.build());
        renderer.end_node(100);
        if frame == 2 {
            renderer.transform(Transform2D::new(-1.0, 0.0, 0.0, 1.0, 64.0, 0.0));
        }
        for (id, color) in [
            (1, Color::rgba(1.0, 0.0, 0.0, 0.5)),
            (2, Color::rgba(0.0, 0.0, 1.0, 0.5)),
        ] {
            if frame == 1 && id == 2 {
                continue;
            }
            renderer.begin_node(id, id as i32, 0);
            renderer.save();
            if frame > 0 {
                renderer.transform(Transform2D::new(-1.0, 0.0, 0.0, 1.0, 64.0, 0.0));
            }
            if id == 1 {
                renderer.fill_path(rect(0.0, 0.0, 64.0, 32.0), Fill::Solid(color));
            } else {
                renderer.draw_image_with_opacity(
                    "blue-plane",
                    0,
                    &Image {
                        rgba: vec![0, 0, 255, 255],
                        pixel_width: 1,
                        pixel_height: 1,
                    },
                    Box2D::new(point(0.0, 0.0), point(64.0, 32.0)),
                    0.5,
                );
            }
            renderer.restore();
            renderer.end_node(id);
        }
        renderer.restore();
        renderer.set_node_depth_plane(
            1,
            Some(DepthPlane {
                presentation_layer: 0,
                coefficients,
                backface: false,
            }),
        );
        renderer.request_screenshot_capture(frame as u32);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let pixels = renderer.take_screenshot_capture(frame as u32).unwrap();
        for x in [8, 24, 40, 56] {
            let red_front = coefficients[0] * (x as f64 + 0.5) + coefficients[2] > 0.0;
            // Screenshot data is premultiplied; intersecting translucent surfaces
            // must change color order within one draw, retaining 75% total coverage.
            assert_pixel(
                &pixels,
                x,
                16,
                if red_front {
                    [128, 0, 64, 191]
                } else {
                    [64, 0, 128, 191]
                },
            );
        }
        assert_pixel(&pixels, 16, 1, [0, 0, 0, 0]);
    }
    let stats = renderer.take_resource_churn_stats();
    assert_eq!(stats.vector_geometry_rebuilds, 1);
}

#[test]
#[ignore = "requires a macOS Metal device"]
#[cfg(feature = "3d")]
fn disjoint_spatial_surfaces_share_clips_without_replaying_the_entire_scene() {
    use pax_runtime_api::DepthPlane;
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 128, 96, [1.0, 1.0]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    renderer.set_viewport(128.0, 96.0, [1.0, 1.0]);
    renderer.begin_node(1, 0, 0);
    renderer.fill_path(
        rect(0.0, 0.0, 128.0, 96.0),
        Fill::Solid(Color::rgba(0.0, 0.0, 1.0, 1.0)),
    );
    renderer.end_node(1);
    renderer.set_node_depth_plane(
        1,
        Some(DepthPlane {
            presentation_layer: 0,
            coefficients: [0.0, 0.0, -100.0],
            backface: false,
        }),
    );
    for i in 0..8_u32 {
        let x = 8.0 + (i % 4) as f32 * 32.0;
        let y = 12.0 + (i / 4) as f32 * 48.0;
        renderer.begin_node(i + 2, i as i32 + 1, 0);
        renderer.fill_path(
            rect(x, y, 8.0, 8.0),
            Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 0.5)),
        );
        renderer.end_node(i + 2);
        let a = (i as f64 * 0.77).cos();
        let b = (i as f64 * 0.77).sin();
        renderer.set_node_depth_plane(
            i + 2,
            Some(DepthPlane {
                presentation_layer: 0,
                coefficients: [a, b, 30.0 - a * x as f64 - b * y as f64],
                backface: false,
            }),
        );
    }
    // Empty clip owners still participate in clipping, but never in paint order.
    for i in 20..40 {
        renderer.begin_node(i, i as i32, 0);
        renderer.end_node(i);
        renderer.set_node_depth_plane(
            i,
            Some(DepthPlane {
                presentation_layer: 0,
                coefficients: [i as f64, -(i as f64), 13.0],
                backface: false,
            }),
        );
    }
    renderer.request_screenshot_capture(0);
    renderer.flush();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let pixels = renderer.take_screenshot_capture(0).unwrap();
    for i in 0..8 {
        assert_pixel(
            &pixels,
            12 + (i % 4) * 32,
            16 + (i / 4) * 48,
            [128, 0, 128, 255],
        );
    }
    assert_pixel(&pixels, 4, 4, [0, 0, 255, 255]);
    let stats = renderer.take_resource_churn_stats();
    assert!(
        stats.retained_draws < 200,
        "{} retained draws",
        stats.retained_draws
    );
}

#[test]
#[ignore = "requires a macOS Metal device"]
#[cfg(feature = "3d")]
fn repeated_local_intersections_bound_scene_replays() {
    use pax_runtime_api::DepthPlane;
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 256, 128, [1.0, 1.0]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    renderer.set_viewport(256.0, 128.0, [1.0, 1.0]);
    renderer.begin_node(1, 0, 0);
    renderer.fill_path(
        rect(0.0, 0.0, 256.0, 128.0),
        Fill::Solid(Color::rgba(0.0, 0.0, 0.0, 1.0)),
    );
    renderer.end_node(1);
    renderer.set_node_depth_plane(
        1,
        Some(DepthPlane {
            presentation_layer: 0,
            coefficients: [0.0, 0.0, -1000.0],
            backface: false,
        }),
    );
    // Eight independent intersections: each contains two translucent surfaces.
    // Their equality lines must not split all the other solids' draw regions.
    for i in 0..8u32 {
        let x = (i % 4) as f32 * 64.0 + 12.0;
        let y = (i / 4) as f32 * 64.0 + 12.0;
        for side in 0..2u32 {
            let id = 2 + i * 2 + side;
            renderer.begin_node(id, id as i32, 0);
            renderer.fill_path(
                rect(x, y, 40.0, 40.0),
                Fill::Solid(if side == 0 {
                    Color::rgba(1.0, 0.0, 0.0, 0.5)
                } else {
                    Color::rgba(0.0, 0.0, 1.0, 0.5)
                }),
            );
            renderer.end_node(id);
        }
    }
    let mut max_draws = 0;
    for frame in 0..120u32 {
        for i in 0..8u32 {
            let x = (i % 4) as f64 * 64.0 + 32.0;
            let y = (i / 4) as f64 * 64.0 + 32.0;
            let angle = i as f64 * 0.37 + frame as f64 * 0.015;
            let (b, a) = angle.sin_cos();
            renderer.set_node_depth_plane(
                2 + i * 2,
                Some(DepthPlane {
                    presentation_layer: 0,
                    coefficients: [a, b, -a * x - b * y],
                    backface: false,
                }),
            );
            renderer.set_node_depth_plane(3 + i * 2, Some(DepthPlane::default()));
        }
        renderer.request_screenshot_capture(frame);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let pixels = renderer.take_screenshot_capture(frame).unwrap();
        for i in 0..8u32 {
            let x = i % 4 * 64 + 32;
            let y = i / 4 * 64 + 32;
            let angle = i as f64 * 0.37 + frame as f64 * 0.015;
            for dx in [-12i32, 12] {
                let px = (x as i32 + dx) as u32;
                let delta = angle.cos() * (dx as f64 + 0.5) + angle.sin() * 0.5;
                if delta.abs() > 1.0 {
                    assert_pixel(
                        &pixels,
                        px as usize,
                        y as usize,
                        if delta > 0.0 {
                            [128, 0, 64, 255]
                        } else {
                            [64, 0, 128, 255]
                        },
                    );
                }
            }
        }
        assert_pixel(&pixels, 2, 2, [0, 0, 0, 255]);
        let stats = renderer.take_resource_churn_stats();
        max_draws = max_draws.max(stats.retained_draws);
        assert!(
            stats.retained_draws <= 80,
            "frame {frame}: {} draws for 17 nodes",
            stats.retained_draws
        );
    }
    eprintln!("120 intersection frames: maximum {max_draws} retained draws for 17 nodes");
}

#[test]
#[ignore = "requires a macOS Metal device"]
#[cfg(feature = "3d")]
fn spatial_group_opacity_and_backface_reuse_paint_without_stale_captures() {
    use pax_runtime_api::{DepthPlane, OpacityScope};
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 64, 32, [1.0, 1.0]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    renderer.set_viewport(64.0, 32.0, [1.0, 1.0]);
    for (id, color) in [
        (1, Color::rgba(1.0, 0.0, 0.0, 1.0)),
        (2, Color::rgba(0.0, 0.0, 1.0, 1.0)),
        (3, Color::rgba(0.0, 1.0, 0.0, 1.0)),
    ] {
        renderer.begin_node(id, id as i32, 0);
        renderer.fill_path(rect(0.0, 0.0, 64.0, 32.0), Fill::Solid(color));
        renderer.end_node(id);
        if id < 3 {
            renderer.set_node_opacity_scopes(
                id,
                &[OpacityScope {
                    node_id: 10,
                    opacity: 0.5,
                }],
            );
        }
    }
    for (frame, (slope, backface)) in [(1.0, false), (-1.0, false), (-1.0, true)]
        .into_iter()
        .enumerate()
    {
        for id in [1, 2] {
            renderer.set_node_depth_plane(
                id,
                Some(DepthPlane {
                    presentation_layer: 0,
                    coefficients: [slope, 0.0, -32.0 * slope],
                    backface,
                }),
            );
        }
        renderer.request_screenshot_capture(frame as u32);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let pixels = renderer.take_screenshot_capture(frame as u32).unwrap();
        for x in 0..64 {
            let group_front = slope * (x as f64 + 0.5 - 32.0) > 0.0;
            assert_pixel(
                &pixels,
                x,
                16,
                if !group_front {
                    [0, 255, 0, 255]
                } else if backface {
                    [128, 128, 0, 255]
                } else {
                    [0, 128, 128, 255]
                },
            );
        }
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn opacity_texture_capacity_survives_silhouette_resizing() {
    use super::capture::CaptureKey;
    let layer = MetalLayer::new();
    let mut backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 160, 96, [1., 1.]),
        )
    })
    .expect("Metal backend");
    let key = CaptureKey::Opacity(1, 1);
    assert!(!backend.capture_is_cached(key, 0));
    let mut previous = None;
    for (frame, (width, height)) in [(32, 20), (31, 21), (60, 24), (70, 30), (31, 20)]
        .into_iter()
        .enumerate()
    {
        backend.begin_capture();
        backend.end_capture_rect(key, frame as u64, (0, 0, width, height));
        let texture = backend.capture_texture_for_test(key);
        if let Some(previous) = previous {
            if frame == 3 {
                assert_ne!(texture, previous);
            } else {
                assert_eq!(texture, previous);
            }
        }
        assert!(texture.width() <= 160 && texture.height() <= 96);
        previous = Some(texture);
        backend.submit_pending_commands();
    }
    backend
        .device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
}

#[test]
#[ignore = "requires a macOS Metal device"]
#[cfg(feature = "3d")]
fn near_edge_on_group_uses_projected_coverage_without_false_interleaving() {
    use pax_runtime_api::{DepthPlane, OpacityScope};
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 64, 32, [1., 1.]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    renderer.set_viewport(64., 32., [1., 1.]);
    renderer.begin_node(1, 0, 0);
    renderer.fill_path(
        rect(0., 0., 64., 32.),
        Fill::Solid(Color::rgba(0., 1., 0., 1.)),
    );
    renderer.end_node(1);
    renderer.set_node_depth_plane(
        1,
        Some(DepthPlane {
            presentation_layer: 0,
            coefficients: [0., 0., -100.],
            backface: false,
        }),
    );
    for (frame, degrees) in [80.0f64, 89., 89.9, 89.99, 90.01, 90.1, 91., 100.]
        .into_iter()
        .enumerate()
    {
        let (s, c) = degrees.to_radians().sin_cos();
        let (sx, cx) = 0.4f64.sin_cos();
        let a = -s / c;
        let b = sx / (c * cx);
        renderer.begin_node(2, 1, 0);
        renderer.save();
        renderer.transform(crate::Transform2D::new(
            c as f32,
            0.,
            (s * sx) as f32,
            cx as f32,
            20.,
            4.,
        ));
        renderer.fill_path(
            rect(0., 0., 30., 20.),
            Fill::Solid(Color::rgba(1., 0., 0., 1.)),
        );
        renderer.restore();
        renderer.end_node(2);
        renderer.set_node_depth_plane(
            2,
            Some(DepthPlane {
                presentation_layer: 0,
                coefficients: [a, b, -a * 20. - b * 4.],
                backface: c < 0.,
            }),
        );
        renderer.set_node_opacity_scopes(
            2,
            &[OpacityScope {
                node_id: 10,
                opacity: 0.5,
            }],
        );
        renderer.request_screenshot_capture(frame as u32);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let capture = renderer.take_screenshot_capture(frame as u32).unwrap();
        assert_pixel(&capture, 50, 16, [0, 255, 0, 255]);
        let stats = renderer.take_resource_churn_stats();
        assert_eq!(stats.opacity_group_renders, 1, "pose {degrees}");
        assert_eq!(stats.spatial_paint_passes, 2, "pose {degrees}");
        assert_eq!(
            stats.spatial_clip_fragments, 0,
            "the backdrop never crosses this surface"
        );
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
#[cfg(feature = "3d")]
fn distant_tilted_panel_does_not_fragment_a_translucent_group_into_captures() {
    use pax_runtime_api::{DepthPlane, OpacityScope};
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 64, 96, [1., 1.]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    renderer.set_viewport(64., 96., [1., 1.]);
    for (id, path, color, plane) in [
        (
            1,
            rect(0., 0., 64., 96.),
            Color::rgba(0., 0., 0., 1.),
            [0., 0., -100.],
        ),
        // Its infinite plane crosses the group above; its finite pixels do not.
        (
            2,
            rect(0., 60., 64., 30.),
            Color::rgba(0., 1., 0., 1.),
            [0., 1., -20.],
        ),
        (
            3,
            rect(10., 10., 30., 20.),
            Color::rgba(1., 0., 0., 1.),
            [0., 0., -5.],
        ),
        (
            4,
            rect(10., 10., 30., 20.),
            Color::rgba(0., 0., 1., 1.),
            [0., 0., 5.],
        ),
    ] {
        renderer.begin_node(id, id as i32, 0);
        renderer.fill_path(path, Fill::Solid(color));
        renderer.end_node(id);
        renderer.set_node_depth_plane(
            id,
            Some(DepthPlane {
                presentation_layer: 0,
                coefficients: plane,
                backface: false,
            }),
        );
        if id >= 3 {
            renderer.set_node_opacity_scopes(
                id,
                &[OpacityScope {
                    node_id: 10,
                    opacity: 0.5,
                }],
            );
        }
    }
    renderer.request_screenshot_capture(0);
    renderer.flush();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let capture = renderer.take_screenshot_capture(0).unwrap();
    for y in [12, 18, 22, 28] {
        assert_pixel(&capture, 20, y, [0, 0, 128, 255]);
    }
    assert_pixel(&capture, 20, 70, [0, 255, 0, 255]);
    let stats = renderer.take_resource_churn_stats();
    assert_eq!(stats.opacity_group_renders, 1);
    assert!(stats.retained_draws <= 8, "{stats:?}");
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn many_opacity_captures_share_encoding_without_exhausting_metal_queue() {
    use pax_runtime_api::OpacityScope;
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 100, 80, [1.0, 1.0]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    renderer.set_viewport(100., 80., [1., 1.]);
    renderer.begin_node(1000, -1, 0);
    renderer.fill_path(
        rect(0., 0., 100., 80.),
        Fill::Solid(Color::rgba(0., 0., 0., 1.)),
    );
    renderer.end_node(1000);
    // Separate capture clear/copy/present encoders used to exhaust Metal's pool
    // of unsubmitted command buffers before flush could submit any of them.
    for id in 0..80 {
        renderer.begin_node(id, id as i32, 0);
        renderer.fill_path(
            rect((id % 10 * 10) as f32, (id / 10 * 10) as f32, 10., 10.),
            Fill::Solid(Color::rgba(1., 0., 0., 1.)),
        );
        renderer.end_node(id);
        renderer.set_node_opacity_scopes(
            id,
            &[OpacityScope {
                node_id: id,
                opacity: 0.5,
            }],
        );
    }
    renderer.request_screenshot_capture(0);
    renderer.flush();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let capture = renderer.take_screenshot_capture(0).unwrap();
    for y in (5..80).step_by(10) {
        for x in (5..100).step_by(10) {
            assert_pixel(&capture, x, y, [128, 0, 0, 255]);
        }
    }
    assert_eq!(
        renderer.take_resource_churn_stats().opacity_group_renders,
        80
    );
}

#[test]
#[ignore = "requires a macOS Metal device"]
#[cfg(feature = "3d")]
fn spatial_opacity_reuses_alpha_but_invalidates_changed_fragment_clips() {
    use pax_runtime_api::{DepthPlane, OpacityScope};
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 64, 32, [1.0, 1.0]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    renderer.set_viewport(64.0, 32.0, [1.0, 1.0]);
    for (id, color) in [
        (1, Color::rgba(1., 0., 0., 1.)),
        (2, Color::rgba(0., 0., 1., 1.)),
        (3, Color::rgba(0., 1., 0., 1.)),
    ] {
        renderer.begin_node(id, id as i32, 0);
        renderer.fill_path(rect(0., 0., 64., 32.), Fill::Solid(color));
        renderer.end_node(id);
    }
    // Alpha-only changes reuse the capture. Moving the intersection changes only
    // fragment clips, and an outside plane then requires exact presence views.
    for (frame, (alpha, crossing, outside_z)) in [
        (0.5, 32., -100.),
        (0.25, 32., -100.),
        (0.5, 16., -100.),
        (0.5, 16., 0.),
        (0.5, 32., -100.),
        (0.25, 32., -100.),
    ]
    .into_iter()
    .enumerate()
    {
        for (id, slope) in [(1, 1.), (2, -1.)] {
            renderer.set_node_depth_plane(
                id,
                Some(DepthPlane {
                    presentation_layer: 0,
                    coefficients: [slope, 0., -crossing * slope],
                    backface: false,
                }),
            );
            renderer.set_node_opacity_scopes(
                id,
                &[OpacityScope {
                    node_id: 10,
                    opacity: alpha,
                }],
            );
        }
        renderer.set_node_depth_plane(
            3,
            Some(DepthPlane {
                presentation_layer: 0,
                coefficients: [0., 0., outside_z],
                backface: false,
            }),
        );
        renderer.request_screenshot_capture(frame as u32);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let pixels = renderer.take_screenshot_capture(frame as u32).unwrap();
        for x in [8, 24, 48] {
            let red = x as f64 + 0.5 > crossing;
            let expected = [
                if red { alpha } else { 0. },
                1. - alpha,
                if red { 0. } else { alpha },
            ];
            for channel in 0..3 {
                let actual = pixels.rgba[(16 * 64 + x) * 4 + channel] as f32;
                assert!(
                    (actual - expected[channel] * 255.).abs() < 1.1,
                    "frame {frame}, x={x}: {actual}, expected {expected:?}"
                );
            }
        }
        let stats = renderer.take_resource_churn_stats();
        if frame == 1 {
            assert_eq!(stats.opacity_group_renders, 0);
            assert_eq!(stats.opacity_group_cache_hits, 1);
        } else if frame < 3 {
            assert_eq!(stats.opacity_group_renders, 1);
        } else {
            assert_eq!(stats.opacity_group_renders, 0);
            if frame == 5 {
                assert_eq!(stats.spatial_fade_renders, 0);
                assert_eq!(stats.spatial_fade_cache_hits, 2);
            } else {
                assert_eq!(stats.spatial_fade_renders, 2);
            }
        }
        if frame > 0 {
            assert_eq!(stats.vector_geometry_rebuilds, 0);
        }
    }
}

#[test]
#[ignore = "requires a macOS Metal device"]
#[cfg(feature = "3d")]
fn nested_rotating_cuboids_bound_spatial_work_and_match_depth_oracle() {
    check_nested_rotating_cuboids(false);
}

#[test]
#[ignore = "requires a macOS Metal device"]
#[cfg(feature = "3d")]
fn fading_spatial_group_matches_depth_oracle_with_one_capture() {
    check_nested_rotating_cuboids(true);
}

#[cfg(feature = "3d")]
fn check_nested_rotating_cuboids(grouped: bool) {
    use pax_runtime_api::{DepthPlane, OpacityScope};
    let polygon_path = |points: &[[f64; 2]]| {
        let mut path = Path::builder();
        path.begin(point(points[0][0] as f32, points[0][1] as f32));
        for p in &points[1..] {
            path.line_to(point(p[0] as f32, p[1] as f32));
        }
        path.close();
        path.build()
    };
    // Signed edge distance also excludes antialiased boundary pixels from the oracle.
    let coverage = |points: &[[f64; 2]], p: [f64; 2]| {
        let distances: Vec<_> = (0..points.len())
            .map(|i| {
                let a = points[i];
                let b = points[(i + 1) % points.len()];
                ((b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]))
                    / (b[0] - a[0]).hypot(b[1] - a[1])
            })
            .collect();
        (
            distances.iter().all(|d| *d >= 0.0) || distances.iter().all(|d| *d <= 0.0),
            distances.iter().any(|d| d.abs() < 1.0),
        )
    };
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 160, 112, [1.0, 1.0]),
        )
    })
    .expect("Metal backend");
    // These pixel assertions exercise inline stencil updates, not screenshot-mirror fallback.
    assert!(backend.surface_supports_copy_src());
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    renderer.set_viewport(160.0, 112.0, [1.0, 1.0]);
    let aperture = [[4.0, 5.0], [156.0, 9.0], [150.0, 107.0], [8.0, 103.0]];
    let cube_faces = [
        [[-1., -1., 1.], [1., -1., 1.], [1., 1., 1.], [-1., 1., 1.]],
        [
            [1., -1., -1.],
            [-1., -1., -1.],
            [-1., 1., -1.],
            [1., 1., -1.],
        ],
        [[1., -1., 1.], [1., -1., -1.], [1., 1., -1.], [1., 1., 1.]],
        [
            [-1., -1., -1.],
            [-1., -1., 1.],
            [-1., 1., 1.],
            [-1., 1., -1.],
        ],
        [
            [-1., -1., -1.],
            [1., -1., -1.],
            [1., -1., 1.],
            [-1., -1., 1.],
        ],
        [[-1., 1., 1.], [1., 1., 1.], [1., 1., -1.], [-1., 1., -1.]],
    ];
    let mut max_fragments = 0;
    for frame in 0..24 {
        renderer.begin_node(1, 0, 0);
        renderer.fill_path(
            rect(0., 0., 160., 112.),
            Fill::Solid(Color::rgba(0., 0., 0., 1.)),
        );
        renderer.end_node(1);
        renderer.set_node_depth_plane(
            1,
            Some(DepthPlane {
                // Keep even near-edge-on bounding-plane extrapolations ahead of
                // this backdrop: this case specifically checks the contiguous path.
                coefficients: [0., 0., -1.0e6],
                backface: false,
            }),
        );
        renderer.begin_node(1000, 0, 0);
        renderer.save();
        renderer.clip(polygon_path(&aperture));
        renderer.end_node(1000);
        let mut oracle = Vec::new();
        for cube in 0..2 {
            let angle = frame as f64 * 0.09 + cube as f64 * 1.1;
            let (sy, cy) = angle.sin_cos();
            let (sx, cx) = (-0.31f64).sin_cos();
            // Two tilted route panels, each with a nested solid and its own aperture.
            let route = [
                [12.0 + cube as f64 * 24.0, 15.0],
                [139.0, 18.0],
                [145.0, 98.0],
                [16.0, 95.0],
            ];
            renderer.begin_node(1001 + cube, 0, 0);
            renderer.save();
            renderer.clip(polygon_path(&route));
            renderer.end_node(1001 + cube);
            let mut surfaces = vec![(
                route.to_vec(),
                DepthPlane {
                    presentation_layer: 0,
                    coefficients: [0.08 * (cube as f64 - 0.5), 0.02, -50.0],
                    backface: false,
                },
                [0.2, 0.15, 0.1, 0.5],
            )];
            for (face, corners) in cube_faces.iter().enumerate() {
                let world: Vec<[f64; 3]> = corners
                    .iter()
                    .map(|p| {
                        let x = p[0] * 26.;
                        let y = p[1] * 26. * cx - p[2] * 26. * sx;
                        let z = p[1] * 26. * sx + p[2] * 26. * cx;
                        [
                            x * cy + z * sy + 61. + cube as f64 * 35.,
                            y + 56.,
                            -x * sy + z * cy + cube as f64 * 10.,
                        ]
                    })
                    .collect();
                let u: [f64; 3] = std::array::from_fn(|i| world[1][i] - world[0][i]);
                let v: [f64; 3] = std::array::from_fn(|i| world[3][i] - world[0][i]);
                let n = [
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ];
                let a = -n[0] / n[2];
                let b = -n[1] / n[2];
                let plane = DepthPlane {
                    presentation_layer: 0,
                    coefficients: [a, b, world[0][2] - a * world[0][0] - b * world[0][1]],
                    backface: n[2] < 0.,
                };
                let color = [
                    (face % 3 == 0) as u8 as f64,
                    (face % 3 == 1) as u8 as f64,
                    (face % 3 == 2) as u8 as f64,
                    0.5,
                ];
                surfaces.push((world.iter().map(|p| [p[0], p[1]]).collect(), plane, color));
            }
            for (i, (points, plane, color)) in surfaces.into_iter().enumerate() {
                let id = 2 + cube * 7 + i as u32;
                renderer.begin_node(id, id as i32, 0);
                renderer.save();
                // Sibling scissor changes must not constrain the next stencil push/pop.
                renderer.clip(rect(10., 10., 140., 92.));
                renderer.fill_path(
                    polygon_path(&points),
                    Fill::Solid(Color::rgba(
                        color[0] as f32,
                        color[1] as f32,
                        color[2] as f32,
                        color[3] as f32,
                    )),
                );
                renderer.restore();
                renderer.end_node(id);
                renderer.set_node_depth_plane(id, Some(plane));
                if grouped {
                    renderer.set_node_opacity_scopes(
                        id,
                        &[OpacityScope {
                            node_id: 2000,
                            opacity: 0.25 + (frame % 3) as f32 * 0.25,
                        }],
                    );
                }
                oracle.push((points, plane, color, route));
            }
            renderer.restore();
        }
        renderer.restore();
        renderer.request_screenshot_capture(frame);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let pixels = renderer.take_screenshot_capture(frame).unwrap();
        let mut samples = 0;
        for y in (12..100).step_by(7) {
            for x in (12..148).step_by(7) {
                let p = [x as f64 + 0.5, y as f64 + 0.5];
                if coverage(&aperture, p).1 {
                    continue;
                }
                let mut layers = Vec::new();
                let mut edge = false;
                for (polygon, plane, color, route) in &oracle {
                    let (inside, near) = coverage(polygon, p);
                    let (in_route, near_route) = coverage(route, p);
                    edge |= near || near_route;
                    if inside && in_route && coverage(&aperture, p).0 {
                        layers.push((plane.depth_at(p[0], p[1]), color, plane));
                    }
                }
                if edge {
                    continue;
                }
                layers.sort_by(|a, b| a.0.total_cmp(&b.0));
                // MSAA pixels straddling an intersection blend both depth orders.
                // Compare the center-depth oracle only away from those boundaries.
                if layers.iter().enumerate().any(|(i, a)| {
                    layers[..i].iter().any(|b| {
                        let slope = (a.2.coefficients[0] - b.2.coefficients[0])
                            .hypot(a.2.coefficients[1] - b.2.coefficients[1]);
                        (a.0 - b.0).abs() < slope + 0.1
                    })
                }) {
                    continue;
                }
                let mut rgb = [0.0; 3];
                for (_, color, _) in layers {
                    for i in 0..3 {
                        rgb[i] = rgb[i] * (1. - color[3]) + color[i] * color[3];
                    }
                }
                if grouped {
                    for channel in &mut rgb {
                        *channel *= 0.25 + (frame % 3) as f64 * 0.25;
                    }
                }
                let offset = (y * 160 + x) * 4;
                for i in 0..3 {
                    let expected = (rgb[i] * 255.).round() as i32;
                    assert!(
                        (pixels.rgba[offset + i] as i32 - expected).abs() <= 3,
                        "frame {frame}, ({x},{y}), channel {i}: {} != {expected}",
                        pixels.rgba[offset + i]
                    );
                }
                samples += 1;
            }
        }
        assert!(samples > 80, "only {samples} interior samples");
        let stats = renderer.take_resource_churn_stats();
        max_fragments = max_fragments.max(stats.spatial_fragments);
        assert_eq!(
            stats.spatial_paint_passes,
            if grouped { 2 } else { 1 },
            "clip changes must stay in one color pass per target"
        );
        assert_eq!(stats.opacity_group_renders, u64::from(grouped));
        assert!(
            stats.retained_draws <= 200,
            "frame {frame}: {} draws for 15 surfaces",
            stats.retained_draws
        );
        assert!(stats.spatial_clip_fragments < stats.spatial_fragments);
    }
    eprintln!("nested cuboids: maximum {max_fragments} spatial fragments");
}

#[test]
#[ignore = "requires a macOS Metal device"]
#[cfg(feature = "3d")]
fn spatial_lighting_uses_world_depth_and_normal_without_retessellation() {
    use crate::{LightShape, Material, SceneLight, SceneLighting};
    use pax_runtime_api::LightingSurface;
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 32, 32, [1., 1.]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    let mut lighting = SceneLighting {
        active: true,
        ambient_intensity: 0.,
        lights: vec![SceneLight {
            shape: LightShape::Point,
            position: [1016.5, 2016.5, 100.],
            direction: [0., 0., -1.],
            color: Color::rgba(1., 1., 1., 1.),
            intensity: 1.,
            radius: 100.,
        }],
        ..Default::default()
    };
    let material = Material::custom(0., 1., 0., 0.5, Color::rgba(0., 0., 0., 1.), 0.);
    // World coordinates deliberately differ from raster coordinates: a tile/capture
    // rebase must never move a material relative to its light.
    for (frame, z, normal, expected) in [
        (0, 0., [0., 0., 1.], 0),
        (1, 50., [0., 0., 1.], 64),
        (2, 50., [1., 0., 0.], 0),
        (3, 50., [0., 0., -1.], 0),
    ] {
        renderer.set_scene_lighting(lighting.clone());
        renderer.begin_node(1, 1, 1);
        renderer.set_node_lighting_surface(
            1,
            Some(LightingSurface {
                origin: [1000., 2000., z],
                tangent_x: [1., 0., 0.],
                tangent_y: [0., 1., 0.],
                normal,
            }),
        );
        renderer.fill_path_with_material_and_opacity(
            rect(0., 0., 32., 32.),
            Fill::Solid(Color::rgba(1., 1., 1., 1.)),
            material,
            1.,
        );
        renderer.end_node(1);
        renderer.request_screenshot_capture(frame);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let capture = renderer.take_screenshot_capture(frame).unwrap();
        let pixel = &capture.rgba[(16 * 32 + 16) * 4..(16 * 32 + 16) * 4 + 4];
        assert!(
            (pixel[0] as i32 - expected).abs() <= 2,
            "frame {frame}: {pixel:?}"
        );
        let stats = renderer.take_resource_churn_stats();
        if frame > 0 {
            assert_eq!(stats.vector_geometry_rebuilds, 0);
            assert_eq!(stats.tessellated_vertices, 0);
        }
    }
    // Antiparallel light/view directions must remain finite (zero half-vector).
    lighting.lights[0].shape = LightShape::Directional;
    lighting.lights[0].direction = [0., 0., 1.];
    renderer.set_scene_lighting(lighting);
    renderer.request_screenshot_capture(4);
    renderer.flush();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    assert!(renderer.take_screenshot_capture(4).is_some());
    // Opacity capture changes raster origin, not the world-space lighting frame.
    let point_light = SceneLighting {
        active: true,
        ambient_intensity: 0.,
        lights: vec![SceneLight {
            shape: LightShape::Point,
            position: [1016.5, 2016.5, 100.],
            direction: [0., 0., -1.],
            color: Color::rgba(1., 1., 1., 1.),
            intensity: 1.,
            radius: 100.,
        }],
        ..Default::default()
    };
    renderer.set_scene_lighting(point_light);
    renderer.begin_node(1, 1, 1);
    renderer.set_node_lighting_surface(
        1,
        Some(LightingSurface {
            origin: [1000., 2000., 50.],
            tangent_x: [1., 0., 0.],
            tangent_y: [0., 1., 0.],
            normal: [0., 0., 1.],
        }),
    );
    renderer.set_node_opacity_scopes(
        1,
        &[pax_runtime_api::OpacityScope {
            node_id: 10,
            opacity: 0.5,
        }],
    );
    renderer.fill_path_with_material_and_opacity(
        rect(0., 0., 32., 32.),
        Fill::Solid(Color::rgba(1., 1., 1., 1.)),
        material,
        1.,
    );
    renderer.end_node(1);
    renderer.request_screenshot_capture(5);
    renderer.flush();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let capture = renderer.take_screenshot_capture(5).unwrap();
    let pixel = &capture.rgba[(16 * 32 + 16) * 4..(16 * 32 + 16) * 4 + 4];
    assert!(
        (pixel[0] as i32 - 32).abs() <= 2,
        "captured lighting: {pixel:?}"
    );
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn planar_lighting_shares_a_world_frame_across_canvas_draws() {
    use crate::{LightShape, Material, SceneLight, SceneLighting};
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 32, 32, [1., 1.]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    let mut lighting = SceneLighting {
        active: true,
        ambient_intensity: 0.,
        lights: vec![SceneLight {
            shape: LightShape::Point,
            position: [16.5, 16.5, 50.],
            direction: [0., 0., -1.],
            color: Color::rgba(1., 1., 1., 1.),
            intensity: 1.,
            radius: 100.,
        }],
        ..Default::default()
    };
    renderer.begin_node(1, 1, 1);
    renderer.set_node_opacity_scopes(
        1,
        &[pax_runtime_api::OpacityScope {
            node_id: 10,
            opacity: 0.5,
        }],
    );
    renderer.fill_path_with_material_and_opacity(
        rect(0., 0., 32., 32.),
        Fill::Solid(Color::rgba(1., 1., 1., 1.)),
        Material::custom(0., 1., 0., 0.5, Color::rgba(0., 0., 0., 1.), 0.),
        1.,
    );
    renderer.end_node(1);
    let mut reference = Vec::new();
    for frame in 0..3 {
        if frame == 1 {
            // A quarter-turn and translation of the owning canvas and lamp preserve
            // their relative pose. This also exercises the opacity capture's cache.
            lighting.surface_to_world = [0., 1., -1., 0., 1000., 2000.];
            lighting.lights[0].position = [983.5, 2016.5, 50.];
        } else if frame == 2 {
            lighting.surface_to_world[4] += 500.;
        }
        assert!(renderer.set_scene_lighting(lighting.clone()));
        renderer.request_screenshot_capture(frame);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let capture = renderer.take_screenshot_capture(frame).unwrap();
        let center = &capture.rgba[(16 * 32 + 16) * 4..(16 * 32 + 16) * 4 + 4];
        if frame == 0 {
            assert!(center[0] > 20, "fixture must be lit: {center:?}");
            reference = capture.rgba;
        } else if frame == 1 {
            assert!(capture
                .rgba
                .iter()
                .zip(&reference)
                .all(|(a, b)| a.abs_diff(*b) <= 1));
        } else {
            assert!(
                center[0] < 2,
                "moving only the canvas must invalidate lighting: {center:?}"
            );
        }
        let stats = renderer.take_resource_churn_stats();
        if frame > 0 {
            assert_eq!(stats.vector_geometry_rebuilds, 0);
        }
    }
}

#[cfg(feature = "3d")]
#[test]
#[ignore = "requires a macOS Metal device"]
fn opaque_ellipsoid_intersection_matches_mesh_depth_and_reuses_topology() {
    use pax_runtime_api::{
        mesh3::{ConvexMesh, OpaqueMeshDraw},
        DepthPlane,
    };
    let layer = MetalLayer::new();
    let mut backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 128, 128, [1., 1.]),
        )
    })
    .unwrap();
    // Also exercise the browser's mirrored screenshot path with shared depth/stencil.
    backend.surface_config.usage.remove(TextureUsages::COPY_SRC);
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    let mesh = ConvexMesh::sphere(64, 32);
    let plane = DepthPlane {
        presentation_layer: 0,
        coefficients: [0.25, 0., -16.],
        backface: false,
    };
    for frame in 0..64u32 {
        let z = (frame as f64 - 31.) * 2.;
        let pose = [
            42., 0., 12., 0., 0., 38., 0., 0., 8., 0., 28., 0., 64., 64., z, 1.,
        ];
        let posed = mesh.posed(pose).unwrap();
        renderer.begin_node(1, 1, 0);
        renderer.save();
        renderer.clip(rect(28., 18., 70., 88.));
        renderer.draw_convex_mesh(
            &OpaqueMeshDraw {
                mesh: mesh.clone(),
                world: pose,
                surface: pax_runtime_api::mesh3::MeshProjection::new(pose, false).unwrap(),
                color: pax_runtime_api::Color::RED,
                material: pax_runtime_api::ResolvedMaterial::Unlit,
            },
            Color::rgba(1., 0., 0., 1.),
            crate::Material::unlit(),
        );
        renderer.restore();
        renderer.end_node(1);
        if frame == 0 {
            renderer.begin_node(2, 2, 0);
            renderer.set_node_depth_plane(2, Some(plane));
            renderer.save();
            renderer.clip(rect(16., 30., 96., 68.));
            renderer.fill_path(
                rect(0., 0., 128., 128.),
                Fill::Solid(Color::rgba(0., 0., 1., 1.)),
            );
            renderer.restore();
            renderer.end_node(2);
            renderer.begin_node(0, 0, 0);
            renderer.set_node_depth_plane(
                0,
                Some(DepthPlane {
                    presentation_layer: 0,
                    coefficients: [0., 0., -1000.],
                    backface: false,
                }),
            );
            renderer.fill_path(
                rect(0., 0., 128., 128.),
                Fill::Solid(Color::rgba(0., 0., 0., 1.)),
            );
            renderer.end_node(0);
        }
        renderer.request_screenshot_capture(frame);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let image = renderer.take_screenshot_capture(frame).unwrap();
        for y in (8..120).step_by(8) {
            for x in (8..120).step_by(8) {
                let sample = (x as f64 + 0.5, y as f64 + 0.5);
                let solid_depth = if x >= 28 && x < 98 && y >= 18 && y < 106 {
                    posed.depth_at(sample.0, sample.1)
                } else {
                    None
                };
                let panel = x >= 16 && x < 112 && y >= 30 && y < 98;
                // Exclude only antialiased intersection/silhouette edge samples.
                let unstable = [(-1., 0.), (1., 0.), (0., -1.), (0., 1.)]
                    .iter()
                    .any(|(dx, dy)| {
                        let d = posed.depth_at(sample.0 + dx, sample.1 + dy);
                        d.is_some() != posed.depth_at(sample.0, sample.1).is_some()
                            || d.zip(solid_depth).is_some_and(|(d, z)| {
                                panel
                                    && (d - plane.depth_at(sample.0 + dx, sample.1 + dy)).signum()
                                        != (z - plane.depth_at(sample.0, sample.1)).signum()
                            })
                    });
                if unstable {
                    continue;
                }
                let expected = if solid_depth
                    .is_some_and(|z| !panel || z > plane.depth_at(sample.0, sample.1))
                {
                    [255, 0, 0, 255]
                } else if panel {
                    [0, 0, 255, 255]
                } else {
                    [0, 0, 0, 255]
                };
                assert_pixel(&image, x, y, expected);
            }
        }
        let stats = renderer.take_resource_churn_stats();
        if frame > 0 {
            assert_eq!(
                stats.solid_geometry_upload_bytes, 0,
                "frame {frame} reuploaded immutable topology"
            );
            assert_eq!(stats.solid_uniform_updates, 1);
            assert_eq!(stats.vector_geometry_rebuilds, 0);
        }
    }
}

#[cfg(feature = "3d")]
#[test]
#[ignore = "requires a macOS Metal device"]
fn overlapping_opaque_solids_respect_concave_clips_and_planar_opacity() {
    use pax_runtime_api::{
        mesh3::{ConvexMesh, OpaqueMeshDraw},
        DepthPlane, OpacityScope,
    };
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 128, 128, [1., 1.]),
        )
    })
    .unwrap();
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    let mesh = ConvexMesh::sphere(64, 32);
    let draw = |x, z| {
        let pose = [
            38., 0., 0., 0., 0., 38., 0., 0., 0., 0., 38., 0., x, 64., z, 1.,
        ];
        OpaqueMeshDraw {
            mesh: mesh.clone(),
            world: pose,
            surface: pax_runtime_api::mesh3::MeshProjection::new(pose, false).unwrap(),
            color: pax_runtime_api::Color::RED,
            material: pax_runtime_api::ResolvedMaterial::Unlit,
        }
    };
    for frame in 0..3 {
        let red = draw(50., if frame == 0 { 5. } else { 30. });
        let green = draw(78., 15.);
        renderer.begin_node(1, 2, 0);
        renderer.draw_convex_mesh(&red, Color::rgba(1., 0., 0., 1.), crate::Material::unlit());
        renderer.end_node(1);
        renderer.begin_node(2, 1, 0);
        renderer.save();
        let mut clip = Path::builder();
        clip.begin(point(10., 10.));
        for p in [
            [118., 10.],
            [118., 118.],
            [70., 118.],
            [70., 80.],
            [10., 80.],
        ] {
            clip.line_to(point(p[0], p[1]));
        }
        clip.close();
        renderer.clip(clip.build());
        renderer.draw_convex_mesh(
            &green,
            Color::rgba(0., 1., 0., 1.),
            crate::Material::unlit(),
        );
        renderer.restore();
        renderer.end_node(2);
        if frame == 0 {
            renderer.begin_node(0, 0, 0);
            renderer.set_node_depth_plane(
                0,
                Some(DepthPlane {
                    presentation_layer: 0,
                    coefficients: [0., 0., -1000.],
                    backface: false,
                }),
            );
            renderer.fill_path(
                rect(0., 0., 128., 128.),
                Fill::Solid(Color::rgba(0., 0., 0., 1.)),
            );
            renderer.end_node(0);
            renderer.begin_node(3, 3, 0);
            renderer.set_node_depth_plane(
                3,
                Some(DepthPlane {
                    presentation_layer: 0,
                    coefficients: [0., 0., 42.],
                    backface: false,
                }),
            );
            renderer.set_node_opacity_scopes(
                3,
                &[OpacityScope {
                    node_id: 10,
                    opacity: 0.5,
                }],
            );
            renderer.fill_path(
                rect(20., 20., 88., 88.),
                Fill::Solid(Color::rgba(0., 0., 1., 1.)),
            );
            renderer.end_node(3);
        }
        if frame == 2 {
            renderer.remove_node(2);
        }
        renderer.request_screenshot_capture(frame);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let image = renderer.take_screenshot_capture(frame).unwrap();
        let red = mesh.posed(red.surface.matrix()).unwrap();
        let green = mesh.posed(green.surface.matrix()).unwrap();
        for (x, y) in [
            (50, 48),
            (68, 60),
            (65, 90),
            (80, 94),
            (36, 50),
            (96, 60),
            (8, 8),
        ] {
            let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
            let r = red.depth_at(px, py);
            let g = if frame != 2 && (x >= 70 || y < 80) {
                green.depth_at(px, py)
            } else {
                None
            };
            let (depth, mut color) = match (r, g) {
                (Some(r), Some(g)) if g > r => (g, [0u8, 255, 0, 255]),
                (Some(r), _) => (r, [255, 0, 0, 255]),
                (_, Some(g)) => (g, [0, 255, 0, 255]),
                _ => (-1000., [0, 0, 0, 255]),
            };
            if x >= 20 && x < 108 && y >= 20 && y < 108 && depth < 42. {
                color[0] /= 2;
                color[1] /= 2;
                color[2] = 128;
            }
            let actual = &image.rgba[(y * 128 + x) * 4..][..4];
            for c in 0..4 {
                assert!(
                    actual[c].abs_diff(color[c]) <= 1,
                    "frame {frame} at ({x},{y}): {actual:?} != {color:?}"
                );
            }
        }
        let stats = renderer.take_resource_churn_stats();
        if frame > 0 {
            assert_eq!(stats.solid_geometry_upload_bytes, 0);
        }
    }
}

#[cfg(feature = "3d")]
#[test]
#[ignore = "requires a macOS Metal device"]
fn camera_clipped_solids_match_depth_and_keep_shared_topology() {
    use pax_runtime_api::{
        camera::{Camera, CameraProjection, CameraSpace},
        math::{Generic, PlanarProjection, Transform3, Vector3},
        mesh3::{ConvexMesh, MeshProjection, OpaqueMeshDraw},
        DepthPlane,
    };
    type M = Transform3<Generic>;
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 128, 128, [1., 1.]),
        )
    })
    .unwrap();
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    let mesh = ConvexMesh::sphere(32, 16);
    let camera = Camera::new(
        Transform3::<Generic, CameraSpace>::identity(),
        CameraProjection::perspective((128., 128.), 1.2, 1., 20.).unwrap(),
    )
    .unwrap();
    let view = camera.project_plane(M::identity()).unwrap().matrix();
    let plane = DepthPlane {
        presentation_layer: 0,
        coefficients: [0.001, 0., 0.45],
        backface: false,
    };
    for (frame, z) in [-4., -2., -1.3, -0.5, 1., -25., -2.]
        .into_iter()
        .enumerate()
    {
        let world = M::translate(Vector3::new(0., 0., z))
            * M::rotate_y(0.6)
            * M::scale(Vector3::new(1.4, 0.9, 1.));
        let mapping = if frame == 6 {
            PlanarProjection([1., 0., 0.003, 0., 1., 0., 20., 10., 1.])
        } else {
            PlanarProjection::default()
        };
        let projection = MeshProjection::new((view * world).coeffs(), true)
            .unwrap()
            .post_mapping(mapping);
        let projected = mesh.projected(projection, Some(kurbo::Rect::new(0., 0., 128., 128.)));
        renderer.begin_node(1, 1, 0);
        renderer.draw_convex_mesh(
            &OpaqueMeshDraw {
                mesh: mesh.clone(),
                world: world.coeffs(),
                surface: projection,
                color: pax_runtime_api::Color::RED,
                material: pax_runtime_api::ResolvedMaterial::Unlit,
            },
            Color::rgba(1., 0., 0., 1.),
            crate::Material::unlit(),
        );
        renderer.end_node(1);
        if frame == 0 {
            renderer.begin_node(2, 2, 0);
            renderer.set_node_depth_plane(2, Some(plane));
            renderer.fill_path(
                rect(0., 0., 128., 128.),
                Fill::Solid(Color::rgba(0., 0., 1., 1.)),
            );
            renderer.end_node(2);
        }
        renderer.request_screenshot_capture(frame as u32);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let image = renderer.take_screenshot_capture(frame as u32).unwrap();
        let front = |x, y| {
            projected
                .depth_at(x, y)
                .is_some_and(|z| z > plane.depth_at(x, y) + 1e-7)
        };
        let mut samples = 0;
        for y in 4..124 {
            for x in 4..124 {
                let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                let expected = front(px, py);
                if [(-1., 0.), (1., 0.), (0., -1.), (0., 1.)]
                    .iter()
                    .any(|(dx, dy)| front(px + dx, py + dy) != expected)
                {
                    continue;
                }
                assert_eq!(
                    &image.rgba[(y * 128 + x) * 4..(y * 128 + x) * 4 + 4],
                    &if expected {
                        [255, 0, 0, 255]
                    } else {
                        [0, 0, 255, 255]
                    },
                    "frame {frame} z={z} at {x},{y}, depth {:?}",
                    projected.depth_at(px, py)
                );
                samples += 1;
            }
        }
        assert!(samples > 400, "frame {frame}: insufficient stable samples");
        let stats = renderer.take_resource_churn_stats();
        if frame > 0 {
            assert_eq!(stats.solid_geometry_upload_bytes, 0);
            assert_eq!(stats.solid_uniform_updates, 1);
        }
    }
}

#[cfg(feature = "shaders")]
#[test]
#[ignore = "requires a macOS Metal device"]
fn shader_composite_respects_overlay_domain_above_background() {
    use pax_runtime_api::{DepthPlane, ShaderEffect, ShaderInput, ShaderPass, ShaderProgram};
    static PROGRAM: ShaderProgram = ShaderProgram {
        label: "overlay composite",
        passes: &[ShaderPass {
            label: "identity",
            source: "fn effect(p: vec2<f32>) -> vec4<f32> { return sample_input0(p); }",
            parameters: &[],
            inputs: [ShaderInput::Subtree; 2],
            downsample: 1,
        }],
    };
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 64, 32, [1., 1.]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    renderer.set_viewport(64., 32., [1., 1.]);
    renderer.begin_node(1, 1, 0);
    renderer.fill_path(
        rect(0., 0., 64., 32.),
        Fill::Solid(Color::rgba(1., 0., 0., 1.)),
    );
    renderer.end_node(1);
    renderer.set_node_depth_plane(
        1,
        Some(DepthPlane {
            presentation_layer: 1,
            ..Default::default()
        }),
    );
    let effect = ShaderEffect {
        program: &PROGRAM,
        parameters: vec![vec![]],
        outset: 0.,
    };
    for (frame, domain) in [0, 2, 1, 0].into_iter().enumerate() {
        renderer.begin_shader_effect(
            2,
            2,
            &effect,
            Transform2D::identity(),
            (64., 32.),
            &[],
            &[3],
        );
        renderer.set_node_depth_plane(
            2,
            (domain > 0).then_some(DepthPlane {
                presentation_layer: domain,
                ..Default::default()
            }),
        );
        renderer.begin_node(3, 3, 0);
        renderer.fill_path(
            rect(8., 8., 48., 16.),
            Fill::Solid(Color::rgba(0., 0., 1., 1.)),
        );
        renderer.end_node(3);
        renderer.set_node_depth_plane(
            3,
            Some(DepthPlane {
                presentation_layer: 2,
                ..Default::default()
            }),
        );
        renderer.end_shader_effect();
        renderer.request_screenshot_capture(frame as u32);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let pixels = renderer.take_screenshot_capture(frame as u32).unwrap();
        assert_pixel(
            &pixels,
            32,
            16,
            if domain == 0 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 255]
            },
        );
        assert_pixel(&pixels, 2, 16, [255, 0, 0, 255]);
    }
}

#[cfg(feature = "3d")]
#[test]
#[ignore = "requires a macOS Metal device"]
fn spatial_group_fade_keeps_hidden_surfaces_hidden_across_external_geometry() {
    use pax_runtime_api::{
        mesh3::{ConvexMesh, OpaqueMeshDraw},
        DepthPlane, OpacityScope,
    };
    let layer = MetalLayer::new();
    let mut backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 128, 128, [1., 1.]),
        )
    })
    .unwrap();
    backend.surface_config.usage.remove(TextureUsages::COPY_SRC);
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    let mesh = ConvexMesh::sphere(64, 32);
    let blue_plane = DepthPlane {
        presentation_layer: 0,
        coefficients: [0.15, 0., -20.],
        backface: false,
    };
    for frame in 0..36u32 {
        let g = [1., 0.5, 0.2, 0., 0.8, 1.][frame as usize % 6];
        let h = if frame < 12 { 1. } else { 0.65 };
        // First twelve frames change only opacity; subsequent poses invalidate
        // affected presence views while retaining mesh and clip-resource identity.
        let z = if frame < 12 {
            0.
        } else {
            (frame as f64 * 0.35).sin() * 24.
        };
        let poses = [
            [
                34.,
                0.,
                0.,
                0.,
                0.,
                36.,
                0.,
                0.,
                0.,
                0.,
                26.,
                0.,
                49.,
                64.,
                30. + z,
                1.,
            ],
            [
                34., 0., 0., 0., 0., 32., 0., 0., 0., 0., 22., 0., 78., 64., -40., 1.,
            ],
        ];
        for (index, pose) in poses.iter().enumerate() {
            let id = index as u32 + 1;
            renderer.begin_node(id, id as i32, 0);
            renderer.set_node_opacity_scopes(
                id,
                &[OpacityScope {
                    node_id: 100,
                    opacity: g,
                }],
            );
            renderer.draw_convex_mesh(
                &OpaqueMeshDraw {
                    mesh: mesh.clone(),
                    world: *pose,
                    surface: pax_runtime_api::mesh3::MeshProjection::new(*pose, false).unwrap(),
                    color: pax_runtime_api::Color::RED,
                    material: pax_runtime_api::ResolvedMaterial::Unlit,
                },
                if index == 0 {
                    Color::rgba(1., 0., 0., 1.)
                } else {
                    Color::rgba(0., 1., 0., 1.)
                },
                crate::Material::unlit(),
            );
            renderer.end_node(id);
        }
        if frame == 0 {
            renderer.begin_node(0, 0, 0);
            renderer.set_node_depth_plane(
                0,
                Some(DepthPlane {
                    presentation_layer: 0,
                    coefficients: [0., 0., -1000.],
                    backface: false,
                }),
            );
            renderer.fill_path(
                rect(0., 0., 128., 128.),
                Fill::Solid(Color::rgba(0., 0., 0., 1.)),
            );
            renderer.end_node(0);
            renderer.begin_node(3, 3, 0);
            renderer.set_node_depth_plane(
                3,
                Some(DepthPlane {
                    presentation_layer: 0,
                    coefficients: [0., 0., -75.],
                    backface: false,
                }),
            );
            renderer.fill_path(
                rect(22., 22., 84., 84.),
                Fill::Solid(Color::rgba(1., 1., 1., 1.)),
            );
            renderer.end_node(3);
            renderer.begin_node(4, 4, 0);
            renderer.set_node_depth_plane(4, Some(blue_plane));
            renderer.fill_path(
                rect(8., 8., 112., 112.),
                Fill::Solid(Color::rgba(0., 0., 1., 0.5)),
            );
            renderer.end_node(4);
        }
        renderer.set_node_opacity_scopes(
            3,
            &[OpacityScope {
                node_id: 100,
                opacity: g,
            }],
        );
        // Reparent the blue surface into the assembly in the final phase.
        // Its child opacity must become conditional on the parent being present.
        let nested = frame >= 24;
        let mut outside_scopes = vec![OpacityScope {
            node_id: 200,
            opacity: h,
        }];
        if nested {
            outside_scopes.insert(
                0,
                OpacityScope {
                    node_id: 100,
                    opacity: g,
                },
            );
        }
        renderer.set_node_opacity_scopes(4, &outside_scopes);
        renderer.request_screenshot_capture(frame);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let image = renderer.take_screenshot_capture(frame).unwrap();
        let red = mesh.posed(poses[0]).unwrap();
        let green = mesh.posed(poses[1]).unwrap();
        for (x, y) in [
            (49, 64),
            (72, 64),
            (88, 64),
            (54, 50),
            (38, 80),
            (28, 28),
            (110, 110),
            (3, 3),
        ] {
            let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
            let mut expected = [0.; 3];
            for present in [false, true] {
                for outside in [false, true] {
                    let weight = if nested && !present {
                        if outside {
                            continue;
                        }
                        (1. - g) as f64
                    } else {
                        (if present { g } else { 1. - g }) as f64
                            * (if outside { h } else { 1. - h }) as f64
                    };
                    let mut fragments = vec![(-1000., [0., 0., 0., 1.])];
                    if present {
                        if let Some(z) = red.depth_at(px, py) {
                            fragments.push((z, [1., 0., 0., 1.]));
                        }
                        if let Some(z) = green.depth_at(px, py) {
                            fragments.push((z, [0., 1., 0., 1.]));
                        }
                        if (22..106).contains(&x) && (22..106).contains(&y) {
                            fragments.push((-75., [1., 1., 1., 1.]));
                        }
                    }
                    if outside && (8..120).contains(&x) && (8..120).contains(&y) {
                        fragments.push((blue_plane.depth_at(px, py), [0., 0., 1., 0.5]));
                    }
                    fragments.sort_by(|a, b| a.0.total_cmp(&b.0));
                    let mut color = [0.; 3];
                    for (_, rgba) in fragments {
                        for c in 0..3 {
                            color[c] = rgba[c] * rgba[3] + color[c] * (1. - rgba[3]);
                        }
                    }
                    for c in 0..3 {
                        expected[c] += color[c] * weight;
                    }
                }
            }
            let actual = &image.rgba[(y * 128 + x) * 4..][..4];
            for c in 0..3 {
                assert!(
                    actual[c].abs_diff((expected[c] * 255.).round() as u8) <= 3,
                    "frame {frame}, pixel {x},{y}: {actual:?}, expected {expected:?}"
                );
            }
            assert!(actual[3] >= 253);
        }
        let stats = renderer.take_resource_churn_stats();
        assert_eq!(stats.spatial_fade_overflows, 0);
        assert!(stats.spatial_fade_renders <= 4);
        if frame > 0 {
            assert_eq!(stats.solid_geometry_upload_bytes, 0);
        }
        if (2..12).contains(&frame) {
            assert_eq!(
                stats.spatial_fade_renders, 0,
                "opacity-only updates must reuse both scene views"
            );
        }
        assert!(renderer.spatial_fade_capture_count() <= 8);
    }
    // Scope removal and physical-tile reuse must retire both capture residency
    // and admission state, even when the new tile reuses the same node IDs.
    for id in 1..=4 {
        renderer.set_node_opacity_scopes(id, &[]);
    }
    renderer.flush();
    assert_eq!(renderer.spatial_fade_capture_count(), 0);
    renderer.reset_retained_scene();
    renderer.begin_node(1, 1, 0);
    renderer.fill_path(
        rect(0., 0., 128., 128.),
        Fill::Solid(Color::rgba(0., 1., 0., 1.)),
    );
    renderer.end_node(1);
    renderer.request_screenshot_capture(99);
    renderer.flush();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    assert_pixel(
        &renderer.take_screenshot_capture(99).unwrap(),
        64,
        64,
        [0, 255, 0, 255],
    );
    assert_eq!(renderer.spatial_fade_capture_count(), 0);
}

#[cfg(feature = "3d")]
#[test]
#[ignore = "requires a macOS Metal device"]
fn spatial_fade_disjoint_components_and_overflow_are_bounded() {
    use pax_runtime_api::{
        mesh3::{ConvexMesh, OpaqueMeshDraw},
        DepthPlane, OpacityScope,
    };
    for dpr in [1., 2.] {
        let layer = MetalLayer::new();
        let backend = pollster::block_on(unsafe {
            RenderBackend::to_core_animation_layer(
                layer.0.cast(),
                RenderConfig::new(true, (128. * dpr) as u32, (128. * dpr) as u32, [dpr, dpr]),
            )
        })
        .unwrap();
        let device = backend.device.clone();
        let mut renderer = WgpuRenderer::new(backend);
        renderer.set_viewport(128., 128., [dpr, dpr]);
        let pose = [
            3., 0., 0., 0., 0., 3., 0., 0., 0., 0., 3., 0., 12., 40., 20., 1.,
        ];
        renderer.begin_node(9, 9, 0);
        renderer.set_node_opacity_scopes(
            9,
            &[OpacityScope {
                node_id: 100,
                opacity: 0.5,
            }],
        );
        renderer.draw_convex_mesh(
            &OpaqueMeshDraw {
                mesh: ConvexMesh::sphere(16, 8),
                world: pose,
                surface: pax_runtime_api::mesh3::MeshProjection::new(pose, false).unwrap(),
                color: pax_runtime_api::Color::RED,
                material: pax_runtime_api::ResolvedMaterial::Unlit,
            },
            Color::rgba(1., 0., 0., 1.),
            crate::Material::unlit(),
        );
        renderer.end_node(9);
        for overlapping in [false, true] {
            for id in 0..4u32 {
                renderer.begin_node(id, id as i32, 0);
                renderer.set_node_depth_plane(
                    id,
                    Some(DepthPlane {
                        presentation_layer: 0,
                        coefficients: [0., 0., id as f64],
                        backface: false,
                    }),
                );
                renderer.set_node_opacity_scopes(
                    id,
                    &[OpacityScope {
                        node_id: 100 + id,
                        opacity: 0.5,
                    }],
                );
                renderer.fill_path(
                    rect(
                        if overlapping {
                            32.
                        } else {
                            id as f32 * 30. + 2.
                        },
                        20.,
                        24.,
                        60.,
                    ),
                    Fill::Solid(Color::rgba(1., 0., 0., 1.)),
                );
                renderer.end_node(id);
            }
            renderer.request_screenshot_capture(1);
            renderer.flush();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let image = renderer.take_screenshot_capture(1).unwrap();
            let stats = renderer.take_resource_churn_stats();
            if overlapping {
                assert_eq!(stats.spatial_fade_overflows, 1);
                assert_eq!(stats.spatial_fade_renders, 0);
                assert_pixel(
                    &image,
                    (40. * dpr) as usize,
                    (40. * dpr) as usize,
                    [255, 0, 0, 255],
                );
            } else {
                assert_eq!(stats.spatial_fade_renders, 8);
                for x in [12, 42, 72, 102] {
                    assert_pixel(
                        &image,
                        (x as f32 * dpr) as usize,
                        (40. * dpr) as usize,
                        [128, 0, 0, 128],
                    );
                }
            }
        }
    }
}

#[cfg(feature = "3d")]
#[test]
#[ignore = "requires a macOS Metal device"]
fn spatial_fade_reuses_opacity_but_invalidates_camera_projection_and_clip_planes() {
    use pax_runtime_api::{
        camera::{Camera, CameraProjection, CameraSpace},
        math::{Generic, Transform3, Vector3},
        mesh3::{ConvexMesh, MeshProjection, OpaqueMeshDraw},
        DepthPlane, OpacityScope,
    };
    type M = Transform3<Generic>;
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 128, 128, [1., 1.]),
        )
    })
    .unwrap();
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    let mesh = ConvexMesh::sphere(32, 16);
    let camera = Camera::new(
        Transform3::<Generic, CameraSpace>::identity(),
        CameraProjection::perspective((128., 128.), 1.2, 2., 20.).unwrap(),
    )
    .unwrap();
    let world = M::translate(Vector3::new(0., 0., -2.));
    let matrix = (camera.project_plane(M::identity()).unwrap().matrix() * world).coeffs();
    let plane = DepthPlane {
        presentation_layer: 0,
        coefficients: [0., 0., 0.9],
        backface: false,
    };
    renderer.begin_node(2, 2, 0);
    renderer.set_node_depth_plane(2, Some(plane));
    renderer.fill_path(
        rect(0., 0., 128., 128.),
        Fill::Solid(Color::rgba(0., 0., 1., 1.)),
    );
    renderer.end_node(2);
    for (frame, (clipped, pan, opacity)) in [
        (false, 0., 0.5),
        (false, 0., 0.25),
        (true, 0., 0.25),
        (false, 0., 0.5),
        (false, 24., 0.5),
        (false, 24., 0.25),
    ]
    .into_iter()
    .enumerate()
    {
        let projection = MeshProjection::new(matrix, clipped)
            .unwrap()
            .post_mapping(kurbo::Affine::translate((pan, 0.)).into());
        let projected = mesh.projected(projection, Some(kurbo::Rect::new(0., 0., 128., 128.)));
        renderer.begin_node(1, 1, 0);
        renderer.set_node_opacity_scopes(
            1,
            &[OpacityScope {
                node_id: 10,
                opacity,
            }],
        );
        renderer.draw_convex_mesh(
            &OpaqueMeshDraw {
                mesh: mesh.clone(),
                world: world.coeffs(),
                surface: projection,
                color: pax_runtime_api::Color::RED,
                material: pax_runtime_api::ResolvedMaterial::Unlit,
            },
            Color::rgba(1., 0., 0., 1.),
            crate::Material::unlit(),
        );
        renderer.end_node(1);
        renderer.request_screenshot_capture(frame as u32);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let image = renderer.take_screenshot_capture(frame as u32).unwrap();
        let front = |x, y| projected.depth_at(x, y).is_some_and(|z| z > 0.9 + 1e-7);
        for y in (4..124).step_by(5) {
            for x in (4..124).step_by(5) {
                let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                let red = front(px, py);
                if [(-1., 0.), (1., 0.), (0., -1.), (0., 1.)]
                    .iter()
                    .any(|(dx, dy)| front(px + dx, py + dy) != red)
                {
                    continue;
                }
                let alpha = if red { opacity } else { 0. };
                assert_pixel(
                    &image,
                    x,
                    y,
                    [
                        (255. * alpha).round() as u8,
                        0,
                        (255. * (1. - alpha)).round() as u8,
                        255,
                    ],
                );
            }
        }
        let stats = renderer.take_resource_churn_stats();
        if frame > 0 {
            assert_eq!(stats.solid_geometry_upload_bytes, 0);
        }
        if frame == 1 || frame == 5 {
            assert_eq!(
                stats.spatial_fade_renders, 0,
                "opacity preserves both camera views"
            );
        } else {
            assert!(
                stats.spatial_fade_renders > 0,
                "camera/clip changes invalidate captures"
            );
        }
    }
}

#[cfg(feature = "3d")]
#[test]
#[ignore = "requires a macOS Metal device"]
fn shared_solid_relations_reuse_stencils_and_retire_tile_dependencies() {
    use pax_runtime_api::{
        mesh3::{ConvexMesh, MeshProjection, OpaqueMeshDraw},
        visibility::{CoverageContext, Relations},
        DepthPlane,
    };
    use std::{cell::RefCell, rc::Rc};
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 128, 128, [1., 1.]),
        )
    })
    .unwrap();
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    let relations = Rc::new(RefCell::new(Relations::default()));
    let context = |node, support| CoverageContext {
        node: (node, 1),
        support: (support, 1),
        domain: 4,
        relations: relations.clone(),
    };
    let mesh = ConvexMesh::sphere(16, 8);
    let pose = [
        40., 0., 0., 0., 0., 40., 0., 0., 0., 0., 30., 0., 64., 64., 0., 1.,
    ];
    for frame in 0..3 {
        renderer.begin_node(1, 1, 0);
        renderer.set_node_coverage_context(1, context(1, 1));
        renderer.save();
        renderer.clip(rect(if frame == 2 { 64. } else { 0. }, 0., 128., 128.));
        renderer.draw_convex_mesh(
            &OpaqueMeshDraw {
                mesh: mesh.clone(),
                world: pose,
                surface: MeshProjection::new(pose, false).unwrap(),
                color: pax_runtime_api::Color::RED,
                material: pax_runtime_api::ResolvedMaterial::Unlit,
            },
            Color::rgba(1., 0., 0., 1.),
            crate::Material::unlit(),
        );
        renderer.restore();
        renderer.end_node(1);
        for id in 2..8 {
            renderer.begin_node(id, id as i32, 0);
            renderer.set_node_coverage_context(id, context(id, 100));
            renderer.set_node_depth_plane(id, Some(DepthPlane::default()));
            renderer.fill_path(
                rect(0., 0., 128., 128.),
                Fill::Solid(if frame == 0 {
                    Color::rgba(0., 0., 1., 1.)
                } else {
                    Color::rgba(0., 1., 0., 1.)
                }),
            );
            renderer.end_node(id);
        }
        renderer.request_screenshot_capture(frame);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let capture = renderer.take_screenshot_capture(frame).unwrap();
        let pixel = |x: usize, y: usize| &capture.rgba[(y * 128 + x) * 4..(y * 128 + x) * 4 + 4];
        assert_eq!(
            pixel(48, 64),
            if frame == 2 {
                &[0, 255, 0, 255]
            } else {
                &[255, 0, 0, 255]
            }
        );
        let stats = renderer.take_resource_churn_stats();
        assert_eq!(
            relations.borrow().stats.evaluations,
            1,
            "six coplanar leaves and paint/clip updates share one geometric relation"
        );
        if frame == 1 {
            assert_eq!(stats.solid_clip_preparations, 0);
            assert!(stats.solid_clip_reuses >= 6);
            assert_eq!(stats.solid_geometry_upload_bytes, 0);
        } else {
            assert!(
                stats.solid_clip_preparations >= 6,
                "changed hard clip invalidates stencils"
            );
        }
    }
    assert_eq!(relations.borrow().residency().0, 1);
    drop(renderer);
    assert_eq!(
        relations.borrow().residency(),
        (0, 0),
        "tile destruction releases its chart"
    );
}

#[cfg(feature = "3d")]
#[test]
#[ignore = "requires a macOS Metal device"]
fn overlay_domain_stays_above_world_depth_and_reorders_retained_paint() {
    use pax_runtime_api::DepthPlane;
    let layer = MetalLayer::new();
    let backend = pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer(
            layer.0.cast(),
            RenderConfig::new(true, 64, 32, [1.0, 1.0]),
        )
    })
    .expect("Metal backend");
    let device = backend.device.clone();
    let mut renderer = WgpuRenderer::new(backend);
    renderer.set_viewport(64.0, 32.0, [1.0, 1.0]);
    for (id, z, color) in [
        (1, 10, Color::rgba(1.0, 0.0, 0.0, 1.0)),
        (2, -10, Color::rgba(0.0, 0.0, 1.0, 1.0)),
    ] {
        renderer.begin_node(id, z, 0);
        renderer.fill_path(rect(0.0, 0.0, 64.0, 32.0), Fill::Solid(color));
        renderer.end_node(id);
    }
    renderer.set_node_depth_plane(
        1,
        Some(DepthPlane {
            presentation_layer: 0,
            coefficients: [1.0, 0.0, 1_000_000.0],
            ..Default::default()
        }),
    );
    for (frame, domain) in [1, 0, 1].into_iter().enumerate() {
        renderer.set_node_depth_plane(
            2,
            Some(DepthPlane {
                presentation_layer: domain,
                ..Default::default()
            }),
        );
        renderer.request_screenshot_capture(frame as u32);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let pixels = renderer.take_screenshot_capture(frame as u32).unwrap();
        for x in [8, 32, 56] {
            assert_pixel(
                &pixels,
                x,
                16,
                if domain == 1 {
                    [0, 0, 255, 255]
                } else {
                    [255, 0, 0, 255]
                },
            );
        }
    }
    renderer.set_node_depth_plane(
        2,
        Some(DepthPlane {
            presentation_layer: 1,
            ..Default::default()
        }),
    );
    renderer.begin_node(3, -20, 0);
    renderer.fill_path(
        rect(0.0, 0.0, 64.0, 32.0),
        Fill::Solid(Color::rgba(0.0, 1.0, 0.0, 1.0)),
    );
    renderer.end_node(3);
    renderer.begin_node(4, -30, 0);
    renderer.fill_path(
        rect(32.0, 0.0, 64.0, 32.0),
        Fill::Solid(Color::rgba(1.0, 1.0, 0.0, 1.0)),
    );
    renderer.end_node(4);
    renderer.set_node_depth_plane(
        4,
        Some(DepthPlane {
            presentation_layer: 3,
            ..Default::default()
        }),
    );
    for (frame, domain) in [2, 0, 2].into_iter().enumerate() {
        renderer.set_node_depth_plane(
            3,
            Some(DepthPlane {
                presentation_layer: domain,
                ..Default::default()
            }),
        );
        let capture = frame as u32 + 10;
        renderer.request_screenshot_capture(capture);
        renderer.flush();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let pixels = renderer.take_screenshot_capture(capture).unwrap();
        assert_pixel(
            &pixels,
            8,
            16,
            if domain == 2 {
                [0, 255, 0, 255]
            } else {
                [0, 0, 255, 255]
            },
        );
        assert_pixel(&pixels, 56, 16, [255, 255, 0, 255]);
    }
}
