use super::layer_surface::{
    replay_batches_by_directional_priority, surface_intersects_coverage_bounds, LayerSurfaceEntry,
    LayerSurfaceLayout, LayoutChangeKind, ReplayPriorityEntry, SurfaceReplayCoordinator,
};
#[cfg(debug_assertions)]
use super::layer_surface::{visible_surface_escape, VisibleSurfaceEscape};
use kurbo::{BezPath, PathEl, Rect, Shape};
use pax_gpu::{
    point, Box2D, DrawRange as PixelDrawRange, Image, LightShape as PixelLightShape,
    Material as PixelMaterial, Path, ResourceChurnStats, SceneLight as PixelSceneLight,
    SceneLighting as PixelSceneLighting, Stroke as PixelStroke, StrokeCap as PixelStrokeCap,
    StrokeJoin as PixelStrokeJoin, Transform2D, WgpuRenderer,
};
use pax_runtime_api::{
    Axis, LayerSurfaceScreenshotData, Material, PathSmoothing, RenderContext,
    ReplayCanvasLayerUpdate, SceneLighting, ScreenshotData, Stroke, StrokeCap, StrokeJoin,
};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    future::Future,
    pin::Pin,
    rc::Rc,
};

#[cfg(not(target_arch = "wasm32"))]
use pollster;

/// Retained renderer bound to one physical surface tile for a logical layer.
pub struct LayerRenderer {
    key: String,
    host_signature: String,
    renderer: WgpuRenderer<'static>,
    origin_x: f32,
    origin_y: f32,
    logical_width: f32,
    logical_height: f32,
    surface_width: u32,
    surface_height: u32,
    dpr: [f32; 2],
}

/// Current renderer set for one logical layer.
pub struct LayerTarget {
    renderers: Vec<LayerRenderer>,
    active: bool,
    needs_replay: bool,
}

#[cfg(debug_assertions)]
#[derive(Clone, Copy, Default)]
struct TileCullStats {
    nodes_considered: u64,
    selected_surfaces: u64,
    skipped_surfaces: u64,
    stale_surface_removal_attempts: u64,
    origin_only_retargets: u64,
    targeted_replay_batches: u64,
    targeted_replay_surfaces: u64,
    targeted_replay_visible_batches: u64,
    targeted_replay_warm_batches: u64,
    targeted_replay_visible_surfaces: u64,
    targeted_replay_warm_surfaces: u64,
    full_layer_replays: u64,
    resize_resets: u64,
}

#[cfg(debug_assertions)]
fn replay_priority_stats(
    entries: &[ReplayPriorityEntry],
    batches: &[Vec<usize>],
) -> (usize, usize, usize, usize) {
    let visible_surfaces = entries.iter().filter(|entry| entry.priority <= 0).count();
    let warm_surfaces = entries.len().saturating_sub(visible_surfaces);
    let priority_by_index: HashMap<usize, i32> = entries
        .iter()
        .map(|entry| (entry.index, entry.priority))
        .collect();
    let visible_batches = batches
        .iter()
        .filter(|batch| {
            batch.iter().any(|index| {
                priority_by_index
                    .get(index)
                    .map_or(false, |priority| *priority <= 0)
            })
        })
        .count();
    let warm_batches = batches.len().saturating_sub(visible_batches);

    (
        visible_batches,
        warm_batches,
        visible_surfaces,
        warm_surfaces,
    )
}

#[cfg(debug_assertions)]
fn log_visible_surface_escape(layer: usize, escape: VisibleSurfaceEscape) {
    log::warn!(
        "[pax-tile-window-escape] backend=wgpu layer={} visible_surfaces={} escaped_visible_surfaces={} max_gap_x={:.1} max_gap_y={:.1} previous_bounds={} visible_bounds={}",
        layer,
        escape.visible_surfaces,
        escape.escaped_visible_surfaces,
        escape.max_gap_x,
        escape.max_gap_y,
        format_rect(escape.previous_bounds),
        format_rect(escape.visible_bounds),
    );
}

#[cfg(debug_assertions)]
fn format_rect(bounds: Rect) -> String {
    format!(
        "{:.1},{:.1}-{:.1},{:.1}",
        bounds.x0, bounds.y0, bounds.x1, bounds.y1
    )
}

#[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
fn should_print_gpu_profile_logs() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("PAX_GPU_PROFILE")
            .map(|value| {
                let value = value.trim().to_ascii_lowercase();
                !(value.is_empty() || value == "0" || value == "false" || value == "off")
            })
            .unwrap_or(false)
    })
}

type LayerDef = (LayerTarget, Pin<Box<dyn Fn() -> LayerSurfaceLayout>>);

impl LayerRenderer {
    /// Create a renderer wrapper with its current tile geometry.
    pub fn new(
        key: String,
        host_signature: String,
        renderer: WgpuRenderer<'static>,
        origin_x: f32,
        origin_y: f32,
        logical_width: f32,
        logical_height: f32,
        surface_width: u32,
        surface_height: u32,
        dpr: [f32; 2],
    ) -> Self {
        Self {
            key,
            host_signature,
            renderer,
            origin_x,
            origin_y,
            logical_width,
            logical_height,
            surface_width,
            surface_height,
            dpr,
        }
    }

    /// Access the underlying retained `pax-gpu` renderer.
    pub fn renderer_mut(&mut self) -> &mut WgpuRenderer<'static> {
        &mut self.renderer
    }

    fn intersects_coverage_bounds(&self, bounds: &Rect) -> bool {
        surface_intersects_coverage_bounds(
            bounds,
            self.origin_x as f64,
            self.origin_y as f64,
            self.logical_width as f64,
            self.logical_height as f64,
        )
    }

    fn coverage_bounds(&self) -> Rect {
        Rect::new(
            self.origin_x as f64,
            self.origin_y as f64,
            self.origin_x as f64 + self.logical_width as f64,
            self.origin_y as f64 + self.logical_height as f64,
        )
    }

    fn update_layout(&mut self, surface: &LayerSurfaceEntry) -> LayoutChangeKind {
        let origin_changed = (self.origin_x - surface.origin_x).abs() > f32::EPSILON
            || (self.origin_y - surface.origin_y).abs() > f32::EPSILON;
        let size_changed = self.logical_width != surface.surface.logical_width
            || self.logical_height != surface.surface.logical_height
            || self.surface_width != surface.surface.surface_width
            || self.surface_height != surface.surface.surface_height
            || self.dpr != surface.surface.dpr;

        self.origin_x = surface.origin_x;
        self.origin_y = surface.origin_y;
        self.logical_width = surface.surface.logical_width;
        self.logical_height = surface.surface.logical_height;
        self.surface_width = surface.surface.surface_width;
        self.surface_height = surface.surface.surface_height;
        self.dpr = surface.surface.dpr;

        if size_changed {
            LayoutChangeKind::Resized
        } else if origin_changed {
            LayoutChangeKind::OriginOnly
        } else {
            LayoutChangeKind::Unchanged
        }
    }

    /// Update the retained layout metadata after the owner has already applied matching backend
    /// surface/view transforms.
    pub fn sync_layout_metadata(&mut self, surface: &LayerSurfaceEntry) {
        self.update_layout(surface);
    }
}

impl LayerTarget {
    /// Create a layer target from physical surface renderers.
    pub fn new(mut renderers: Vec<LayerRenderer>, active: bool) -> Self {
        // GPU resources remain surface-local, but CPU tessellation is pure geometry work. Share that
        // cache across the physical surfaces for one logical layer so tile replays can reuse meshes.
        if let Some((first, rest)) = renderers.split_first_mut() {
            for renderer in rest {
                renderer.renderer.share_vector_caches_from(&first.renderer);
            }
        }
        Self {
            renderers,
            active,
            needs_replay: false,
        }
    }

    /// Mutable access to each physical renderer backing this logical layer.
    pub fn renderers_mut(&mut self) -> &mut [LayerRenderer] {
        &mut self.renderers
    }

    fn activate(&mut self) {
        if !self.active {
            self.active = true;
            self.needs_replay = true;
        }
    }

    fn deactivate(&mut self) {
        self.active = false;
    }

    fn prepare_for_render(&mut self) {
        if !self.needs_replay {
            return;
        }
        for renderer in &mut self.renderers {
            renderer.renderer.reset_retained_scene();
        }
        self.needs_replay = false;
    }
}

fn layer_layout_matches_target(target: &LayerTarget, layout: &LayerSurfaceLayout) -> bool {
    // `host_signature` identifies the DOM surface host that currently owns a layer tile. It is
    // intentionally separate from node identity: the same expanded node can be rebound to a new
    // browser surface host when scroller islands are recreated.
    layout.surfaces.len() == target.renderers.len()
        && layout
            .surfaces
            .iter()
            .zip(target.renderers.iter())
            .all(|(surface, renderer)| {
                surface.key == renderer.key && surface.host_signature == renderer.host_signature
            })
}

fn layer_layout_matches_bootstrapped_target(
    target: &LayerTarget,
    layout: &LayerSurfaceLayout,
) -> bool {
    // Async layer creation can overlap DOM scroller setup. Only publish a freshly-created renderer
    // if it still matches the current host geometry; once published, refresh_layer_layouts handles
    // ordinary resize/origin changes in place.
    target.active == layout.active
        && layer_layout_matches_target(target, layout)
        && layout
            .surfaces
            .iter()
            .zip(target.renderers.iter())
            .all(|(surface, renderer)| {
                (surface.origin_x - renderer.origin_x).abs() <= f32::EPSILON
                    && (surface.origin_y - renderer.origin_y).abs() <= f32::EPSILON
                    && (surface.surface.logical_width - renderer.logical_width).abs()
                        <= f32::EPSILON
                    && (surface.surface.logical_height - renderer.logical_height).abs()
                        <= f32::EPSILON
                    && surface.surface.surface_width == renderer.surface_width
                    && surface.surface.surface_height == renderer.surface_height
                    && (surface.surface.dpr[0] - renderer.dpr[0]).abs() <= f32::EPSILON
                    && (surface.surface.dpr[1] - renderer.dpr[1]).abs() <= f32::EPSILON
            })
}

