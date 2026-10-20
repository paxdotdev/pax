use super::*;
use kurbo::{Affine, BezPath, Rect, Shape};
use pax_runtime_api::{
    LocalProperty as Property, Paint, ResolvedMaterial, ResolvedStroke, SceneLighting,
};
use std::rc::Weak;

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
    priorities: HashMap<usize, i32>,
    initializations: Vec<usize>,
    async_initialization: bool,
    in_flight: Vec<usize>,
    events: Vec<(&'static str, usize)>,
    ordinary_priorities: HashMap<usize, i32>,
    clip_depths: HashMap<usize, usize>,
    failed_flushes: HashSet<usize>,
    accepts_order_updates: bool,
    order_updates: Vec<(usize, Vec<(u32, i32)>)>,
}

impl RenderContext for RecordingRenderer {
    fn update_node_orders(&mut self, layer: usize, orders: &[(u32, i32)]) -> bool {
        self.order_updates.push((layer, orders.to_vec()));
        self.accepts_order_updates
    }
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
    fn flush(&mut self, layer: usize, dirty: Rc<RefCell<Vec<bool>>>) {
        assert!(layer < self.layer_count);
        self.flushes.push(layer);
        assert_eq!(self.clip_depths.get(&layer).copied().unwrap_or(0), 0);
        if self.failed_flushes.contains(&layer) {
            dirty.borrow_mut()[layer] = true;
        }
        self.presented.push(layer);
        self.events.push(("submit", layer));
        if let Some(remaining) = self.followups.get_mut(&layer) {
            if *remaining > 0 {
                *remaining -= 1;
                self.jobs.push(replay_job(layer));
            }
        }
    }
    fn take_settled_surface_replays(&mut self) -> Vec<pax_runtime_api::ReplayCanvasLayerUpdate> {
        self.jobs.sort_by_key(|job| {
            (
                self.priorities.get(&job.layer).copied().unwrap_or(0),
                job.layer,
            )
        });
        std::mem::take(&mut self.jobs)
    }
    fn surface_replay_priority(&self, layer: usize) -> i32 {
        self.priorities.get(&layer).copied().unwrap_or(0)
    }
    fn ordinary_layer_priority(&self, layer: usize) -> Option<i32> {
        self.ordinary_priorities.get(&layer).copied()
    }
    fn begin_node(&mut self, layer: usize, _: u32, _: i32, _: u32) -> bool {
        assert_eq!(self.clip_depths[&layer], 1, "ancestor clip missing");
        self.events.push(("draw", layer));
        true
    }
    fn start_surface_initialization(&mut self, before: i32) -> bool {
        let selected = self
            .initializations
            .iter()
            .enumerate()
            .filter(|(_, layer)| self.surface_replay_priority(**layer).max(0) < before)
            .min_by_key(|(_, layer)| self.surface_replay_priority(**layer))
            .map(|(index, _)| index);
        let Some(index) = selected else {
            return false;
        };
        let layer = self.initializations.remove(index);
        self.events.push(("initialize", layer));
        if self.async_initialization {
            self.in_flight.push(layer);
        } else {
            self.jobs.push(replay_job(layer));
        }
        true
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
    fn save(&mut self, layer: usize) {
        *self.clip_depths.entry(layer).or_default() += 1;
    }
    fn restore(&mut self, layer: usize) {
        *self.clip_depths.get_mut(&layer).unwrap() -= 1;
    }
    fn clip(&mut self, layer: usize, _: BezPath) {
        assert!(self.clip_depths[&layer] > 0);
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

struct RenderProbe {
    base: crate::BaseInstance,
    clips: bool,
}

impl crate::InstanceNode for RenderProbe {
    fn instantiate(args: crate::InstantiationArgs) -> Rc<Self> {
        let clips = args.children.is_some();
        Rc::new(Self {
            base: crate::BaseInstance::new(
                args,
                crate::InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    layer: if clips {
                        crate::api::Layer::DontCare
                    } else {
                        crate::api::Layer::Canvas
                    },
                    is_component: false,
                    is_slot: false,
                },
            ),
            clips,
        })
    }
    fn base(&self) -> &crate::BaseInstance {
        &self.base
    }
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("RenderProbe").finish()
    }
    fn handle_pre_render(
        &self,
        _: &ExpandedNode,
        ctx: &Rc<RuntimeContext>,
        rcs: &mut dyn RenderContext,
    ) {
        if self.clips {
            for layer in ctx.dirty_canvas_layers() {
                rcs.save(layer);
                rcs.clip(layer, Rect::new(0.0, 0.0, 32.0, 32.0).to_path(0.1));
            }
        }
    }
    fn handle_post_render(
        &self,
        _: &ExpandedNode,
        ctx: &Rc<RuntimeContext>,
        rcs: &mut dyn RenderContext,
    ) {
        if self.clips {
            for layer in ctx.dirty_canvas_layers() {
                rcs.restore(layer);
            }
        }
    }
    fn render(&self, node: &ExpandedNode, ctx: &Rc<RuntimeContext>, rcs: &mut dyn RenderContext) {
        if !self.clips && ctx.is_canvas_node_dirty(&node.id) {
            let layer = node.occlusion.get().render_layer_id;
            if rcs.begin_node(layer, node.id.to_u32(), node.occlusion.get().z_index, 0)
                && rcs.end_node(layer, node.id.to_u32())
            {
                ctx.clear_canvas_node_dirty(&node.id);
            }
        }
    }
}

fn render_probe_args() -> crate::InstantiationArgs {
    crate::InstantiationArgs {
        prototypical_common_properties: crate::CommonPropertiesInit::Default,
        prototypical_properties: crate::PropertiesInit::Factory(Box::new(|_, existing| {
            existing
                .is_none()
                .then(|| Rc::new(RefCell::new(PaxAny::Builtin(Default::default()))))
        })),
        handler_registry: None,
        children: None,
        component_template: None,
        component_settings: None,
        template_node_identifier: None,
        template_node_type_id: None,
        template_node_selector_info: None,
        transition_config: Default::default(),
        properties_scope: crate::PropertiesScopeInit::None,
    }
}

fn dirty_render_probe() -> (PaxEngine, RecordingRenderer, Vec<Rc<ExpandedNode>>) {
    use crate::InstanceNode;
    let mut engine = engine();
    let _graph = engine.runtime_context.property_graph.enter();
    let mut clip_args = render_probe_args();
    clip_args.children = Some(RefCell::new(vec![
        RenderProbe::instantiate(render_probe_args()),
        RenderProbe::instantiate(render_probe_args()),
    ]));
    let mut root_args = render_probe_args();
    root_args.component_template = Some(RefCell::new(vec![RenderProbe::instantiate(clip_args)]));
    let root = ExpandedNode::initialize_root(
        ComponentInstance::instantiate(root_args),
        &engine.runtime_context,
    );
    engine.root_expanded_node = Some(root.clone());
    engine.tick();
    let nodes = root.children.get()[0].children.get();
    assert_eq!(nodes.len(), 2);
    engine.runtime_context.layer_count.set(2);
    engine.runtime_context.resize_canvas_layers_to(2);
    let mut renderer = RecordingRenderer::default();
    engine.render(&mut renderer);
    for (index, node) in nodes.iter().enumerate() {
        let mut occlusion = node.occlusion.get();
        occlusion.render_layer_id = 1 - index; // Unprioritized traversal draws warm layer 0 first.
        node.occlusion.set(occlusion);
        engine.runtime_context.mark_canvas_node_dirty(node.id);
        engine.runtime_context.set_canvas_dirty(1 - index);
    }
    renderer.events.clear();
    renderer.ordinary_priorities = HashMap::from([(0, 5), (1, 0)]);
    renderer.priorities = renderer.ordinary_priorities.clone();
    renderer.cached = true;
    (engine, renderer, nodes)
}

#[test]
fn cooperative_layers_yield_without_lifecycle_or_stale_selection() {
    let (mut engine, mut renderer, nodes) = dirty_render_probe();
    let frames = engine.runtime_context.globals().elapsed_frames.get();
    let mut state = surface_work::RenderWorkState::default();
    state.begin(1);
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    let first = engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    assert!(first.pending);
    assert_eq!(first.ordinary_layers, 1);
    assert_eq!(first.ordinary_drawn_layers, 1);
    assert_eq!(first.ordinary_flush_only_layers, 0);
    assert_eq!(renderer.events, [("draw", 1), ("submit", 1)]);
    assert!(engine.runtime_context.is_canvas_node_dirty(&nodes[1].id));
    assert!(engine.runtime_context.is_canvas_dirty(&0));
    // A reversal changes the next selection, not a copied queue from first call.
    renderer.ordinary_priorities.insert(0, -2);
    let next = engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    assert_eq!(next.ordinary_layers, 1);
    assert_eq!(
        renderer.events,
        [("draw", 1), ("submit", 1), ("draw", 0), ("submit", 0)]
    );
    assert_eq!(
        engine.runtime_context.globals().elapsed_frames.get(),
        frames
    );
    assert!(!engine.runtime_context.has_dirty_canvas_layers());
    assert!(renderer.clip_depths.values().all(|depth| *depth == 0));
    let done = engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    assert!(!done.pending);
    assert_eq!(done.units, 0);
}

#[test]
fn order_only_updates_flush_without_recording_and_fallback_redraws() {
    for accepted in [true, false] {
        let (mut engine, mut renderer, nodes) = dirty_render_probe();
        engine.render(&mut renderer);
        renderer.events.clear();
        renderer.order_updates.clear();
        renderer.accepts_order_updates = accepted;
        let node = &nodes[0];
        let layer = node.occlusion.get().render_layer_id;
        engine
            .runtime_context
            .queue_canvas_node_order(layer, node.id.to_u32(), 100);
        assert!(!engine.runtime_context.can_prepare_settled_layer(layer));
        engine.render(&mut renderer);
        assert_eq!(
            renderer.order_updates,
            vec![(layer, vec![(node.id.to_u32(), 100)])]
        );
        assert!(renderer.events.contains(&("submit", layer)));
        assert_eq!(renderer.events.contains(&("draw", layer)), !accepted);
        assert!(!engine.runtime_context.has_canvas_render_work());
    }
}

#[test]
fn cooperative_order_only_work_submits_without_a_scene_traversal() {
    let (mut engine, mut renderer, nodes) = dirty_render_probe();
    engine.render(&mut renderer);
    renderer.events.clear();
    renderer.order_updates.clear();
    renderer.accepts_order_updates = true;
    engine.filtered_render_plan_inputs.borrow_mut().clear();
    engine
        .runtime_context
        .queue_canvas_node_order(1, nodes[0].id.to_u32(), 100);
    let mut state = surface_work::RenderWorkState::default();
    state.begin(1);
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    let report = engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    assert_eq!(report.ordinary_layers, 1);
    assert_eq!(renderer.events, [("submit", 1)]);
    assert!(engine.filtered_render_plan_inputs.borrow().is_empty());
    assert!(
        report.pending,
        "urgent submission yields before optional work"
    );
    let drained = engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    assert!(!drained.pending);
    assert_eq!(renderer.events, [("submit", 1)]);
    assert!(!engine.runtime_context.has_canvas_render_work());
}

#[test]
fn cooperative_layer_builds_and_consumes_only_one_current_plan() {
    let (mut engine, mut renderer, nodes) = dirty_render_probe();
    engine.filtered_render_plan_inputs.borrow_mut().clear();
    let mut state = surface_work::RenderWorkState::default();
    state.begin(1);
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    let first = engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    assert_eq!(first.ordinary_layers, 1);
    assert_eq!(first.atomic_fallbacks, 0);
    assert_eq!(
        *engine.filtered_render_plan_inputs.borrow(),
        [vec![nodes[0].id]]
    );
    assert_eq!(renderer.events, [("draw", 1), ("submit", 1)]);
    assert!(engine.runtime_context.is_canvas_node_dirty(&nodes[1].id));

    // The next host task must resolve today's ancestry, not a saved traversal.
    let parent = nodes[1].render_parent.replace(Weak::new());
    let next = engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    assert_eq!(next.atomic_fallbacks, 1);
    assert_eq!(next.ordinary_layers, 0);
    assert!(!engine.runtime_context.is_canvas_node_dirty(&nodes[1].id));
    nodes[1].render_parent.replace(parent);
    assert!(renderer.clip_depths.values().all(|depth| *depth == 0));
}

#[test]
fn cooperative_unrelated_invalid_path_does_not_delay_visible_layer() {
    let (mut engine, mut renderer, nodes) = dirty_render_probe();
    // An invalid warm path cannot force visible, separable content into a
    // full-tree render. Its own later turn still uses the conservative fallback.
    let parent = nodes[1].render_parent.replace(Weak::new());
    engine.filtered_render_plan_inputs.borrow_mut().clear();
    let mut state = surface_work::RenderWorkState::default();
    state.begin(1);
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    let first = engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    assert_eq!(first.atomic_fallbacks, 0);
    assert_eq!(first.ordinary_layers, 1);
    assert_eq!(
        *engine.filtered_render_plan_inputs.borrow(),
        [vec![nodes[0].id]]
    );
    assert_eq!(renderer.events, [("draw", 1), ("submit", 1)]);
    assert!(engine.runtime_context.is_canvas_node_dirty(&nodes[1].id));
    let next = engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    assert_eq!(next.atomic_fallbacks, 1);
    assert_eq!(
        renderer.events,
        [("draw", 1), ("submit", 1), ("draw", 0), ("submit", 0)]
    );
    nodes[1].render_parent.replace(parent);
}

#[test]
fn cooperative_layer_plan_keeps_ancestor_scopes_and_rejects_unclippable_paths() {
    let (engine, _, nodes) = dirty_render_probe();
    let root = engine.root_expanded_node.as_ref().unwrap();
    let clip = root.children.get()[0].clone();
    let plan = engine.build_layer_render_plan(1).unwrap().unwrap();
    assert_eq!(plan.dirty_nodes, HashSet::from([nodes[0].id]));
    assert_eq!(
        plan.render_path_nodes,
        HashSet::from([root.id, clip.id, nodes[0].id])
    );
    clip.get_common_properties()
        .borrow()
        .unclippable
        .set(Some(true));
    assert!(matches!(
        engine.build_layer_render_plan(1),
        Err("unclippable_path")
    ));
    clip.get_common_properties()
        .borrow()
        .unclippable
        .set(Some(false));
    assert!(engine.build_layer_render_plan(1).is_ok());
}

#[test]
fn cooperative_removal_only_layer_submits_without_building_a_plan() {
    let mut engine = engine();
    engine.tick();
    engine.runtime_context.layer_count.set(2);
    let mut renderer = RecordingRenderer::default();
    renderer.ordinary_priorities.insert(1, 0);
    engine.render(&mut renderer);
    engine.filtered_render_plan_inputs.borrow_mut().clear();
    renderer.events.clear();
    engine.runtime_context.enqueue_canvas_node_removal(1, 999);
    let mut state = surface_work::RenderWorkState::default();
    state.begin(1);
    let report = engine.service_render_work(
        &mut renderer,
        &mut state,
        &mut surface_work::SurfaceWorkBudget::new(4.0),
        3.0,
        || 0.0,
    );
    assert_eq!(report.ordinary_layers, 1);
    assert!(engine.filtered_render_plan_inputs.borrow().is_empty());
    assert_eq!(renderer.events, [("submit", 1)]);
    assert!(!engine.runtime_context.has_canvas_node_removals());
}

#[test]
fn cooperative_failed_flush_and_removal_remain_pending_without_spinning() {
    let (mut engine, mut renderer, _) = dirty_render_probe();
    renderer.failed_flushes.insert(1);
    renderer.pending = true;
    engine.runtime_context.enqueue_canvas_node_removal(1, 999);
    let mut state = surface_work::RenderWorkState::default();
    state.begin(1);
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    for _ in 0..3 {
        engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    }
    assert_eq!(renderer.presented.iter().filter(|&&l| l == 1).count(), 2); // probe + one attempt
    assert!(engine.runtime_context.is_canvas_dirty(&1));
    assert!(engine.runtime_context.has_canvas_node_removals());
    assert!(!engine.runtime_context.is_canvas_dirty(&0));
    renderer.failed_flushes.clear();
    renderer.pending = false;
    state.begin(2);
    for _ in 0..3 {
        engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    }
    assert!(!engine.runtime_context.has_canvas_node_removals());
    assert!(!engine.runtime_context.has_dirty_canvas_layers());
}

#[test]
fn cooperative_mutation_between_slices_requires_settlement_and_preserves_dirty_work() {
    let (mut engine, mut renderer, nodes) = dirty_render_probe();
    let mut state = surface_work::RenderWorkState::default();
    state.begin(1);
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    engine.runtime_context.mark_occlusion_dirty();
    let before = renderer.events.clone();
    let result = engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    assert!(result.blocked);
    assert_eq!(result.blocker, Some("occlusion"));
    assert!(!result.pending);
    assert_eq!(renderer.events, before);
    assert!(engine.runtime_context.is_canvas_dirty(&0));
    assert!(engine.runtime_context.is_canvas_node_dirty(&nodes[1].id));
}

#[test]
fn cooperative_preparation_overrun_does_not_restart_at_the_next_host_task() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer::default();
    engine.tick();
    engine.runtime_context.layer_count.set(3);
    engine.render(&mut renderer);
    renderer.presented.clear();
    renderer.cached = true;
    renderer.jobs = vec![replay_job(1), replay_job(2)];
    renderer.priorities = HashMap::from([(1, 2), (2, 3)]);
    let mut state = surface_work::RenderWorkState::default();
    state.begin(1);
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    let clock = std::cell::Cell::new(0.0);
    let now = || {
        let t = clock.get();
        clock.set(t + 3.0);
        t
    };
    // First call can spend its slice on setup; the next finishes one atomic tile.
    for _ in 0..4 {
        engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, &now);
    }
    assert_eq!(renderer.presented, [1]);
    assert!(renderer.jobs.iter().any(|j| j.layer == 2));
    state.begin(1); // A completion/property-driven extra tick is not a new frame.
    for _ in 0..4 {
        engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, &now);
    }
    assert_eq!(renderer.presented, [1]);
    state.begin(2);
    for _ in 0..4 {
        engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, &now);
    }
    assert_eq!(renderer.presented, [1, 2]);
}

