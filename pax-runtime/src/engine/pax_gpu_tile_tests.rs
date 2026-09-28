//! Real Metal resource/replay regression. Run with `cargo test -p pax-runtime incremental_tiles -- --ignored`.
use super::*;
use crate::engine::layer_surface::LayerSurfaceSize;
use objc2::{
    msg_send,
    runtime::{AnyClass, AnyObject},
};
use pax_gpu::render_backend::{RenderBackend, RenderConfig};

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

fn entry(key: &str, generation: &str, x: f32) -> LayerSurfaceEntry {
    LayerSurfaceEntry {
        key: key.into(),
        host_signature: generation.into(),
        origin_x: x,
        origin_y: 0.0,
        replay_priority: 0,
        surface: LayerSurfaceSize {
            logical_width: 32.0,
            logical_height: 32.0,
            surface_width: 32,
            surface_height: 32,
            dpr: [1.0, 1.0],
        },
    }
}

fn with_target<T>(renderer: &mut PaxGpuRenderer, f: impl FnOnce(&mut LayerTarget) -> T) -> T {
    let mut states = renderer.backends.borrow_mut();
    let RenderLayerState::Ready((target, _)) = &mut states[0] else {
        panic!("ready layer lost")
    };
    f(target)
}

fn paint(renderer: &mut PaxGpuRenderer, dirty: Rc<RefCell<Vec<bool>>>) {
    renderer.clear(0);
    for (id, x) in [(1, 0.0), (2, 32.0), (3, 64.0)] {
        let bounds = Rect::new(x + 1.0, 1.0, x + 31.0, 31.0);
        if renderer.begin_node_with_bounds(0, id, 0, bounds, 0) {
            renderer.draw_image(0, "red", bounds);
            renderer.end_node(0, id);
        }
    }
    renderer.flush(0, dirty);
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn incremental_tiles_preserve_survivor_scene_and_uploads() {
    let a = entry("a", "a:1", 0.0);
    let b = entry("b", "b:1", 32.0);
    let c = entry("c", "c:1", 64.0);
    let native: Rc<HashMap<String, MetalLayer>> = Rc::new(
        ["a:1", "b:1", "c:1", "c:2"]
            .into_iter()
            .map(|key| (key.into(), MetalLayer::new()))
            .collect(),
    );
    let layout = Rc::new(RefCell::new(LayerSurfaceLayout {
        surfaces: vec![a.clone(), b.clone()],
        active: true,
    }));
    let creations = Rc::new(RefCell::new(Vec::new()));
    let fail = Rc::new(Cell::new(false));
    let after_creation = Rc::new(RefCell::new(None::<LayerSurfaceLayout>));
    let mut renderer = PaxGpuRenderer::new({
        let layout = Rc::clone(&layout);
        let native = Rc::clone(&native);
        let creations = Rc::clone(&creations);
        let fail = Rc::clone(&fail);
        let after_creation = Rc::clone(&after_creation);
        move |_, request| {
            let layout = Rc::clone(&layout);
            let native = Rc::clone(&native);
            let creations = Rc::clone(&creations);
            let fail = Rc::clone(&fail);
            let after_creation = Rc::clone(&after_creation);
            Box::pin(async move {
                if fail.get() {
                    return None;
                }
                let snapshot = layout.borrow().clone();
                let mut context = request.shared_context.clone();
                let mut tiles = Vec::new();
                for surface in &snapshot.surfaces {
                    if !request.needs_surface(&surface.key, &surface.host_signature) {
                        continue;
                    }
                    let (backend, shared) = unsafe {
                        RenderBackend::to_core_animation_layer_with_context(
                            native[&surface.host_signature].0.cast(),
                            RenderConfig::new(
                                true,
                                surface.surface.surface_width,
                                surface.surface.surface_height,
                                surface.surface.dpr,
                            ),
                            context.clone(),
                        )
                    }
                    .await
                    .expect("Metal backend");
                    context = Some(shared);
                    let mut gpu = WgpuRenderer::new(backend);
                    gpu.set_surface_transform(Transform2D::from_array([
                        1.0,
                        0.0,
                        0.0,
                        1.0,
                        -surface.origin_x,
                        0.0,
                    ]));
                    gpu.set_viewport(
                        surface.surface.logical_width,
                        surface.surface.logical_height,
                        surface.surface.dpr,
                    );
                    creations.borrow_mut().push(surface.key.clone());
                    tiles.push(LayerRenderer::new(
                        surface.key.clone(),
                        surface.host_signature.clone(),
                        gpu,
                        surface.origin_x,
                        surface.origin_y,
                        surface.surface.logical_width,
                        surface.surface.logical_height,
                        surface.surface.surface_width,
                        surface.surface.surface_height,
                        surface.surface.dpr,
                    ));
                }
                if let Some(changed) = after_creation.borrow_mut().take() {
                    *layout.borrow_mut() = changed;
                }
                Some((
                    LayerTarget::new(tiles, snapshot.active),
                    Box::pin(move || layout.borrow().clone())
                        as Pin<Box<dyn Fn() -> LayerSurfaceLayout>>,
                ))
            })
        }
    });
    let dirty = Rc::new(RefCell::new(vec![true]));
    renderer.load_image("red", &[255, 0, 0, 255], 1, 1);
    renderer.resize_layers_to(1, Rc::clone(&dirty));
    assert_eq!(renderer.take_ready_canvas_layers(), vec![0]);
    renderer.take_replay_canvas_layer_updates();
    paint(&mut renderer, Rc::clone(&dirty));
    let context = with_target(&mut renderer, |target| {
        for tile in &mut target.renderers {
            tile.renderer.take_resource_churn_stats();
        }
        target.renderers[1].renderer.shared_gpu_context_id()
    });

    // B survives A's eviction and C's addition. Only C may be replayed or encoded.
    layout.borrow_mut().surfaces = vec![b.clone(), c.clone()];
    renderer.refresh_layers(&[0]);
    assert!(renderer.take_ready_canvas_layers().is_empty());
    assert_eq!(&*creations.borrow(), &["a", "b", "c"]);
    assert_eq!(renderer.targeted_replay_scope(0), Some(vec![1]));
    let updates = renderer.take_replay_canvas_layer_updates();
    assert_eq!(updates.len(), 1);
    assert_eq!(
        updates[0].regions,
        Some(vec![Rect::new(64.0, 0.0, 96.0, 32.0)])
    );
    with_target(&mut renderer, |target| {
        for tile in &target.renderers {
            assert_eq!(tile.renderer.shared_gpu_context_id(), context);
        }
        target.renderers[1].renderer.request_screenshot_capture(99);
    });
    paint(&mut renderer, Rc::clone(&dirty));
    with_target(&mut renderer, |target| {
        let survivor = target.renderers[0].renderer.take_resource_churn_stats();
        assert_eq!(survivor.flushes, 0);
        assert_eq!(survivor.retained_scene_resets, 0);
        assert_eq!(survivor.texture_creates, 0);
        assert_eq!(survivor.retained_nodes_considered, 0);
        // The public renderer API uses nonblocking polling. Submit small follow-up frames
        // to service the original readback callback without exposing a device solely for tests.
        let tile = &mut target.renderers[1].renderer;
        let mut frame = tile.take_screenshot_capture(99);
        for _ in 0..100 {
            if frame.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
            tile.clear();
            tile.flush();
            frame = tile.take_screenshot_capture(99);
        }
        let frame = frame.expect("new tile painted");
        assert_eq!(
            &frame.rgba[(16 * 32 + 16) * 4..(16 * 32 + 16) * 4 + 4],
            &[255, 0, 0, 255]
        );
    });

    let flushed = renderer.flushed_surfaces.borrow().clone();
    assert_eq!(flushed.len(), 1, "clean survivor was encoded");
    assert_eq!((flushed[0].0, flushed[0].1), (0, 1));
    assert_eq!(flushed[0].2.texture_creates, 0);
    assert_eq!(flushed[0].2.texture_cache_hits, 1);
    assert_eq!(flushed[0].2.retained_nodes_considered, 1);

    // Pure reordering does not reset or replay either tile.
    layout.borrow_mut().surfaces = vec![c.clone(), b.clone()];
    renderer.refresh_layers(&[0]);
    assert!(renderer.take_ready_canvas_layers().is_empty());
    assert!(renderer.take_replay_canvas_layer_updates().is_empty());
    assert_eq!(&*creations.borrow(), &["a", "b", "c"]);

    // Resize/DPR changes invalidate only the affected tile.
    layout.borrow_mut().surfaces[0].surface.dpr = [2.0, 2.0];
    layout.borrow_mut().surfaces[0].surface.surface_width = 64;
    layout.borrow_mut().surfaces[0].surface.surface_height = 64;
    renderer.refresh_layers(&[0]);
    assert_eq!(renderer.targeted_replay_scope(0), Some(vec![0]));
    renderer.take_replay_canvas_layer_updates();
    paint(&mut renderer, Rc::clone(&dirty));

    // Replacement + failed addition keeps B live and does not repeatedly allocate.
    let replacement = entry("c", "c:2", 64.0);
    layout.borrow_mut().surfaces = vec![b.clone(), replacement];
    fail.set(true);
    renderer.refresh_layers(&[0]);
    with_target(&mut renderer, |target| {
        assert_eq!(target.renderers.len(), 1)
    });
    renderer.refresh_layers(&[0]);
    assert!(renderer.take_ready_canvas_layers().is_empty());
    assert_eq!(&*creations.borrow(), &["a", "b", "c"]);
    fail.set(false);
    layout.borrow_mut().surfaces[1].origin_y = 1.0; // a new layout permits retry
    renderer.refresh_layers(&[0]);
    renderer.take_ready_canvas_layers();
    assert_eq!(&*creations.borrow(), &["a", "b", "c", "c"]);
    assert_eq!(renderer.targeted_replay_scope(0), Some(vec![1]));

    // A canvas removed while creation awaits is never published. A replacement generation
    // with the same key is initialized in the bounded completion drain, leaving B untouched.
    layout.borrow_mut().surfaces = vec![b.clone()];
    renderer.refresh_layers(&[0]);
    renderer.take_replay_canvas_layer_updates();
    layout.borrow_mut().surfaces.push(c.clone());
    *after_creation.borrow_mut() = Some(LayerSurfaceLayout {
        surfaces: vec![b.clone(), entry("c", "c:2", 64.0)],
        active: true,
    });
    renderer.refresh_layers(&[0]);
    renderer.take_ready_canvas_layers();
    with_target(&mut renderer, |target| {
        assert_eq!(target.renderers.len(), 2);
        assert_eq!(target.renderers[1].host_signature, "c:2");
        assert_eq!(
            target.renderers[0].renderer.shared_gpu_context_id(),
            context
        );
    });
    assert_eq!(renderer.targeted_replay_scope(0), Some(vec![1]));
    assert_eq!(&*creations.borrow(), &["a", "b", "c", "c", "c", "c"]);

    let weak_context = with_target(&mut renderer, |target| {
        Rc::downgrade(&target.renderers[0].renderer.shared_gpu_context())
    });
    layout.borrow_mut().surfaces.clear();
    renderer.refresh_layers(&[0]);
    with_target(&mut renderer, |target| assert!(target.renderers.is_empty()));
    assert!(
        weak_context.upgrade().is_none(),
        "empty layer retained its GPU context"
    );
    assert!(renderer.targeted_replay_scope(0).is_none());
    layout.borrow_mut().surfaces = vec![b];
    renderer.refresh_layers(&[0]);
    assert!(renderer.take_ready_canvas_layers().is_empty());
    assert_eq!(&*creations.borrow(), &["a", "b", "c", "c", "c", "c", "b"]);
}