const MAX_CONCURRENT_LAYER_INITIALIZATIONS: usize = 4;

fn pump_layer_initialization_queue(
    factory: Rc<dyn Fn(usize) -> Pin<Box<dyn Future<Output = Option<LayerDef>>>>>,
    backends: Rc<RefCell<Vec<RenderLayerState>>>,
    queue: Rc<RefCell<VecDeque<usize>>>,
    scheduled: Rc<RefCell<HashSet<usize>>>,
    in_flight: Rc<Cell<usize>>,
    ready_layers: Rc<RefCell<Vec<usize>>>,
) {
    while in_flight.get() < MAX_CONCURRENT_LAYER_INITIALIZATIONS {
        let next_layer = queue.borrow_mut().pop_front();
        let Some(layer_index) = next_layer else {
            break;
        };

        in_flight.set(in_flight.get() + 1);
        let factory = Rc::clone(&factory);
        let backends = Rc::clone(&backends);
        let queue = Rc::clone(&queue);
        let scheduled = Rc::clone(&scheduled);
        let in_flight_count = Rc::clone(&in_flight);
        let ready_layers = Rc::clone(&ready_layers);

        let task = async move {
            let backend = (factory)(layer_index).await;
            let mut should_requeue = false;
            match backend {
                Some(layer_def) => {
                    let current_layout = layer_def.1();
                    let layout_matches =
                        layer_layout_matches_bootstrapped_target(&layer_def.0, &current_layout);
                    let mut backend_states = backends.borrow_mut();
                    match backend_states.get_mut(layer_index) {
                        Some(change) if layout_matches => {
                            *change = RenderLayerState::Ready(layer_def);
                            ready_layers.borrow_mut().push(layer_index);
                        }
                        Some(change) => {
                            // Layer initialization is async relative to host ownership on the
                            // web. If the DOM host changes while this backend is bootstrapping,
                            // discard it and retry against the latest layout instead of
                            // publishing a stale renderer.
                            *change = RenderLayerState::Pending;
                            should_requeue = true;
                        }
                        None => {
                            log::warn!(
                                "failed to set poll state to ready: layer {} doesn't exist anymore",
                                layer_index
                            );
                        }
                    }
                }
                None => match backends.borrow_mut().get_mut(layer_index) {
                    Some(change) => {
                        *change = RenderLayerState::Failed;
                        log::warn!(
                            "failed to initialize render backend for layer {}",
                            layer_index
                        );
                    }
                    None => log::warn!(
                        "failed to set poll state to ready: layer {} doesn't exist and backend failed to initialize",
                        layer_index
                    ),
                },
            }

            scheduled.borrow_mut().remove(&layer_index);
            if should_requeue && scheduled.borrow_mut().insert(layer_index) {
                queue.borrow_mut().push_back(layer_index);
            }

            in_flight_count.set(in_flight_count.get().saturating_sub(1));
            pump_layer_initialization_queue(
                factory,
                backends,
                queue,
                scheduled,
                in_flight_count,
                ready_layers,
            );
        };

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(task);
        #[cfg(not(target_arch = "wasm32"))]
        {
            pollster::block_on(task);
        }
    }
}
/// Runtime `RenderContext` implementation backed by `pax-gpu`/wgpu.
pub struct PaxGpuRenderer {
    backends: Rc<RefCell<Vec<RenderLayerState>>>,
    layer_factory: Rc<dyn Fn(usize) -> Pin<Box<dyn Future<Output = Option<LayerDef>>>>>,
    image_map: HashMap<String, Image>,
    image_versions: HashMap<String, u64>,
    failed_context_gets: RefCell<Vec<bool>>,
    ready_layers: Rc<RefCell<Vec<usize>>>,
    replay_layers: Rc<RefCell<Vec<usize>>>,
    pending_layer_initializations: Rc<RefCell<VecDeque<usize>>>,
    scheduled_layer_initializations: Rc<RefCell<HashSet<usize>>>,
    layer_initializations_in_flight: Rc<Cell<usize>>,
    active_render_scopes: RefCell<Vec<Vec<Vec<usize>>>>,
    dirty_render_surfaces: RefCell<Vec<HashSet<usize>>>,
    last_scene_lighting: RefCell<Vec<Option<PixelSceneLighting>>>,
    surface_replay: RefCell<SurfaceReplayCoordinator>,
    clean_skipped_canvas_nodes: RefCell<HashSet<(usize, u32)>>,
    #[cfg(debug_assertions)]
    tile_cull_stats: RefCell<Vec<TileCullStats>>,
}

/// Lifecycle state for a lazily-created render layer.
pub enum RenderLayerState {
    Pending,
    Failed,
    Ready(LayerDef),
}

impl PaxGpuRenderer {
    /// Create a renderer that lazily asks the chassis for layer backends.
    pub fn new(
        layer_factory: impl Fn(usize) -> Pin<Box<dyn Future<Output = Option<LayerDef>>>> + 'static,
    ) -> Self {
        Self {
            backends: Default::default(),
            layer_factory: Rc::new(layer_factory),
            image_map: Default::default(),
            image_versions: Default::default(),
            failed_context_gets: RefCell::new(vec![]),
            ready_layers: Default::default(),
            replay_layers: Default::default(),
            pending_layer_initializations: Rc::new(RefCell::new(VecDeque::new())),
            scheduled_layer_initializations: Rc::new(RefCell::new(HashSet::new())),
            layer_initializations_in_flight: Rc::new(Cell::new(0)),
            active_render_scopes: Default::default(),
            dirty_render_surfaces: Default::default(),
            last_scene_lighting: Default::default(),
            surface_replay: Default::default(),
            clean_skipped_canvas_nodes: Default::default(),
            #[cfg(debug_assertions)]
            tile_cull_stats: Default::default(),
        }
    }
}

impl PaxGpuRenderer {
    fn queue_layer_initialization(&self, layer_index: usize) {
        if self
            .scheduled_layer_initializations
            .borrow_mut()
            .insert(layer_index)
        {
            self.pending_layer_initializations
                .borrow_mut()
                .push_back(layer_index);
        }
    }

    fn enqueue_layer_initialization(&self, layer_index: usize) {
        self.queue_layer_initialization(layer_index);
        self.schedule_layer_initialization();
    }

    fn with_layer_context(&self, layer: usize, mut f: impl FnMut(&mut WgpuRenderer)) {
        let mut backends = self.backends.borrow_mut();
        match backends.get_mut(layer) {
            Some(layer_state) => match layer_state {
                RenderLayerState::Pending => {
                    let mut failed_context_gets = self.failed_context_gets.borrow_mut();
                    if failed_context_gets.len() <= layer {
                        failed_context_gets.resize(layer + 1, false);
                    }
                    failed_context_gets[layer] = true;
                }
                RenderLayerState::Failed => {}
                RenderLayerState::Ready((target, _)) => {
                    if !target.active {
                        return;
                    }
                    target.prepare_for_render();
                    let scoped_indices = self.active_render_scope(layer).or_else(|| {
                        // Targeted replay is active outside a node scope during clear(), and then
                        // nested into per-node scopes for actual draw operations.
                        self.targeted_replay_scope(layer)
                    });
                    if let Some(indices) = scoped_indices {
                        for index in indices {
                            if let Some(renderer) = target.renderers.get_mut(index) {
                                f(&mut renderer.renderer);
                            }
                        }
                    } else {
                        for renderer in &mut target.renderers {
                            f(&mut renderer.renderer);
                        }
                    }
                }
            },
            None => log::warn!(
                "tried to retrieve layer {} context for non-existent layer",
                layer
            ),
        }
    }

    fn active_render_scope(&self, layer: usize) -> Option<Vec<usize>> {
        self.active_render_scopes
            .borrow()
            .get(layer)
            .and_then(|scopes| scopes.last().cloned())
    }

    fn push_render_scope(&self, layer: usize, renderer_indices: Vec<usize>) {
        let mut scopes = self.active_render_scopes.borrow_mut();
        if scopes.len() <= layer {
            scopes.resize_with(layer + 1, Vec::new);
        }
        scopes[layer].push(renderer_indices);
    }

    fn pop_render_scope(&self, layer: usize) {
        if let Some(scopes) = self.active_render_scopes.borrow_mut().get_mut(layer) {
            scopes.pop();
        }
    }

    fn mark_render_surfaces_dirty(
        &self,
        layer: usize,
        renderer_indices: impl IntoIterator<Item = usize>,
    ) {
        let mut dirty_surfaces = self.dirty_render_surfaces.borrow_mut();
        if dirty_surfaces.len() <= layer {
            dirty_surfaces.resize_with(layer + 1, HashSet::new);
        }
        dirty_surfaces[layer].extend(renderer_indices);
    }

    fn dirty_render_surface_scope(&self, layer: usize) -> Option<Vec<usize>> {
        let mut indices: Vec<_> = self
            .dirty_render_surfaces
            .borrow()
            .get(layer)?
            .iter()
            .copied()
            .collect();
        if indices.is_empty() {
            return None;
        }
        indices.sort_unstable();
        Some(indices)
    }

