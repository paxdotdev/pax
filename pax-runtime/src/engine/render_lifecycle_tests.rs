use super::*;
use kurbo::{Affine, BezPath, Rect};
use pax_runtime_api::{
    LocalProperty as Property, Paint, ResolvedMaterial, ResolvedStroke, SceneLighting,
};

#[derive(Default)]
struct RecordingRenderer {
    layer_count: usize,
    pending: bool,
    removals: Vec<(usize, u32)>,
    clears: Vec<usize>,
    lighting_updates: Vec<usize>,
    flushes: Vec<usize>,
    jobs: Vec<pax_runtime_api::ReplayCanvasLayerUpdate>,
    deferred: Vec<usize>,
    presented: Vec<usize>,
    cached: bool,
    blocked_layers: HashSet<usize>,
    followups: HashMap<usize, usize>,
}

impl RenderContext for RecordingRenderer {
    fn layers(&self) -> usize {
        self.layer_count
    }
    fn resize_layers_to(&mut self, count: usize, _: Rc<RefCell<Vec<bool>>>) {
        self.layer_count = count;
    }
    fn clear(&mut self, layer: usize) {
        assert!(layer < self.layer_count);
        self.clears.push(layer);
    }
    fn flush(&mut self, layer: usize, _: Rc<RefCell<Vec<bool>>>) {
        assert!(layer < self.layer_count);
        self.flushes.push(layer);
        self.presented.push(layer);
        if let Some(remaining) = self.followups.get_mut(&layer) {
            if *remaining > 0 {
                *remaining -= 1;
                self.jobs.push(replay_job(layer));
            }
        }
    }
    fn take_settled_surface_replays(&mut self) -> Vec<pax_runtime_api::ReplayCanvasLayerUpdate> {
        self.jobs.sort_by_key(|job| job.layer);
        std::mem::take(&mut self.jobs)
    }
    fn defer_surface_replay(&mut self, layer: usize) {
        self.deferred.push(layer);
        if !self.jobs.iter().any(|job| job.layer == layer) {
            self.jobs.push(replay_job(layer));
        }
    }
    fn prepare_settled_surface_replay(&mut self, layer: usize, _: &[u32]) -> bool {
        self.cached && !self.blocked_layers.contains(&layer)
    }
    fn set_scene_lighting(&mut self, layer: usize, _: &SceneLighting) {
        assert!(layer < self.layer_count);
        self.lighting_updates.push(layer);
    }
    fn remove_node(&mut self, layer: usize, node: u32) -> bool {
        assert!(layer < self.layer_count);
        self.removals.push((layer, node));
        !self.pending
    }
    fn fill_with_opacity(&mut self, _: usize, _: BezPath, _: &Paint, _: f64) {
        unreachable!()
    }
    fn stroke_with_opacity(&mut self, _: usize, _: BezPath, _: &ResolvedStroke, _: f64) {
        unreachable!()
    }
    fn stroke_with_draw_range_and_material_and_opacity(
        &mut self,
        _: usize,
        _: BezPath,
        _: &ResolvedStroke,
        _: &ResolvedMaterial,
        _: f64,
        _: f64,
        _: f64,
    ) {
        unreachable!()
    }
    fn save(&mut self, _: usize) {
        unreachable!()
    }
    fn restore(&mut self, _: usize) {
        unreachable!()
    }
    fn clip(&mut self, _: usize, _: BezPath) {
        unreachable!()
    }
    fn transform(&mut self, _: usize, _: Affine) {
        unreachable!()
    }
    fn load_image(&mut self, _: &str, _: &[u8], _: usize, _: usize) {
        unreachable!()
    }
    fn draw_image(&mut self, _: usize, _: &str, _: Rect) {
        unreachable!()
    }
    fn draw_image_with_opacity(&mut self, _: usize, _: &str, _: Rect, _: f64) {
        unreachable!()
    }
    fn get_image_size(&mut self, _: &str) -> Option<(usize, usize)> {
        unreachable!()
    }
    fn image_loaded(&self, _: &str) -> bool {
        unreachable!()
    }
    fn resize(&mut self, _: usize, _: usize) {
        unreachable!()
    }
    fn refresh_layers(&mut self, _: &[usize]) {
        unreachable!()
    }
}

fn engine() -> PaxEngine {
    crate::test_support::empty_engine(
        (320.0, 240.0),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    )
}

fn replay_job(layer: usize) -> pax_runtime_api::ReplayCanvasLayerUpdate {
    pax_runtime_api::ReplayCanvasLayerUpdate {
        layer,
        regions: Some(vec![Rect::new(0.0, 0.0, 32.0, 32.0)]),
    }
}

