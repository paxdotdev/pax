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
        let mut commands = Vec::new();
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
            commands.extend(renderer.take_pending_command_buffers());
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