#[test]
fn cooperative_unsupported_layers_use_atomic_fallback_and_shutdown_cancels_work() {
    let (mut engine, mut renderer, _) = dirty_render_probe();
    renderer.ordinary_priorities.clear();
    let mut state = surface_work::RenderWorkState::default();
    state.begin(1);
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    // No artificial host boundary when there was no preceding urgent work.
    let result = engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    assert_eq!(result.atomic_fallbacks, 1);
    assert_eq!(
        renderer.events,
        [("draw", 0), ("draw", 1), ("submit", 0), ("submit", 1)]
    );
    engine.shutdown(pax_runtime_api::StopReason::HostClosed);
    let before = renderer.events.clone();
    state.begin(2);
    let stopped = engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    assert!(stopped.blocked);
    assert_eq!(stopped.blocker, Some("app_closing"));
    assert_eq!(renderer.events, before);
}

#[test]
fn cooperative_idle_frame_does_not_schedule_an_empty_continuation() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer::default();
    engine.tick();
    engine.render(&mut renderer);
    let mut state = surface_work::RenderWorkState::default();
    state.begin(1);
    let report = engine.service_render_work(
        &mut renderer,
        &mut state,
        &mut surface_work::SurfaceWorkBudget::new(4.0),
        3.0,
        || 0.0,
    );
    assert_eq!(report.units, 0);
    assert!(!report.pending);
}