    fn clear_dirty_render_surfaces(&self, layer: usize, renderer_indices: &[usize]) {
        if let Some(dirty_surfaces) = self.dirty_render_surfaces.borrow_mut().get_mut(layer) {
            for index in renderer_indices {
                dirty_surfaces.remove(index);
            }
        }
    }

    fn update_scene_lighting_cache(&self, layer: usize, lighting: &PixelSceneLighting) -> bool {
        let mut last_scene_lighting = self.last_scene_lighting.borrow_mut();
        if last_scene_lighting.len() <= layer {
            last_scene_lighting.resize_with(layer + 1, || None);
        }
        let changed = last_scene_lighting[layer].as_ref() != Some(lighting);
        if changed {
            last_scene_lighting[layer] = Some(lighting.clone());
        }
        changed
    }

    fn targeted_replay_scope(&self, layer: usize) -> Option<Vec<usize>> {
        self.surface_replay.borrow().targeted_replay_scope(layer)
    }

    fn set_targeted_replay_batches(
        &self,
        layer: usize,
        batches: Vec<Vec<usize>>,
        bounds_by_surface: HashMap<usize, Vec<Rect>>,
    ) {
        self.surface_replay
            .borrow_mut()
            .set_targeted_replay_batches(layer, batches, bounds_by_surface);
    }

    fn advance_targeted_replay_queue(&self, layer: usize) -> bool {
        self.surface_replay
            .borrow_mut()
            .advance_targeted_replay_queue(layer)
    }

    fn flush_targeted_or_dirty(&self, layer: usize, target: &mut LayerTarget) -> bool {
        let targeted_indices = self.targeted_replay_scope(layer);
        let dirty_indices = self.dirty_render_surface_scope(layer);
        let used_targeted_replay = targeted_indices.is_some();
        #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
        let targeted_surface_count = targeted_indices.as_ref().map(Vec::len).unwrap_or(0);
        #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
        let dirty_surface_count = dirty_indices.as_ref().map(Vec::len).unwrap_or(0);
        let mut indices = targeted_indices.unwrap_or_default();
        if let Some(dirty_indices) = dirty_indices {
            indices.extend(dirty_indices);
        }
        indices.sort_unstable();
        indices.dedup();

        if !indices.is_empty() {
            #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
            let flush_start = std::time::Instant::now();
            #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
            let mut encode_us = 0u128;
            #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
            let mut submit_us = 0u128;
            #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
            let mut cleanup_us = 0u128;
            #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
            let mut present_us = 0u128;
            let mut resource_stats = ResourceChurnStats::default();
            let mut command_buffers = Vec::new();
            let mut submitter_index = None;
            let mut cleanup_indices = Vec::new();
            let mut present_indices = Vec::new();
            for index in &indices {
                if let Some(renderer) = target.renderers.get_mut(*index) {
                    #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
                    let encode_start = std::time::Instant::now();
                    renderer.renderer.flush_deferred();
                    #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
                    {
                        encode_us += encode_start.elapsed().as_micros();
                    }
                    let mut renderer_command_buffers =
                        renderer.renderer.take_pending_command_buffers();
                    if !renderer_command_buffers.is_empty() {
                        if submitter_index.is_none() {
                            submitter_index = Some(*index);
                        }
                        command_buffers.append(&mut renderer_command_buffers);
                    }
                    if renderer.renderer.has_pending_submission_cleanup() {
                        cleanup_indices.push(*index);
                    }
                    present_indices.push(*index);
                    resource_stats.merge(renderer.renderer.take_resource_churn_stats());
                }
            }
            let command_buffer_count = command_buffers.len();
            if command_buffer_count != 0 {
                if let Some(index) = submitter_index {
                    if let Some(renderer) = target.renderers.get(index) {
                        #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
                        let submit_start = std::time::Instant::now();
                        renderer.renderer.submit_command_buffers(command_buffers);
                        #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
                        {
                            submit_us += submit_start.elapsed().as_micros();
                        }
                        log::trace!(
                            "[pax-gpu-submit] layer={} surfaces={} command_buffers={} shared_submit=1",
                            layer,
                            indices.len(),
                            command_buffer_count
                        );
                    }
                }
            }
            for index in &cleanup_indices {
                if let Some(renderer) = target.renderers.get_mut(*index) {
                    #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
                    let cleanup_start = std::time::Instant::now();
                    renderer.renderer.complete_submitted_work();
                    #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
                    {
                        cleanup_us += cleanup_start.elapsed().as_micros();
                    }
                }
            }
            for index in &present_indices {
                if let Some(renderer) = target.renderers.get_mut(*index) {
                    #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
                    let present_start = std::time::Instant::now();
                    renderer.renderer.present_deferred_frame();
                    #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
                    {
                        present_us += present_start.elapsed().as_micros();
                    }
                }
            }
            #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
            {
                let total_us = flush_start.elapsed().as_micros();
                let line = format!(
                    "[pax-gpu-flush] layer={} surfaces={} targeted={} targeted_surfaces={} dirty_surfaces={} command_buffers={} cleanup_surfaces={} present_surfaces={} total_us={} encode_us={} submit_us={} cleanup_us={} present_us={} flushes={} retained_nodes={}/{} retained_batches={} retained_draws={} vector_draws={} image_draws={} batch_flushes={} geometry_rebuilds={} geometry_cache_hits={} geometry_cache_misses={} geometry_cache_evictions={} geometry_cache_bytes={} tessellated_vertices={} tessellated_indices={} cached_vertices_reused={} cached_indices_reused={} resource_creates={} resource_updates={} resource_recreates={} create_bytes={} update_bytes={} cache_hits={} cache_misses={} cache_evictions={} cache_bytes={} texture_creates={} texture_bytes={}",
                    layer,
                    indices.len(),
                    used_targeted_replay,
                    targeted_surface_count,
                    dirty_surface_count,
                    command_buffer_count,
                    cleanup_indices.len(),
                    present_indices.len(),
                    total_us,
                    encode_us,
                    submit_us,
                    cleanup_us,
                    present_us,
                    resource_stats.flushes,
                    resource_stats.retained_nodes_visible,
                    resource_stats.retained_nodes_considered,
                    resource_stats.retained_draw_batches,
                    resource_stats.retained_draws,
                    resource_stats.retained_vector_draws,
                    resource_stats.retained_image_draws,
                    resource_stats.vector_batch_flushes,
                    resource_stats.vector_geometry_rebuilds,
                    resource_stats.vector_geometry_cache_hits,
                    resource_stats.vector_geometry_cache_misses,
                    resource_stats.vector_geometry_cache_evictions,
                    resource_stats.vector_geometry_cache_bytes,
                    resource_stats.tessellated_vertices,
                    resource_stats.tessellated_indices,
                    resource_stats.cached_vertices_reused,
                    resource_stats.cached_indices_reused,
                    resource_stats.vector_resource_creates,
                    resource_stats.vector_resource_updates,
                    resource_stats.vector_resource_recreates,
                    resource_stats.vector_resource_create_bytes,
                    resource_stats.vector_resource_update_bytes,
                    resource_stats.vector_resource_cache_hits,
                    resource_stats.vector_resource_cache_misses,
                    resource_stats.vector_resource_cache_evictions,
                    resource_stats.vector_resource_cache_bytes,
                    resource_stats.texture_creates,
                    resource_stats.texture_upload_bytes,
                );
                log::trace!("{}", line);
                #[cfg(not(target_arch = "wasm32"))]
                if should_print_gpu_profile_logs()
                    && (used_targeted_replay
                        || total_us >= 8_000
                        || resource_stats.has_resource_churn())
                {
                    println!("{}", line);
                }
            }
            if resource_stats.has_resource_churn() {
                log::trace!(
                    "[pax-gpu-resources] layer={} surfaces={} targeted={} {:?}",
                    layer,
                    indices.len(),
                    used_targeted_replay,
                    resource_stats
                );
            }
            self.clear_dirty_render_surfaces(layer, &indices);
        }

        used_targeted_replay
    }

    fn clear_targeted_replay_scope(&self, layer: usize) {
        self.surface_replay
            .borrow_mut()
            .clear_targeted_replay_scope(layer);
    }

    fn targeted_or_all_indices(&self, layer: usize, renderer_count: usize) -> Vec<usize> {
        self.surface_replay
            .borrow()
            .targeted_or_all_indices(layer, renderer_count)
    }

    fn remember_canvas_node_coverage(&self, layer: usize, node_id: u32, coverage_bounds: Rect) {
        self.surface_replay
            .borrow_mut()
            .remember_canvas_node_coverage(layer, node_id, coverage_bounds);
    }

    fn forget_canvas_node_coverage(&self, layer: usize, node_id: u32) {
        self.surface_replay
            .borrow_mut()
            .forget_canvas_node_coverage(layer, node_id);
    }

    fn mark_clean_skipped_node(&self, layer: usize, node_id: u32) {
        self.clean_skipped_canvas_nodes
            .borrow_mut()
            .insert((layer, node_id));
    }

    fn targeted_replay_surface_bounds(
        &self,
        layer: usize,
        target: &LayerTarget,
    ) -> Option<Vec<Rect>> {
        self.surface_replay
            .borrow()
            .targeted_replay_surface_bounds(layer, |index| {
                let renderer = target.renderers.get(index)?;
                Some(Rect::new(
                    renderer.origin_x as f64,
                    renderer.origin_y as f64,
                    renderer.origin_x as f64 + renderer.logical_width as f64,
                    renderer.origin_y as f64 + renderer.logical_height as f64,
                ))
            })
    }

