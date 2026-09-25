//! Hardware regression coverage for retained image/vector clip ordering.
//! Run with `cargo test -p pax-gpu retained_clip_pixels -- --ignored` on macOS.

use super::*;
use crate::{point, Angle, Color, Fill, Path, WgpuRenderer};
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