#[test]
fn cooperative_drain_matches_synchronous_draw_submission() {
    let (mut sync_engine, mut sync_renderer, _) = dirty_render_probe();
    sync_engine.render_with_surface_preparation(
        &mut sync_renderer,
        &mut surface_work::SurfaceWorkBudget::new(4.0),
        || 0.0,
        &mut |_| {},
    );
    let (mut engine, mut renderer, _) = dirty_render_probe();
    let mut state = surface_work::RenderWorkState::default();
    state.begin(1);
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    for _ in 0..8 {
        if !engine
            .service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0)
            .pending
        {
            break;
        }
    }
    assert_eq!(renderer.events, sync_renderer.events);
    assert!(!engine.runtime_context.has_canvas_render_work());
}

#[test]
fn visible_dirty_drawing_and_replay_finish_before_warm_drawing() {
    let (mut engine, mut renderer, nodes) = dirty_render_probe();
    renderer.jobs = vec![replay_job(1)];
    let (visible, _) = engine.render_with_surface_preparation(
        &mut renderer,
        &mut surface_work::SurfaceWorkBudget::new(4.0),
        || 0.0,
        &mut |_| {},
    );
    assert_eq!(visible.submitted, 1);
    assert_eq!(
        renderer.events,
        vec![
            ("draw", 1),
            ("submit", 1),
            ("submit", 1),
            ("draw", 0),
            ("submit", 0)
        ]
    );
    assert!(!engine.runtime_context.has_dirty_canvas_layers());
    for node in nodes {
        assert!(!engine.runtime_context.is_canvas_node_dirty(&node.id));
    }
}