#[test]
fn settled_replay_enters_its_owner_graph_and_stops_after_shutdown() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer::default();
    engine.tick();
    engine.runtime_context.layer_count.set(1);
    engine.render(&mut renderer);
    renderer.cached = true;
    renderer.jobs = vec![replay_job(0)];

    let foreign = pax_runtime_api::PropertyGraph::new(|| {});
    let _foreign = foreign.enter();
    let effect = Property::computed(|| (), &[]);
    pax_runtime_api::properties::register_effect_property(&effect);
    assert!(pax_runtime_api::properties::has_pending_effects());
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    let report = engine.present_settled_surfaces(&mut renderer, &mut budget, || 0.0);
    assert!(!report.blocked);
    assert_eq!(report.submitted, 1);
    assert!(pax_runtime_api::properties::has_pending_effects());

    engine.shutdown(pax_runtime_api::StopReason::HostClosed);
    renderer.jobs = vec![replay_job(0)];
    let report = engine.present_settled_surfaces(&mut renderer, &mut budget, || 0.0);
    assert_eq!(report.blocker, Some("app_closing"));
    assert_eq!(renderer.jobs.len(), 1);
    assert!(engine
        .tick_with_phase_observer(|_| panic!("closed app ticked"))
        .is_empty());
}

#[test]
fn settled_replay_preserves_clock_and_defers_atomic_overruns_and_misses() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer::default();
    engine.tick();
    engine.runtime_context.layer_count.set(3);
    engine.render(&mut renderer);
    renderer.presented.clear();
    renderer.cached = true;
    renderer.jobs = vec![replay_job(1), replay_job(2)];
    let frames = engine
        .runtime_context
        .globals()
        .stack_frame()
        .resolve_symbol("@frames")
        .unwrap()
        .get_as_pax_value();
    let now = std::cell::Cell::new(0.0);
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    let report = engine.present_settled_surfaces(&mut renderer, &mut budget, || {
        let t = now.get();
        now.set(t + 3.0);
        t
    });
    assert!(!report.blocked);
    assert_eq!(report.submitted, 1);
    assert_eq!(report.deferred, 1);
    assert_eq!(renderer.presented, vec![1]);
    assert_eq!(
        renderer
            .jobs
            .iter()
            .map(|job| job.layer)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert_eq!(
        frames,
        engine
            .runtime_context
            .globals()
            .stack_frame()
            .resolve_symbol("@frames")
            .unwrap()
            .get_as_pax_value()
    );
    renderer.cached = false;
    renderer.jobs = vec![replay_job(2)];
    let report = engine.present_settled_surfaces(&mut renderer, &mut budget, || 0.0);
    assert_eq!(report.cache_misses, 1);
    assert_eq!(report.submitted, 0);
    assert_eq!(renderer.presented, vec![1]);

    // An old cold layer awaiting removal must not starve an unrelated surface.
    engine.runtime_context.enqueue_canvas_node_removal(0, 42);
    renderer.cached = true;
    renderer.jobs = vec![replay_job(2)];
    let report = engine.present_settled_surfaces(&mut renderer, &mut budget, || 0.0);
    assert_eq!(report.submitted, 1);
    assert_eq!(renderer.presented, vec![1, 2]);
}

#[test]
fn settled_replay_drains_successive_batches_and_reselects_priority() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer::default();
    engine.tick();
    engine.runtime_context.layer_count.set(3);
    engine.render(&mut renderer);
    renderer.presented.clear();
    renderer.cached = true;
    renderer.jobs = vec![replay_job(1), replay_job(2)];
    renderer.followups.insert(1, 2);
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    let report = engine.present_settled_surfaces(&mut renderer, &mut budget, || 0.0);
    assert_eq!(report.submitted, 4);
    assert_eq!(renderer.presented, vec![1, 1, 1, 2]);
    assert!(renderer.jobs.is_empty());
}

#[test]
fn settled_replay_skips_a_blocked_head_without_retrying_or_starving_siblings() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer::default();
    engine.tick();
    engine.runtime_context.layer_count.set(3);
    engine.render(&mut renderer);
    renderer.presented.clear();
    renderer.cached = true;
    renderer.blocked_layers.insert(1);
    renderer.jobs = vec![replay_job(1), replay_job(2)];
    renderer.followups.insert(2, 2);
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    let report = engine.present_settled_surfaces(&mut renderer, &mut budget, || 0.0);
    assert_eq!(report.submitted, 3);
    assert_eq!(report.cache_misses, 1);
    assert_eq!(report.deferred, 1);
    assert_eq!(renderer.presented, vec![2, 2, 2]);
    assert_eq!(renderer.jobs.len(), 1);
    assert_eq!(renderer.jobs[0].layer, 1);
    renderer.blocked_layers.clear();
    assert_eq!(
        engine
            .present_settled_surfaces(&mut renderer, &mut budget, || 0.0)
            .submitted,
        1
    );
}

#[test]
fn settled_replay_stops_between_successive_batches_at_the_same_time_budget() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer::default();
    engine.tick();
    engine.runtime_context.layer_count.set(2);
    engine.render(&mut renderer);
    renderer.presented.clear();
    renderer.cached = true;
    renderer.jobs = vec![replay_job(1)];
    renderer.followups.insert(1, 10);
    let now = std::cell::Cell::new(0.0);
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    let report = engine.present_settled_surfaces(&mut renderer, &mut budget, || {
        let t = now.get();
        now.set(t + 0.5);
        t
    });
    assert!(report.submitted > 1 && report.submitted < 11);
    assert_eq!(report.deferred, 1);
    assert_eq!(renderer.jobs.len(), 1);
}

