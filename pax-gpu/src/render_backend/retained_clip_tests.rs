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