    fn spatial_replay_node_ids(&self, layer: usize) -> Option<Vec<u32>> {
        let backends = self.backends.borrow();
        let Some(RenderLayerState::Ready((target, _))) = backends.get(layer) else {
            return None;
        };
        let surface_bounds = self.targeted_replay_surface_bounds(layer, target)?;
        self.surface_replay
            .borrow()
            .spatial_replay_node_ids_for_surface_bounds(layer, &surface_bounds)
    }

    fn take_replay_layer_ids(&mut self) -> Vec<usize> {
        let mut replay_layers = self.replay_layers.borrow_mut();
        let mut replay = std::mem::take(&mut *replay_layers);
        replay.sort_unstable();
        replay.dedup();
        replay
    }

    #[cfg(debug_assertions)]
    fn update_tile_cull_stats(&self, layer: usize, update: impl FnOnce(&mut TileCullStats)) {
        let mut stats = self.tile_cull_stats.borrow_mut();
        if stats.len() <= layer {
            stats.resize_with(layer + 1, TileCullStats::default);
        }
        update(&mut stats[layer]);
    }

    #[cfg(debug_assertions)]
    fn emit_tile_cull_stats(&self, layer: usize) {
        let stats = self
            .tile_cull_stats
            .borrow_mut()
            .get_mut(layer)
            .map(std::mem::take);
        if let Some(stats) = stats {
            if stats.origin_only_retargets > 0
                || stats.targeted_replay_batches > 0
                || stats.full_layer_replays > 0
                || stats.resize_resets > 0
            {
                let line = format!(
                    "[pax-tile-cull] layer={} nodes={} selected_surfaces={} skipped_surfaces={} stale_removal_attempts={} origin_only_retargets={} targeted_replay_batches={} targeted_replay_surfaces={} visible_batches={} warm_batches={} visible_surfaces={} warm_surfaces={} full_layer_replays={} resize_resets={}",
                    layer,
                    stats.nodes_considered,
                    stats.selected_surfaces,
                    stats.skipped_surfaces,
                    stats.stale_surface_removal_attempts,
                    stats.origin_only_retargets,
                    stats.targeted_replay_batches,
                    stats.targeted_replay_surfaces,
                    stats.targeted_replay_visible_batches,
                    stats.targeted_replay_warm_batches,
                    stats.targeted_replay_visible_surfaces,
                    stats.targeted_replay_warm_surfaces,
                    stats.full_layer_replays,
                    stats.resize_resets
                );
                log::trace!("{}", line);
                #[cfg(not(target_arch = "wasm32"))]
                if should_print_gpu_profile_logs() {
                    println!("{}", line);
                }
            }
        }
    }

    fn schedule_layer_initialization(&self) {
        pump_layer_initialization_queue(
            Rc::clone(&self.layer_factory),
            Rc::clone(&self.backends),
            Rc::clone(&self.pending_layer_initializations),
            Rc::clone(&self.scheduled_layer_initializations),
            Rc::clone(&self.layer_initializations_in_flight),
            Rc::clone(&self.ready_layers),
        );
    }

    fn refresh_layer_layouts<I>(&mut self, layer_indices: I)
    where
        I: IntoIterator<Item = usize>,
    {
        let mut needs_reinitialization = false;
        let mut backends = self.backends.borrow_mut();
        for layer_index in layer_indices {
            let Some(backend) = backends.get_mut(layer_index) else {
                continue;
            };
            match backend {
                RenderLayerState::Pending => {}
                RenderLayerState::Failed => {}
                RenderLayerState::Ready((target, layout_provider)) => {
                    let layout = (layout_provider)();
                    let layout_matches = layer_layout_matches_target(target, &layout);

                    if layout.active {
                        target.activate();
                    } else {
                        target.deactivate();
                        self.clear_targeted_replay_scope(layer_index);
                    }

                    if !layout_matches {
                        // A different keyed surface set means the DOM host really changed shape
                        // underneath us. Stable slot keys let ordinary scroll slide tile origins in
                        // place; reserve full reinitialization for real additions/removals.
                        self.clear_targeted_replay_scope(layer_index);
                        *backend = RenderLayerState::Pending;
                        self.queue_layer_initialization(layer_index);
                        needs_reinitialization = true;
                        continue;
                    }

                    let mut targeted_replay_entries = Vec::new();
                    let mut targeted_replay_bounds = HashMap::new();
                    let mut needs_full_layer_replay = false;
                    #[cfg(debug_assertions)]
                    {
                        let previous_surface_bounds: Vec<_> = target
                            .renderers
                            .iter()
                            .map(LayerRenderer::coverage_bounds)
                            .collect();
                        if let Some(escape) =
                            visible_surface_escape(&previous_surface_bounds, &layout.surfaces)
                        {
                            log_visible_surface_escape(layer_index, escape);
                        }
                    }
                    for (index, (surface, renderer)) in layout
                        .surfaces
                        .iter()
                        .zip(target.renderers.iter_mut())
                        .enumerate()
                    {
                        let previous_bounds = renderer.coverage_bounds();
                        let layout_change = renderer.update_layout(surface);
                        let current_bounds = renderer.coverage_bounds();
                        match layout_change {
                            LayoutChangeKind::Unchanged => {}
                            LayoutChangeKind::OriginOnly => {
                                targeted_replay_entries.push(ReplayPriorityEntry::new(
                                    index,
                                    surface.replay_priority,
                                    previous_bounds,
                                    current_bounds,
                                ));
                                // Retained nodes carry transforms/resources built against the
                                // previous surface origin, so an origin retarget must drop the
                                // old scene before replaying nodes into the new tile.
                                targeted_replay_bounds
                                    .insert(index, vec![previous_bounds, current_bounds]);
                                renderer.renderer.reset_retained_scene();
                                renderer
                                    .renderer
                                    .set_surface_transform(Transform2D::from_array([
                                        1.0,
                                        0.0,
                                        0.0,
                                        1.0,
                                        -surface.origin_x,
                                        -surface.origin_y,
                                    ]));
                                self.replay_layers.borrow_mut().push(layer_index);
                            }
                            LayoutChangeKind::Resized => {
                                #[cfg(debug_assertions)]
                                self.update_tile_cull_stats(layer_index, |stats| {
                                    stats.resize_resets += 1;
                                });
                                renderer.renderer.reset_retained_scene();
                                renderer
                                    .renderer
                                    .set_surface_transform(Transform2D::from_array([
                                        1.0,
                                        0.0,
                                        0.0,
                                        1.0,
                                        -surface.origin_x,
                                        -surface.origin_y,
                                    ]));
                                renderer.renderer.resize_surface(
                                    surface.surface.surface_width as f32,
                                    surface.surface.surface_height as f32,
                                );
                                renderer.renderer.set_viewport(
                                    surface.surface.logical_width,
                                    surface.surface.logical_height,
                                    surface.surface.dpr,
                                );
                                needs_full_layer_replay = true;
                            }
                        }
                    }
                    if needs_full_layer_replay {
                        self.clear_targeted_replay_scope(layer_index);
                        #[cfg(debug_assertions)]
                        self.update_tile_cull_stats(layer_index, |stats| {
                            stats.full_layer_replays += 1;
                        });
                        self.mark_render_surfaces_dirty(layer_index, 0..target.renderers.len());
                        self.replay_layers.borrow_mut().push(layer_index);
                    } else if !targeted_replay_entries.is_empty() {
                        let replay_batches =
                            replay_batches_by_directional_priority(&targeted_replay_entries);
                        #[cfg(debug_assertions)]
                        let replay_priority_stats =
                            replay_priority_stats(&targeted_replay_entries, &replay_batches);
                        #[cfg(debug_assertions)]
                        self.update_tile_cull_stats(layer_index, |stats| {
                            stats.origin_only_retargets +=
                                replay_batches.iter().map(Vec::len).sum::<usize>() as u64;
                            stats.targeted_replay_batches += replay_batches.len() as u64;
                            stats.targeted_replay_surfaces +=
                                replay_batches.iter().map(Vec::len).sum::<usize>() as u64;
                            stats.targeted_replay_visible_batches += replay_priority_stats.0 as u64;
                            stats.targeted_replay_warm_batches += replay_priority_stats.1 as u64;
                            stats.targeted_replay_visible_surfaces +=
                                replay_priority_stats.2 as u64;
                            stats.targeted_replay_warm_surfaces += replay_priority_stats.3 as u64;
                        });
                        self.set_targeted_replay_batches(
                            layer_index,
                            replay_batches,
                            targeted_replay_bounds,
                        );
                        self.replay_layers.borrow_mut().push(layer_index);
                    }
                }
            }
        }
        drop(backends);
        if needs_reinitialization {
            self.schedule_layer_initialization();
        }
    }
}

impl RenderContext for PaxGpuRenderer {
    fn fill_with_opacity(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        fill: &pax_runtime_api::Fill,
        opacity: f64,
    ) {
        self.with_layer_context(layer, |context| {
            let bounds = path.bounding_box();
            let path = convert_kurbo_to_lyon_path(&path);
            let fill = to_pax_gpu_fill(fill, bounds, context.current_transform());
            context.fill_path_with_opacity(path, fill, opacity as f32);
        });
    }

    fn fill_with_material_and_opacity(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        fill: &pax_runtime_api::Fill,
        material: &Material,
        opacity: f64,
    ) {
        self.with_layer_context(layer, |context| {
            let bounds = path.bounding_box();
            let path = convert_kurbo_to_lyon_path(&path);
            let fill = to_pax_gpu_fill(fill, bounds, context.current_transform());
            let material = to_pax_gpu_material(material);
            context.fill_path_with_material_and_opacity(path, fill, material, opacity as f32);
        });
    }

