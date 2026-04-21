use kurbo::{BezPath, PathEl, Rect, Shape};
use pax_gpu::{
    point, Box2D, Image, Path, Stroke as PixelStroke, StrokeCap as PixelStrokeCap, Transform2D,
    WgpuRenderer,
};
use pax_runtime_api::{
    Axis, LayerSurfaceScreenshotData, RenderContext, ScreenshotData, Stroke, StrokeCap,
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
    origin_only_resets: u64,
}

/// Desired surface geometry for one tile in a layer layout.
pub struct LayerSurfaceEntry {
    pub key: String,
    pub host_signature: String,
    pub origin_x: f32,
    pub origin_y: f32,
    pub replay_priority: i32,
    pub surface: LayerSurfaceSize,
}

/// Desired set of physical surfaces for a logical layer.
pub struct LayerSurfaceLayout {
    pub surfaces: Vec<LayerSurfaceEntry>,
    pub active: bool,
}

type LayerDef = (LayerTarget, Pin<Box<dyn Fn() -> LayerSurfaceLayout>>);

#[derive(Clone, Copy, Debug, Default)]
enum LayoutChangeKind {
    #[default]
    Unchanged,
    OriginOnly,
    Resized,
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
}

fn surface_intersects_coverage_bounds(
    bounds: &Rect,
    origin_x: f64,
    origin_y: f64,
    width: f64,
    height: f64,
) -> bool {
    if !bounds.x0.is_finite()
        || !bounds.y0.is_finite()
        || !bounds.x1.is_finite()
        || !bounds.y1.is_finite()
    {
        return true;
    }

    let x1 = origin_x + width;
    let y1 = origin_y + height;
    bounds.x1 >= origin_x && bounds.x0 <= x1 && bounds.y1 >= origin_y && bounds.y0 <= y1
}