#[test]
fn visible_flush_failure_survives_remaining_drawing_and_blocks_replay() {
    let (mut engine, mut renderer, _) = dirty_render_probe();
    renderer.failed_flushes.insert(1);
    renderer.jobs = vec![replay_job(1)];
    let (visible, remaining) = engine.render_with_surface_preparation(
        &mut renderer,
        &mut surface_work::SurfaceWorkBudget::new(4.0),
        || 0.0,
        &mut |_| {},
    );
    assert_eq!(visible.submitted + remaining.submitted, 0);
    assert!(engine.runtime_context.is_canvas_dirty(&1));
    assert!(!engine.runtime_context.is_canvas_dirty(&0));
    assert_eq!(
        renderer.events,
        vec![("draw", 1), ("submit", 1), ("draw", 0), ("submit", 0)]
    );
}

#[test]
fn dirty_visible_boundary_does_not_restart_preparation_allowance() {
    let (mut engine, mut renderer, _) = dirty_render_probe();
    engine.runtime_context.layer_count.set(3);
    engine.update_layer_count(&mut renderer);
    engine.runtime_context.dirty_canvases.borrow_mut()[2] = false;
    renderer.priorities.insert(2, -1);
    renderer.jobs = vec![replay_job(2), replay_job(1)];
    let now = std::cell::Cell::new(0.0);
    let (visible, remaining) = engine.render_with_surface_preparation(
        &mut renderer,
        &mut surface_work::SurfaceWorkBudget::new(4.0),
        || {
            let time = now.get();
            now.set(time + 3.0);
            time
        },
        &mut |_| {},
    );
    assert_eq!(visible.submitted, 1);
    assert_eq!(remaining.submitted, 0);
    assert!(renderer.jobs.iter().any(|job| job.layer == 1));
    assert_eq!(
        renderer.events,
        vec![
            ("submit", 2),
            ("draw", 1),
            ("submit", 1),
            ("draw", 0),
            ("submit", 0)
        ]
    );
}

