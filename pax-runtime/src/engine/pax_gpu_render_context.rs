use super::layer_surface::{
    replay_batches_by_directional_priority, surface_intersects_coverage_bounds, LayerSurfaceEntry,
    LayerSurfaceLayout, LayoutChangeKind, ReplayPriorityEntry, SurfaceReplayCoordinator,
};
#[cfg(debug_assertions)]
use super::layer_surface::{visible_surface_escape, VisibleSurfaceEscape};
use super::recorded_drawing::{Command as DrawingCommand, Drawing, DrawingCache};
use crate::render_instrumentation::{count, Counter};
use kurbo::{BezPath, PathEl, Rect, Shape};
use pax_gpu::render_backend::SharedGpuContext;
use pax_gpu::{
    point, DrawRange as PixelDrawRange, Image, LightShape as PixelLightShape,
    Material as PixelMaterial, Path, ResourceChurnStats, SceneLight as PixelSceneLight,
    SceneLighting as PixelSceneLighting, StrokeCap as PixelStrokeCap,
    StrokeJoin as PixelStrokeJoin, Transform2D, WgpuRenderer,
};
use pax_runtime_api::{
    Axis, LayerSurfaceScreenshotData, PathSmoothing, RenderContext, ReplayCanvasLayerUpdate,
    ResolvedMaterial, ResolvedStroke, SceneLighting, ScreenshotData, StrokeCap, StrokeJoin,
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
    needs_initial_replay: bool,
    presentation_ready: bool,
}

/// Current renderer set for one logical layer.
pub struct LayerTarget {
    renderers: Vec<LayerRenderer>,
    active: bool,
    needs_replay: bool,
    failed_layout: Option<LayerSurfaceLayout>,
    incremental_initialization: bool,
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
type LayerFactory =
    dyn Fn(usize, LayerCreationRequest) -> Pin<Box<dyn Future<Output = Option<LayerDef>>>>;

/// Snapshot of surviving tiles supplied to a chassis factory. Return only missing surfaces;
/// the runtime keeps survivors ready and validates returned tiles against the current layout.
pub struct LayerCreationRequest {
    retained: Vec<(String, String)>,
    /// Context shared by this logical layer's surviving tiles, if any.
    pub shared_context: Option<SharedGpuContext>,
    /// Adapter limit already established by the surviving tiles.
    pub max_surface_dimension: u32,
}

impl Default for LayerCreationRequest {
    fn default() -> Self {
        Self {
            retained: Vec::new(),
            shared_context: None,
            max_surface_dimension: u32::MAX,
        }
    }
}

impl LayerCreationRequest {
    /// Whether the factory must create this key and backing-surface generation.
    pub fn needs_surface(&self, key: &str, host_signature: &str) -> bool {
        !self
            .retained
            .iter()
            .any(|(k, h)| k == key && h == host_signature)
    }