#[test]
fn unsettled_replay_does_not_consume_jobs_or_run_effects() {
    let mut engine = engine();
    let _graph = engine.runtime_context.property_graph.enter();
    let mut renderer = RecordingRenderer::default();
    engine.tick();
    engine.runtime_context.layer_count.set(1);
    engine.render(&mut renderer);
    renderer.jobs = vec![replay_job(0)];
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    let input = Property::new(0);
    let read = input.clone();
    let calls = Rc::new(std::cell::Cell::new(0));
    let observed = calls.clone();
    let effect = Property::computed(
        move || {
            let _ = read.get();
            observed.set(observed.get() + 1);
        },
        &[input.untyped()],
    );
    pax_runtime_api::properties::register_effect_property(&effect);
    assert!(
        engine
            .present_settled_surfaces(&mut renderer, &mut budget, || 0.0)
            .blocked
    );
    assert_eq!(calls.get(), 0);
    assert_eq!(renderer.jobs.len(), 1);
    engine.runtime_context.drain_node_effects();
    engine.runtime_context.set_all_canvases_dirty();
    assert_eq!(
        engine
            .present_settled_surfaces(&mut renderer, &mut budget, || 0.0)
            .deferred,
        1
    );
    engine.runtime_context.clear_all_dirty_canvases();
    renderer.jobs = vec![replay_job(0)];
    engine.runtime_context.enqueue_canvas_node_removal(0, 42);
    assert_eq!(
        engine
            .present_settled_surfaces(&mut renderer, &mut budget, || 0.0)
            .deferred,
        1
    );
    assert_eq!(renderer.deferred, vec![0, 0]);
    renderer.jobs = vec![replay_job(0)];
    engine.runtime_context.mark_occlusion_dirty();
    assert!(
        engine
            .present_settled_surfaces(&mut renderer, &mut budget, || 0.0)
            .blocked
    );
    assert_eq!(renderer.jobs.len(), 1);
}

#[test]
fn lighting_only_work_resolves_and_flushes_without_clearing_canvases() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer::default();
    engine.runtime_context.layer_count.set(3);
    engine.render(&mut renderer);
    renderer.clears.clear();
    renderer.lighting_updates.clear();
    renderer.flushes.clear();
    engine.runtime_context.mark_scene_lighting_dirty();
    assert!(engine.runtime_context.has_canvas_render_work());
    assert!(!engine.runtime_context.has_dirty_canvas_layers());
    assert!(!engine.runtime_context.has_dirty_canvas_nodes());
    engine.render(&mut renderer);
    assert_eq!(renderer.lighting_updates, vec![0, 1, 2]);
    assert_eq!(renderer.flushes, vec![0, 1, 2]);
    assert!(renderer.clears.is_empty());
    assert!(!engine.runtime_context.has_canvas_render_work());

    engine.runtime_context.mark_scene_lighting_dirty();
    engine.runtime_context.set_canvas_dirty(1);
    engine.render(&mut renderer);
    assert_eq!(renderer.clears, vec![1], "independent redraw survives");
}

#[test]
fn removed_layers_discard_queued_node_removals() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer::default();
    engine.runtime_context.layer_count.set(7);
    engine.render(&mut renderer);
    engine.runtime_context.enqueue_canvas_node_removal(6, 42);
    engine.runtime_context.enqueue_canvas_node_removal(0, 43);
    engine.runtime_context.layer_count.set(6);
    engine.render(&mut renderer);
    assert_eq!(renderer.removals, vec![(0, 43)]);
    assert!(!engine.runtime_context.has_canvas_render_work());
    engine.render(&mut renderer);
    assert_eq!(renderer.removals, vec![(0, 43)]);
}

#[test]
fn layer_synchronization_is_local_to_each_renderer_and_engine() {
    let mut first = engine();
    let mut second = engine();
    let mut first_renderer = RecordingRenderer::default();
    let mut second_renderer = RecordingRenderer::default();
    first.runtime_context.layer_count.set(2);
    second.runtime_context.layer_count.set(2);
    first.render(&mut first_renderer);
    second.render(&mut second_renderer);
    assert_eq!((first_renderer.layers(), second_renderer.layers()), (2, 2));
    assert_eq!(second.runtime_context.dirty_canvases.borrow().len(), 2);
    let mut replacement = RecordingRenderer::default();
    second.render(&mut replacement);
    assert_eq!(replacement.layers(), 2);
}

#[test]
fn pending_layer_removals_retry_until_the_backend_is_ready() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer {
        pending: true,
        ..Default::default()
    };
    engine.runtime_context.layer_count.set(1);
    engine.runtime_context.enqueue_canvas_node_removal(0, 42);
    engine.render(&mut renderer);
    assert!(engine.runtime_context.has_canvas_node_removals());
    renderer.pending = false;
    engine.render(&mut renderer);
    assert_eq!(renderer.removals, vec![(0, 42), (0, 42)]);
    assert!(!engine.runtime_context.has_canvas_render_work());
}