#[test]
fn failed_removal_prevents_early_dirty_layer_completion() {
    let (mut engine, mut renderer, _) = dirty_render_probe();
    renderer.pending = true;
    engine.runtime_context.enqueue_canvas_node_removal(1, 999);
    engine.render_with_surface_preparation(
        &mut renderer,
        &mut surface_work::SurfaceWorkBudget::new(4.0),
        || 0.0,
        &mut |_| {},
    );
    assert_eq!(
        renderer.events,
        vec![("draw", 0), ("draw", 1), ("submit", 0), ("submit", 1)]
    );
    assert!(engine.runtime_context.is_canvas_dirty(&1));
    assert!(engine.runtime_context.has_canvas_node_removals());
}

#[test]
fn pending_nodes_on_clean_cold_layers_do_not_disable_visible_partition() {
    let (engine, renderer, nodes) = dirty_render_probe();
    engine.runtime_context.dirty_canvases.borrow_mut()[0] = false;
    let (layers, visible, remaining) = engine
        .partition_visible_render_work(
            &renderer,
            &[1],
            &nodes.iter().map(|n| n.id).collect::<Vec<_>>(),
            &[],
        )
        .unwrap()
        .unwrap();
    assert_eq!(layers, vec![1]);
    assert_eq!(visible, vec![nodes[0].id]);
    assert_eq!(remaining, vec![nodes[1].id]);
    assert!(engine.runtime_context.is_canvas_node_dirty(&nodes[1].id));
}

#[test]
fn node_removal_retires_render_work_without_losing_gpu_removal_or_remount() {
    let (engine, renderer, nodes) = dirty_render_probe();
    let ctx = &engine.runtime_context;
    let removed = &nodes[1];
    ctx.clear_canvas_node_dirty(&removed.id);
    ctx.mark_targeted_canvas_replay_nodes(0, &[removed.id.to_u32()]);
    ctx.enqueue_canvas_node_removal(0, removed.id.to_u32());
    ctx.remove_from_cache(removed);
    assert!(ctx
        .take_targeted_canvas_replay_node_ids()
        .values()
        .all(|ids| ids.is_empty()));
    // Late invalidation during teardown must not resurrect an unresolved ID.
    ctx.mark_canvas_node_dirty(removed.id);
    assert!(!ctx.is_canvas_node_dirty(&removed.id));
    assert!(ctx.has_canvas_node_removals());
    assert!(engine
        .partition_visible_render_work(&renderer, &[0, 1], &ctx.dirty_canvas_node_ids(), &[])
        .unwrap()
        .is_some());

    ctx.add_to_cache(removed);
    assert!(ctx.is_canvas_node_dirty(&removed.id));
    ctx.remove_from_cache(removed);
    assert!(!ctx.is_canvas_node_dirty(&removed.id));
}

#[test]
fn unclippable_drawing_retains_unsplit_traversal() {
    let (mut engine, mut renderer, nodes) = dirty_render_probe();
    nodes[1]
        .get_common_properties()
        .borrow()
        .unclippable
        .set(Some(true));
    // Validate without drawing: a queued unclippable would intentionally leave
    // this fixture's ancestor clip, so its draw assertion should not apply.
    assert!(engine
        .partition_visible_render_work(
            &renderer,
            &[0, 1],
            &nodes.iter().map(|n| n.id).collect::<Vec<_>>(),
            &[]
        )
        .is_err());
    nodes[1]
        .get_common_properties()
        .borrow()
        .unclippable
        .set(Some(false));
    renderer.ordinary_priorities.clear();
    engine.render_with_surface_preparation(
        &mut renderer,
        &mut surface_work::SurfaceWorkBudget::new(4.0),
        || 0.0,
        &mut |_| {},
    );
    assert_eq!(
        renderer.events,
        vec![("draw", 0), ("draw", 1), ("submit", 0), ("submit", 1)]
    );
}

#[test]
fn ordinary_render_services_new_visible_work_before_unrelated_dirty_layers() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer::default();
    engine.tick();
    engine.runtime_context.layer_count.set(4);
    engine.render(&mut renderer);
    renderer.events.clear();
    renderer.cached = true;
    renderer.priorities = HashMap::from([(1, -1), (2, 5), (3, 0)]);
    renderer.jobs = vec![replay_job(1)];
    renderer.initializations = vec![2, 3];
    engine.runtime_context.set_canvas_dirty(0);
    let (visible, remaining) = engine.render_with_surface_preparation(
        &mut renderer,
        &mut surface_work::SurfaceWorkBudget::new(4.0),
        || 0.0,
        &mut |_| {},
    );
    assert_eq!(visible.submitted, 2);
    assert_eq!(visible.initializations, 1);
    assert_eq!(remaining.initializations, 1);
    assert_eq!(
        renderer.events,
        vec![
            ("submit", 1),
            ("initialize", 3),
            ("submit", 3),
            ("submit", 0),
            ("initialize", 2),
            ("submit", 2),
        ]
    );
}

