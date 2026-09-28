//! Opt-in Metal coverage: cargo test -p pax-gpu shared_image_textures -- --ignored
use super::*;
use crate::{point, Color, Fill, Image, Path, Transform2D, WgpuRenderer};
use objc2::{
    msg_send,
    runtime::{AnyClass, AnyObject},
};

#[link(name = "QuartzCore", kind = "framework")]
extern "C" {}
struct MetalLayer(*mut AnyObject);
impl MetalLayer {
    fn new() -> Self {
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

fn backend(
    layer: &MetalLayer,
    context: Option<SharedGpuContext>,
) -> (RenderBackend<'static>, SharedGpuContext) {
    pollster::block_on(unsafe {
        RenderBackend::to_core_animation_layer_with_context(
            layer.0.cast(),
            RenderConfig::new(true, 32, 32, [1.0, 1.0]),
            context,
        )
    })
    .expect("Metal backend")
}

fn draw(
    renderer: &mut WgpuRenderer,
    version: u64,
    rgba: [u8; 4],
    x: f32,
    opacity: f32,
    clipped: bool,
) {
    renderer.begin_node(1, 0, 0);
    renderer.save();
    if clipped {
        renderer.clip(rect(x + 4.0, 0.0, 8.0, 32.0));
    }
    renderer.draw_image_with_opacity(
        "artwork",
        version,
        &Image {
            rgba: rgba.to_vec(),
            pixel_width: 1,
            pixel_height: 1,
        },
        Box2D::new(point(x, 0.0), point(x + 16.0, 32.0)),
        opacity,
    );
    renderer.restore();
    renderer.end_node(1);
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Path {
    let mut path = Path::builder();
    path.begin(point(x, y));
    path.line_to(point(x + w, y));
    path.line_to(point(x + w, y + h));
    path.line_to(point(x, y + h));
    path.end(true);
    path.build()
}

fn capture(renderer: &mut WgpuRenderer, device: &wgpu::Device) -> CapturedFrame {
    renderer.request_screenshot_capture(1);
    renderer.flush();
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("GPU completion");
    renderer.take_screenshot_capture(1).expect("capture")
}
fn pixel(frame: &CapturedFrame, x: usize) -> &[u8] {
    let offset = (16 * frame.width as usize + x) * 4;
    &frame.rgba[offset..offset + 4]
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn shared_image_textures_preserve_tile_bindings_versions_and_lifetime() {
    let layer_a = MetalLayer::new();
    let layer_b = MetalLayer::new();
    let layer_c = MetalLayer::new();
    let (a, context) = backend(&layer_a, None);
    let device = context.device.clone();
    let (b, _) = backend(&layer_b, Some(context.clone()));
    let (c, separate) = backend(&layer_c, None);
    let mut a = WgpuRenderer::new(a);
    let mut b = WgpuRenderer::new(b);
    let mut c = WgpuRenderer::new(c);
    b.set_surface_transform(Transform2D::translation(-32.0, 0.0));

    draw(&mut a, 1, [255, 0, 0, 255], 0.0, 1.0, false);
    draw(&mut b, 1, [255, 0, 0, 255], 32.0, 0.5, true);
    assert_eq!(a.take_resource_churn_stats().texture_upload_bytes, 4);
    let hit = b.take_resource_churn_stats();
    assert_eq!(
        (
            hit.texture_creates,
            hit.texture_cache_hits,
            hit.texture_upload_bytes
        ),
        (0, 1, 0)
    );
    assert_eq!(context.image_textures.borrow().live_count(), 1);
    assert_eq!(pixel(&capture(&mut a, &device), 8), &[255, 0, 0, 255]);
    let b_frame = capture(&mut b, &device);
    assert!(
        pixel(&b_frame, 8)[3].abs_diff(128) <= 1,
        "tile-local opacity"
    );
    assert_eq!(
        pixel(&b_frame, 1),
        &[0, 0, 0, 0],
        "tile-local clip/transform"
    );

    // The same identity in an unrelated GPU context must neither alias nor suppress upload.
    draw(&mut c, 1, [0, 0, 255, 255], 0.0, 1.0, false);
    assert_eq!(c.take_resource_churn_stats().texture_creates, 1);
    assert_eq!(
        pixel(&capture(&mut c, &separate.device), 8),
        &[0, 0, 255, 255]
    );

    // An update is immutable: B may still draw the old version while A uses the new one.
    draw(&mut a, 2, [0, 255, 0, 255], 0.0, 1.0, false);
    assert_eq!(a.take_resource_churn_stats().texture_creates, 1);
    assert_eq!(context.image_textures.borrow().live_count(), 2);
    assert_eq!(pixel(&capture(&mut a, &device), 8), &[0, 255, 0, 255]);
    b.clear();
    let old = capture(&mut b, &device);
    assert_eq!(
        pixel(&old, 8)[1],
        0,
        "version update changed a sibling's pixels"
    );
    draw(&mut b, 2, [0, 255, 0, 255], 32.0, 1.0, true);
    assert_eq!(b.take_resource_churn_stats().texture_cache_hits, 1);
    assert_eq!(pixel(&capture(&mut b, &device), 8), &[0, 255, 0, 255]);
    assert_eq!(context.image_textures.borrow().live_count(), 1);

    // Extent changes cannot reuse an incompatible allocation, even at the same version.
    a.begin_node(1, 0, 0);
    a.draw_image(
        "artwork",
        2,
        &Image {
            rgba: vec![255, 255, 0, 255, 255, 255, 0, 255],
            pixel_width: 2,
            pixel_height: 1,
        },
        Box2D::new(point(0.0, 0.0), point(16.0, 32.0)),
    );
    a.end_node(1);
    assert_eq!(a.take_resource_churn_stats().texture_creates, 1);
    assert_eq!(pixel(&capture(&mut a, &device), 8), &[255, 255, 0, 255]);
    assert_eq!(context.image_textures.borrow().live_count(), 2);
    drop(a);
    assert_eq!(
        context.image_textures.borrow().live_count(),
        1,
        "eviction retains no unused image"
    );
    b.clear();
    assert_eq!(pixel(&capture(&mut b, &device), 8), &[0, 255, 0, 255]);

    // Exercise the vector-only batching path, which must release the last image reference too.
    b.remove_node(1);
    for id in 2..66 {
        b.begin_node(id, id as i32, 0);
        b.fill_path(
            rect(32.0, 0.0, 32.0, 32.0),
            Fill::Solid(Color::rgba(0.0, 0.0, 1.0, 1.0)),
        );
        b.end_node(id);
    }
    assert_eq!(pixel(&capture(&mut b, &device), 8), &[0, 0, 255, 255]);
    assert_eq!(
        context.image_textures.borrow().live_count(),
        0,
        "cache must not own dead image pixels"
    );
    draw(&mut b, 2, [0, 255, 0, 255], 32.0, 1.0, false);
    assert_eq!(
        b.take_resource_churn_stats().texture_creates,
        1,
        "unreferenced image is uploaded again"
    );
    assert_eq!(context.image_textures.borrow().live_count(), 1);
    drop(b);
    assert_eq!(context.image_textures.borrow().live_count(), 0);
}