    fn fill_with_material_and_opacity_and_smoothing(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        fill: &pax_runtime_api::Fill,
        material: &Material,
        opacity: f64,
        smoothing: PathSmoothing,
    ) {
        self.with_layer_context(layer, |context| {
            let bounds = path.bounding_box();
            let path = convert_kurbo_to_lyon_path(&path);
            let fill = to_pax_gpu_fill(fill, bounds, context.current_transform());
            let material = to_pax_gpu_material(material);
            context.fill_path_with_material_and_opacity_and_smoothing(
                path,
                fill,
                material,
                opacity as f32,
                smoothing,
            );
        });
    }

    fn stroke_with_opacity(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        stroke: &Stroke,
        opacity: f64,
    ) {
        self.with_layer_context(layer, |context| {
            let bounds = path.bounding_box();
            context.stroke_path_with_opacity(
                convert_kurbo_to_lyon_path(&path),
                PixelStroke {
                    fill: to_pax_gpu_fill(
                        &pax_runtime_api::Fill::Solid(stroke.color.get()),
                        bounds,
                        context.current_transform(),
                    ),
                    weight: stroke.width.get().expect_pixels().to_float() as f32,
                    cap: match stroke.cap.get() {
                        StrokeCap::Butt => PixelStrokeCap::Butt,
                        StrokeCap::Round => PixelStrokeCap::Round,
                        StrokeCap::Square => PixelStrokeCap::Square,
                    },
                    join: match stroke.join.get() {
                        StrokeJoin::Miter => PixelStrokeJoin::Miter,
                        StrokeJoin::Round => PixelStrokeJoin::Round,
                        StrokeJoin::Bevel => PixelStrokeJoin::Bevel,
                    },
                },
                opacity as f32,
            );
        });
    }

    fn stroke_with_material_and_opacity(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        stroke: &Stroke,
        material: &Material,
        opacity: f64,
    ) {
        self.with_layer_context(layer, |context| {
            let bounds = path.bounding_box();
            context.stroke_path_with_material_and_opacity(
                convert_kurbo_to_lyon_path(&path),
                PixelStroke {
                    fill: to_pax_gpu_fill(
                        &pax_runtime_api::Fill::Solid(stroke.color.get()),
                        bounds,
                        context.current_transform(),
                    ),
                    weight: stroke.width.get().expect_pixels().to_float() as f32,
                    cap: match stroke.cap.get() {
                        StrokeCap::Butt => PixelStrokeCap::Butt,
                        StrokeCap::Round => PixelStrokeCap::Round,
                        StrokeCap::Square => PixelStrokeCap::Square,
                    },
                    join: match stroke.join.get() {
                        StrokeJoin::Miter => PixelStrokeJoin::Miter,
                        StrokeJoin::Round => PixelStrokeJoin::Round,
                        StrokeJoin::Bevel => PixelStrokeJoin::Bevel,
                    },
                },
                to_pax_gpu_material(material),
                opacity as f32,
            );
        });
    }

    fn stroke_with_material_and_opacity_and_smoothing(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        stroke: &Stroke,
        material: &Material,
        opacity: f64,
        smoothing: PathSmoothing,
    ) {
        self.with_layer_context(layer, |context| {
            let bounds = path.bounding_box();
            context.stroke_path_with_material_and_opacity_and_smoothing(
                convert_kurbo_to_lyon_path(&path),
                PixelStroke {
                    fill: to_pax_gpu_fill(
                        &pax_runtime_api::Fill::Solid(stroke.color.get()),
                        bounds,
                        context.current_transform(),
                    ),
                    weight: stroke.width.get().expect_pixels().to_float() as f32,
                    cap: match stroke.cap.get() {
                        StrokeCap::Butt => PixelStrokeCap::Butt,
                        StrokeCap::Round => PixelStrokeCap::Round,
                        StrokeCap::Square => PixelStrokeCap::Square,
                    },
                    join: match stroke.join.get() {
                        StrokeJoin::Miter => PixelStrokeJoin::Miter,
                        StrokeJoin::Round => PixelStrokeJoin::Round,
                        StrokeJoin::Bevel => PixelStrokeJoin::Bevel,
                    },
                },
                to_pax_gpu_material(material),
                opacity as f32,
                smoothing,
            );
        });
    }

    fn stroke_with_draw_range_and_material_and_opacity(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        stroke: &Stroke,
        material: &Material,
        opacity: f64,
        draw_start: f64,
        draw_end: f64,
    ) {
        self.with_layer_context(layer, |context| {
            let bounds = path.bounding_box();
            context.stroke_path_with_draw_range_and_material_and_opacity(
                convert_kurbo_to_lyon_path(&path),
                PixelStroke {
                    fill: to_pax_gpu_fill(
                        &pax_runtime_api::Fill::Solid(stroke.color.get()),
                        bounds,
                        context.current_transform(),
                    ),
                    weight: stroke.width.get().expect_pixels().to_float() as f32,
                    cap: match stroke.cap.get() {
                        StrokeCap::Butt => PixelStrokeCap::Butt,
                        StrokeCap::Round => PixelStrokeCap::Round,
                        StrokeCap::Square => PixelStrokeCap::Square,
                    },
                    join: match stroke.join.get() {
                        StrokeJoin::Miter => PixelStrokeJoin::Miter,
                        StrokeJoin::Round => PixelStrokeJoin::Round,
                        StrokeJoin::Bevel => PixelStrokeJoin::Bevel,
                    },
                },
                to_pax_gpu_material(material),
                opacity as f32,
                PixelDrawRange::enabled(draw_start as f32, draw_end as f32),
            );
        });
    }

    fn stroke_with_draw_range_and_material_and_opacity_and_smoothing(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        stroke: &Stroke,
        material: &Material,
        opacity: f64,
        draw_start: f64,
        draw_end: f64,
        smoothing: PathSmoothing,
    ) {
        self.with_layer_context(layer, |context| {
            let bounds = path.bounding_box();
            context.stroke_path_with_draw_range_and_material_and_opacity_and_smoothing(
                convert_kurbo_to_lyon_path(&path),
                PixelStroke {
                    fill: to_pax_gpu_fill(
                        &pax_runtime_api::Fill::Solid(stroke.color.get()),
                        bounds,
                        context.current_transform(),
                    ),
                    weight: stroke.width.get().expect_pixels().to_float() as f32,
                    cap: match stroke.cap.get() {
                        StrokeCap::Butt => PixelStrokeCap::Butt,
                        StrokeCap::Round => PixelStrokeCap::Round,
                        StrokeCap::Square => PixelStrokeCap::Square,
                    },
                    join: match stroke.join.get() {
                        StrokeJoin::Miter => PixelStrokeJoin::Miter,
                        StrokeJoin::Round => PixelStrokeJoin::Round,
                        StrokeJoin::Bevel => PixelStrokeJoin::Bevel,
                    },
                },
                to_pax_gpu_material(material),
                opacity as f32,
                PixelDrawRange::enabled(draw_start as f32, draw_end as f32),
                smoothing,
            );
        });
    }

    fn save(&mut self, layer: usize) {
        self.with_layer_context(layer, |context| {
            context.save();
        });
    }
    fn restore(&mut self, layer: usize) {
        self.with_layer_context(layer, |context| {
            context.restore();
        });
    }
    fn clip(&mut self, layer: usize, path: kurbo::BezPath) {
        self.with_layer_context(layer, |context| {
            let path = convert_kurbo_to_lyon_path(&path);
            context.clip(path);
        });
    }
    fn clip_alpha(
        &mut self,
        layer: usize,
        paints: &[pax_runtime_api::AlphaMaskPaint],
        feather: f64,
    ) {
        self.with_layer_context(layer, |context| {
            let paints = paints
                .iter()
                .map(|paint| {
                    let transform =
                        Transform2D::from_array(paint.transform.as_coeffs().map(|v| v as f32));
                    pax_gpu::AlphaMaskPaint {
                        path: convert_kurbo_to_lyon_path(&paint.path),
                        fill: to_pax_gpu_alpha_fill(
                            &paint.fill,
                            paint.path.bounding_box(),
                            transform.then(&context.current_transform()),
                        ),
                        transform,
                        opacity: paint.opacity.clamp(0.0, 1.0) as f32,
                    }
                })
                .collect();
            context.clip_alpha(paints, feather.max(0.0) as f32);
        });
    }

    fn transform(&mut self, layer: usize, affine: kurbo::Affine) {
        self.with_layer_context(layer, |context| {
            context.transform(Transform2D::from_array(
                affine.as_coeffs().map(|v| v as f32),
            ))
        });
    }

    fn load_image(&mut self, identifier: &str, image: &[u8], width: usize, height: usize) {
        self.image_map.insert(
            identifier.to_string(),
            Image {
                rgba: image.into(),
                pixel_width: width as u32,
                pixel_height: height as u32,
            },
        );
        *self
            .image_versions
            .entry(identifier.to_string())
            .or_insert(0) += 1;
    }

    fn draw_image_with_opacity(
        &mut self,
        layer: usize,
        image_path: &str,
        rect: kurbo::Rect,
        opacity: f64,
    ) {
        self.with_layer_context(layer, |context| {
            if let Some(image) = self.image_map.get(image_path) {
                let version = *self.image_versions.get(image_path).unwrap_or(&0);
                context.draw_image_with_opacity(
                    image_path,
                    version,
                    image,
                    Box2D {
                        min: point(rect.x0 as f32, rect.y0 as f32),
                        max: point(rect.x1 as f32, rect.y1 as f32),
                    },
                    opacity as f32,
                );
            }
        });
    }