#[test]
fn early_preparation_does_not_bypass_dirty_layers_or_failed_removals() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer::default();
    engine.tick();
    engine.runtime_context.layer_count.set(3);
    engine.render(&mut renderer);
    renderer.events.clear();
    renderer.cached = true;
    renderer.pending = true;
    engine.runtime_context.set_canvas_dirty(0);
    engine.runtime_context.enqueue_canvas_node_removal(1, 42);
    renderer.jobs = vec![replay_job(0), replay_job(1), replay_job(2)];
    let (visible, remaining) = engine.render_with_surface_preparation(
        &mut renderer,
        &mut surface_work::SurfaceWorkBudget::new(4.0),
        || 0.0,
        &mut |_| {},
    );
    assert_eq!(visible.submitted, 1);
    assert_eq!(visible.dirty_layers, 2);
    assert_eq!(renderer.events[0], ("submit", 2));
    assert_eq!(remaining.submitted, 1); // Layer 0 is now settled, layer 1 is not.
    assert!(engine.runtime_context.is_canvas_dirty(&1));
    assert!(engine.runtime_context.has_canvas_node_removals());
    assert_eq!(
        renderer
            .jobs
            .iter()
            .map(|job| job.layer)
            .collect::<Vec<_>>(),
        vec![1]
    );
}

#[test]
fn ordinary_preparation_phases_share_one_allowance_without_a_second_first_job() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer::default();
    engine.tick();
    engine.runtime_context.layer_count.set(3);
    engine.render(&mut renderer);
    renderer.events.clear();
    renderer.cached = true;
    renderer.priorities = HashMap::from([(1, 0), (2, 5)]);
    renderer.jobs = vec![replay_job(1), replay_job(2)];
    engine.runtime_context.set_canvas_dirty(0);
    let now = std::cell::Cell::new(0.0);
    let (visible, remaining) = engine.render_with_surface_preparation(
        &mut renderer,
        &mut surface_work::SurfaceWorkBudget::new(4.0),
        || {
            let t = now.get();
            now.set(t + 3.0);
            t
        },
        &mut |_| {},
    );
    assert_eq!(visible.submitted, 1);
    assert_eq!(remaining.submitted, 0);
    assert_eq!(renderer.events, vec![("submit", 1), ("submit", 0)]);
    assert_eq!(renderer.jobs.len(), 1);
}

#[test]
fn idle_ordinary_render_still_prepares_warm_work() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer::default();
    engine.tick();
    engine.runtime_context.layer_count.set(1);
    engine.render(&mut renderer);
    renderer.cached = true;
    renderer.priorities.insert(0, 5);
    renderer.initializations.push(0);
    let (visible, remaining) = engine.render_with_surface_preparation(
        &mut renderer,
        &mut surface_work::SurfaceWorkBudget::new(4.0),
        || 0.0,
        &mut |_| {},
    );
    assert_eq!(visible.initializations, 0);
    assert_eq!(remaining.initializations, 1);
    assert_eq!(remaining.submitted, 1);
}

#[test]
fn unfiltered_capture_waits_until_unrelated_dirty_work_has_finished() {
    use crate::{
        CommonPropertiesInit, ComponentInstance, InstanceNode, InstantiationArgs, PropertiesInit,
        PropertiesScopeInit,
    };
    let mut engine = engine();
    let _graph = engine.runtime_context.property_graph.enter();
    let root = ExpandedNode::initialize_root(
        ComponentInstance::instantiate(InstantiationArgs {
            prototypical_common_properties: CommonPropertiesInit::Default,
            prototypical_properties: PropertiesInit::Factory(Box::new(|_, existing| {
                existing
                    .is_none()
                    .then(|| Rc::new(RefCell::new(PaxAny::Builtin(Default::default()))))
            })),
            handler_registry: None,
            children: None,
            component_template: Some(RefCell::new(Vec::new())),
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: PropertiesScopeInit::None,
        }),
        &engine.runtime_context,
    );
    engine.root_expanded_node = Some(root.clone());
    engine
        .runtime_context
        .dirty_canvases
        .borrow_mut()
        .resize(1, true);
    assert!(engine.can_capture_surface_nodes(&[root.id.to_u32()]));
    root.get_common_properties()
        .borrow()
        .unclippable
        .set(Some(true));
    assert!(!engine.can_capture_surface_nodes(&[root.id.to_u32()]));
    assert!(engine.runtime_context.is_canvas_dirty(&0));
    engine.runtime_context.clear_all_dirty_canvases();
    assert!(engine.can_capture_surface_nodes(&[root.id.to_u32()]));
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
fn speculation_cannot_block_visible_submissions_or_replenish_optional_allowance() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer::default();
    engine.tick();
    engine.runtime_context.layer_count.set(3);
    engine.render(&mut renderer);
    renderer.cached = true;
    renderer.presented.clear();
    renderer.priorities = HashMap::from([(0, 5), (1, 0), (2, 0)]);
    renderer.jobs = vec![replay_job(0)];
    let mut state = surface_work::RenderWorkState::default();
    state.begin(1);
    let mut budget = surface_work::SurfaceWorkBudget::new(0.0);
    engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    assert_eq!(renderer.presented, [0]);
    assert_eq!(state.preparation.urgent_units, 0);
    // Visible work uses host slices, not the spent speculative allowance.
    renderer.jobs = vec![replay_job(1), replay_job(2)];
    for _ in 0..3 {
        state.begin(1);
        engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    }
    assert_eq!(renderer.presented, [0, 1, 2]);
    assert_eq!(state.preparation.urgent_units, 2);
    renderer.jobs = vec![replay_job(0)];
    state.begin(1);
    engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    assert_eq!(renderer.presented, [0, 1, 2]);
    state.begin(2);
    engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    assert_eq!(renderer.presented, [0, 1, 2, 0]);
}

