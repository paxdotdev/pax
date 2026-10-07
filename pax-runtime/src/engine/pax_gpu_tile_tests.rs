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
    let replay = renderer.replay_region_bounds(0).map(|regions| {
        let ids: Vec<_> = [(1, 0.0), (2, 32.0), (3, 64.0)]
            .into_iter()
            .filter(|(_, x)| {
                regions
                    .iter()
                    .any(|r| r.intersect(Rect::new(x + 1.0, 1.0, x + 31.0, 31.0)).area() > 0.0)
            })
            .map(|(id, _)| id)
            .collect();
        renderer.prepare_canvas_replay(0, &ids)
    });
    for (id, x) in [(1, 0.0), (2, 32.0), (3, 64.0)] {
        if replay
            .as_ref()
            .is_some_and(|missing| !missing.contains(&id))
        {
            continue;
        }
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
fn lighting_invalidates_only_recipient_tiles_across_layers() {
    use pax_runtime_api::{SceneLight, SceneLighting};
    let native = [MetalLayer::new(), MetalLayer::new(), MetalLayer::new()];
    let mut renderer = PaxGpuRenderer::new(|_, _| unreachable!("prebuilt targets"));
    let mut context = None;
    for (layer, slots) in [(0, vec![0, 1]), (1, vec![2])] {
        let mut tiles = Vec::new();
        let mut surfaces = Vec::new();
        for slot in slots {
            let x = if slot == 1 { 32.0 } else { 0.0 };
            let surface = entry(&slot.to_string(), &slot.to_string(), x);
            let (backend, shared) = pollster::block_on(unsafe {
                RenderBackend::to_core_animation_layer_with_context(
                    native[slot].0.cast(),
                    RenderConfig::new(true, 32, 32, [1.0, 1.0]),
                    context.clone(),
                )
            })
            .expect("Metal backend");
            context = Some(shared);
            let mut gpu = WgpuRenderer::new(backend);
            gpu.set_surface_transform(Transform2D::translation(-x, 0.0));
            tiles.push(LayerRenderer::new(
                surface.key.clone(),
                surface.host_signature.clone(),
                gpu,
                x,
                0.0,
                32.0,
                32.0,
                32,
                32,
                [1.0, 1.0],
            ));
            surfaces.push(surface);
        }
        let layout = LayerSurfaceLayout {
            surfaces,
            active: true,
        };
        renderer
            .backends
            .borrow_mut()
            .push(RenderLayerState::Ready((
                LayerTarget::new(tiles, true),
                Box::pin(move || layout.clone()),
            )));
        assert_eq!(renderer.backends.borrow().len(), layer + 1);
    }
    let dirty = Rc::new(RefCell::new(vec![true; 2]));
    for (layer, id, x, mask) in [(0, 1, 0.0, 1), (0, 2, 32.0, 0), (1, 3, 0.0, 0)] {
        let bounds = Rect::new(x + 1.0, 1.0, x + 31.0, 31.0);
        assert!(renderer.begin_node_with_bounds(layer, id, 0, bounds, mask));
        renderer.fill_with_opacity(
            layer,
            bounds.to_path(0.1),
            &pax_runtime_api::Paint::Solid(pax_runtime_api::Color::WHITE),
            1.0,
        );
        renderer.end_node(layer, id);
    }
    let mut lighting = SceneLighting::with_default_ambient(vec![SceneLight {
        enabled: true,
        ..Default::default()
    }]);
    for layer in 0..2 {
        renderer.set_scene_lighting(layer, &lighting);
        renderer.flush(layer, Rc::clone(&dirty));
    }
    lighting.lights[0].intensity += 0.5;
    for layer in 0..2 {
        renderer.set_scene_lighting(layer, &lighting);
        renderer.flush(layer, Rc::clone(&dirty));
    }
    let flushed = renderer.flushed_surfaces.borrow().clone();
    assert_eq!(flushed.len(), 1);
    assert_eq!((flushed[0].0, flushed[0].1), (0, 0));
    assert_eq!(flushed[0].2.vector_geometry_rebuilds, 0);
    assert_eq!(flushed[0].2.vector_resource_updates, 0);
    assert_eq!(flushed[0].2.texture_upload_bytes, 0);

    // Tile migration removes the previous recipient and updates the new tile's
    // dependency summary. Lighting still arrives before subsequent node recording.
    let bounds = Rect::new(33.0, 1.0, 63.0, 31.0);
    assert!(renderer.begin_node_with_bounds(0, 1, 0, bounds, 1));
    renderer.fill_with_opacity(
        0,
        bounds.to_path(0.1),
        &pax_runtime_api::Paint::Solid(pax_runtime_api::Color::WHITE),
        1.0,
    );
    renderer.end_node(0, 1);
    renderer.flush(0, Rc::clone(&dirty));
    lighting.lights[0].intensity += 0.5;
    renderer.set_scene_lighting(0, &lighting);
    renderer.flush(0, Rc::clone(&dirty));
    let flushed = renderer.flushed_surfaces.borrow().clone();
    assert_eq!(flushed.len(), 1);
    assert_eq!((flushed[0].0, flushed[0].1), (0, 1));
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn incremental_tiles_preserve_survivor_scene_and_uploads() {
    exercise_tile_lifecycle(false, false);
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn incremental_batches_preserve_survivor_scene_and_uploads() {
    exercise_tile_lifecycle(true, false);
}

#[test]
#[ignore = "requires a macOS Metal device"]
fn prepared_tiles_publish_only_complete_current_jobs() {
    exercise_tile_lifecycle(false, true);
}

fn exercise_tile_lifecycle(incremental: bool, prepared: bool) {
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
                    if incremental {
                        break;
                    }
                }
                if let Some(changed) = after_creation.borrow_mut().take() {
                    *layout.borrow_mut() = changed;
                }
                let target = LayerTarget::new(tiles, snapshot.active);
                Some((
                    if incremental {
                        target.with_incremental_initialization()
                    } else {
                        target
                    },
                    Box::pin(move || layout.borrow().clone())
                        as Pin<Box<dyn Fn() -> LayerSurfaceLayout>>,
                ))
            })
        }
    });
    if prepared {
        renderer = renderer.with_surface_content_preparation();
    }
    let dirty = Rc::new(RefCell::new(vec![true]));
    renderer.load_image("red", &[255, 0, 0, 255], 1, 1);
    renderer.resize_layers_to(1, Rc::clone(&dirty));
    if prepared {
        let jobs = renderer.take_settled_surface_replays();
        assert_eq!(jobs.len(), 1);
        assert_eq!(renderer.targeted_replay_scope(0), Some(vec![0]));
        let bounds = Rect::new(1.0, 1.0, 31.0, 31.0);
        assert!(renderer.begin_node_with_bounds(0, 1, 0, bounds, 0));
        renderer.draw_image(0, "red", bounds);
        assert!(renderer.end_node(0, 1));
        renderer.clear(0);
        renderer.flush(0, Rc::clone(&dirty));
        assert!(with_target(&mut renderer, |t| t
            .renderers
            .iter()
            .all(|r| !r.presentation_ready)));
        assert_eq!(renderer.missing_surface_drawings(0, &[1]), Some(vec![]));
        renderer.invalidate_drawing(0, 1);
        assert!(renderer.begin_surface_recording(0));
        assert!(renderer.begin_node_with_bounds(0, 1, 0, bounds, 0));
        renderer.draw_image(0, "red", bounds);
        renderer.end_node(0, 1);
        assert!(renderer.end_surface_recording(0));
        assert!(renderer.flushed_surfaces.borrow().is_empty());
        assert!(renderer.prepare_settled_surface_replay(0, &[1]));
        renderer.flush(0, Rc::clone(&dirty));
        assert_eq!(
            with_target(&mut renderer, |t| t
                .renderers
                .iter()
                .map(|r| r.presentation_ready)
                .collect::<Vec<_>>()),
            vec![true, false]
        );
        assert_eq!(renderer.flushed_surfaces.borrow().len(), 1);
        assert_eq!(renderer.flushed_surfaces.borrow()[0].1, 0);
        renderer.take_settled_surface_replays();
        assert_eq!(renderer.targeted_replay_scope(0), Some(vec![1]));
        assert!(renderer.begin_surface_fallback(0));
        assert!(!renderer.begin_node_with_bounds(0, 1, 0, bounds, 0));
        let second = Rect::new(33.0, 1.0, 63.0, 31.0);
        assert!(renderer.begin_node_with_bounds(0, 2, 0, second, 0));
        renderer.draw_image(0, "red", second);
        renderer.end_node(0, 2);
        renderer.flush(0, Rc::clone(&dirty));
        assert!(with_target(&mut renderer, |t| t
            .renderers
            .iter()
            .all(|r| r.presentation_ready)));
        assert_eq!(renderer.flushed_surfaces.borrow().len(), 1);
        assert_eq!(renderer.flushed_surfaces.borrow()[0].1, 1);
        return;
    }
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
    let hits_before = renderer.drawings.hits;
    paint(&mut renderer, Rc::clone(&dirty));
    assert!(
        renderer.drawings.hits > hits_before,
        "resized tile must reuse logical drawing"
    );
    let flushed = renderer.flushed_surfaces.borrow().clone();
    assert_eq!(flushed.len(), 1, "cached replay must leave B untouched");

    // Content work is independent of the replay scope: an update to B still lands while
    // C needs a replay, and C's cached command record survives B's unrelated update.
    layout.borrow_mut().surfaces[0].surface.dpr = [1.0, 1.0];
    layout.borrow_mut().surfaces[0].surface.surface_width = 32;
    layout.borrow_mut().surfaces[0].surface.surface_height = 32;
    renderer.refresh_layers(&[0]);
    renderer.take_replay_canvas_layer_updates();
    assert!(renderer.prepare_canvas_replay(0, &[3]).is_empty());
    let bounds = Rect::new(33.0, 1.0, 63.0, 31.0);
    assert!(renderer.begin_node_with_bounds(0, 2, 0, bounds, 0));
    renderer.draw_image_with_opacity(0, "red", bounds, 0.5);
    renderer.end_node(0, 2);
    renderer.flush(0, Rc::clone(&dirty));
    assert_eq!(renderer.flushed_surfaces.borrow().len(), 2);
    assert!(renderer.drawings.get(0, 3).is_some());

    // A cached leaf carries a complete detached alpha-source program and its clip
    // owner. Replaying it at another density must not rerun source components or
    // turn the masked image into an unmasked rectangle.
    renderer.remove_node(0, 2);
    renderer.remove_node(0, 3);
    assert!(renderer.begin_node(0, 10, 0, 0));
    renderer.save(0);
    renderer.begin_alpha_source(0, kurbo::Affine::IDENTITY, 0, 0.0);
    let source = Rect::new(40.0, 8.0, 88.0, 24.0);
    assert!(renderer.begin_node_with_bounds(0, 20, 0, source, 0));
    renderer.fill(
        0,
        source.to_path(0.1),
        &pax_runtime_api::Paint::Solid(pax_runtime_api::Color::WHITE),
    );
    renderer.end_node(0, 20);
    renderer.end_alpha_source(0);
    renderer.end_node(0, 10);
    let bounds = Rect::new(32.0, 0.0, 96.0, 32.0);
    assert!(renderer.begin_node_with_bounds(0, 50, 0, bounds, 0));
    renderer.set_node_opacity_scopes(
        0,
        50,
        &[pax_runtime_api::OpacityScope {
            node_id: 90,
            opacity: 0.5,
        }],
    );
    renderer.save(0);
    renderer.draw_image(0, "red", bounds);
    renderer.restore(0);
    renderer.end_node(0, 50);
    renderer.restore(0);
    renderer.flush(0, Rc::clone(&dirty));
    layout.borrow_mut().surfaces[0].surface.dpr = [2.0, 2.0];
    layout.borrow_mut().surfaces[0].surface.surface_width = 64;
    layout.borrow_mut().surfaces[0].surface.surface_height = 64;
    renderer.refresh_layers(&[0]);
    renderer.take_replay_canvas_layer_updates();
    assert!(!renderer.prepare_settled_surface_replay(0, &[50, 999_999]));
    assert!(
        renderer
            .pending_drawings
            .get(&0)
            .is_none_or(|p| p.is_empty()),
        "a cache miss must not queue a partial frame"
    );
    assert!(renderer.prepare_settled_surface_replay(0, &[50]));
    with_target(&mut renderer, |target| {
        target.renderers[0].renderer.request_screenshot_capture(101)
    });
    renderer.flush(0, Rc::clone(&dirty));
    with_target(&mut renderer, |target| {
        let gpu = &mut target.renderers[0].renderer;
        let mut frame = gpu.take_screenshot_capture(101);
        for _ in 0..100 {
            if frame.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
            gpu.clear();
            gpu.flush();
            frame = gpu.take_screenshot_capture(101);
        }
        let frame = frame.expect("cached mask replay readback");
        let pixel = &frame.rgba[(32 * 64 + 32) * 4..(32 * 64 + 32) * 4 + 4];
        assert!(
            (127..=128).contains(&pixel[0]) && (127..=128).contains(&pixel[3]),
            "cached opacity scope lost: {pixel:?}"
        );
        assert_eq!(&pixel[1..3], &[0, 0]);
        assert_eq!(
            frame.rgba[(4 * 64 + 4) * 4 + 3],
            0,
            "alpha clip lost during cached replay"
        );
    });

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
