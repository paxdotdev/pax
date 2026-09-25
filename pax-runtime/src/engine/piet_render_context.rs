use pax_runtime_api::{Fill, Stroke, StrokeCap, StrokeJoin};
use piet::{
    kurbo::{self, Affine, Shape},
    FixedRadialGradient, LineCap, LineJoin, LinearGradient, StrokeStyle,
};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use super::layer_surface::{
    replay_batches_by_directional_priority, surface_intersects_coverage_bounds, LayerSurfaceEntry,
    LayerSurfaceLayout, LayoutChangeKind, ReplayPriorityEntry, SurfaceReplayCoordinator,
};
#[cfg(debug_assertions)]
use super::layer_surface::{visible_surface_escape, VisibleSurfaceEscape};
use crate::api;

#[cfg(debug_assertions)]
fn log_visible_surface_escape(layer: usize, escape: VisibleSurfaceEscape) {
    log::warn!(
        "[pax-tile-window-escape] backend=piet layer={} visible_surfaces={} escaped_visible_surfaces={} max_gap_x={:.1} max_gap_y={:.1} previous_bounds={} visible_bounds={}",
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
fn format_rect(bounds: kurbo::Rect) -> String {
    format!(
        "{:.1},{:.1}-{:.1},{:.1}",
        bounds.x0, bounds.y0, bounds.x1, bounds.y1
    )
}

struct ImgData<R: piet::RenderContext> {
    img: R::Image,
    size: (usize, usize),
}

type ClearFn = Box<dyn Fn()>;
type ConfigureFn = Box<dyn Fn(f32, f32, u32, u32, [f32; 2], bool)>;
type DrawBlendFn<R> = Box<dyn FnMut(&mut R, &kurbo::BezPath, &[(Fill, f64)], f64)>;
type DrawImageFn<R> = Box<dyn Fn(&mut R, &<R as piet::RenderContext>::Image, kurbo::Rect, f64)>;
type LayerDef<R> = (PietLayerTarget<R>, Box<dyn Fn() -> LayerSurfaceLayout>);

/// Retained metadata for one piet-backed browser canvas surface.
pub struct PietLayerRenderer<R: piet::RenderContext> {
    key: String,
    host_signature: String,
    context: R,
    clear_fn: ClearFn,
    configure_fn: ConfigureFn,
    draw_image_fn: DrawImageFn<R>,
    draw_blend_fn: DrawBlendFn<R>,
    origin_x: f32,
    origin_y: f32,
    logical_width: f32,
    logical_height: f32,
    surface_width: u32,
    surface_height: u32,
    dpr: [f32; 2],
}

impl<R: piet::RenderContext> PietLayerRenderer<R> {
    pub fn new(
        key: String,
        host_signature: String,
        context: R,
        clear_fn: ClearFn,
        configure_fn: ConfigureFn,
        draw_image_fn: DrawImageFn<R>,
        draw_blend_fn: DrawBlendFn<R>,
        surface: &LayerSurfaceEntry,
    ) -> Self {
        Self {
            key,
            host_signature,
            context,
            clear_fn,
            configure_fn,
            draw_image_fn,
            draw_blend_fn,
            origin_x: surface.origin_x,
            origin_y: surface.origin_y,
            logical_width: surface.surface.logical_width,
            logical_height: surface.surface.logical_height,
            surface_width: surface.surface.surface_width,
            surface_height: surface.surface.surface_height,
            dpr: surface.surface.dpr,
        }
    }

    fn context_mut(&mut self) -> &mut R {
        &mut self.context
    }

    fn clear(&self) {
        (self.clear_fn)();
    }

    fn intersects_coverage_bounds(&self, bounds: &kurbo::Rect) -> bool {
        surface_intersects_coverage_bounds(
            bounds,
            self.origin_x as f64,
            self.origin_y as f64,
            self.logical_width as f64,
            self.logical_height as f64,
        )
    }

    fn coverage_bounds(&self) -> kurbo::Rect {
        kurbo::Rect::new(
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

        if size_changed || origin_changed {
            (self.configure_fn)(
                self.origin_x,
                self.origin_y,
                self.surface_width,
                self.surface_height,
                self.dpr,
                size_changed,
            );
        }

        if size_changed {
            self.clear();
        }

        if size_changed {
            LayoutChangeKind::Resized
        } else if origin_changed {
            LayoutChangeKind::OriginOnly
        } else {
            LayoutChangeKind::Unchanged
        }
    }
}

/// Current piet surface set for one logical layer.
pub struct PietLayerTarget<R: piet::RenderContext> {
    renderers: Vec<PietLayerRenderer<R>>,
    active: bool,
}

impl<R: piet::RenderContext> PietLayerTarget<R> {
    pub fn new(renderers: Vec<PietLayerRenderer<R>>, active: bool) -> Self {
        Self { renderers, active }
    }

    fn activate(&mut self) -> bool {
        let changed = !self.active;
        self.active = true;
        changed
    }

    fn deactivate(&mut self) {
        self.active = false;
    }
}

/// `RenderContext` implementation backed by piet.
pub struct PietRenderer<R: piet::RenderContext> {
    layers: Vec<LayerDef<R>>,
    image_map: HashMap<String, ImgData<R>>,
    layer_factory: Box<dyn Fn(usize) -> LayerDef<R>>,
    ready_layers: Vec<usize>,
    replay_layers: Vec<usize>,
    surface_replay: SurfaceReplayCoordinator,
    active_render_scopes: Vec<Vec<Vec<usize>>>,
    clean_skipped_canvas_nodes: HashSet<(usize, u32)>,
}

impl<R: piet::RenderContext> PietRenderer<R> {
    /// Create a piet renderer with a chassis-provided logical layer factory.
    pub fn new(layer_factory: impl Fn(usize) -> LayerDef<R> + 'static) -> Self {
        Self {
            layer_factory: Box::new(layer_factory),
            layers: Vec::new(),
            image_map: HashMap::new(),
            ready_layers: Vec::new(),
            replay_layers: Vec::new(),
            surface_replay: SurfaceReplayCoordinator::default(),
            active_render_scopes: Vec::new(),
            clean_skipped_canvas_nodes: HashSet::new(),
        }
    }

    fn first_context_mut(&mut self) -> Option<&mut R> {
        self.layers
            .iter_mut()
            .find_map(|(target, _)| target.renderers.first_mut())
            .map(PietLayerRenderer::context_mut)
    }

    fn with_layer_context(&mut self, layer: usize, mut f: impl FnMut(&mut R)) {
        self.with_layer_renderer(layer, |renderer| f(renderer.context_mut()));
    }

    fn with_layer_renderer(&mut self, layer: usize, mut f: impl FnMut(&mut PietLayerRenderer<R>)) {
        let scoped_indices = self
            .active_render_scopes
            .get(layer)
            .and_then(|scopes| scopes.last())
            .cloned();

        let Some((target, _)) = self.layers.get_mut(layer) else {
            return;
        };
        if !target.active {
            return;
        }

        if let Some(indices) = scoped_indices {
            for index in indices {
                if let Some(renderer) = target.renderers.get_mut(index) {
                    f(renderer);
                }
            }
        } else {
            for renderer in &mut target.renderers {
                f(renderer);
            }
        }
    }

    fn push_render_scope(&mut self, layer: usize, renderer_indices: Vec<usize>) {
        if self.active_render_scopes.len() <= layer {
            self.active_render_scopes.resize_with(layer + 1, Vec::new);
        }
        self.active_render_scopes[layer].push(renderer_indices);
    }

    fn pop_render_scope(&mut self, layer: usize) {
        if let Some(scopes) = self.active_render_scopes.get_mut(layer) {
            scopes.pop();
        }
    }

    fn take_replay_layer_ids(&mut self) -> Vec<usize> {
        let mut replay = std::mem::take(&mut self.replay_layers);
        replay.sort_unstable();
        replay.dedup();
        replay
    }

    fn targeted_or_all_indices(&self, layer: usize, renderer_count: usize) -> Vec<usize> {
        self.surface_replay
            .targeted_or_all_indices(layer, renderer_count)
    }

    fn clear_targeted_replay_scope(&mut self, layer: usize) {
        self.surface_replay.clear_targeted_replay_scope(layer);
    }

    fn set_targeted_replay_batches(
        &mut self,
        layer: usize,
        batches: Vec<Vec<usize>>,
        bounds_by_surface: HashMap<usize, Vec<kurbo::Rect>>,
    ) {
        self.surface_replay
            .set_targeted_replay_batches(layer, batches, bounds_by_surface);
    }

    fn advance_targeted_replay_queue(&mut self, layer: usize) -> bool {
        self.surface_replay.advance_targeted_replay_queue(layer)
    }

    fn remember_canvas_node_coverage(
        &mut self,
        layer: usize,
        node_id: u32,
        coverage_bounds: kurbo::Rect,
    ) {
        self.surface_replay
            .remember_canvas_node_coverage(layer, node_id, coverage_bounds);
    }

    fn forget_canvas_node_coverage(&mut self, layer: usize, node_id: u32) {
        self.surface_replay
            .forget_canvas_node_coverage(layer, node_id);
    }

    fn targeted_replay_surface_bounds(
        &self,
        layer: usize,
        target: &PietLayerTarget<R>,
    ) -> Option<Vec<kurbo::Rect>> {
        self.surface_replay
            .targeted_replay_surface_bounds(layer, |index| {
                target
                    .renderers
                    .get(index)
                    .map(PietLayerRenderer::coverage_bounds)
            })
    }

    fn spatial_replay_node_ids(&self, layer: usize) -> Option<Vec<u32>> {
        let Some((target, _)) = self.layers.get(layer) else {
            return None;
        };
        let surface_bounds = self.targeted_replay_surface_bounds(layer, target)?;
        self.surface_replay
            .spatial_replay_node_ids_for_surface_bounds(layer, &surface_bounds)
    }

    fn refresh_layer_layouts<I>(&mut self, layer_indices: I)
    where
        I: IntoIterator<Item = usize>,
    {
        for layer_index in layer_indices {
            let Some((target, layout_provider)) = self.layers.get_mut(layer_index) else {
                continue;
            };
            let layout = (layout_provider)();
            let layout_matches = layer_layout_matches_target(target, &layout);

            if !layout.active {
                target.deactivate();
                self.clear_targeted_replay_scope(layer_index);
                continue;
            }

            if !layout_matches {
                self.clear_targeted_replay_scope(layer_index);
                let (new_target, new_provider) = (self.layer_factory)(layer_index);
                let has_surfaces = !new_target.renderers.is_empty();
                self.layers[layer_index] = (new_target, new_provider);
                if has_surfaces {
                    self.ready_layers.push(layer_index);
                    self.replay_layers.push(layer_index);
                }
                continue;
            }

            let activated_replay = target.activate() && !target.renderers.is_empty();

            let mut targeted_replay_entries = Vec::new();
            let mut targeted_replay_bounds = HashMap::new();
            let mut needs_full_layer_replay = false;
            #[cfg(debug_assertions)]
            {
                let previous_surface_bounds: Vec<_> = target
                    .renderers
                    .iter()
                    .map(PietLayerRenderer::coverage_bounds)
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
                        targeted_replay_bounds.insert(index, vec![previous_bounds, current_bounds]);
                    }
                    LayoutChangeKind::Resized => {
                        needs_full_layer_replay = true;
                    }
                }
            }
            if activated_replay || needs_full_layer_replay {
                self.clear_targeted_replay_scope(layer_index);
                self.replay_layers.push(layer_index);
            } else if !targeted_replay_entries.is_empty() {
                self.set_targeted_replay_batches(
                    layer_index,
                    piet_replay_batches_for_retarget(targeted_replay_entries),
                    targeted_replay_bounds,
                );
                self.replay_layers.push(layer_index);
            }
        }
    }
}

fn piet_replay_batches_for_retarget(entries: Vec<ReplayPriorityEntry>) -> Vec<Vec<usize>> {
    let batches = replay_batches_by_directional_priority(&entries);
    let visible_indices: HashSet<_> = entries
        .iter()
        .filter(|entry| entry.priority <= 0)
        .map(|entry| entry.index)
        .collect();
    if visible_indices.is_empty() {
        return batches;
    }

    let mut visible_batch = Vec::new();
    let mut warm_batches = Vec::new();
    for batch in batches {
        let mut warm_batch = Vec::new();
        for index in batch {
            if visible_indices.contains(&index) {
                visible_batch.push(index);
            } else {
                warm_batch.push(index);
            }
        }
        if !warm_batch.is_empty() {
            warm_batches.push(warm_batch);
        }
    }

    let mut coalesced_batches = Vec::with_capacity(warm_batches.len() + 1);
    coalesced_batches.push(visible_batch);
    coalesced_batches.extend(warm_batches);
    coalesced_batches
}

fn layer_layout_matches_target<R: piet::RenderContext>(
    target: &PietLayerTarget<R>,
    layout: &LayerSurfaceLayout,
) -> bool {
    layout.surfaces.len() == target.renderers.len()
        && layout
            .surfaces
            .iter()
            .zip(target.renderers.iter())
            .all(|(surface, renderer)| {
                surface.key == renderer.key && surface.host_signature == renderer.host_signature
            })
}

#[cfg(test)]
mod tests {
    use super::piet_replay_batches_for_retarget;
    use crate::engine::layer_surface::ReplayPriorityEntry;
    use kurbo::Rect;

    #[test]
    fn piet_replay_batches_coalesce_visible_surfaces_first() {
        let batches = piet_replay_batches_for_retarget(vec![
            replay_entry(0, 0, 0.0, 100.0),
            replay_entry(1, 0, 100.0, 200.0),
            replay_entry(2, 1, 200.0, 300.0),
            replay_entry(3, 1, -100.0, 0.0),
        ]);

        assert_eq!(batches[0], vec![1, 0]);
        assert_eq!(batches[1], vec![2]);
        assert_eq!(batches[2], vec![3]);
    }

    #[test]
    fn piet_replay_batches_keep_warm_only_directional_batches() {
        let batches = piet_replay_batches_for_retarget(vec![
            replay_entry(0, 1, 0.0, 100.0),
            replay_entry(1, 1, 100.0, 200.0),
            replay_entry(2, 1, -100.0, 0.0),
        ]);

        assert_eq!(batches, vec![vec![1], vec![0], vec![2]]);
    }

    fn replay_entry(
        index: usize,
        priority: i32,
        previous_y: f64,
        current_y: f64,
    ) -> ReplayPriorityEntry {
        ReplayPriorityEntry::new(
            index,
            priority,
            Rect::new(0.0, previous_y, 100.0, previous_y + 100.0),
            Rect::new(0.0, current_y, 100.0, current_y + 100.0),
        )
    }
}

impl<R: piet::RenderContext> api::RenderContext for PietRenderer<R> {
    fn fill_with_opacity(&mut self, layer: usize, path: kurbo::BezPath, fill: &Fill, opacity: f64) {
        if let Fill::Blend(terms) = fill {
            self.with_layer_renderer(layer, |renderer| {
                (renderer.draw_blend_fn)(&mut renderer.context, &path, terms, opacity);
            });
        } else if let Some(brush) =
            fill_to_piet_brush(&fill.with_alpha_factor(opacity), path.bounding_box())
        {
            self.with_layer_context(layer, |context| context.fill(path.clone(), &brush));
        }
    }

    fn stroke_with_opacity(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        stroke: &Stroke,
        opacity: f64,
    ) {
        let rect = path.bounding_box();
        let brush = fill_to_piet_brush(
            &Fill::Solid(stroke.color.get()).with_alpha_factor(opacity),
            rect,
        )
        .expect("solid strokes have a Piet brush");
        let width = stroke.width.get().expect_pixels().to_float();
        let style = stroke_to_piet_style(stroke);
        self.with_layer_context(layer, |context| {
            context.stroke_styled(path.clone(), &brush, width, &style)
        });
    }

    fn stroke_with_draw_range_and_material_and_opacity(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        stroke: &Stroke,
        _material: &api::Material,
        opacity: f64,
        draw_start: f64,
        draw_end: f64,
    ) {
        let draw_start = draw_start.clamp(0.0, 1.0);
        let draw_end = draw_end.clamp(0.0, 1.0);
        if draw_start >= draw_end {
            return;
        }
        let path = if draw_start <= f64::EPSILON && draw_end >= 1.0 - f64::EPSILON {
            path
        } else {
            api::drawing::path_trim::trim_bez_path(&path, draw_start, draw_end)
        };
        self.stroke_with_opacity(layer, path, stroke, opacity);
    }

    fn save(&mut self, layer: usize) {
        self.with_layer_context(layer, |context| {
            let _ = context.save();
        });
    }

    fn transform(&mut self, layer: usize, affine: Affine) {
        self.with_layer_context(layer, |context| context.transform(affine));
    }

    fn clip(&mut self, layer: usize, path: kurbo::BezPath) {
        self.with_layer_context(layer, |context| context.clip(path.clone()));
    }

    fn restore(&mut self, layer: usize) {
        self.with_layer_context(layer, |context| {
            let _ = context.restore();
        });
    }

    fn load_image(&mut self, path: &str, buf: &[u8], width: usize, height: usize) {
        let Some(render_context) = self.first_context_mut() else {
            return;
        };
        let img = render_context
            .make_image(width, height, buf, piet::ImageFormat::RgbaSeparate)
            .expect("image creation successful");
        self.image_map.insert(
            path.to_owned(),
            ImgData {
                img,
                size: (width, height),
            },
        );
    }

    fn get_image_size(&mut self, image_path: &str) -> Option<(usize, usize)> {
        self.image_map.get(image_path).map(|img| img.size)
    }

    fn draw_image_with_opacity(
        &mut self,
        layer: usize,
        image_path: &str,
        rect: kurbo::Rect,
        opacity: f64,
    ) {
        let Some(data) = self.image_map.get(image_path) else {
            return;
        };
        let scoped_indices = self
            .active_render_scopes
            .get(layer)
            .and_then(|scopes| scopes.last())
            .cloned();
        let Some((target, _)) = self.layers.get_mut(layer) else {
            return;
        };
        if !target.active {
            return;
        }
        if let Some(indices) = scoped_indices {
            for index in indices {
                if let Some(renderer) = target.renderers.get_mut(index) {
                    (renderer.draw_image_fn)(
                        &mut renderer.context,
                        &data.img,
                        rect,
                        opacity.clamp(0.0, 1.0),
                    );
                }
            }
        } else {
            for renderer in &mut target.renderers {
                (renderer.draw_image_fn)(
                    &mut renderer.context,
                    &data.img,
                    rect,
                    opacity.clamp(0.0, 1.0),
                );
            }
        }
    }

    fn layers(&self) -> usize {
        self.layers.len()
    }

    fn resize_layers_to(&mut self, layer_count: usize, dirty_canvases: Rc<RefCell<Vec<bool>>>) {
        let current_len = self.layers.len();
        match layer_count.cmp(&current_len) {
            std::cmp::Ordering::Less => {
                self.layers.truncate(layer_count);
                self.active_render_scopes.truncate(layer_count);
                self.surface_replay.truncate_layers(layer_count);
            }
            std::cmp::Ordering::Equal => return,
            std::cmp::Ordering::Greater => {
                for i in current_len..layer_count {
                    let (target, provider) = (self.layer_factory)(i);
                    let has_surfaces = target.active && !target.renderers.is_empty();
                    self.layers.push((target, provider));
                    if let Some(dirty_bit) = dirty_canvases.borrow_mut().get_mut(i) {
                        *dirty_bit = true;
                    }
                    if has_surfaces {
                        self.ready_layers.push(i);
                    }
                }
            }
        }
    }

    fn image_loaded(&self, path: &str) -> bool {
        self.image_map.contains_key(path)
    }

    fn clear(&mut self, layer: usize) {
        let scoped_indices = self
            .active_render_scopes
            .get(layer)
            .and_then(|scopes| scopes.last())
            .cloned()
            .or_else(|| {
                self.layers
                    .get(layer)
                    .map(|(target, _)| self.targeted_or_all_indices(layer, target.renderers.len()))
            });
        let Some((target, _)) = self.layers.get_mut(layer) else {
            return;
        };
        if !target.active {
            return;
        }
        if let Some(indices) = scoped_indices {
            for index in indices {
                if let Some(renderer) = target.renderers.get(index) {
                    renderer.clear();
                }
            }
        } else {
            for renderer in &target.renderers {
                renderer.clear();
            }
        }
    }

    fn flush(&mut self, layer: usize, _dirty_canvases: Rc<RefCell<Vec<bool>>>) {
        if self.advance_targeted_replay_queue(layer) {
            self.replay_layers.push(layer);
        }
    }

    fn resize(&mut self, _width: usize, _height: usize) {
        let layer_count = self.layers.len();
        self.refresh_layer_layouts(0..layer_count);
    }

    fn refresh_layers(&mut self, layers: &[usize]) {
        self.refresh_layer_layouts(layers.iter().copied());
    }

    fn clear_targeted_replay(&mut self, layer: usize) {
        self.clear_targeted_replay_scope(layer);
    }

    fn take_ready_canvas_layers(&mut self) -> Vec<usize> {
        let mut ready = std::mem::take(&mut self.ready_layers);
        ready.sort_unstable();
        ready.dedup();
        ready
    }

    fn take_replay_canvas_layer_updates(
        &mut self,
    ) -> Vec<pax_runtime_api::ReplayCanvasLayerUpdate> {
        let replay_layers = self.take_replay_layer_ids();
        replay_layers
            .into_iter()
            .map(|layer| pax_runtime_api::ReplayCanvasLayerUpdate {
                layer,
                node_ids: self.spatial_replay_node_ids(layer),
            })
            .collect()
    }

    fn retains_canvas_nodes(&self) -> bool {
        false
    }

    fn begin_node(&mut self, layer: usize, _node_id: u32, _z_index: i32, _light_mask: u32) -> bool {
        let renderer_count = {
            let Some((target, _)) = self.layers.get(layer) else {
                return false;
            };
            if !target.active {
                return false;
            }
            target.renderers.len()
        };
        self.remember_canvas_node_coverage(
            layer,
            _node_id,
            kurbo::Rect::new(
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::INFINITY,
            ),
        );
        let selected = self.targeted_or_all_indices(layer, renderer_count);
        let began = !selected.is_empty();
        if began {
            self.push_render_scope(layer, selected);
        }
        began
    }

    fn begin_node_with_bounds(
        &mut self,
        layer: usize,
        node_id: u32,
        _z_index: i32,
        coverage_bounds: kurbo::Rect,
        _light_mask: u32,
    ) -> bool {
        let renderer_count = {
            let Some((target, _)) = self.layers.get(layer) else {
                return false;
            };
            if !target.active {
                return false;
            }
            target.renderers.len()
        };
        self.remember_canvas_node_coverage(layer, node_id, coverage_bounds);
        let candidate_indices = self.targeted_or_all_indices(layer, renderer_count);
        let Some((target, _)) = self.layers.get(layer) else {
            return false;
        };
        let selected: Vec<_> = target
            .renderers
            .iter()
            .enumerate()
            .filter(|(index, _)| candidate_indices.contains(index))
            .filter_map(|(index, renderer)| {
                renderer
                    .intersects_coverage_bounds(&coverage_bounds)
                    .then_some(index)
            })
            .collect();
        let began = !selected.is_empty();
        if began {
            self.push_render_scope(layer, selected);
        } else {
            self.clean_skipped_canvas_nodes.insert((layer, node_id));
        }
        began
    }

    fn take_clean_skipped_node(&mut self, layer: usize, node_id: u32) -> bool {
        self.clean_skipped_canvas_nodes.remove(&(layer, node_id))
    }

    fn end_node(&mut self, layer: usize, _node_id: u32) -> bool {
        self.pop_render_scope(layer);
        true
    }

    fn remove_node(&mut self, layer: usize, node_id: u32) -> bool {
        self.forget_canvas_node_coverage(layer, node_id);
        true
    }
}

/// Resolves a single paint to a Piet brush. Mixtures require the chassis paint
/// accumulator, since generic Piet has no additive compositing operation.
pub fn fill_to_piet_brush(fill: &Fill, rect: kurbo::Rect) -> Option<piet::PaintBrush> {
    Some(match fill {
        Fill::Blend(_) => return None,
        Fill::Solid(color) => color.to_piet_color().into(),
        Fill::LinearGradient(linear) => {
            let linear_gradient = LinearGradient::new(
                Fill::to_unit_point(linear.start, (rect.width(), rect.height())),
                Fill::to_unit_point(linear.end, (rect.width(), rect.height())),
                Fill::to_piet_gradient_stops(linear.stops.clone()),
            );
            linear_gradient.into()
        }
        Fill::RadialGradient(radial) => {
            let Some(g) = radial.resolve_geometry(rect) else {
                return Some(piet::Color::TRANSPARENT.into());
            };
            if radial.stops.is_empty() {
                return Some(piet::Color::TRANSPARENT.into());
            }
            let stops = radial
                .stops
                .iter()
                .map(|s| piet::GradientStop {
                    pos: (s
                        .position
                        .evaluate((g.radius, 0.0), pax_runtime_api::Axis::X)
                        / g.radius) as f32,
                    color: s.color.to_piet_color(),
                })
                .collect();
            FixedRadialGradient {
                center: g.center,
                origin_offset: g.focal_point - g.center,
                radius: g.radius,
                stops,
            }
            .into()
        }
    })
}

fn stroke_to_piet_style(stroke: &Stroke) -> StrokeStyle {
    let mut style = StrokeStyle::new();
    style.set_line_cap(match stroke.cap.get() {
        StrokeCap::Butt => LineCap::Butt,
        StrokeCap::Round => LineCap::Round,
        StrokeCap::Square => LineCap::Square,
    });
    style.set_line_join(match stroke.join.get() {
        StrokeJoin::Miter => LineJoin::Miter { limit: 4.0 },
        StrokeJoin::Round => LineJoin::Round,
        StrokeJoin::Bevel => LineJoin::Bevel,
    });
    style
}

#[cfg(test)]
mod radial_tests {
    use super::*;
    use pax_runtime_api::{Color, GradientStop, RadialGradient, Size};

    #[test]
    fn radial_brush_uses_local_bounds_radius_and_focal_point() {
        let mut radial = RadialGradient {
            start: (Size::Percent(75.into()), Size::Percent(25.into())),
            end: (Size::Percent(50.into()), Size::Percent(50.into())),
            radius: 50.0,
            stops: vec![
                GradientStop::get(Color::BLACK, Size::Pixels(25.into())),
                GradientStop::get(Color::WHITE, Size::Percent(100.into())),
            ],
        };
        let rect = kurbo::Rect::new(10.0, 20.0, 210.0, 120.0);
        let Some(piet::PaintBrush::Fixed(piet::FixedGradient::Radial(g))) =
            fill_to_piet_brush(&Fill::RadialGradient(radial.clone()), rect)
        else {
            panic!("radial brush")
        };
        assert_eq!(g.center, kurbo::Point::new(110.0, 70.0));
        assert_eq!(g.origin_offset, kurbo::Vec2::new(50.0, -25.0));
        assert_eq!(g.radius, 50.0);
        assert_eq!(
            g.stops.iter().map(|s| s.pos).collect::<Vec<_>>(),
            vec![0.5, 1.0]
        );
        for radius in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            radial.radius = radius;
            assert!(radial.resolve_geometry(rect).is_none());
            assert!(
                matches!(fill_to_piet_brush(&Fill::RadialGradient(radial.clone()), rect),
                Some(piet::PaintBrush::Color(c)) if c == piet::Color::TRANSPARENT)
            );
        }
    }
}