fn exhausted_cooperative_probe() -> (PaxEngine, RecordingRenderer, surface_work::RenderWorkState) {
    let mut engine = engine();
    engine.tick();
    engine.runtime_context.layer_count.set(3);
    let mut renderer = RecordingRenderer::default();
    engine.render(&mut renderer);
    renderer.events.clear();
    renderer.cached = true;
    renderer.priorities = HashMap::from([(0, 5), (1, -2), (2, -1)]);
    renderer.ordinary_priorities = renderer.priorities.clone();
    engine.runtime_context.set_canvas_dirty(0);
    let mut state = surface_work::RenderWorkState::default();
    state.begin(1);
    state.preparation.elapsed_ms = 4.0;
    state.preparation.submitted = 1;
    (engine, renderer, state)
}

#[test]
fn cooperative_visible_initialization_keeps_priority_through_submission() {
    let (mut engine, mut renderer, mut state) = exhausted_cooperative_probe();
    renderer.initializations = vec![1, 2];
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    // Zero slices separate setup and every atomic unit, including immediate init.
    for _ in 0..5 {
        let report =
            engine.service_render_work(&mut renderer, &mut state, &mut budget, 0.0, || 0.0);
        assert!(report.units <= 1);
    }
    assert_eq!(
        renderer.events,
        [
            ("initialize", 1),
            ("submit", 1),
            ("initialize", 2),
            ("submit", 2)
        ]
    );
    assert!(engine.runtime_context.is_canvas_dirty(&0));
}

#[test]
fn cooperative_visible_overrun_yields_without_losing_submission_priority() {
    let (mut engine, mut renderer, mut state) = exhausted_cooperative_probe();
    renderer.jobs = vec![replay_job(1), replay_job(2)];
    let frames = engine.runtime_context.globals().elapsed_frames.get();
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    let clock = std::cell::Cell::new(0.0);
    let now = || {
        let t = clock.get();
        clock.set(t + 3.0);
        t
    };
    let setup = engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, &now);
    assert_eq!(setup.units, 0);
    for layer in [1, 2] {
        let report = engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, &now);
        assert_eq!(report.units, 1);
        assert!(report.pending);
        assert_eq!(renderer.events.last(), Some(&("submit", layer)));
        assert!(engine.runtime_context.is_canvas_dirty(&0));
    }
    assert_eq!(
        engine.runtime_context.globals().elapsed_frames.get(),
        frames
    );
}

#[test]
fn cooperative_async_completion_preempts_remaining_ordinary_work() {
    let (mut engine, mut renderer, mut state) = exhausted_cooperative_probe();
    renderer.async_initialization = true;
    renderer.initializations = vec![1];
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    for _ in 0..2 {
        engine.service_render_work(&mut renderer, &mut state, &mut budget, 0.0, || 0.0);
    }
    assert_eq!(renderer.in_flight, [1]);
    renderer
        .jobs
        .push(replay_job(renderer.in_flight.pop().unwrap()));
    engine.service_render_work(&mut renderer, &mut state, &mut budget, 0.0, || 0.0);
    assert_eq!(renderer.events, [("initialize", 1), ("submit", 1)]);
    assert!(engine.runtime_context.is_canvas_dirty(&0));
}

#[test]
fn cooperative_visible_replay_waits_for_its_dirty_content_not_unrelated_layers() {
    let (mut engine, mut renderer, _) = dirty_render_probe();
    let mut state = surface_work::RenderWorkState::default();
    state.begin(1);
    state.preparation.elapsed_ms = 4.0;
    state.preparation.submitted = 1;
    renderer.jobs.push(replay_job(1));
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    for _ in 0..3 {
        engine.service_render_work(&mut renderer, &mut state, &mut budget, 0.0, || 0.0);
    }
    // Current application content is drawn before the ready replay consumes it.
    assert_eq!(renderer.events, [("draw", 1), ("submit", 1), ("submit", 1)]);
    assert!(!engine.runtime_context.is_canvas_dirty(&1));
    assert!(engine.runtime_context.is_canvas_dirty(&0));
}

#[test]
fn cooperative_reversal_reclassifies_completion_and_queued_initialization() {
    let (mut engine, mut renderer, mut state) = exhausted_cooperative_probe();
    renderer.async_initialization = true;
    renderer.initializations = vec![1, 2];
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    for _ in 0..2 {
        engine.service_render_work(&mut renderer, &mut state, &mut budget, 0.0, || 0.0);
    }
    renderer.priorities.insert(1, 10); // Old direction completes after reversal.
    renderer
        .jobs
        .push(replay_job(renderer.in_flight.pop().unwrap()));
    renderer.async_initialization = false;
    for _ in 0..2 {
        engine.service_render_work(&mut renderer, &mut state, &mut budget, 0.0, || 0.0);
    }
    assert_eq!(
        renderer.events,
        [("initialize", 1), ("initialize", 2), ("submit", 2)]
    );
    assert!(renderer.jobs.iter().any(|job| job.layer == 1));
}

#[test]
fn cooperative_pending_initialization_does_not_spin_or_block_ordinary_work() {
    let (mut engine, mut renderer, mut state) = exhausted_cooperative_probe();
    renderer.async_initialization = true;
    renderer.initializations = vec![1];
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    let first = engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    assert!(first.pending);
    let next = engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    assert!(!next.pending);
    assert_eq!(renderer.events, [("initialize", 1), ("submit", 0)]);
    assert_eq!(renderer.in_flight, [1]);
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
fn settled_service_finishes_visible_before_initializing_speculative_work() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer::default();
    engine.tick();
    engine.runtime_context.layer_count.set(4);
    engine.render(&mut renderer);
    renderer.events.clear();
    renderer.cached = true;
    renderer.priorities = HashMap::from([(0, 5), (1, -1), (2, -2), (3, 8)]);
    renderer.jobs = vec![replay_job(0), replay_job(1)];
    renderer.initializations = vec![3, 2];
    let report = engine.present_settled_surfaces(
        &mut renderer,
        &mut surface_work::SurfaceWorkBudget::new(4.0),
        || 0.0,
    );
    assert_eq!(report.initializations, 2);
    assert_eq!(
        renderer.events,
        vec![
            ("submit", 1),
            ("initialize", 2),
            ("submit", 2),
            ("submit", 0),
            ("initialize", 3),
            ("submit", 3),
        ]
    );
}