    fn get_image_size(&mut self, image_path: &str) -> Option<(usize, usize)> {
        self.image_map
            .get(image_path)
            .map(|img| (img.pixel_width as usize, img.pixel_height as usize))
    }

    fn image_loaded(&self, image_path: &str) -> bool {
        self.image_map.contains_key(image_path)
    }

    fn layers(&self) -> usize {
        self.backends.borrow().len()
    }

    fn resize_layers_to(&mut self, layer_count: usize, _dirty_canvases: Rc<RefCell<Vec<bool>>>) {
        let current_len = self.backends.borrow().len();
        match layer_count.cmp(&current_len) {
            std::cmp::Ordering::Less => {
                self.backends.borrow_mut().truncate(layer_count);
            }
            std::cmp::Ordering::Equal => return,
            std::cmp::Ordering::Greater => {
                for i in current_len..layer_count {
                    self.backends.borrow_mut().push(RenderLayerState::Pending);
                    self.enqueue_layer_initialization(i);
                }
            }
        }
    }

    fn clear(&mut self, layer: usize) {
        let mut backends = self.backends.borrow_mut();
        match backends.get_mut(layer) {
            Some(RenderLayerState::Pending) => {
                let mut failed_context_gets = self.failed_context_gets.borrow_mut();
                if failed_context_gets.len() <= layer {
                    failed_context_gets.resize(layer + 1, false);
                }
                failed_context_gets[layer] = true;
            }
            Some(RenderLayerState::Failed) => {}
            Some(RenderLayerState::Ready((target, _))) => {
                if !target.active {
                    return;
                }
                target.prepare_for_render();
                let indices = self.targeted_or_all_indices(layer, target.renderers.len());
                for index in &indices {
                    if let Some(renderer) = target.renderers.get_mut(*index) {
                        renderer.renderer.clear();
                    }
                }
                self.mark_render_surfaces_dirty(layer, indices);
            }
            None => log::warn!(
                "tried to clear layer {} context for non-existent layer",
                layer
            ),
        }
    }

    fn flush(&mut self, layer: usize, dirty_canvases: Rc<RefCell<Vec<bool>>>) {
        #[cfg(debug_assertions)]
        self.emit_tile_cull_stats(layer);

        let mut flushed_targeted_batch = false;
        let mut backends = self.backends.borrow_mut();
        match backends.get_mut(layer) {
            Some(RenderLayerState::Pending) => {}
            Some(RenderLayerState::Failed) => {}
            Some(RenderLayerState::Ready((target, _))) => {
                if !target.active {
                    return;
                }
                if let Some(failed) = self.failed_context_gets.borrow_mut().get_mut(layer) {
                    if *failed {
                        if let Some(dirty_bit) = dirty_canvases.borrow_mut().get_mut(layer) {
                            // If we failed to draw to this layer because the context wasn't
                            // available yet, retry once the backend is ready.
                            *dirty_bit = true;
                        }
                        *failed = false;
                    }
                }
                flushed_targeted_batch = self.flush_targeted_or_dirty(layer, target);
            }
            None => log::warn!(
                "tried to flush layer {} context for non-existent layer",
                layer
            ),
        }
        drop(backends);

        if flushed_targeted_batch && self.advance_targeted_replay_queue(layer) {
            self.replay_layers.borrow_mut().push(layer);
        }
    }

    fn resize(&mut self, _width: usize, _height: usize) {
        let layer_count = self.backends.borrow().len();
        self.refresh_layer_layouts(0..layer_count);
    }

    fn refresh_layers(&mut self, layers: &[usize]) {
        self.refresh_layer_layouts(layers.iter().copied());
    }

    fn set_scene_lighting(&mut self, layer: usize, lighting: &SceneLighting) {
        let lighting = to_pax_gpu_scene_lighting(lighting);
        let lighting_changed = self.update_scene_lighting_cache(layer, &lighting);
        let mut backends = self.backends.borrow_mut();
        match backends.get_mut(layer) {
            Some(RenderLayerState::Pending) => {}
            Some(RenderLayerState::Failed) => {}
            Some(RenderLayerState::Ready((target, _))) => {
                if !target.active {
                    return;
                }
                if lighting_changed {
                    self.mark_render_surfaces_dirty(layer, 0..target.renderers.len());
                }
                for renderer in &mut target.renderers {
                    renderer
                        .renderer
                        .set_scene_lighting(translate_scene_lighting_for_surface(
                            &lighting,
                            renderer.origin_x,
                            renderer.origin_y,
                        ));
                }
            }
            None => log::warn!(
                "tried to set lighting for layer {} context for non-existent layer",
                layer
            ),
        }
    }

    fn clear_targeted_replay(&mut self, layer: usize) {
        self.clear_targeted_replay_scope(layer);
    }

    fn take_ready_canvas_layers(&mut self) -> Vec<usize> {
        let mut ready_layers = self.ready_layers.borrow_mut();
        let mut ready = std::mem::take(&mut *ready_layers);
        drop(ready_layers);
        ready.sort_unstable();
        ready.dedup();
        if !ready.is_empty() {
            // A DOM surface update may have been routed through the engine while this layer was
            // still bootstrapping. Reconcile the just-published renderer against the current
            // layout before the chassis asks the runtime to replay retained canvas nodes into it.
            self.refresh_layer_layouts(ready.iter().copied());
        }
        ready
    }

    fn take_replay_canvas_layers(&mut self) -> Vec<usize> {
        self.take_replay_layer_ids()
    }

    fn take_replay_canvas_layer_updates(&mut self) -> Vec<ReplayCanvasLayerUpdate> {
        self.take_replay_layer_ids()
            .into_iter()
            .map(|layer| ReplayCanvasLayerUpdate {
                layer,
                node_ids: self.spatial_replay_node_ids(layer),
            })
            .collect()
    }

    fn request_layer_screenshot(&mut self, layer: usize, request_id: u32) {
        self.with_layer_context(layer, |context| {
            context.request_screenshot_capture(request_id);
        });
        self.replay_layers.borrow_mut().push(layer);
    }

    fn take_layer_screenshot(&mut self, layer: usize, request_id: u32) -> Option<ScreenshotData> {
        let mut screenshot = None;
        self.with_layer_context(layer, |context| {
            screenshot =
                context
                    .take_screenshot_capture(request_id)
                    .map(|capture| ScreenshotData {
                        id: request_id,
                        data: capture.rgba,
                        width: capture.width as usize,
                        height: capture.height as usize,
                    });
        });
        screenshot
    }

    fn take_layer_surface_screenshots(
        &mut self,
        layer: usize,
        request_id: u32,
    ) -> Vec<LayerSurfaceScreenshotData> {
        let mut screenshots = Vec::new();
        let mut backends = self.backends.borrow_mut();
        let Some(RenderLayerState::Ready((target, _))) = backends.get_mut(layer) else {
            return screenshots;
        };
        if !target.active {
            return screenshots;
        }

        for renderer in target.renderers_mut() {
            let Some(capture) = renderer.renderer_mut().take_screenshot_capture(request_id) else {
                continue;
            };
            screenshots.push(LayerSurfaceScreenshotData {
                id: request_id,
                key: renderer.key.clone(),
                data: capture.rgba,
                width: capture.width as usize,
                height: capture.height as usize,
                origin_x: renderer.origin_x,
                origin_y: renderer.origin_y,
                logical_width: renderer.logical_width,
                logical_height: renderer.logical_height,
            });
        }
        screenshots
    }

    fn supports_subtree_opacity(&self) -> bool {
        true
    }

    fn set_node_opacity_scopes(
        &mut self,
        layer: usize,
        node_id: u32,
        scopes: &[pax_runtime_api::OpacityScope],
    ) {
        self.with_layer_context(layer, |context| {
            context.set_node_opacity_scopes(node_id, scopes)
        });
    }

    fn begin_node(&mut self, layer: usize, node_id: u32, z_index: i32, light_mask: u32) -> bool {
        let mut backends = self.backends.borrow_mut();
        match backends.get_mut(layer) {
            Some(RenderLayerState::Pending) => {
                let mut failed_context_gets = self.failed_context_gets.borrow_mut();
                if failed_context_gets.len() <= layer {
                    failed_context_gets.resize(layer + 1, false);
                }
                failed_context_gets[layer] = true;
                false
            }
            Some(RenderLayerState::Failed) => false,
            Some(RenderLayerState::Ready((target, _))) => {
                if !target.active {
                    return false;
                }
                self.remember_canvas_node_coverage(
                    layer,
                    node_id,
                    Rect::new(
                        f64::NEG_INFINITY,
                        f64::NEG_INFINITY,
                        f64::INFINITY,
                        f64::INFINITY,
                    ),
                );
                target.prepare_for_render();
                let candidate_indices = self.targeted_or_all_indices(layer, target.renderers.len());
                let mut selected = Vec::new();
                for index in candidate_indices {
                    let Some(renderer) = target.renderers.get_mut(index) else {
                        continue;
                    };
                    if renderer.renderer.begin_node(node_id, z_index, light_mask) {
                        selected.push(index);
                    }
                }
                let began = !selected.is_empty();
                if began {
                    self.mark_render_surfaces_dirty(layer, selected.iter().copied());
                    self.push_render_scope(layer, selected);
                }
                began
            }
            None => {
                log::warn!(
                    "tried to retrieve layer {} context for non-existent layer",
                    layer
                );
                false
            }
        }
    }