impl LayerTarget {
    /// Create a layer target from physical surface renderers.
    pub fn new(renderers: Vec<LayerRenderer>, active: bool) -> Self {
        // The first tiled pass keeps node drawing opaque by replaying the same retained scene into
        // each active physical surface. That duplicates retained scene state per tile, but it keeps
        // tiling below the RenderContext seam so primitives stay unaware of browser-surface
        // partitioning.
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

fn replay_batches_by_priority(mut entries: Vec<(usize, i32)>) -> Vec<Vec<usize>> {
    if entries.is_empty() {
        return Vec::new();
    }
    entries.sort_by_key(|(index, priority)| (*priority, *index));

    let first_priority = entries[0].1;
    // A browser compositor can expose the first warm row before the next RAF reaches the second
    // replay batch. Include the nearest off-viewport ring with the visible batch; farther warm
    // tiles still defer to later frames and remain cancellable by newer scroll plans.
    let urgent_cutoff = if first_priority == 0 {
        2
    } else {
        first_priority
    };
    let mut urgent = Vec::new();
    let mut remaining = Vec::new();
    for (index, priority) in entries {
        if priority <= urgent_cutoff {
            urgent.push(index);
        } else {
            remaining.push(index);
        }
    }

    if remaining.is_empty() {
        vec![urgent]
    } else {
        vec![urgent, remaining]
    }
}

#[cfg(test)]
mod replay_priority_tests {
    use super::replay_batches_by_priority;

    #[test]
    fn visible_batch_includes_nearest_warm_ring() {
        let batches = replay_batches_by_priority(vec![(3, 4), (0, 0), (2, 2), (1, 0)]);

        assert_eq!(batches[0], vec![0, 1, 2]);
        assert_eq!(batches[1], vec![3]);
    }
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
                        layer_layout_matches_target(&layer_def.0, &current_layout);
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
/// Logical and backing-pixel dimensions for one physical surface.
pub struct LayerSurfaceSize {
    pub logical_width: f32,
    pub logical_height: f32,
    pub surface_width: u32,
    pub surface_height: u32,
    pub dpr: [f32; 2],
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
    targeted_replay_queues: RefCell<Vec<VecDeque<Vec<usize>>>>,
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
            targeted_replay_queues: Default::default(),
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

    fn targeted_replay_scope(&self, layer: usize) -> Option<Vec<usize>> {
        self.targeted_replay_queues
            .borrow()
            .get(layer)
            .and_then(|queue| queue.front().cloned())
    }

    fn set_targeted_replay_batches(&self, layer: usize, batches: Vec<Vec<usize>>) {
        let mut queue = VecDeque::new();
        for mut batch in batches {
            batch.sort_unstable();
            batch.dedup();
            if !batch.is_empty() {
                queue.push_back(batch);
            }
        }
        if queue.is_empty() {
            return;
        }

        let mut queues = self.targeted_replay_queues.borrow_mut();
        if queues.len() <= layer {
            queues.resize_with(layer + 1, VecDeque::new);
        }
        queues[layer] = queue;
    }

    fn advance_targeted_replay_queue(&self, layer: usize) -> bool {
        let mut queues = self.targeted_replay_queues.borrow_mut();
        let Some(queue) = queues.get_mut(layer) else {
            return false;
        };
        queue.pop_front();
        !queue.is_empty()
    }

    fn flush_targeted_or_dirty(&self, layer: usize, target: &mut LayerTarget) -> bool {
        let targeted_indices = self.targeted_replay_scope(layer);
        let dirty_indices = self.dirty_render_surface_scope(layer);
        let used_targeted_replay = targeted_indices.is_some();
        let mut indices = targeted_indices.unwrap_or_default();
        if let Some(dirty_indices) = dirty_indices {
            indices.extend(dirty_indices);
        }
        indices.sort_unstable();
        indices.dedup();

        if !indices.is_empty() {
            for index in &indices {
                if let Some(renderer) = target.renderers.get_mut(*index) {
                    renderer.renderer.flush();
                }
            }
            self.clear_dirty_render_surfaces(layer, &indices);
        }

        used_targeted_replay
    }

    fn clear_targeted_replay_scope(&self, layer: usize) {
        if let Some(queue) = self.targeted_replay_queues.borrow_mut().get_mut(layer) {
            queue.clear();
        }
    }

    fn targeted_or_all_indices(&self, layer: usize, renderer_count: usize) -> Vec<usize> {
        self.targeted_replay_scope(layer)
            .unwrap_or_else(|| (0..renderer_count).collect())
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
            if stats.nodes_considered > 0 || stats.origin_only_resets > 0 {
                log::trace!(
                    "[pax-tile-cull] layer={} nodes={} selected_surfaces={} skipped_surfaces={} stale_removal_attempts={} origin_only_resets={}",
                    layer,
                    stats.nodes_considered,
                    stats.selected_surfaces,
                    stats.skipped_surfaces,
                    stats.stale_surface_removal_attempts,
                    stats.origin_only_resets
                );
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
                    let mut needs_full_layer_replay = false;
                    for (index, (surface, renderer)) in layout
                        .surfaces
                        .iter()
                        .zip(target.renderers.iter_mut())
                        .enumerate()
                    {
                        let layout_change = renderer.update_layout(surface);
                        match layout_change {
                            LayoutChangeKind::Unchanged => {}
                            LayoutChangeKind::OriginOnly => {
                                // Reassigning a stable ring slot to a new absolute tile origin
                                // clears the retained scene for that physical surface. The chassis
                                // must replay the logical layer contents into it on the next tick.
                                targeted_replay_entries.push((index, surface.replay_priority));
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
                        self.mark_render_surfaces_dirty(layer_index, 0..target.renderers.len());
                        self.replay_layers.borrow_mut().push(layer_index);
                    } else if !targeted_replay_entries.is_empty() {
                        #[cfg(debug_assertions)]
                        self.update_tile_cull_stats(layer_index, |stats| {
                            stats.origin_only_resets += targeted_replay_entries.len() as u64;
                        });
                        self.set_targeted_replay_batches(
                            layer_index,
                            replay_batches_by_priority(targeted_replay_entries),
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
                },
                opacity as f32,
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

    fn draw_image(&mut self, layer: usize, image_path: &str, rect: kurbo::Rect) {
        self.with_layer_context(layer, |context| {
            if let Some(image) = self.image_map.get(image_path) {
                let version = *self.image_versions.get(image_path).unwrap_or(&0);
                context.draw_image(
                    image_path,
                    version,
                    image,
                    Box2D {
                        min: point(rect.x0 as f32, rect.y0 as f32),
                        max: point(rect.x1 as f32, rect.y1 as f32),
                    },
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

    fn take_ready_canvas_layers(&mut self) -> Vec<usize> {
        let mut ready_layers = self.ready_layers.borrow_mut();
        let mut ready = std::mem::take(&mut *ready_layers);
        ready.sort_unstable();
        ready.dedup();
        ready
    }

    fn take_replay_canvas_layers(&mut self) -> Vec<usize> {
        let mut replay_layers = self.replay_layers.borrow_mut();
        let mut replay = std::mem::take(&mut *replay_layers);
        replay.sort_unstable();
        replay.dedup();
        replay
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

    fn begin_node(&mut self, layer: usize, node_id: u32, z_index: i32) -> bool {
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
                let candidate_indices = self.targeted_or_all_indices(layer, target.renderers.len());
                let mut selected = Vec::new();
                for index in candidate_indices {
                    let Some(renderer) = target.renderers.get_mut(index) else {
                        continue;
                    };
                    if renderer.renderer.begin_node(node_id, z_index) {
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
                target.prepare_for_render();
                let candidate_indices = self.targeted_or_all_indices(layer, target.renderers.len());
                let renderer_count = candidate_indices.len() as u64;
                #[cfg(debug_assertions)]
                self.update_tile_cull_stats(layer, |stats| {
                    stats.nodes_considered += 1;
                });
                let mut selected = Vec::new();
                let mut removed = Vec::new();
                for index in candidate_indices {
                    let Some(renderer) = target.renderers.get_mut(index) else {
                        continue;
                    };
                    if !renderer.intersects_coverage_bounds(&coverage_bounds) {
                        // If a dirty node moved out of this tile, skipping begin_node is not
                        // enough: the renderer may still retain that node from an earlier frame.
                        if renderer.renderer.remove_node(node_id) {
                            removed.push(index);
                        }
                        continue;
                    }
                    if renderer.renderer.begin_node(node_id, z_index) {
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
                    stats.skipped_surfaces += renderer_count.saturating_sub(selected_count);
                    stats.stale_surface_removal_attempts +=
                        renderer_count.saturating_sub(selected_count);
                });
                if began {
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
            let start_x = gradient.start.0.evaluate(bounds, Axis::X);
            let start_y = gradient.start.1.evaluate(bounds, Axis::Y);
            let end_x = gradient.end.0.evaluate(bounds, Axis::X);
            let end_y = gradient.end.1.evaluate(bounds, Axis::Y);
            let r = gradient.radius as f32;
            let local_pos =
                pax_gpu::Point2D::new((orig.x + start_x) as f32, (orig.y + start_y) as f32);
            let local_main_axis =
                pax_gpu::Vector2D::new(r * (end_x - start_x) as f32, r * (end_y - start_y) as f32);
            let local_off_axis = pax_gpu::Vector2D::new(-local_main_axis.y, local_main_axis.x);
            let world_pos = transform.transform_point(local_pos);
            let world_main_axis = transform.transform_point(pax_gpu::Point2D::new(
                local_pos.x + local_main_axis.x,
                local_pos.y + local_main_axis.y,
            )) - world_pos;
            let world_off_axis = transform.transform_point(pax_gpu::Point2D::new(
                local_pos.x + local_off_axis.x,
                local_pos.y + local_off_axis.y,
            )) - world_pos;
            pax_gpu::Fill::Gradient {
                gradient_type: pax_gpu::GradientType::Radial,
                pos: world_pos,
                main_axis: world_main_axis,
                off_axis: world_off_axis,
                stops: gradient
                    .stops
                    .iter()
                    .map(|g| pax_gpu::GradientStop {
                        color: to_pax_gpu_color(&g.color),
                        stop: g
                            .position
                            .evaluate((world_main_axis.length() as f64, 0.0), Axis::X)
                            as f32,
                    })
                    .collect(),
            }
        }
    }
}

/// Convert a runtime API color into the `pax-gpu` render-context color.
pub fn to_pax_gpu_color(color: &pax_runtime_api::Color) -> pax_gpu::Color {
    let [r, g, b, a] = color.to_rgba_0_1();
    pax_gpu::Color::rgba(r as f32, g as f32, b as f32, a as f32)
}

/// Convert a kurbo path emitted by primitives into a lyon path consumed by `pax-gpu`.
pub fn convert_kurbo_to_lyon_path(kurbo_path: &BezPath) -> Path {
    let mut builder = Path::builder();
    let mut closed = false;
    for el in kurbo_path.elements() {
        match el {
            PathEl::MoveTo(p) => {
                closed = false;
                builder.begin(point(p.x as f32, p.y as f32));
            }
            PathEl::LineTo(p) => {
                builder.line_to(point(p.x as f32, p.y as f32));
            }
            PathEl::QuadTo(p1, p2) => {
                builder.quadratic_bezier_to(
                    point(p1.x as f32, p1.y as f32),
                    point(p2.x as f32, p2.y as f32),
                );
            }
            PathEl::CurveTo(p1, p2, p3) => {
                builder.cubic_bezier_to(
                    point(p1.x as f32, p1.y as f32),
                    point(p2.x as f32, p2.y as f32),
                    point(p3.x as f32, p3.y as f32),
                );
            }
            PathEl::ClosePath => {
                closed = true;
                builder.end(true);
            }
        }
    }
    if !closed {
        builder.end(false);
    }

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