#[test]
fn initialization_is_budgeted_and_does_not_jump_a_visible_replay_on_resume() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer::default();
    engine.tick();
    engine.runtime_context.layer_count.set(2);
    engine.render(&mut renderer);
    renderer.events.clear();
    renderer.cached = true;
    renderer.priorities = HashMap::from([(0, 0), (1, 5)]);
    renderer.initializations = vec![0, 1];
    let mut budget = surface_work::SurfaceWorkBudget::new(0.0);
    let now = std::cell::Cell::new(0.0);
    let clock = || {
        let t = now.get();
        now.set(t + 1.0);
        t
    };
    let first = engine.present_settled_surfaces(&mut renderer, &mut budget, clock);
    assert_eq!(first.initializations, 1);
    assert_eq!(first.submitted, 0);
    let second = engine.present_settled_surfaces(&mut renderer, &mut budget, clock);
    assert_eq!(second.initializations, 0);
    assert_eq!(second.submitted, 1);
    assert_eq!(renderer.events, vec![("initialize", 0), ("submit", 0)]);
    // Current visibility, not old enqueue urgency, selects the next pass.
    renderer.priorities.insert(1, -1);
    assert_eq!(
        engine
            .present_settled_surfaces(&mut renderer, &mut budget, clock)
            .initializations,
        1
    );
}

#[test]
fn blocked_visible_replay_does_not_prevent_another_surface_initializing() {
    let mut engine = engine();
    let mut renderer = RecordingRenderer::default();
    engine.tick();
    engine.runtime_context.layer_count.set(2);
    engine.render(&mut renderer);
    renderer.events.clear();
    renderer.cached = true;
    renderer.jobs = vec![replay_job(0)];
    renderer.blocked_layers.insert(0);
    renderer.initializations = vec![1];
    renderer.priorities.insert(1, 4);
    let report = engine.present_settled_surfaces(
        &mut renderer,
        &mut surface_work::SurfaceWorkBudget::new(4.0),
        || 0.0,
    );
    assert_eq!(report.initializations, 1);
    assert_eq!(report.deferred, 1);
    assert_eq!(renderer.events, vec![("initialize", 1), ("submit", 1)]);
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
fn cooperative_lighting_flushes_survive_yield_and_next_tick_without_canvas_clears() {
    for priorities in [HashMap::new(), HashMap::from([(0, 3), (1, 0), (2, 4)])] {
        let mut engine = engine();
        let mut renderer = RecordingRenderer::default();
        engine.tick();
        engine.runtime_context.layer_count.set(3);
        engine.render(&mut renderer);
        engine.filtered_render_plan_inputs.borrow_mut().clear();
        renderer.clears.clear();
        renderer.lighting_updates.clear();
        renderer.flushes.clear();
        renderer.ordinary_priorities = priorities;
        engine.runtime_context.mark_scene_lighting_dirty();
        let mut state = surface_work::RenderWorkState::default();
        let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
        state.begin(1);
        // Stop after setup, then start a new frame before those uniforms submit.
        let setup = engine.service_render_work(&mut renderer, &mut state, &mut budget, 0.0, || 0.0);
        assert!(setup.pending);
        assert_eq!(renderer.lighting_updates, [0, 1, 2]);
        assert!(renderer.flushes.is_empty());
        assert!(!engine.runtime_context.has_dirty_canvas_layers());
        state.begin(2);
        let mut flush_only_layers = 0;
        for _ in 0..4 {
            let report =
                engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
            assert_eq!(report.ordinary_drawn_layers, 0);
            flush_only_layers += report.ordinary_flush_only_layers;
            if !report.pending {
                break;
            }
        }
        assert_eq!(flush_only_layers, 3);
        if !renderer.ordinary_priorities.is_empty() {
            assert_eq!(
                renderer.flushes,
                [1, 0, 2],
                "visible lighting submits first"
            );
        }
        renderer.flushes.sort_unstable();
        assert_eq!(renderer.flushes, [0, 1, 2]);
        assert!(renderer.clears.is_empty());
        assert!(state.pending_lighting_flushes.is_empty());
        assert!(!engine.runtime_context.has_canvas_render_work());
        assert!(engine.filtered_render_plan_inputs.borrow().is_empty());
    }
}

#[test]
fn cooperative_lighting_only_layers_survive_atomic_content_fallback() {
    let (mut engine, mut renderer, _) = dirty_render_probe();
    engine.runtime_context.layer_count.set(3);
    engine.render(&mut renderer);
    renderer.clears.clear();
    renderer.lighting_updates.clear();
    renderer.flushes.clear();
    renderer.ordinary_priorities.clear();
    engine.runtime_context.mark_scene_lighting_dirty();
    engine.runtime_context.set_canvas_dirty(1);
    let mut state = surface_work::RenderWorkState::default();
    state.begin(1);
    let mut budget = surface_work::SurfaceWorkBudget::new(4.0);
    let report = engine.service_render_work(&mut renderer, &mut state, &mut budget, 3.0, || 0.0);
    assert_eq!(report.atomic_fallbacks, 1);
    renderer.flushes.sort_unstable();
    assert_eq!(renderer.flushes, [0, 1, 2]);
    assert_eq!(
        renderer.clears,
        [1, 1],
        "only the content-dirty layer is cleared"
    );
    assert!(state.pending_lighting_flushes.is_empty());
    assert!(!engine.runtime_context.has_canvas_render_work());
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