    fn begin_node_with_bounds(
        &mut self,
        layer: usize,
        node_id: u32,
        z_index: i32,
        coverage_bounds: Rect,
        light_mask: u32,
    ) -> bool {
        let mut backends = self.backends.borrow_mut();
        match backends.get_mut(layer) {
            Some(RenderLayerState::Pending) => {
                let mut failed_context_gets = self.failed_context_gets.borrow_mut();
                if failed_context_gets.len() <= layer {
                    failed_context_gets.resize(layer + 1, false);
                }
                failed_context_gets[layer] = true;
                false
            }
            Some(RenderLayerState::Failed) => false,
            Some(RenderLayerState::Ready((target, _))) => {
                if !target.active {
                    return false;
                }
                self.remember_canvas_node_coverage(layer, node_id, coverage_bounds);
                target.prepare_for_render();
                let renderer_count = target.renderers.len();
                let candidate_indices = self.targeted_or_all_indices(layer, renderer_count);
                let candidate_indices: HashSet<_> = candidate_indices.into_iter().collect();
                #[cfg(debug_assertions)]
                self.update_tile_cull_stats(layer, |stats| {
                    stats.nodes_considered += 1;
                });
                let mut selected = Vec::new();
                let mut removed = Vec::new();
                #[cfg(debug_assertions)]
                let mut skipped_surfaces = 0u64;
                #[cfg(debug_assertions)]
                let mut stale_surface_removal_attempts = 0u64;
                for (index, renderer) in target.renderers.iter_mut().enumerate() {
                    let intersects = renderer.intersects_coverage_bounds(&coverage_bounds);
                    if !intersects {
                        #[cfg(debug_assertions)]
                        {
                            skipped_surfaces += 1;
                            stale_surface_removal_attempts += 1;
                        }
                        // If a dirty node moved out of this tile, skipping begin_node is not
                        // enough: the renderer may still retain that node from an earlier frame.
                        if renderer.renderer.remove_node(node_id) {
                            removed.push(index);
                        }
                        continue;
                    }
                    if !candidate_indices.contains(&index) {
                        #[cfg(debug_assertions)]
                        {
                            skipped_surfaces += 1;
                        }
                        continue;
                    }
                    if renderer.renderer.begin_node(node_id, z_index, light_mask) {
                        selected.push(index);
                    }
                }
                let began = !selected.is_empty();
                self.mark_render_surfaces_dirty(
                    layer,
                    selected.iter().chain(removed.iter()).copied(),
                );
                #[cfg(debug_assertions)]
                self.update_tile_cull_stats(layer, |stats| {
                    let selected_count = selected.len() as u64;
                    stats.selected_surfaces += selected_count;
                    stats.skipped_surfaces += skipped_surfaces;
                    stats.stale_surface_removal_attempts += stale_surface_removal_attempts;
                });
                if began {
                    self.push_render_scope(layer, selected);
                } else {
                    self.mark_clean_skipped_node(layer, node_id);
                }
                began
            }
            None => {
                log::warn!(
                    "tried to retrieve layer {} context for non-existent layer",
                    layer
                );
                false
            }
        }
    }

    fn take_clean_skipped_node(&mut self, layer: usize, node_id: u32) -> bool {
        self.clean_skipped_canvas_nodes
            .borrow_mut()
            .remove(&(layer, node_id))
    }

    fn end_node(&mut self, layer: usize, node_id: u32) -> bool {
        let mut ended = false;
        self.with_layer_context(layer, |context| {
            ended = context.end_node(node_id);
        });
        self.pop_render_scope(layer);
        ended
    }

    fn remove_node(&mut self, layer: usize, node_id: u32) -> bool {
        let mut backends = self.backends.borrow_mut();
        match backends.get_mut(layer) {
            Some(RenderLayerState::Pending) => {
                let mut failed_context_gets = self.failed_context_gets.borrow_mut();
                if failed_context_gets.len() <= layer {
                    failed_context_gets.resize(layer + 1, false);
                }
                failed_context_gets[layer] = true;
                false
            }
            Some(RenderLayerState::Failed) => false,
            Some(RenderLayerState::Ready((target, _))) => {
                if !target.active {
                    return false;
                }
                self.forget_canvas_node_coverage(layer, node_id);
                target.prepare_for_render();
                let candidate_indices = 0..target.renderers.len();
                let mut removed_indices = Vec::new();
                for index in candidate_indices {
                    let Some(renderer) = target.renderers.get_mut(index) else {
                        continue;
                    };
                    if renderer.renderer.remove_node(node_id) {
                        removed_indices.push(index);
                    }
                }
                let removed = !removed_indices.is_empty();
                if removed {
                    self.mark_render_surfaces_dirty(layer, removed_indices);
                }
                true
            }
            None => {
                log::warn!(
                    "tried to remove node from layer {} context for non-existent layer",
                    layer
                );
                false
            }
        }
    }
}

fn to_pax_gpu_fill(
    fill: &pax_runtime_api::Fill,
    rect: kurbo::Rect,
    transform: pax_gpu::Transform2D,
) -> pax_gpu::Fill {
    let bounds = (rect.width(), rect.height());
    let orig = rect.origin();
    match fill {
        pax_runtime_api::Fill::Blend(terms) => pax_gpu::Fill::Blend(
            terms
                .iter()
                .map(|(fill, weight)| (to_pax_gpu_fill(fill, rect, transform), *weight as f32))
                .collect(),
        ),
        pax_runtime_api::Fill::Solid(color) => pax_gpu::Fill::Solid(to_pax_gpu_color(color)),
        pax_runtime_api::Fill::LinearGradient(gradient) => {
            let start_x = gradient.start.0.evaluate(bounds, Axis::X);
            let start_y = gradient.start.1.evaluate(bounds, Axis::Y);
            let end_x = gradient.end.0.evaluate(bounds, Axis::X);
            let end_y = gradient.end.1.evaluate(bounds, Axis::Y);
            let local_pos =
                pax_gpu::Point2D::new((orig.x + start_x) as f32, (orig.y + start_y) as f32);
            let local_end = pax_gpu::Point2D::new((orig.x + end_x) as f32, (orig.y + end_y) as f32);
            let world_pos = transform.transform_point(local_pos);
            let world_end = transform.transform_point(local_end);
            let main_axis = world_end - world_pos;
            pax_gpu::Fill::Gradient {
                stops: gradient
                    .stops
                    .iter()
                    .map(|g| pax_gpu::GradientStop {
                        color: to_pax_gpu_color(&g.color),
                        stop: g
                            .position
                            .evaluate((main_axis.length() as f64, 0.0), Axis::X)
                            as f32,
                    })
                    .collect(),
                gradient_type: pax_gpu::GradientType::Linear,
                pos: world_pos,
                main_axis,
                off_axis: pax_gpu::Vector2D::zero(), //not used for linear
            }
        }
        pax_runtime_api::Fill::RadialGradient(gradient) => {
            let Some(g) = gradient.resolve_geometry(rect) else {
                return pax_gpu::Fill::Solid(pax_gpu::Color::rgba(0.0, 0.0, 0.0, 0.0));
            };
            let center = point(g.center.x as f32, g.center.y as f32);
            let pos = transform.transform_point(center);
            let main_axis =
                transform.transform_vector(pax_gpu::Vector2D::new(g.radius as f32, 0.0));
            let off_axis = transform.transform_vector(pax_gpu::Vector2D::new(0.0, g.radius as f32));
            let focal_point = point(
                ((g.focal_point.x - g.center.x) / g.radius) as f32,
                ((g.focal_point.y - g.center.y) / g.radius) as f32,
            );
            // Check after f64 -> f32 conversion too; GPU paint must remain finite.
            if ![
                pos.x,
                pos.y,
                main_axis.x,
                main_axis.y,
                off_axis.x,
                off_axis.y,
                focal_point.x,
                focal_point.y,
            ]
            .into_iter()
            .all(f32::is_finite)
            {
                return pax_gpu::Fill::Solid(pax_gpu::Color::rgba(0.0, 0.0, 0.0, 0.0));
            }
            pax_gpu::Fill::Gradient {
                gradient_type: pax_gpu::GradientType::Radial { focal_point },
                pos,
                main_axis,
                off_axis,
                stops: gradient
                    .stops
                    .iter()
                    .map(|s| pax_gpu::GradientStop {
                        color: to_pax_gpu_color(&s.color),
                        stop: (s.position.evaluate((g.radius, 0.0), Axis::X) / g.radius) as f32,
                    })
                    .collect(),
            }
        }
    }
}

fn to_pax_gpu_alpha_fill(
    fill: &pax_runtime_api::Fill,
    rect: Rect,
    transform: Transform2D,
) -> pax_gpu::Fill {
    // Linear masks retain both basis vectors under shear; radial masks use
    // exactly the same focal geometry and normalized stops as color fills.
    let bounds = (rect.width(), rect.height());
    match fill {
        pax_runtime_api::Fill::Blend(terms) => pax_gpu::Fill::Blend(
            terms
                .iter()
                .map(|(fill, weight)| {
                    (to_pax_gpu_alpha_fill(fill, rect, transform), *weight as f32)
                })
                .collect(),
        ),
        pax_runtime_api::Fill::LinearGradient(g) => {
            let mut resolved = to_pax_gpu_fill(fill, rect, transform);
            if let pax_gpu::Fill::Gradient { off_axis, .. } = &mut resolved {
                let dx = (g.end.0.evaluate(bounds, Axis::X) - g.start.0.evaluate(bounds, Axis::X))
                    as f32;
                let dy = (g.end.1.evaluate(bounds, Axis::Y) - g.start.1.evaluate(bounds, Axis::Y))
                    as f32;
                *off_axis = transform.transform_vector(pax_gpu::Vector2D::new(-dy, dx));
            }
            resolved
        }
        _ => to_pax_gpu_fill(fill, rect, transform),
    }
}