    fn for_target(target: &LayerTarget) -> Self {
        Self {
            retained: target
                .renderers
                .iter()
                .map(|r| (r.key.clone(), r.host_signature.clone()))
                .collect(),
            shared_context: target
                .renderers
                .first()
                .map(|r| r.renderer.shared_gpu_context()),
            max_surface_dimension: target
                .renderers
                .first()
                .map_or(u32::MAX, |r| r.renderer.max_surface_dimension()),
        }
    }
}

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
        count(Counter::TileCreated, 1);
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
            needs_initial_replay: true,
            presentation_ready: false,
        }
    }

    /// Access the underlying retained `pax-gpu` renderer.
    pub fn renderer_mut(&mut self) -> &mut WgpuRenderer<'static> {
        &mut self.renderer
    }

    /// Whether this renderer belongs to the entry's current physical surface generation.
    pub fn matches_surface(&self, surface: &LayerSurfaceEntry) -> bool {
        self.key == surface.key && self.host_signature == surface.host_signature
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
        // Draw bindings remain surface-local. Share CPU geometry/resource caches as well as the
        // image textures already indexed by the logical layer's shared GPU context.
        if let Some((first, rest)) = renderers.split_first_mut() {
            for renderer in rest {
                renderer.renderer.share_vector_caches_from(&first.renderer);
            }
        }
        Self {
            renderers,
            active,
            needs_replay: false,
            failed_layout: None,
            incremental_initialization: false,
        }
    }

    /// Mutable access to each physical renderer backing this logical layer.
    pub fn renderers_mut(&mut self) -> &mut [LayerRenderer] {
        &mut self.renderers
    }

    /// Publish valid partial surface batches and requeue missing surfaces.
    /// Web uses this to reconsider global priority between GPU creations.
    /// The default remains whole-layer initialization for native chassis.
    pub fn with_incremental_initialization(mut self) -> Self {
        self.incremental_initialization = true;
        self
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
            count(Counter::SceneReset, 1);
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
    let partial_layout = LayerSurfaceLayout {
        active: layout.active,
        surfaces: layout
            .surfaces
            .iter()
            .filter(|surface| {
                target
                    .renderers
                    .iter()
                    .any(|renderer| renderer.matches_surface(surface))
            })
            .cloned()
            .collect(),
    };
    let layout = if target.incremental_initialization {
        &partial_layout
    } else {
        layout
    };
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

// Match survivors in desired order; unmatched entries are created by the chassis factory.
// Reject duplicate identities instead of moving the same owned renderer twice.
fn retained_surface_order(
    existing: &[(&str, &str)],
    desired: &[(&str, &str)],
) -> Option<Vec<usize>> {
    let mut seen = HashSet::new();
    let mut order = Vec::new();
    for identity in desired {
        if !seen.insert(*identity) {
            return None;
        }
        if let Some(index) = existing.iter().position(|candidate| candidate == identity) {
            order.push(index);
        }
    }
    Some(order)
}

fn retain_surfaces<T>(surfaces: &mut Vec<T>, order: Vec<usize>) {
    let mut previous: Vec<_> = std::mem::take(surfaces).into_iter().map(Some).collect();
    *surfaces = order
        .into_iter()
        .map(|index| previous[index].take().unwrap())
        .collect();
}

const MAX_CONCURRENT_LAYER_INITIALIZATIONS: usize = 4;

// A token belongs to one lifetime of a layer index. Removed layers invalidate it,
// so an async completion cannot overwrite a replacement at the same index.
struct LayerInitialization {
    index: usize,
    cancelled: Cell<bool>,
    scheduled: Cell<bool>,
}

impl LayerInitialization {
    fn new(index: usize) -> Self {
        Self {
            index,
            cancelled: Cell::new(false),
            scheduled: Cell::new(false),
        }
    }

    fn complete(
        &self,
        backend: Option<LayerDef>,
        requested_layout: Option<LayerSurfaceLayout>,
        backends: &RefCell<Vec<RenderLayerState>>,
        ready_layers: &RefCell<Vec<usize>>,
        changed_layers: &RefCell<Vec<usize>>,
    ) -> bool {
        if self.cancelled.get() {
            return false;
        }
        let mut states = backends.borrow_mut();
        let Some(state) = states.get_mut(self.index) else {
            return false;
        };
        match backend {
            Some((mut additions, provider)) => {
                let layout = provider();
                let incremental = additions.incremental_initialization;
                if let RenderLayerState::Ready((target, old_provider)) = state {
                    // A surface can be rebound during await. Keep only additions
                    // whose physical host still belongs to this layer; geometry
                    // changes are applied by refresh before targeted replay.
                    additions
                        .renderers
                        .retain(|r| layout.surfaces.iter().any(|s| r.matches_surface(s)));
                    for mut addition in additions.renderers {
                        if target.renderers.iter().any(|r| {
                            r.key == addition.key && r.host_signature == addition.host_signature
                        }) {
                            continue;
                        }
                        if let Some(first) = target.renderers.first() {
                            addition.renderer.share_vector_caches_from(&first.renderer);
                        }
                        target.renderers.push(addition);
                    }
                    target.failed_layout = None;
                    *old_provider = provider;
                    changed_layers.borrow_mut().push(self.index);
                    incremental && target.renderers.len() < layout.surfaces.len()
                } else if layer_layout_matches_bootstrapped_target(&additions, &layout) {
                    let incomplete =
                        incremental && additions.renderers.len() < layout.surfaces.len();
                    *state = RenderLayerState::Ready((additions, provider));
                    ready_layers.borrow_mut().push(self.index);
                    incomplete
                } else {
                    // Fresh layers have no survivor to keep ready. Retry a stale
                    // bootstrap against current hosts before publishing it.
                    *state = RenderLayerState::Pending;
                    true
                }
            }
            None => {
                // Failed additions must not discard a surviving scene. Retry
                // when its layout changes instead of spinning on allocation.
                if let RenderLayerState::Ready((target, _)) = state {
                    target.failed_layout = requested_layout;
                } else {
                    *state = RenderLayerState::Failed;
                }
                log::warn!(
                    "failed to initialize render surfaces for layer {}",
                    self.index
                );
                false
            }
        }
    }
}

fn pop_next_layer_initialization(
    queue: &RefCell<VecDeque<Rc<LayerInitialization>>>,
    priority: &dyn Fn(usize) -> i32,
) -> Option<Rc<LayerInitialization>> {
    let mut queue = queue.borrow_mut();
    queue.retain(|entry| !entry.cancelled.get());
    // Re-evaluate when a slot opens: native scrolling can make queued warm work
    // visible while another layer is awaiting GPU initialization. Ties stay FIFO.
    let index = queue
        .iter()
        .enumerate()
        .filter(|(_, entry)| priority(entry.index) != i32::MAX)
        .min_by_key(|(index, entry)| (priority(entry.index), *index))
        .map(|(index, _)| index)?;
    queue.remove(index)
}

fn pump_layer_initialization_queue(
    factory: Rc<LayerFactory>,
    backends: Rc<RefCell<Vec<RenderLayerState>>>,
    queue: Rc<RefCell<VecDeque<Rc<LayerInitialization>>>>,
    in_flight: Rc<Cell<usize>>,
    ready_layers: Rc<RefCell<Vec<usize>>>,
    changed_layers: Rc<RefCell<Vec<usize>>>,
    priority: Rc<dyn Fn(usize) -> i32>,
    surface_ready_callback: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
) {
    // A browser with a shared device can create a surface synchronously. Do not
    // reserve all slots for unpolled microtasks while ordinary frames keep
    // running. Bound immediate creation; subsequent refresh/presentation passes
    // pump again, and genuinely pending device work retains its async waker.
    let mut immediate = 0;
    while in_flight.get() < MAX_CONCURRENT_LAYER_INITIALIZATIONS {
        if cfg!(target_arch = "wasm32") && immediate >= MAX_CONCURRENT_LAYER_INITIALIZATIONS {
            break;
        }
        let next_layer = pop_next_layer_initialization(&queue, priority.as_ref());
        let Some(initialization) = next_layer else {
            break;
        };
        if initialization.cancelled.get() {
            continue;
        }

        let (request, requested_layout) = match backends.borrow().get(initialization.index) {
            Some(RenderLayerState::Ready((target, provider))) => {
                (LayerCreationRequest::for_target(target), Some(provider()))
            }
            _ => (LayerCreationRequest::default(), None),
        };
        let init_timing = crate::render_instrumentation::Span::new(
            crate::render_instrumentation::Phase::LayerInit,
        );
        let mut future = (factory)(initialization.index, request);
        if let std::task::Poll::Ready(backend) = poll_initialization_once(future.as_mut()) {
            let should_requeue = initialization.complete(
                backend,
                requested_layout,
                &backends,
                &ready_layers,
                &changed_layers,
            );
            initialization.scheduled.set(false);
            if should_requeue {
                initialization.scheduled.set(true);
                queue.borrow_mut().push_back(initialization);
            }
            immediate += 1;
            if let Some(callback) = surface_ready_callback.borrow().clone() {
                callback();
            }
            continue;
        }
        in_flight.set(in_flight.get() + 1);
        let factory = Rc::clone(&factory);
        let backends = Rc::clone(&backends);
        let queue = Rc::clone(&queue);
        let in_flight_count = Rc::clone(&in_flight);
        let ready_layers = Rc::clone(&ready_layers);
        let changed_layers = Rc::clone(&changed_layers);
        let priority = Rc::clone(&priority);
        let surface_ready_callback = Rc::clone(&surface_ready_callback);

        let task = async move {
            let should_requeue = if initialization.cancelled.get() {
                false
            } else {
                let _init_timing = init_timing;
                let backend = future.await;
                initialization.complete(
                    backend,
                    requested_layout,
                    &backends,
                    &ready_layers,
                    &changed_layers,
                )
            };
            initialization.scheduled.set(false);
            if should_requeue {
                initialization.scheduled.set(true);
                queue.borrow_mut().push_back(initialization);
            }
            in_flight_count.set(in_flight_count.get().saturating_sub(1));
            if let Some(callback) = surface_ready_callback.borrow().clone() {
                callback();
            }
            pump_layer_initialization_queue(
                factory,
                backends,
                queue,
                in_flight_count,
                ready_layers,
                changed_layers,
                priority,
                surface_ready_callback,
            );
        };
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(task);
        #[cfg(not(target_arch = "wasm32"))]
        pollster::block_on(task);
    }
}

fn poll_initialization_once<T>(future: Pin<&mut dyn Future<Output = T>>) -> std::task::Poll<T> {
    // Pending futures are retained and immediately handed to the platform
    // executor, which polls again with its live waker. Never restart the factory
    // or discard an in-progress adapter/device request after this first poll.
    future.poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
}
/// Runtime `RenderContext` implementation backed by `pax-gpu`/wgpu.
pub struct PaxGpuRenderer {
    backends: Rc<RefCell<Vec<RenderLayerState>>>,
    #[cfg(test)]
    flushed_surfaces: RefCell<Vec<(usize, usize, ResourceChurnStats)>>,
    layer_factory: Rc<LayerFactory>,
    layer_initialization_priority: Rc<dyn Fn(usize) -> i32>,
    surface_ready_callback: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    changed_layers: Rc<RefCell<Vec<usize>>>,
    image_map: HashMap<String, Image>,
    image_versions: HashMap<String, u64>,
    failed_context_gets: RefCell<Vec<bool>>,
    ready_layers: Rc<RefCell<Vec<usize>>>,
    replay_layers: Rc<RefCell<Vec<usize>>>,
    pending_layer_initializations: Rc<RefCell<VecDeque<Rc<LayerInitialization>>>>,
    layer_initializations: Vec<Rc<LayerInitialization>>,
    layer_initializations_in_flight: Rc<Cell<usize>>,
    active_render_scopes: RefCell<Vec<Vec<Vec<usize>>>>,
    dirty_render_surfaces: RefCell<Vec<HashSet<usize>>>,
    last_scene_lighting: RefCell<Vec<Option<PixelSceneLighting>>>,
    surface_replay: RefCell<SurfaceReplayCoordinator>,
    clean_skipped_canvas_nodes: RefCell<HashSet<(usize, u32)>>,
    alpha_source_scopes: Vec<(usize, usize)>,
    #[cfg(feature = "shaders")]
    shader_effect_scopes: Vec<(usize, usize)>,
    drawings: DrawingCache,
    pending_drawings: HashMap<usize, HashMap<u32, Rc<Drawing>>>,
    prepare_surface_content: bool,
    recording_layer: Option<usize>,
    recording_evictions: u64,
    direct_surface_layer: Option<usize>,
    submitting_prepared_surface: bool,
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
    fn is_capturing_source(&self) -> bool {
        #[cfg(feature = "shaders")]
        if !self.shader_effect_scopes.is_empty() {
            return true;
        }
        !self.alpha_source_scopes.is_empty()
    }

    /// Create a renderer that lazily asks the chassis for layer backends.
    pub fn new(
        layer_factory: impl Fn(usize, LayerCreationRequest) -> Pin<Box<dyn Future<Output = Option<LayerDef>>>>
            + 'static,
    ) -> Self {
        Self {
            backends: Default::default(),
            #[cfg(test)]
            flushed_surfaces: Default::default(),
            layer_factory: Rc::new(layer_factory),
            layer_initialization_priority: Rc::new(|_| 0),
            surface_ready_callback: Default::default(),
            changed_layers: Default::default(),
            image_map: Default::default(),
            image_versions: Default::default(),
            failed_context_gets: RefCell::new(vec![]),
            ready_layers: Default::default(),
            replay_layers: Default::default(),
            pending_layer_initializations: Rc::new(RefCell::new(VecDeque::new())),
            layer_initializations: Vec::new(),
            layer_initializations_in_flight: Rc::new(Cell::new(0)),
            active_render_scopes: Default::default(),
            dirty_render_surfaces: Default::default(),
            last_scene_lighting: Default::default(),
            surface_replay: Default::default(),
            clean_skipped_canvas_nodes: Default::default(),
            alpha_source_scopes: Vec::new(),
            #[cfg(feature = "shaders")]
            shader_effect_scopes: Vec::new(),
            drawings: DrawingCache::default(),
            pending_drawings: HashMap::new(),
            prepare_surface_content: false,
            recording_layer: None,
            recording_evictions: 0,
            direct_surface_layer: None,
            submitting_prepared_surface: false,
            #[cfg(debug_assertions)]
            tile_cull_stats: Default::default(),
        }
    }
}

impl PaxGpuRenderer {
    /// Opt into bounded cold drawing preparation. The chassis must drive
    /// `PaxEngine::present_settled_surfaces` after ordinary scene updates settle.
    /// Native chassis retain ordinary first drawing unless they opt in explicitly.
    pub fn with_surface_content_preparation(mut self) -> Self {
        self.prepare_surface_content = true;
        self
    }

    fn apply_drawing_command(&mut self, layer: usize, command: DrawingCommand) {
        if self
            .recording_layer
            .or(self.direct_surface_layer)
            .is_some_and(|owner| owner != layer)
        {
            return;
        }
        let command = Rc::new(command);
        self.drawings.push(layer, Rc::clone(&command));
        if self.recording_layer.is_some() {
            return;
        }
        self.with_layer_context(layer, |context| {
            command.execute(context, &self.image_map, &self.image_versions)
        });
    }

    fn record_stroke(
        &mut self,
        layer: usize,
        path: BezPath,
        stroke: &ResolvedStroke,
        material: &ResolvedMaterial,
        opacity: f64,
        range: PixelDrawRange,
        smoothing: PathSmoothing,
    ) {
        let bounds = pax_runtime_api::drawing::path_smoothing::smooth_bez_path(&path, smoothing)
            .bounding_box();
        self.apply_drawing_command(
            layer,
            DrawingCommand::Stroke {
                path: convert_kurbo_to_lyon_path(&path),
                bounds,
                paint: stroke.paint.clone(),
                material: to_pax_gpu_material(material),
                opacity: opacity as f32,
                width: stroke.width_pixels() as f32,
                cap: match stroke.cap {
                    StrokeCap::Butt => PixelStrokeCap::Butt,
                    StrokeCap::Round => PixelStrokeCap::Round,
                    StrokeCap::Square => PixelStrokeCap::Square,
                },
                join: match stroke.join {
                    StrokeJoin::Miter => PixelStrokeJoin::Miter,
                    StrokeJoin::Round => PixelStrokeJoin::Round,
                    StrokeJoin::Bevel => PixelStrokeJoin::Bevel,
                },
                range,
                smoothing,
            },
        );
    }

    fn invalidate_drawing(&mut self, layer: usize, node: u32) {
        self.drawings.remove(layer, node);
        if let Some(pending) = self.pending_drawings.get_mut(&layer) {
            pending.remove(&node);
        }
    }

    fn replay_drawings(&mut self, layer: usize) {
        let Some(drawings) = self.pending_drawings.remove(&layer) else {
            return;
        };
        let mut backends = self.backends.borrow_mut();
        let Some(RenderLayerState::Ready((target, _))) = backends.get_mut(layer) else {
            return;
        };
        if !target.active {
            return;
        }
        target.prepare_for_render();
        let indices = self.targeted_or_all_indices(layer, target.renderers.len());
        for index in indices {
            let Some(tile) = target.renderers.get_mut(index) else {
                continue;
            };
            for drawing in drawings.values() {
                if !tile.intersects_coverage_bounds(&drawing.bounds) {
                    continue;
                }
                tile.renderer.reset_drawing_state();
                for command in &drawing.commands {
                    command.execute(&mut tile.renderer, &self.image_map, &self.image_versions);
                }
                tile.renderer.reset_drawing_state();
                self.mark_render_surfaces_dirty(layer, [index]);
            }
        }
    }

    /// Prioritize queued layer creation using current chassis presentation state.
    /// Lower values start first; equal values retain FIFO order. This does not
    /// preempt in-flight work or increase the initialization concurrency limit.
    /// Return `i32::MAX` while a host has no installed targets; it remains queued
    /// without occupying an initialization slot until a later layout refresh.
    pub fn with_layer_initialization_priority(
        mut self,
        priority: impl Fn(usize) -> i32 + 'static,
    ) -> Self {
        self.layer_initialization_priority = Rc::new(priority);
        self
    }

    fn queue_layer_initialization(&self, layer_index: usize) {
        let initialization = &self.layer_initializations[layer_index];
        if !initialization.scheduled.replace(true) {
            self.pending_layer_initializations
                .borrow_mut()
                .push_back(Rc::clone(initialization));
        }
    }

    fn enqueue_layer_initialization(&self, layer_index: usize) {
        self.queue_layer_initialization(layer_index);
        self.schedule_layer_initialization();
    }

    fn with_layer_context(&self, layer: usize, mut f: impl FnMut(&mut WgpuRenderer)) {
        if self.recording_layer.is_some()
            || self
                .direct_surface_layer
                .is_some_and(|owner| owner != layer)
        {
            return;
        }
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
                    // Component recording is content work and reaches every affected tile.
                    // Cached surface replay is executed separately in replay_drawings().
                    let scoped_indices = self.active_render_scope(layer).or_else(|| {
                        self.direct_surface_layer
                            .map(|_| self.targeted_or_all_indices(layer, target.renderers.len()))
                    });
                    if let Some(indices) = scoped_indices {
                        for index in indices {
                            if let Some(renderer) = target.renderers.get_mut(index) {
                                f(&mut renderer.renderer);
                            }
                        }
                    } else {
                        for renderer in &mut target.renderers {
                            if !self.prepare_surface_content || renderer.presentation_ready {
                                f(&mut renderer.renderer);
                            }
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
        #[cfg(test)]
        self.flushed_surfaces
            .borrow_mut()
            .retain(|(old_layer, _, _)| *old_layer != layer);
        use crate::render_instrumentation::{Phase, Span};
        let targeted_indices = if self.prepare_surface_content && !self.submitting_prepared_surface
        {
            None
        } else {
            self.targeted_replay_scope(layer)
        };
        let dirty_indices = self.dirty_render_surface_scope(layer);
        let used_targeted_replay = targeted_indices.is_some();
        #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
        let targeted_surface_count = targeted_indices.as_ref().map(Vec::len).unwrap_or(0);
        #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
        let dirty_surface_count = dirty_indices.as_ref().map(Vec::len).unwrap_or(0);
        let mut indices = targeted_indices.unwrap_or_default();
        if let Some(dirty_indices) = dirty_indices {
            // Ordinary updates to already-presented tiles may join this submit;
            // other cold tiles must wait for their own complete preparation.
            indices.extend(dirty_indices.into_iter().filter(|i| {
                !self.prepare_surface_content || target.renderers[*i].presentation_ready
            }));
        }
        indices.sort_unstable();
        indices.dedup();
        if self.prepare_surface_content && !self.submitting_prepared_surface {
            indices.retain(|i| target.renderers[*i].presentation_ready);
        }

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
                    let encode_timing = Span::new(Phase::Encode);
                    renderer.renderer.flush_deferred();
                    drop(encode_timing);
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
                    let stats = renderer.renderer.take_resource_churn_stats();
                    #[cfg(test)]
                    self.flushed_surfaces
                        .borrow_mut()
                        .push((layer, *index, stats));
                    count(
                        Counter::ImageTextureUploaded,
                        stats.texture_creates as usize,
                    );
                    count(
                        Counter::ImageTextureUploadBytes,
                        stats.texture_upload_bytes as usize,
                    );
                    count(
                        Counter::ImageTextureReused,
                        stats.texture_cache_hits as usize,
                    );
                    resource_stats.merge(stats);
                }
            }
            let command_buffer_count = command_buffers.len();
            if command_buffer_count != 0 {
                if let Some(index) = submitter_index {
                    if let Some(renderer) = target.renderers.get(index) {
                        #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
                        let submit_start = std::time::Instant::now();
                        let submit_timing = Span::new(Phase::Submit);
                        renderer.renderer.submit_command_buffers(command_buffers);
                        drop(submit_timing);
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
                    let cleanup_timing = Span::new(Phase::Cleanup);
                    renderer.renderer.complete_submitted_work();
                    drop(cleanup_timing);
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
                    let present_timing = Span::new(Phase::Present);
                    renderer.renderer.present_deferred_frame();
                    renderer.presentation_ready = true;
                    drop(present_timing);
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
                    "[pax-gpu-flush] layer={} surfaces={} targeted={} targeted_surfaces={} dirty_surfaces={} command_buffers={} cleanup_surfaces={} present_surfaces={} total_us={} encode_us={} submit_us={} cleanup_us={} present_us={} flushes={} retained_nodes={}/{} retained_batches={} retained_draws={} vector_draws={} image_draws={} batch_flushes={} geometry_rebuilds={} geometry_cache_hits={} geometry_cache_misses={} geometry_cache_evictions={} geometry_cache_bytes={} tessellated_vertices={} tessellated_indices={} cached_vertices_reused={} cached_indices_reused={} resource_creates={} resource_updates={} resource_recreates={} create_bytes={} update_bytes={} cache_hits={} cache_misses={} cache_evictions={} cache_bytes={} texture_creates={} texture_bytes={} texture_cache_hits={}",
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
                    resource_stats.texture_cache_hits,
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

    fn replay_region_bounds(&self, layer: usize) -> Option<Vec<kurbo::Rect>> {
        let backends = self.backends.borrow();
        let Some(RenderLayerState::Ready((target, _))) = backends.get(layer) else {
            return None;
        };
        self.targeted_replay_surface_bounds(layer, target)
    }

    fn take_replay_layer_ids(&mut self) -> Vec<usize> {
        let mut replay_layers = self.replay_layers.borrow_mut();
        let mut replay = std::mem::take(&mut *replay_layers);
        replay.sort_unstable();
        replay.dedup();
        // Native targets retain whole-layer replay. Web incremental targets
        // share the same ordinal as admission/creation: finish visible replay
        // globally before spending a frame on a speculative batch.
        let states = self.backends.borrow();
        let priorities: Vec<_> = replay
            .iter()
            .map(|layer| {
                let Some(RenderLayerState::Ready((target, provider))) = states.get(*layer) else {
                    return (*layer, 0, false);
                };
                let layout = provider();
                let indices = self.targeted_replay_scope(*layer);
                let rank = if let Some(indices) = indices {
                    // A partial initialization can omit earlier host entries;
                    // physical renderer indices are not installed-plan indices.
                    replay_identity_priority(
                        &layout,
                        indices.into_iter().filter_map(|i| {
                            target
                                .renderers
                                .get(i)
                                .map(|r| (r.key.as_str(), r.host_signature.as_str()))
                        }),
                    )
                } else {
                    layout
                        .surfaces
                        .iter()
                        .map(|s| s.replay_priority)
                        .min()
                        .unwrap_or(0)
                };
                (*layer, rank, target.incremental_initialization)
            })
            .collect();
        if priorities.iter().any(|(_, _, global)| *global) {
            let minimum = priorities
                .iter()
                .map(|(_, rank, _)| *rank)
                .min()
                .unwrap_or(0);
            replay.retain(|layer| {
                let (_, rank, _) = priorities.iter().find(|(id, _, _)| id == layer).unwrap();
                let selected = *rank <= 0 || *rank == minimum;
                if !selected {
                    replay_layers.push(*layer);
                }
                selected
            });
            replay.sort_by_key(|layer| {
                let (_, rank, _) = priorities.iter().find(|(id, _, _)| id == layer).unwrap();
                (*rank, *layer)
            });
        }
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
            Rc::clone(&self.layer_initializations_in_flight),
            Rc::clone(&self.ready_layers),
            Rc::clone(&self.changed_layers),
            Rc::clone(&self.layer_initialization_priority),
            Rc::clone(&self.surface_ready_callback),
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
                RenderLayerState::Pending => {
                    needs_reinitialization = true;
                }
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
                        let existing: Vec<_> = target
                            .renderers
                            .iter()
                            .map(|r| (r.key.as_str(), r.host_signature.as_str()))
                            .collect();
                        let desired: Vec<_> = layout
                            .surfaces
                            .iter()
                            .map(|s| (s.key.as_str(), s.host_signature.as_str()))
                            .collect();
                        let Some(order) = retained_surface_order(&existing, &desired) else {
                            log::error!("duplicate surface identity in layer {}", layer_index);
                            continue;
                        };
                        let reordered = order.len() != target.renderers.len()
                            || order.iter().enumerate().any(|(new, old)| new != *old);
                        if reordered {
                            count(Counter::TileSetChanged, 1);
                            count(Counter::TileReused, order.len());
                            count(
                                Counter::MatchingTileDiscarded,
                                existing
                                    .iter()
                                    .enumerate()
                                    .filter(|(index, identity)| {
                                        desired.contains(identity) && !order.contains(index)
                                    })
                                    .count(),
                            );
                            self.surface_replay
                                .borrow_mut()
                                .remap_surfaces(layer_index, &order);
                            if let Some(dirty) =
                                self.dirty_render_surfaces.borrow_mut().get_mut(layer_index)
                            {
                                *dirty = order
                                    .iter()
                                    .enumerate()
                                    .filter_map(|(new, old)| dirty.contains(old).then_some(new))
                                    .collect();
                            }
                            retain_surfaces(&mut target.renderers, order);
                        }
                        if target.renderers.len() < layout.surfaces.len()
                            && target.failed_layout.as_ref() != Some(&layout)
                        {
                            self.queue_layer_initialization(layer_index);
                            needs_reinitialization = true;
                        }
                    }

                    let mut targeted_replay_entries = Vec::new();
                    let mut targeted_replay_bounds = HashMap::new();
                    // Warm work can become visible without an origin change. Reclassify it
                    // using this layout, retaining coverage until the tile has actually replayed.
                    for (index, bounds) in self
                        .surface_replay
                        .borrow()
                        .pending_replay_surfaces(layer_index)
                    {
                        let Some(renderer) = target.renderers.get(index) else {
                            continue;
                        };
                        let Some(surface) = layout
                            .surfaces
                            .iter()
                            .find(|surface| renderer.matches_surface(surface))
                        else {
                            continue;
                        };
                        let current = renderer.coverage_bounds();
                        targeted_replay_entries.push(ReplayPriorityEntry::new(
                            index,
                            surface.replay_priority,
                            current,
                            current,
                        ));
                        targeted_replay_bounds.insert(index, bounds);
                    }
                    let needs_full_layer_replay = target.needs_replay;
                    if needs_full_layer_replay && self.prepare_surface_content {
                        for renderer in &mut target.renderers {
                            renderer.presentation_ready = false;
                        }
                    }
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
                    for (index, renderer) in target.renderers.iter_mut().enumerate() {
                        let Some(surface) = layout.surfaces.iter().find(|surface| {
                            surface.key == renderer.key
                                && surface.host_signature == renderer.host_signature
                        }) else {
                            continue;
                        };
                        if std::mem::take(&mut renderer.needs_initial_replay) {
                            let bounds = Rect::new(
                                surface.origin_x as f64,
                                surface.origin_y as f64,
                                (surface.origin_x + surface.surface.logical_width) as f64,
                                (surface.origin_y + surface.surface.logical_height) as f64,
                            );
                            targeted_replay_entries.push(ReplayPriorityEntry::new(
                                index,
                                surface.replay_priority,
                                bounds,
                                bounds,
                            ));
                            targeted_replay_bounds.insert(index, vec![bounds]);
                            // Lighting may not have changed since the surviving tiles were drawn.
                            if let Some(Some(lighting)) =
                                self.last_scene_lighting.borrow().get(layer_index)
                            {
                                renderer.renderer.set_scene_lighting(
                                    translate_scene_lighting_for_surface(
                                        lighting,
                                        surface.origin_x,
                                        surface.origin_y,
                                    ),
                                );
                            }
                        }
                        let previous_bounds = renderer.coverage_bounds();
                        let layout_change = renderer.update_layout(surface);
                        let current_bounds = renderer.coverage_bounds();
                        match layout_change {
                            LayoutChangeKind::Unchanged => {}
                            LayoutChangeKind::OriginOnly => {
                                renderer.presentation_ready = false;
                                count(Counter::OriginRetargeted, 1);
                                count(Counter::SceneReset, 1);
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
                                renderer.presentation_ready = false;
                                count(Counter::TileResized, 1);
                                count(Counter::SceneReset, 1);
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
                                targeted_replay_entries.push(ReplayPriorityEntry::new(
                                    index,
                                    surface.replay_priority,
                                    previous_bounds,
                                    current_bounds,
                                ));
                                targeted_replay_bounds
                                    .insert(index, vec![previous_bounds, current_bounds]);
                            }
                        }
                    }
                    if needs_full_layer_replay && self.prepare_surface_content {
                        for (index, renderer) in target.renderers.iter().enumerate() {
                            let bounds = renderer.coverage_bounds();
                            let priority = layout
                                .surfaces
                                .iter()
                                .find(|s| s.key == renderer.key)
                                .map_or(0, |s| s.replay_priority);
                            targeted_replay_entries
                                .push(ReplayPriorityEntry::new(index, priority, bounds, bounds));
                            targeted_replay_bounds.insert(index, vec![bounds]);
                        }
                    }
                    if needs_full_layer_replay && !self.prepare_surface_content {
                        self.clear_targeted_replay_scope(layer_index);
                        #[cfg(debug_assertions)]
                        self.update_tile_cull_stats(layer_index, |stats| {
                            stats.full_layer_replays += 1;
                        });
                        self.mark_render_surfaces_dirty(layer_index, 0..target.renderers.len());
                        self.replay_layers.borrow_mut().push(layer_index);
                    } else if !targeted_replay_entries.is_empty() {
                        targeted_replay_entries.sort_by_key(|entry| entry.index);
                        targeted_replay_entries.dedup_by_key(|entry| entry.index);
                        let mut replay_batches =
                            replay_batches_by_directional_priority(&targeted_replay_entries);
                        if self.prepare_surface_content {
                            // Preparation is admitted per physical tile, not a
                            // row-wide burst. Ordering still comes from the one
                            // shared surface coordinator and current intent.
                            replay_batches = replay_batches
                                .into_iter()
                                .flatten()
                                .map(|i| vec![i])
                                .collect();
                        }
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
    fn set_surface_ready_callback(&mut self, callback: Option<Rc<dyn Fn()>>) {
        *self.surface_ready_callback.borrow_mut() = callback;
    }

    fn take_settled_surface_replays(&mut self) -> Vec<ReplayCanvasLayerUpdate> {
        let ready = self.take_ready_canvas_layers();
        self.replay_layers.borrow_mut().extend(ready);
        self.take_replay_layer_ids()
            .into_iter()
            .map(|layer| {
                let regions = self.replay_region_bounds(layer).or_else(|| {
                    let backends = self.backends.borrow();
                    let RenderLayerState::Ready((target, _)) = backends.get(layer)? else {
                        return None;
                    };
                    Some(
                        target
                            .renderers
                            .iter()
                            .map(|r| r.coverage_bounds())
                            .collect(),
                    )
                });
                ReplayCanvasLayerUpdate { layer, regions }
            })
            .collect()
    }

    fn defer_surface_replay(&mut self, layer: usize) {
        self.replay_layers.borrow_mut().push(layer);
    }

    fn prepare_settled_surface_replay(&mut self, layer: usize, nodes: &[u32]) -> bool {
        let valid_target = matches!(self.backends.borrow().get(layer),
            Some(RenderLayerState::Ready((target, _))) if target.active);
        if !valid_target
            || self
                .pending_drawings
                .get(&layer)
                .is_some_and(|p| !p.is_empty())
        {
            return false;
        }
        let drawings: Option<HashMap<_, _>> = nodes
            .iter()
            .map(|id| self.drawings.get(layer, *id).map(|drawing| (*id, drawing)))
            .collect();
        let Some(drawings) = drawings else {
            return false;
        };
        self.pending_drawings.insert(layer, drawings);
        // Even an empty content region needs one transparent submission so the
        // surface's current generation becomes ready instead of staying pending.
        let count = match &self.backends.borrow()[layer] {
            RenderLayerState::Ready((target, _)) => target.renderers.len(),
            _ => 0,
        };
        self.mark_render_surfaces_dirty(layer, self.targeted_or_all_indices(layer, count));
        self.submitting_prepared_surface = true;
        true
    }

    fn prepares_surface_content(&self) -> bool {
        self.prepare_surface_content
    }

    fn missing_surface_drawings(&mut self, layer: usize, nodes: &[u32]) -> Option<Vec<u32>> {
        self.prepare_surface_content.then(|| {
            nodes
                .iter()
                .copied()
                .filter(|id| self.drawings.get(layer, *id).is_none())
                .collect()
        })
    }

    fn begin_surface_recording(&mut self, layer: usize) -> bool {
        if !self.prepare_surface_content || self.recording_layer.is_some() {
            return false;
        }
        self.drawings.finish_frame(layer);
        self.recording_evictions = self.drawings.evictions;
        self.recording_layer = Some(layer);
        true
    }

    fn end_surface_recording(&mut self, layer: usize) -> bool {
        assert_eq!(self.recording_layer.take(), Some(layer));
        assert!(self.alpha_source_scopes.is_empty());
        #[cfg(feature = "shaders")]
        assert!(self.shader_effect_scopes.is_empty());
        self.drawings.finish_frame(layer);
        self.drawings.evictions == self.recording_evictions
    }

    fn begin_surface_fallback(&mut self, layer: usize) -> bool {
        if !self.prepare_surface_content
            || !matches!(self.backends.borrow().get(layer),
            Some(RenderLayerState::Ready((target, _))) if target.active)
        {
            return false;
        }
        self.direct_surface_layer = Some(layer);
        self.submitting_prepared_surface = true;
        self.clear(layer);
        true
    }

    fn separates_surface_replay(&self) -> bool {
        true
    }

    fn prepare_canvas_replay(&mut self, layer: usize, nodes: &[u32]) -> Vec<u32> {
        let mut missing = Vec::new();
        for id in nodes {
            if let Some(drawing) = self.drawings.get(layer, *id) {
                self.pending_drawings
                    .entry(layer)
                    .or_default()
                    .insert(*id, drawing);
            } else {
                missing.push(*id);
            }
        }
        missing
    }

    fn fill_with_opacity(
        &mut self,
        layer: usize,
        path: BezPath,
        fill: &pax_runtime_api::Paint,
        opacity: f64,
    ) {
        self.fill_with_material_and_opacity(
            layer,
            path,
            fill,
            &pax_runtime_api::Material::default().snapshot(),
            opacity,
        );
    }

    fn fill_with_material_and_opacity(
        &mut self,
        layer: usize,
        path: BezPath,
        fill: &pax_runtime_api::Paint,
        material: &ResolvedMaterial,
        opacity: f64,
    ) {
        self.fill_with_material_and_opacity_and_smoothing(
            layer,
            path,
            fill,
            material,
            opacity,
            PathSmoothing::default(),
        );
    }

    fn fill_with_material_and_opacity_and_smoothing(
        &mut self,
        layer: usize,
        path: BezPath,
        fill: &pax_runtime_api::Paint,
        material: &ResolvedMaterial,
        opacity: f64,
        smoothing: PathSmoothing,
    ) {
        let bounds = pax_runtime_api::drawing::path_smoothing::smooth_bez_path(&path, smoothing)
            .bounding_box();
        self.apply_drawing_command(
            layer,
            DrawingCommand::Fill {
                path: convert_kurbo_to_lyon_path(&path),
                bounds,
                paint: fill.clone(),
                material: to_pax_gpu_material(material),
                opacity: opacity as f32,
                smoothing,
            },
        );
    }

    fn stroke_with_opacity(
        &mut self,
        layer: usize,
        path: BezPath,
        stroke: &ResolvedStroke,
        opacity: f64,
    ) {
        self.stroke_with_material_and_opacity(
            layer,
            path,
            stroke,
            &pax_runtime_api::Material::default().snapshot(),
            opacity,
        );
    }

    fn stroke_with_material_and_opacity(
        &mut self,
        layer: usize,
        path: BezPath,
        stroke: &ResolvedStroke,
        material: &ResolvedMaterial,
        opacity: f64,
    ) {
        self.stroke_with_material_and_opacity_and_smoothing(
            layer,
            path,
            stroke,
            material,
            opacity,
            PathSmoothing::default(),
        );
    }

    fn stroke_with_material_and_opacity_and_smoothing(
        &mut self,
        layer: usize,
        path: BezPath,
        stroke: &ResolvedStroke,
        material: &ResolvedMaterial,
        opacity: f64,
        smoothing: PathSmoothing,
    ) {
        self.record_stroke(
            layer,
            path,
            stroke,
            material,
            opacity,
            PixelDrawRange::disabled(),
            smoothing,
        );
    }

    fn stroke_with_draw_range_and_material_and_opacity(
        &mut self,
        layer: usize,
        path: BezPath,
        stroke: &ResolvedStroke,
        material: &ResolvedMaterial,
        opacity: f64,
        draw_start: f64,
        draw_end: f64,
    ) {
        self.stroke_with_draw_range_and_material_and_opacity_and_smoothing(
            layer,
            path,
            stroke,
            material,
            opacity,
            draw_start,
            draw_end,
            PathSmoothing::default(),
        );
    }

    fn stroke_with_draw_range_and_material_and_opacity_and_smoothing(
        &mut self,
        layer: usize,
        path: BezPath,
        stroke: &ResolvedStroke,
        material: &ResolvedMaterial,
        opacity: f64,
        draw_start: f64,
        draw_end: f64,
        smoothing: PathSmoothing,
    ) {
        self.record_stroke(
            layer,
            path,
            stroke,
            material,
            opacity,
            PixelDrawRange::enabled(draw_start as f32, draw_end as f32),
            smoothing,
        );
    }

    fn save(&mut self, layer: usize) {
        self.apply_drawing_command(layer, DrawingCommand::Save);
    }
    fn restore(&mut self, layer: usize) {
        self.apply_drawing_command(layer, DrawingCommand::Restore);
    }
    fn clip(&mut self, layer: usize, path: BezPath) {
        self.apply_drawing_command(
            layer,
            DrawingCommand::Clip(convert_kurbo_to_lyon_path(&path)),
        );
    }
    fn clip_alpha(
        &mut self,
        layer: usize,
        paints: &[pax_runtime_api::AlphaMaskPaint],
        feather: f64,
    ) {
        self.apply_drawing_command(
            layer,
            DrawingCommand::Alpha(paints.to_vec(), feather.max(0.0) as f32),
        );
    }
    fn transform(&mut self, layer: usize, affine: kurbo::Affine) {
        self.apply_drawing_command(
            layer,
            DrawingCommand::Transform(Transform2D::from_array(
                affine.as_coeffs().map(|v| v as f32),
            )),
        );
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
        rect: Rect,
        opacity: f64,
    ) {
        self.apply_drawing_command(
            layer,
            DrawingCommand::Image(image_path.to_owned(), rect, opacity as f32),
        );
    }

    fn get_image_size(&mut self, image_path: &str) -> Option<(usize, usize)> {
        self.image_map
            .get(image_path)
            .map(|img| (img.pixel_width as usize, img.pixel_height as usize))
    }

    fn image_loaded(&self, image_path: &str) -> bool {
        self.image_map.contains_key(image_path)
    }

    fn image_pending(&mut self, layer: usize, node: u32, path: &str) {
        self.with_layer_context(layer, |context| context.trace_image_pending(node, path));
    }

    fn layers(&self) -> usize {
        self.backends.borrow().len()
    }

    fn resize_layers_to(&mut self, layer_count: usize, _dirty_canvases: Rc<RefCell<Vec<bool>>>) {
        let current_len = self.backends.borrow().len();
        match layer_count.cmp(&current_len) {
            std::cmp::Ordering::Less => {
                self.drawings.truncate_layers(layer_count);
                self.pending_drawings
                    .retain(|layer, _| *layer < layer_count);
                for initialization in self.layer_initializations.drain(layer_count..) {
                    initialization.cancelled.set(true);
                }
                self.pending_layer_initializations
                    .borrow_mut()
                    .retain(|initialization| !initialization.cancelled.get());
                self.backends.borrow_mut().truncate(layer_count);
                self.ready_layers
                    .borrow_mut()
                    .retain(|layer| *layer < layer_count);
                self.changed_layers
                    .borrow_mut()
                    .retain(|layer| *layer < layer_count);
                self.replay_layers
                    .borrow_mut()
                    .retain(|layer| *layer < layer_count);
                self.failed_context_gets.borrow_mut().truncate(layer_count);
                self.active_render_scopes.borrow_mut().truncate(layer_count);
                self.dirty_render_surfaces
                    .borrow_mut()
                    .truncate(layer_count);
                self.last_scene_lighting.borrow_mut().truncate(layer_count);
                self.surface_replay
                    .borrow_mut()
                    .truncate_layers(layer_count);
                self.clean_skipped_canvas_nodes
                    .borrow_mut()
                    .retain(|(layer, _)| *layer < layer_count);
                #[cfg(debug_assertions)]
                self.tile_cull_stats.borrow_mut().truncate(layer_count);
            }
            std::cmp::Ordering::Equal => return,
            std::cmp::Ordering::Greater => {
                for i in current_len..layer_count {
                    self.backends.borrow_mut().push(RenderLayerState::Pending);
                    self.layer_initializations
                        .push(Rc::new(LayerInitialization::new(i)));
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
                        if self.prepare_surface_content
                            && !renderer.presentation_ready
                            && self.direct_surface_layer != Some(layer)
                        {
                            continue;
                        }
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
        self.replay_drawings(layer);
        self.drawings.finish_frame(layer);
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
        self.submitting_prepared_surface = false;
        self.direct_surface_layer = None;
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
        self.update_scene_lighting_cache(layer, &lighting);
        let mut backends = self.backends.borrow_mut();
        match backends.get_mut(layer) {
            Some(RenderLayerState::Pending) => {}
            Some(RenderLayerState::Failed) => {}
            Some(RenderLayerState::Ready((target, _))) => {
                if !target.active {
                    return;
                }
                for (index, renderer) in target.renderers.iter_mut().enumerate() {
                    if renderer
                        .renderer
                        .set_scene_lighting(translate_scene_lighting_for_surface(
                            &lighting,
                            renderer.origin_x,
                            renderer.origin_y,
                        ))
                    {
                        self.mark_render_surfaces_dirty(layer, [index]);
                    }
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
        self.schedule_layer_initialization();
        let mut ready = Vec::new();
        // Native creation completes synchronously. Drain its additions before drawing; a stale
        // completion can trigger another creation, so bound retries to avoid monopolizing a frame.
        for _ in 0..MAX_CONCURRENT_LAYER_INITIALIZATIONS {
            let newly_ready = std::mem::take(&mut *self.ready_layers.borrow_mut());
            let mut changed = std::mem::take(&mut *self.changed_layers.borrow_mut());
            changed.extend(newly_ready.iter().copied());
            ready.extend(newly_ready);
            changed.sort_unstable();
            changed.dedup();
            if changed.is_empty() {
                break;
            }
            self.refresh_layer_layouts(changed);
        }
        ready.sort_unstable();
        ready.dedup();
        for layer in &ready {
            if !self.prepare_surface_content {
                self.clear_targeted_replay_scope(*layer);
            }
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
                regions: self.replay_region_bounds(layer),
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

    #[cfg(feature = "shaders")]
    fn begin_shader_effect(
        &mut self,
        layer: usize,
        id: u32,
        z: i32,
        effect: &pax_runtime_api::ShaderEffect,
        transform: kurbo::Affine,
        bounds: (f64, f64),
        scopes: &[pax_runtime_api::OpacityScope],
        children: &[u32],
    ) -> bool {
        if self.recording_layer.is_some()
            || self
                .direct_surface_layer
                .is_some_and(|owner| owner != layer)
        {
            // Effect captures own complete subtrees, not independent leaf drawings.
            // Leave them uncached so preparation uses its atomic tile fallback;
            // a CPU recording pass must never mutate a live GPU capture.
            self.push_render_scope(layer, Vec::new());
            self.shader_effect_scopes.push((layer, scopes.len()));
            return true;
        }
        let inherited = self
            .shader_effect_scopes
            .last()
            .map_or(0, |s| s.1)
            .min(scopes.len());
        let mut replay = false;
        let mut selected = Vec::new();
        {
            let mut backends = self.backends.borrow_mut();
            if let Some(RenderLayerState::Ready((target, _))) = backends.get_mut(layer) {
                if target.active {
                    target.prepare_for_render();
                    selected = self.active_render_scope(layer).unwrap_or_else(|| {
                        self.targeted_or_all_indices(layer, target.renderers.len())
                    });
                    selected.retain(|index| {
                        !self.prepare_surface_content
                            || target.renderers[*index].presentation_ready
                            || self.direct_surface_layer == Some(layer)
                    });
                    for index in &selected {
                        replay |= target.renderers[*index].renderer.begin_shader_effect(
                            id,
                            z,
                            effect,
                            Transform2D::from_array(transform.as_coeffs().map(|v| v as f32)),
                            bounds,
                            &scopes[inherited..],
                            children,
                        );
                    }
                }
            }
        }
        self.mark_render_surfaces_dirty(layer, selected.iter().copied());
        self.push_render_scope(layer, selected);
        self.shader_effect_scopes.push((layer, scopes.len()));
        replay
    }

    #[cfg(feature = "shaders")]
    fn end_shader_effect(&mut self, layer: usize, _id: u32) {
        self.with_layer_context(layer, |context| context.end_shader_effect());
        assert_eq!(self.shader_effect_scopes.pop().map(|s| s.0), Some(layer));
        self.pop_render_scope(layer);
    }

    fn supports_alpha_source_capture(&self) -> bool {
        true
    }

    fn alpha_source_layer(&self) -> Option<usize> {
        self.alpha_source_scopes.last().map(|&(layer, _)| layer)
    }

    fn begin_alpha_source(
        &mut self,
        layer: usize,
        mapping: kurbo::Affine,
        inherited_scopes: usize,
        feather: f64,
    ) {
        self.apply_drawing_command(
            layer,
            DrawingCommand::BeginSource(
                Transform2D::from_array(mapping.as_coeffs().map(|v| v as f32)),
                feather as f32,
            ),
        );
        self.alpha_source_scopes.push((layer, inherited_scopes));
    }

    fn end_alpha_source(&mut self, layer: usize) {
        self.apply_drawing_command(layer, DrawingCommand::EndSource);
        assert_eq!(self.alpha_source_scopes.pop().map(|s| s.0), Some(layer));
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
        let inherited = self.alpha_source_scopes.last();
        #[cfg(feature = "shaders")]
        let inherited = inherited.or_else(|| self.shader_effect_scopes.last());
        let scopes = &scopes[inherited.map_or(0, |s| s.1).min(scopes.len())..];
        self.apply_drawing_command(layer, DrawingCommand::Opacity(node_id, scopes.to_vec()));
    }

    fn begin_node(&mut self, layer: usize, node_id: u32, z_index: i32, light_mask: u32) -> bool {
        if self
            .direct_surface_layer
            .is_some_and(|owner| owner != layer)
        {
            return false;
        }
        if let Some(owner) = self.recording_layer {
            if owner != layer {
                return false;
            }
            self.drawings
                .begin(layer, node_id, z_index, light_mask, None);
            return true;
        }
        self.invalidate_drawing(layer, node_id);
        if self.is_capturing_source() {
            let mut began = self.prepare_surface_content
                && self
                    .active_render_scope(layer)
                    .is_some_and(|scope| scope.is_empty());
            self.with_layer_context(layer, |context| {
                began |= context.begin_node(node_id, z_index, light_mask);
            });
            if began {
                self.drawings
                    .begin(layer, node_id, z_index, light_mask, None);
                let indices = self
                    .active_render_scope(layer)
                    .expect("source has an owning mask scope");
                self.push_render_scope(layer, indices);
            }
            return began;
        }
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
                target.prepare_for_render();
                let candidate_indices = if self.direct_surface_layer.is_some() {
                    self.targeted_or_all_indices(layer, target.renderers.len())
                } else {
                    (0..target.renderers.len()).collect()
                };
                let mut selected = Vec::new();
                for index in candidate_indices {
                    let Some(renderer) = target.renderers.get_mut(index) else {
                        continue;
                    };
                    if self.prepare_surface_content
                        && !renderer.presentation_ready
                        && self.direct_surface_layer != Some(layer)
                    {
                        continue;
                    }
                    if renderer.renderer.begin_node(node_id, z_index, light_mask) {
                        selected.push(index);
                    }
                }
                // Ordinary application updates still record current content for
                // cold resident tiles. They do not upload or present it. In
                // particular, animation must not invalidate its own capture on
                // every tick and starve the first complete surface submission.
                let began = !selected.is_empty()
                    || (self.prepare_surface_content
                        && self.direct_surface_layer.is_none()
                        && !target.renderers.is_empty());
                if began {
                    self.drawings
                        .begin(layer, node_id, z_index, light_mask, None);
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
        if self
            .direct_surface_layer
            .is_some_and(|owner| owner != layer)
        {
            return false;
        }
        if let Some(owner) = self.recording_layer {
            if owner != layer {
                return false;
            }
            self.drawings.begin(
                layer,
                node_id,
                z_index,
                light_mask,
                (!self.is_capturing_source()).then_some(coverage_bounds),
            );
            return true;
        }
        if self.is_capturing_source() {
            // Feather can read beyond a tile's visible viewport; capture culling owns that margin.
            return self.begin_node(layer, node_id, z_index, light_mask);
        }
        self.invalidate_drawing(layer, node_id);
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
                target.prepare_for_render();
                #[cfg(debug_assertions)]
                self.update_tile_cull_stats(layer, |stats| {
                    stats.nodes_considered += 1;
                });
                let mut selected = Vec::new();
                let mut removed = Vec::new();
                let mut cold_intersection = false;
                #[cfg(debug_assertions)]
                let mut skipped_surfaces = 0u64;
                #[cfg(debug_assertions)]
                let mut stale_surface_removal_attempts = 0u64;
                let direct_indices = self
                    .direct_surface_layer
                    .map(|_| self.targeted_or_all_indices(layer, target.renderers.len()));
                for (index, renderer) in target.renderers.iter_mut().enumerate() {
                    if direct_indices
                        .as_ref()
                        .is_some_and(|indices| !indices.contains(&index))
                    {
                        continue;
                    }
                    if self.prepare_surface_content
                        && !renderer.presentation_ready
                        && self.direct_surface_layer != Some(layer)
                    {
                        cold_intersection |= renderer.intersects_coverage_bounds(&coverage_bounds);
                        continue;
                    }
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
                    if renderer.renderer.begin_node(node_id, z_index, light_mask) {
                        selected.push(index);
                    }
                }
                let began = !selected.is_empty() || cold_intersection;
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
                    self.drawings
                        .begin(layer, node_id, z_index, light_mask, Some(coverage_bounds));
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
        self.drawings.end(layer, node_id);
        if self.recording_layer.is_some() {
            return true;
        }
        let mut ended = false;
        let captured_only = self.prepare_surface_content
            && self
                .active_render_scope(layer)
                .is_some_and(|scope| scope.is_empty());
        self.with_layer_context(layer, |context| {
            ended = context.end_node(node_id);
        });
        self.pop_render_scope(layer);
        ended || captured_only
    }

    fn remove_node(&mut self, layer: usize, node_id: u32) -> bool {
        self.invalidate_drawing(layer, node_id);
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

pub(super) fn to_pax_gpu_fill(
    fill: &pax_runtime_api::Paint,
    rect: kurbo::Rect,
    transform: pax_gpu::Transform2D,
) -> pax_gpu::Fill {
    let Some(rect) = pax_runtime_api::drawing::paint_bounds(rect) else {
        return pax_gpu::Fill::Solid(pax_gpu::Color::rgba(0.0, 0.0, 0.0, 0.0));
    };
    let bounds = (rect.width(), rect.height());
    let orig = rect.origin();
    match fill {
        pax_runtime_api::Paint::MeshGradient(mesh) => {
            if let Err(error) = mesh.validate() {
                thread_local! { static REPORTED: std::cell::RefCell<std::collections::HashSet<String>> = std::cell::RefCell::new(std::collections::HashSet::new()); }
                REPORTED.with(|reported| {
                    if reported.borrow_mut().insert(error.clone()) {
                        log::warn!("{error}; mesh paints no field");
                    }
                });
                return pax_gpu::Fill::Solid(pax_gpu::Color::rgba(0.0, 0.0, 0.0, 0.0));
            }
            let pos = transform.transform_point(point(orig.x as f32, orig.y as f32));
            let main_axis =
                transform.transform_vector(pax_gpu::Vector2D::new(bounds.0 as f32, 0.0));
            let off_axis = transform.transform_vector(pax_gpu::Vector2D::new(0.0, bounds.1 as f32));
            let determinant = main_axis.cross(off_axis);
            if ![
                pos.x,
                pos.y,
                main_axis.x,
                main_axis.y,
                off_axis.x,
                off_axis.y,
                determinant,
            ]
            .iter()
            .all(|v| v.is_finite())
                || determinant.abs() < 1e-6
            {
                return pax_gpu::Fill::Solid(pax_gpu::Color::rgba(0.0, 0.0, 0.0, 0.0));
            }
            pax_gpu::Fill::Mesh {
                rows: mesh
                    .rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|p| {
                                (
                                    [
                                        (p.position.0.evaluate(bounds, Axis::X) / bounds.0) as f32,
                                        (p.position.1.evaluate(bounds, Axis::Y) / bounds.1) as f32,
                                    ],
                                    to_pax_gpu_color(&p.color),
                                )
                            })
                            .collect()
                    })
                    .collect(),
                pos,
                main_axis,
                off_axis,
            }
        }
        pax_runtime_api::Paint::Blend(terms) => pax_gpu::Fill::Blend(
            terms
                .iter()
                .map(|(fill, weight)| (to_pax_gpu_fill(fill, rect, transform), *weight as f32))
                .collect(),
        ),
        pax_runtime_api::Paint::Solid(color) => pax_gpu::Fill::Solid(to_pax_gpu_color(color)),
        pax_runtime_api::Paint::LinearGradient(gradient) => {
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
            let local_axis = local_end - local_pos;
            let local_length = local_axis.length();
            let off_axis =
                transform.transform_vector(pax_gpu::Vector2D::new(-local_axis.y, local_axis.x));
            if local_length <= f32::EPSILON
                || !local_length.is_finite()
                || !main_axis.length().is_finite()
            {
                return pax_gpu::Fill::Solid(pax_gpu::Color::rgba(0.0, 0.0, 0.0, 0.0));
            }
            pax_gpu::Fill::Gradient {
                stops: gradient
                    .stops
                    .iter()
                    .map(|g| pax_gpu::GradientStop {
                        color: to_pax_gpu_color(&g.color),
                        stop: g.position.evaluate((local_length as f64, 0.0), Axis::X) as f32
                            / local_length
                            * main_axis.length(),
                    })
                    .collect(),
                gradient_type: pax_gpu::GradientType::Linear,
                pos: world_pos,
                main_axis,
                off_axis,
            }
        }
        pax_runtime_api::Paint::RadialGradient(gradient) => {
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

pub(super) fn to_pax_gpu_alpha_fill(
    fill: &pax_runtime_api::Paint,
    rect: Rect,
    transform: Transform2D,
) -> pax_gpu::Fill {
    to_pax_gpu_fill(fill, rect, transform)
}

fn to_pax_gpu_material(material: &pax_runtime_api::ResolvedMaterial) -> PixelMaterial {
    match material {
        pax_runtime_api::ResolvedMaterial::Unlit => PixelMaterial::unlit(),
        pax_runtime_api::ResolvedMaterial::Lit(params) => {
            let metallic = params.metallic.clamp(0.0, 1.0) as f32;
            let diffuse = params.diffuse.max(0.0) as f32 * (1.0 - metallic * 0.55);
            let specular = params.specular.max(0.0) as f32 + metallic * 0.45;
            let roughness =
                (params.roughness.clamp(0.0, 1.0) as f32 * (1.0 - metallic * 0.35)).clamp(0.0, 1.0);
            PixelMaterial::custom(
                params.ambient.max(0.0) as f32,
                diffuse,
                specular,
                roughness,
                to_pax_gpu_color(&params.emissive.clone()),
                params.emissive_intensity.max(0.0) as f32,
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

fn replay_identity_priority<'a>(
    layout: &LayerSurfaceLayout,
    targets: impl Iterator<Item = (&'a str, &'a str)>,
) -> i32 {
    targets
        .filter_map(|(key, generation)| {
            layout
                .surfaces
                .iter()
                .find(|s| s.key == key && s.host_signature == generation)
                .map(|s| s.replay_priority)
        })
        .min()
        .unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "shaders")]
    #[test]
    fn cold_shader_capture_requires_atomic_replay_without_caching_unfiltered_leaves() {
        use pax_runtime_api::{ShaderEffect, ShaderProgram};
        static PROGRAM: ShaderProgram = ShaderProgram {
            label: "record-only",
            passes: &[],
        };
        let effect = ShaderEffect {
            program: &PROGRAM,
            parameters: vec![],
            outset: 0.0,
        };
        let mut renderer =
            PaxGpuRenderer::new(|_, _| panic!("recording cannot allocate GPU targets"))
                .with_surface_content_preparation();
        assert!(renderer.begin_surface_recording(0));
        assert!(renderer.begin_shader_effect(
            0,
            1,
            0,
            &effect,
            kurbo::Affine::IDENTITY,
            (32.0, 32.0),
            &[],
            &[2]
        ));
        assert!(renderer.begin_node_with_bounds(0, 2, 1, Rect::new(0.0, 0.0, 32.0, 32.0), 0));
        renderer.end_node(0, 2);
        renderer.end_shader_effect(0, 1);
        assert!(renderer.end_surface_recording(0));
        assert_eq!(
            renderer.missing_surface_drawings(0, &[1, 2]),
            Some(vec![1, 2])
        );
        assert!(renderer.backends.borrow().is_empty());
    }

    #[test]
    fn partial_surface_priority_uses_current_identity_not_host_array_position() {
        let surface = |key: &str, rank| LayerSurfaceEntry {
            key: key.into(),
            host_signature: "g1".into(),
            origin_x: 0.0,
            origin_y: 0.0,
            replay_priority: rank,
            surface: super::super::layer_surface::LayerSurfaceSize {
                logical_width: 1.0,
                logical_height: 1.0,
                surface_width: 1,
                surface_height: 1,
                dpr: [1.0, 1.0],
            },
        };
        let mut layout = LayerSurfaceLayout {
            active: true,
            surfaces: vec![surface("missing", -3), surface("resident", 8)],
        };
        assert_eq!(
            replay_identity_priority(&layout, [("resident", "g1")].into_iter()),
            8
        );
        layout.surfaces[1].replay_priority = -4;
        assert_eq!(
            replay_identity_priority(&layout, [("resident", "g1")].into_iter()),
            -4
        );
        assert_eq!(
            replay_identity_priority(&layout, [("resident", "old")].into_iter()),
            i32::MAX
        );
    }

    #[test]
    fn cold_capture_needs_no_physical_target_and_preserves_clip_state() {
        let mut renderer =
            PaxGpuRenderer::new(|_, _| Box::pin(async { None })).with_surface_content_preparation();
        let bounds = Rect::new(0.0, 0.0, 20.0, 20.0);
        assert!(renderer.begin_surface_recording(3));
        assert!(!renderer.begin_node(4, 1, 0, 0));
        assert!(renderer.begin_node(3, 1, 0, 0));
        renderer.save(3);
        renderer.clip(3, bounds.to_path(0.1));
        renderer.end_node(3, 1);
        assert!(renderer.begin_node_with_bounds(3, 2, 0, bounds, 0));
        renderer.fill_with_opacity(
            3,
            bounds.to_path(0.1),
            &pax_runtime_api::Paint::default(),
            1.0,
        );
        renderer.end_node(3, 2);
        renderer.restore(3);
        assert!(renderer.end_surface_recording(3));
        assert_eq!(renderer.missing_surface_drawings(3, &[2, 3]), Some(vec![3]));
        assert!(renderer.backends.borrow().is_empty());
        assert!(renderer.dirty_render_surfaces.borrow().is_empty());
        let drawing = renderer.drawings.get(3, 2).unwrap();
        assert!(drawing
            .commands
            .iter()
            .any(|c| matches!(c.as_ref(), DrawingCommand::Clip(..))));
        renderer.invalidate_drawing(3, 2);
        assert_eq!(renderer.missing_surface_drawings(3, &[2]), Some(vec![2]));
        assert!(!renderer.begin_surface_fallback(3));
    }

    #[test]
    fn native_renderer_keeps_ordinary_first_draw_by_default() {
        let mut renderer = PaxGpuRenderer::new(|_, _| Box::pin(async { None }));
        assert!(!renderer.prepares_surface_content());
        assert!(!renderer.begin_surface_recording(0));
        assert_eq!(renderer.missing_surface_drawings(0, &[1]), None);
    }

    #[test]
    fn surface_eviction_keeps_survivors_in_host_order() {
        let existing = [("a", "row"), ("b", "row"), ("c", "row")];
        assert_eq!(
            retained_surface_order(&existing, &[("c", "row"), ("a", "row")]),
            Some(vec![2, 0])
        );
        assert_eq!(retained_surface_order(&existing, &[]), Some(vec![]));
        assert_eq!(
            retained_surface_order(&existing, &existing),
            Some(vec![0, 1, 2])
        );
    }

    #[test]
    fn new_or_rebound_surface_cannot_reuse_an_old_backend() {
        let existing = [("a", "row"), ("b", "row")];
        assert_eq!(
            retained_surface_order(&existing, &[("a", "row"), ("c", "row")]),
            Some(vec![0])
        );
        assert_eq!(
            retained_surface_order(&existing, &[("a", "new-host")]),
            Some(vec![])
        );
        assert_eq!(
            retained_surface_order(&existing, &[("a", "row"), ("a", "row")]),
            None
        );
    }

    #[test]
    fn surface_eviction_releases_only_removed_resources() {
        struct Resource(Rc<Cell<usize>>);
        impl Drop for Resource {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }
        let drops = Rc::new(Cell::new(0));
        let mut surfaces: Vec<_> = (0..3)
            .map(|_| Box::new(Resource(Rc::clone(&drops))))
            .collect();
        let first = surfaces[0].as_ref() as *const Resource;
        let last = surfaces[2].as_ref() as *const Resource;
        let order = retained_surface_order(
            &[("a", "row"), ("b", "row"), ("c", "row")],
            &[("c", "row"), ("a", "row")],
        )
        .unwrap();
        retain_surfaces(&mut surfaces, order);
        assert_eq!(drops.get(), 1);
        assert_eq!(surfaces[0].as_ref() as *const Resource, last);
        assert_eq!(surfaces[1].as_ref() as *const Resource, first);
        retain_surfaces(&mut surfaces, vec![]);
        assert_eq!(drops.get(), 3);
        assert!(surfaces.is_empty());
    }

    #[test]
    fn stale_initialization_cannot_publish_into_reused_layer_slot() {
        let states = Rc::new(RefCell::new(vec![RenderLayerState::Pending]));
        let first = Rc::new(LayerInitialization::new(0));
        let replacement = Rc::new(LayerInitialization::new(0));
        first.scheduled.set(true);
        replacement.scheduled.set(true);
        let queue = Rc::new(RefCell::new(VecDeque::from([Rc::clone(&first)])));
        let ready = Rc::new(RefCell::new(Vec::new()));
        let changed = Rc::new(RefCell::new(Vec::new()));
        let calls = Rc::new(Cell::new(0));
        let factory: Rc<LayerFactory> = Rc::new({
            let first = Rc::clone(&first);
            let replacement = Rc::clone(&replacement);
            let queue = Rc::clone(&queue);
            let calls = Rc::clone(&calls);
            let states = Rc::clone(&states);
            let ready = Rc::clone(&ready);
            move |_, _| {
                let first = Rc::clone(&first);
                let replacement = Rc::clone(&replacement);
                let queue = Rc::clone(&queue);
                let calls = Rc::clone(&calls);
                let states = Rc::clone(&states);
                let ready = Rc::clone(&ready);
                Box::pin(async move {
                    calls.set(calls.get() + 1);
                    if calls.get() == 1 {
                        // Removal/recreation during await cancels the old token
                        // and queues a distinct request for the same numeric slot.
                        first.cancelled.set(true);
                        queue.borrow_mut().push_back(replacement);
                    } else {
                        assert!(matches!(states.borrow()[0], RenderLayerState::Pending));
                        assert!(ready.borrow().is_empty(), "stale result was published");
                    }
                    Some(empty_layer(true, true))
                })
            }
        });
        let in_flight = Rc::new(Cell::new(0));
        pump_layer_initialization_queue(
            factory,
            Rc::clone(&states),
            Rc::clone(&queue),
            Rc::clone(&in_flight),
            Rc::clone(&ready),
            changed,
            Rc::new(|_| 0),
            Default::default(),
        );
        assert_eq!(calls.get(), 2);
        assert_eq!(*ready.borrow(), vec![0]);
        assert!(matches!(&states.borrow()[0], RenderLayerState::Ready(_)));
        assert!(!first.scheduled.get());
        assert!(!replacement.scheduled.get());
        assert!(queue.borrow().is_empty());
        assert_eq!(in_flight.get(), 0);
    }

    #[test]
    fn initialization_priority_tracks_visibility_and_keeps_fifo_ties() {
        let queue = RefCell::new(VecDeque::from_iter(
            (0..5).map(|index| Rc::new(LayerInitialization::new(index))),
        ));
        queue.borrow()[2].cancelled.set(true);
        let visible = Cell::new(3);
        let priority = |layer| if layer == visible.get() { 0 } else { 1 };
        assert_eq!(
            pop_next_layer_initialization(&queue, &priority)
                .unwrap()
                .index,
            3
        );
        visible.set(4);
        assert_eq!(
            pop_next_layer_initialization(&queue, &priority)
                .unwrap()
                .index,
            4
        );
        assert_eq!(
            pop_next_layer_initialization(&queue, &priority)
                .unwrap()
                .index,
            0
        );
        assert_eq!(
            pop_next_layer_initialization(&queue, &priority)
                .unwrap()
                .index,
            1
        );
        assert!(pop_next_layer_initialization(&queue, &priority).is_none());
    }

    #[test]
    fn uninstalled_targets_wait_without_consuming_initialization_slots() {
        let queued = Rc::new(LayerInitialization::new(0));
        let queue = RefCell::new(VecDeque::from([Rc::clone(&queued)]));
        assert!(pop_next_layer_initialization(&queue, &|_| i32::MAX).is_none());
        assert_eq!(queue.borrow().len(), 1);
        // A later installed snapshot wakes the same token, not a replacement.
        assert!(Rc::ptr_eq(
            &pop_next_layer_initialization(&queue, &|_| -1).unwrap(),
            &queued
        ));
    }

    #[test]
    fn immediate_initialization_runs_before_executor_dispatch_and_retains_pending_work() {
        let calls = Cell::new(0);
        let mut ready = Box::pin(async {
            calls.set(calls.get() + 1);
            42
        });
        assert_eq!(
            poll_initialization_once(ready.as_mut()),
            std::task::Poll::Ready(42)
        );
        assert_eq!(calls.get(), 1);

        let polls = Cell::new(0);
        let mut pending = Box::pin(std::future::poll_fn(|_| {
            polls.set(polls.get() + 1);
            if polls.get() == 1 {
                std::task::Poll::Pending
            } else {
                std::task::Poll::Ready(17)
            }
        }));
        assert_eq!(
            poll_initialization_once(pending.as_mut()),
            std::task::Poll::Pending
        );
        // The executor receives the same future, not a restarted factory.
        assert_eq!(pollster::block_on(pending), 17);
        assert_eq!(polls.get(), 2);
    }

    #[test]
    fn incremental_initialization_is_opt_in_for_web() {
        let target = LayerTarget::new(vec![], true);
        assert!(!target.incremental_initialization);
        assert!(
            target
                .with_incremental_initialization()
                .incremental_initialization
        );
    }

    #[test]
    fn global_web_replay_drains_visible_before_optional_without_changing_native_order() {
        use super::super::layer_surface::LayerSurfaceSize;
        for incremental in [false, true] {
            let mut renderer = PaxGpuRenderer::new(|_, _| panic!("no GPU needed for ordering"));
            let states = [4, -2, 2, -1]
                .into_iter()
                .map(|priority| {
                    let mut target = LayerTarget::new(Vec::new(), true);
                    target.incremental_initialization = incremental;
                    let provider: Pin<Box<dyn Fn() -> LayerSurfaceLayout>> =
                        Box::pin(move || LayerSurfaceLayout {
                            active: true,
                            surfaces: vec![LayerSurfaceEntry {
                                key: "tile".into(),
                                host_signature: "host".into(),
                                origin_x: 0.0,
                                origin_y: 0.0,
                                replay_priority: priority,
                                surface: LayerSurfaceSize {
                                    logical_width: 100.0,
                                    logical_height: 100.0,
                                    surface_width: 100,
                                    surface_height: 100,
                                    dpr: [1.0, 1.0],
                                },
                            }],
                        });
                    RenderLayerState::Ready((target, provider))
                })
                .collect();
            *renderer.backends.borrow_mut() = states;
            *renderer.replay_layers.borrow_mut() = vec![3, 0, 2, 1, 1];
            if incremental {
                assert_eq!(renderer.take_replay_layer_ids(), vec![1, 3]);
                assert_eq!(renderer.take_replay_layer_ids(), vec![2]);
                assert_eq!(renderer.take_replay_layer_ids(), vec![0]);
            } else {
                assert_eq!(renderer.take_replay_layer_ids(), vec![0, 1, 2, 3]);
            }
            assert!(renderer.take_replay_layer_ids().is_empty());
        }
    }

    #[test]
    fn failed_addition_preserves_ready_layer_and_records_retry_layout() {
        let mut target = LayerTarget::new(vec![], true);
        target.needs_replay = true;
        let layout = LayerSurfaceLayout {
            surfaces: vec![],
            active: true,
        };
        let states = Rc::new(RefCell::new(vec![RenderLayerState::Ready((
            target,
            Box::pin(|| LayerSurfaceLayout {
                surfaces: vec![],
                active: true,
            }),
        ))]));
        let ready = Rc::new(RefCell::new(Vec::new()));
        pump_layer_initialization_queue(
            Rc::new(|_, _| Box::pin(async { None })),
            Rc::clone(&states),
            Rc::new(RefCell::new(VecDeque::from([Rc::new(
                LayerInitialization::new(0),
            )]))),
            Rc::new(Cell::new(0)),
            Rc::clone(&ready),
            Rc::new(RefCell::new(Vec::new())),
            Rc::new(|_| 0),
            Default::default(),
        );
        let states = states.borrow();
        let RenderLayerState::Ready((target, _)) = &states[0] else {
            panic!("survivor lost");
        };
        assert!(target.needs_replay);
        assert_eq!(target.failed_layout, Some(layout));
        assert!(ready.borrow().is_empty());
    }

    #[test]
    fn mesh_paint_resolves_full_bounds_origin_combined_units_and_affine_axes() {
        use pax_runtime_api::{Color, MeshGradient, MeshPoint, Paint, Size};
        let paint = Paint::MeshGradient(MeshGradient {
            rows: vec![
                vec![
                    MeshPoint {
                        position: (
                            Size::Combined(5.into(), 50.into()),
                            Size::Percent(100.into())
                        ),
                        color: Color::CYAN,
                    };
                    2
                ];
                2
            ],
        });
        let transform = pax_gpu::Transform2D::new(2.0, 0.5, 1.0, 3.0, 4.0, 7.0);
        let rect = kurbo::Rect::new(10.0, 20.0, 110.0, 70.0);
        let pax_gpu::Fill::Mesh {
            rows,
            pos,
            main_axis,
            off_axis,
        } = to_pax_gpu_fill(&paint, rect, transform)
        else {
            panic!("mesh")
        };
        assert_eq!(pos, point(44.0, 72.0));
        assert_eq!(main_axis, pax_gpu::Vector2D::new(200.0, 50.0));
        assert_eq!(off_axis, pax_gpu::Vector2D::new(50.0, 150.0));
        assert_eq!(rows[0][0].0, [0.55, 1.0]);
        let pax_gpu::Fill::Mesh {
            rows, main_axis, ..
        } = to_pax_gpu_fill(
            &paint,
            kurbo::Rect::new(10.0, 20.0, 10.0, 70.0),
            pax_gpu::Transform2D::identity(),
        )
        else {
            panic!("canonical zero axis")
        };
        assert_eq!(main_axis.x, 1.0);
        assert_eq!(rows[0][0].0[0], 5.5);
        assert!(matches!(
            to_pax_gpu_fill(&paint, rect, pax_gpu::Transform2D::scale(0.0, 1.0)),
            pax_gpu::Fill::Solid(_)
        ));
    }

    fn empty_layer(active: bool, current_active: bool) -> LayerDef {
        (
            LayerTarget::new(Vec::new(), active),
            Box::pin(move || LayerSurfaceLayout {
                active: current_active,
                surfaces: Vec::new(),
            }),
        )
    }

    #[test]
    fn removed_layer_initializations_cannot_publish_into_reused_indices() {
        let mut renderer = PaxGpuRenderer::new(|_, _| panic!("initialization stays queued"));
        renderer
            .layer_initializations_in_flight
            .set(MAX_CONCURRENT_LAYER_INITIALIZATIONS);
        let dirty = Rc::new(RefCell::new(Vec::new()));
        renderer.resize_layers_to(2, Rc::clone(&dirty));
        let removed = Rc::clone(&renderer.layer_initializations[1]);
        renderer.ready_layers.borrow_mut().push(1);
        renderer.changed_layers.borrow_mut().push(1);
        renderer.replay_layers.borrow_mut().push(1);
        renderer.resize_layers_to(1, Rc::clone(&dirty));
        assert!(removed.cancelled.get());
        assert_eq!(renderer.pending_layer_initializations.borrow().len(), 1);
        assert!(renderer.ready_layers.borrow().is_empty());
        assert!(renderer.replay_layers.borrow().is_empty());
        assert!(renderer.changed_layers.borrow().is_empty());
        assert!(!removed.complete(
            Some(empty_layer(true, true)),
            None,
            &renderer.backends,
            &renderer.ready_layers,
            &renderer.changed_layers
        ));

        renderer.resize_layers_to(2, dirty);
        let replacement = &renderer.layer_initializations[1];
        assert!(!Rc::ptr_eq(&removed, replacement));
        for stale_result in [Some(empty_layer(true, true)), None] {
            assert!(!removed.complete(
                stale_result,
                None,
                &renderer.backends,
                &renderer.ready_layers,
                &renderer.changed_layers
            ));
            assert!(matches!(
                renderer.backends.borrow()[1],
                RenderLayerState::Pending
            ));
            assert!(replacement.scheduled.get());
            assert!(renderer.ready_layers.borrow().is_empty());
        }
        assert!(!replacement.complete(
            Some(empty_layer(true, true)),
            None,
            &renderer.backends,
            &renderer.ready_layers,
            &renderer.changed_layers
        ));
        assert!(matches!(
            renderer.backends.borrow()[1],
            RenderLayerState::Ready(_)
        ));
        assert_eq!(*renderer.ready_layers.borrow(), vec![1]);
    }

    #[test]
    fn live_layer_initialization_retries_changed_hosts_and_reports_failure() {
        let initialization = LayerInitialization::new(0);
        let backends = RefCell::new(vec![RenderLayerState::Pending]);
        let ready = RefCell::new(Vec::new());
        assert!(initialization.complete(
            Some(empty_layer(true, false)),
            None,
            &backends,
            &ready,
            &RefCell::new(Vec::new())
        ));
        assert!(matches!(backends.borrow()[0], RenderLayerState::Pending));
        assert!(ready.borrow().is_empty());
        assert!(!initialization.complete(None, None, &backends, &ready, &RefCell::new(Vec::new())));
        assert!(matches!(backends.borrow()[0], RenderLayerState::Failed));
    }

    #[test]
    fn alpha_radial_gradient_retains_radius_with_coincident_endpoints() {
        use pax_runtime_api::{Paint, RadialGradient, Size};
        let fill = Paint::RadialGradient(RadialGradient {
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
        use pax_runtime_api::{Color, GradientStop, Paint, RadialGradient, Size};
        let fill = Paint::RadialGradient(RadialGradient {
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
        use pax_runtime_api::{LinearGradient, Paint, Size};
        let fill = Paint::LinearGradient(LinearGradient {
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

#[cfg(all(test, target_os = "macos"))]
#[path = "pax_gpu_tile_tests.rs"]
mod tile_tests;