fn to_pax_gpu_material(material: &pax_runtime_api::Material) -> PixelMaterial {
    match material {
        pax_runtime_api::Material::Unlit => PixelMaterial::unlit(),
        pax_runtime_api::Material::Lit(params) => {
            let metallic = params.metallic.get().clamp(0.0, 1.0) as f32;
            let diffuse = params.diffuse.get().max(0.0) as f32 * (1.0 - metallic * 0.55);
            let specular = params.specular.get().max(0.0) as f32 + metallic * 0.45;
            let roughness = (params.roughness.get().clamp(0.0, 1.0) as f32
                * (1.0 - metallic * 0.35))
                .clamp(0.0, 1.0);
            PixelMaterial::custom(
                params.ambient.get().max(0.0) as f32,
                diffuse,
                specular,
                roughness,
                to_pax_gpu_color(&params.emissive.get()),
                params.emissive_intensity.get().max(0.0) as f32,
            )
        }
    }
}

fn to_pax_gpu_scene_lighting(lighting: &SceneLighting) -> PixelSceneLighting {
    PixelSceneLighting {
        active: lighting.active,
        ambient_is_authored: lighting.ambient_is_authored,
        ambient_color: to_pax_gpu_color(&lighting.ambient.color),
        ambient_intensity: lighting.ambient.intensity.max(0.0) as f32,
        lights: lighting
            .lights
            .iter()
            .map(|light| PixelSceneLight {
                shape: match light.shape {
                    pax_runtime_api::LightShape::Point => PixelLightShape::Point,
                    pax_runtime_api::LightShape::Directional => PixelLightShape::Directional,
                },
                position: [
                    light.position.x as f32,
                    light.position.y as f32,
                    light.position.z as f32,
                ],
                direction: [
                    light.direction.x as f32,
                    light.direction.y as f32,
                    light.direction.z as f32,
                ],
                color: to_pax_gpu_color(&light.color),
                intensity: light.intensity.max(0.0) as f32,
                radius: light.radius.max(0.0) as f32,
            })
            .collect(),
    }
}

fn translate_scene_lighting_for_surface(
    lighting: &PixelSceneLighting,
    origin_x: f32,
    origin_y: f32,
) -> PixelSceneLighting {
    let mut lighting = lighting.clone();
    for light in &mut lighting.lights {
        if matches!(light.shape, PixelLightShape::Point) {
            light.position[0] -= origin_x;
            light.position[1] -= origin_y;
        }
    }
    lighting
}

/// Convert a runtime API color into the `pax-gpu` render-context color.
pub fn to_pax_gpu_color(color: &pax_runtime_api::Color) -> pax_gpu::Color {
    let [r, g, b, a] = color.to_rgba_0_1();
    pax_gpu::Color::rgba(r as f32, g as f32, b as f32, a as f32)
}

/// Convert a kurbo path emitted by primitives into a lyon path consumed by `pax-gpu`.
pub fn convert_kurbo_to_lyon_path(kurbo_path: &BezPath) -> Path {
    let mut builder = Path::builder();
    let mut has_open_subpath = false;
    for el in kurbo_path.elements() {
        match el {
            PathEl::MoveTo(p) => {
                if has_open_subpath {
                    builder.end(false);
                }
                builder.begin(point(p.x as f32, p.y as f32));
                has_open_subpath = true;
            }
            PathEl::LineTo(p) => {
                if has_open_subpath {
                    builder.line_to(point(p.x as f32, p.y as f32));
                }
            }
            PathEl::QuadTo(p1, p2) => {
                if has_open_subpath {
                    builder.quadratic_bezier_to(
                        point(p1.x as f32, p1.y as f32),
                        point(p2.x as f32, p2.y as f32),
                    );
                }
            }
            PathEl::CurveTo(p1, p2, p3) => {
                if has_open_subpath {
                    builder.cubic_bezier_to(
                        point(p1.x as f32, p1.y as f32),
                        point(p2.x as f32, p2.y as f32),
                        point(p3.x as f32, p3.y as f32),
                    );
                }
            }
            PathEl::ClosePath => {
                if has_open_subpath {
                    builder.end(true);
                    has_open_subpath = false;
                }
            }
        }
    }
    if has_open_subpath {
        builder.end(false);
    }

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_radial_gradient_retains_radius_with_coincident_endpoints() {
        use pax_runtime_api::{Fill, RadialGradient, Size};
        let fill = Fill::RadialGradient(RadialGradient {
            start: (Size::Percent(50.into()), Size::Percent(50.into())),
            end: (Size::Percent(50.into()), Size::Percent(50.into())),
            radius: 25.0,
            stops: vec![],
        });
        let pax_gpu::Fill::Gradient {
            pos,
            main_axis,
            off_axis,
            ..
        } = to_pax_gpu_alpha_fill(
            &fill,
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Transform2D::new(2.0, 0.0, 1.0, 3.0, 0.0, 0.0),
        )
        else {
            panic!("expected a radial gradient")
        };
        assert_eq!(pos, point(150.0, 150.0));
        assert_eq!(main_axis, pax_gpu::Vector2D::new(50.0, 0.0));
        assert_eq!(off_axis, pax_gpu::Vector2D::new(25.0, 75.0));
    }

    #[test]
    fn radial_color_and_mask_share_focal_geometry_and_local_pixel_stops() {
        use pax_runtime_api::{Color, Fill, GradientStop, RadialGradient, Size};
        let fill = Fill::RadialGradient(RadialGradient {
            start: (Size::Percent(75.into()), Size::Percent(25.into())),
            end: (Size::Percent(50.into()), Size::Percent(50.into())),
            radius: 50.0,
            stops: vec![
                GradientStop::get(Color::BLACK, Size::Pixels(25.into())),
                GradientStop::get(Color::WHITE, Size::Percent(100.into())),
            ],
        });
        let rect = Rect::new(10.0, 20.0, 210.0, 120.0);
        let transform = Transform2D::new(-2.0, 0.0, 1.0, 3.0, 5.0, 7.0);
        for paint in [
            to_pax_gpu_fill(&fill, rect, transform),
            to_pax_gpu_alpha_fill(&fill, rect, transform),
        ] {
            let pax_gpu::Fill::Gradient {
                gradient_type,
                pos,
                main_axis,
                off_axis,
                stops,
            } = paint
            else {
                panic!("expected radial paint")
            };
            let pax_gpu::GradientType::Radial { focal_point } = gradient_type else {
                panic!("radial")
            };
            assert_eq!(focal_point, point(1.0, -0.5));
            assert_eq!(pos, point(-145.0, 217.0));
            assert_eq!(main_axis, pax_gpu::Vector2D::new(-100.0, 0.0));
            assert_eq!(off_axis, pax_gpu::Vector2D::new(50.0, 150.0));
            assert_eq!(
                stops.iter().map(|s| s.stop).collect::<Vec<_>>(),
                vec![0.5, 1.0]
            );
        }
    }

    #[test]
    fn alpha_linear_gradient_retains_sheared_basis() {
        use pax_runtime_api::{Fill, LinearGradient, Size};
        let fill = Fill::LinearGradient(LinearGradient {
            start: (Size::Percent(0.into()), Size::Percent(0.into())),
            end: (Size::Percent(100.into()), Size::Percent(0.into())),
            stops: vec![],
        });
        let pax_gpu::Fill::Gradient {
            main_axis,
            off_axis,
            ..
        } = to_pax_gpu_alpha_fill(
            &fill,
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Transform2D::new(1.0, 0.0, 0.5, 1.0, 0.0, 0.0),
        )
        else {
            panic!("expected a linear gradient")
        };
        assert_eq!(main_axis, pax_gpu::Vector2D::new(100.0, 0.0));
        assert_eq!(off_axis, pax_gpu::Vector2D::new(50.0, 100.0));
    }

    #[test]
    fn surface_intersection_includes_edges() {
        let bounds = Rect::new(100.0, 100.0, 200.0, 200.0);

        assert!(surface_intersects_coverage_bounds(
            &bounds, 200.0, 100.0, 100.0, 100.0
        ));
        assert!(surface_intersects_coverage_bounds(
            &bounds, 0.0, 0.0, 100.0, 100.0
        ));
    }

    #[test]
    fn surface_intersection_rejects_disjoint_surfaces() {
        let bounds = Rect::new(100.0, 100.0, 200.0, 200.0);

        assert!(!surface_intersects_coverage_bounds(
            &bounds, 201.0, 100.0, 100.0, 100.0
        ));
        assert!(!surface_intersects_coverage_bounds(
            &bounds, 100.0, 201.0, 100.0, 100.0
        ));
    }

    #[test]
    fn surface_intersection_keeps_non_finite_bounds_conservative() {
        let bounds = Rect::new(f64::NAN, 0.0, 100.0, 100.0);

        assert!(surface_intersects_coverage_bounds(
            &bounds, 500.0, 500.0, 100.0, 100.0
        ));
    }

    #[test]
    fn converts_multiple_kurbo_subpaths_to_lyon_path() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((10.0, 0.0));
        path.move_to((20.0, 0.0));
        path.line_to((30.0, 0.0));
        path.close_path();

        let lyon_path = convert_kurbo_to_lyon_path(&path);

        assert_eq!(lyon_path.iter().count(), 6);
    }
}
