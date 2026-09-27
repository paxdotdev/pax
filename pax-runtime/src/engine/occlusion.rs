use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use kurbo::{Affine, BezPath, Rect, Shape};
use pax_message::{borrow, MaskPathPatch, NativeMaskPatch, ScrollerPatch};
use pax_runtime_api::{bez_path_to_svg_path_data, Layer, Window};

use crate::scene_geometry::PreparedCanvasGeometry;
use crate::{node_interface::NodeLocal, ExpandedNode, RuntimeContext, TransformAndBounds};

use super::expanded_node::Occlusion;

// This pass still carries the historical "occlusion" name, but it no longer builds an
// alternating stack of vector/native compositing layers. It assigns z-order, computes native
// punch-through masks, and partitions canvas work into logical render layers. Layer 0 is the root
// surface stack; non-root layers are reserved for scroller-owned vector islands.

#[derive(Clone, Copy, Debug)]
/// Axis-aligned bounds used by the occlusion and native-mask pass.
pub struct OcclusionBox {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

impl OcclusionBox {
    fn intersects(&self, other: &Self) -> bool {
        if self.x2 <= other.x1 || other.x2 <= self.x1 {
            return false;
        }
        if self.y2 <= other.y1 || other.y2 <= self.y1 {
            return false;
        }
        true
    }

    fn union(&self, other: &Self) -> Self {
        Self {
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
            x2: self.x2.max(other.x2),
            y2: self.y2.max(other.y2),
        }
    }

    fn new_from_transform_and_bounds_with_affine(
        t_and_b: TransformAndBounds<NodeLocal, Window>,
        affine: Affine,
    ) -> Self {
        let corners = t_and_b.corners();
        let mut x1 = f64::MAX;
        let mut y1 = f64::MAX;
        let mut x2 = f64::MIN;
        let mut y2 = f64::MIN;
        for c in corners {
            let c = affine * c;
            x1 = x1.min(c.x);
            y1 = y1.min(c.y);
            x2 = x2.max(c.x);
            y2 = y2.max(c.y);
        }
        OcclusionBox { x1, y1, x2, y2 }
    }

    fn new_from_path(path: &kurbo::BezPath) -> Option<Self> {
        let bounds = path.bounding_box();
        if bounds.is_zero_area() {
            return None;
        }
        Some(Self {
            x1: bounds.x0,
            y1: bounds.y0,
            x2: bounds.x1,
            y2: bounds.y1,
        })
    }

    fn from_viewport(width: f64, height: f64) -> Self {
        Self {
            x1: 0.0,
            y1: 0.0,
            x2: width,
            y2: height,
        }
    }

    fn intersect(&self, other: &Self) -> Option<Self> {
        let intersection = Self {
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
            x2: self.x2.min(other.x2),
            y2: self.y2.min(other.y2),
        };
        (intersection.x2 > intersection.x1 && intersection.y2 > intersection.y1)
            .then_some(intersection)
    }

    fn as_array(&self) -> [f64; 4] {
        [self.x1, self.y1, self.x2, self.y2]
    }
}

#[derive(Clone)]
struct CoverageEntry {
    bounds: OcclusionBox,
    path: BezPath,
    clips: Vec<BezPath>,
    opacity: f64,
}

#[derive(Default)]
struct LayerCoverage {
    bounds: Option<OcclusionBox>,
    entries: Vec<CoverageEntry>,
}

impl LayerCoverage {
    fn push(&mut self, mut entry: CoverageEntry) {
        // A path may animate far outside its Frame while its visible paint remains inside.
        // Cull against the intersection of ancestor clip bounds before comparing it with
        // native leaves. Keep the exact clip paths for the eventual mask: bounding boxes
        // only provide a conservative rejection test for rounded/rotated/compound clips.
        let Some(bounds) = clipped_coverage_bounds(entry.bounds, &entry.clips) else {
            return;
        };
        entry.bounds = bounds;
        self.bounds = Some(match self.bounds {
            Some(bounds) => bounds.union(&entry.bounds),
            None => entry.bounds,
        });
        self.entries.push(entry);
    }
}

fn clipped_coverage_bounds(bounds: OcclusionBox, clips: &[BezPath]) -> Option<OcclusionBox> {
    clips.iter().try_fold(bounds, |bounds, clip| {
        bounds.intersect(&OcclusionBox::new_from_path(clip)?)
    })
}

enum DrawableInfo {
    Canvas {
        layer_id: usize,
        entry: CoverageEntry,
    },
    Native {
        node: Rc<ExpandedNode>,
        layer: Layer,
        layer_id: usize,
        bounds: OcclusionBox,
        presentation_transform: Affine,
    },
}

#[cfg(debug_assertions)]
#[derive(Default)]
struct NativeMaskStats {
    native_nodes: usize,
    updated_masks: usize,
    mask_entries: usize,
}

#[cfg(not(debug_assertions))]
#[derive(Default)]
struct NativeMaskStats;

/// Cumulative native-compositing work, independent of canvas replay counters.
#[derive(Clone, Copy, Default, Debug)]
pub struct OcclusionStats {
    /// Full structural/presentation reconciliations.
    pub rebuilds: u64,
    /// Dirty records refreshed outside structural reconciliation.
    pub records_updated: u64,
    /// Native masks evaluated by the incremental path.
    pub masks_evaluated: u64,
    /// Canvas candidates examined for those masks.
    pub mask_candidates: u64,
}

struct OcclusionRecord {
    layer: Layer,
    render_layer_id: usize,
    scrolls: bool,
    clips_content: bool,
    before_children: bool,
    unclippable: bool,
    // Exact coverage and clips remain in unscrolled world coordinates. Geometry
    // preparation and the shared spatial index supply stable layer coordinates.
    clip: Option<BezPath>,
    clips: Vec<u32>,
    path: Option<BezPath>,
    opacity: f64,
    geometry: Option<Rc<PreparedCanvasGeometry>>,
}

#[derive(Default)]
pub(crate) struct OcclusionState {
    pub(crate) structural: bool,
    pub(crate) dirty: HashSet<u32>,
    pub(crate) scrolled: HashSet<u32>,
    records: HashMap<u32, OcclusionRecord>,
    scroll_regions: HashMap<u32, (usize, Rect)>,
    owned_layers: HashMap<u32, usize>,
    has_native_targets: bool,
    unsafe_layers: HashSet<usize>,
    canvas_counts: HashMap<usize, usize>,
    culling_dirty: bool,
    culling_rebuild: bool,
    culled: HashSet<u32>,
    warm_native: HashMap<usize, HashSet<u32>>,
    pub(crate) stats: OcclusionStats,
}

impl OcclusionState {
    pub(crate) fn remove(&mut self, id: u32) {
        self.records.remove(&id);
        self.dirty.remove(&id);
        self.scrolled.remove(&id);
        self.scroll_regions.remove(&id);
        self.owned_layers.remove(&id);
    }
}

/// Reconcile structural changes, otherwise update only dirty geometry and overlapping masks.
pub fn update_node_occlusion(root_node: &Rc<ExpandedNode>, ctx: &RuntimeContext) {
    let mut state_guard = ctx.occlusion_state.borrow_mut();
    let state = &mut *state_guard;
    state.culling_dirty = true;
    let dirty = std::mem::take(&mut state.dirty);
    let mut scrolled = std::mem::take(&mut state.scrolled);
    // Scroll islands present canvas-only content without a native-mask pass.
    // Preserve that fast path even for large grids of nested scrollers.
    if !state.has_native_targets {
        scrolled.clear();
    }
    let rebuild = std::mem::take(&mut state.structural)
        || state.records.is_empty()
        || dirty.iter().any(|id| {
            let Some(record) = state.records.get(id) else {
                return true;
            };
            let Some(node) = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(*id))
            else {
                return true;
            };
            let instance = borrow!(node.instance_node);
            let layer = if instance.materializes_native_surface(&node) {
                Layer::Native
            } else {
                instance.base().flags().layer
            };
            // Clip/container changes have inherited effects. They retain the
            // complete reconciliation path; leaf geometry does not change topology.
            state.unsafe_layers.contains(&record.render_layer_id)
                || record.scrolls
                || record.clip.is_some()
                || instance.scrolls_content(&node)
                || instance.resolve_effect_clip_path(&node).is_some()
                || record.layer != layer
                || record.before_children
                    != (instance.materializes_native_surface(&node)
                        && instance.materializes_native_surface_before_children(&node))
                || record.unclippable
                    != node
                        .get_common_properties()
                        .borrow()
                        .unclippable
                        .get()
                        .unwrap_or(false)
        })
        || scrolled.iter().any(|id| {
            // Scroll islands with inherited presentation containers need their
            // Frame/Scroller presentation patches reconciled together. The common
            // leaf-only content island can update masks using two region queries.
            !state.scroll_regions.contains_key(id)
        });
    if rebuild {
        state.culling_rebuild = true;
        state.records.clear();
        state.canvas_counts.clear();
        state.scroll_regions.clear();
        state.owned_layers.clear();
        state.unsafe_layers.clear();
        rebuild_node_occlusion(root_node, ctx, state);
        ctx.prepare_scene_geometry();
        let ids: Vec<_> = state.records.keys().copied().collect();
        for id in ids {
            let Some(node) = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id)) else {
                state.records.remove(&id);
                continue;
            };
            let record = state.records.get_mut(&id).unwrap();
            if record.layer != Layer::DontCare {
                let geometry = ctx.canvas_geometry_for_node(&node);
                if record.path.is_some() && record.opacity > f64::EPSILON {
                    *state.canvas_counts.entry(geometry.layer).or_default() += 1;
                }
                state.records.get_mut(&id).unwrap().geometry = Some(geometry);
            }
        }
        // Classify each layer once. Searching all records for every scroller
        // makes the initial mount quadratic for grids of nested scroll regions.
        let mut container_layers = HashSet::new();
        state.has_native_targets = false;
        for (&id, record) in &state.records {
            state.has_native_targets |= record.layer == Layer::Native;
            if record.scrolls || record.clip.is_some() || record.unclippable {
                container_layers.insert(record.render_layer_id);
            }
            if record.unclippable || (record.scrolls && !state.owned_layers.contains_key(&id)) {
                // These layers mix presentation domains or escaped content.
                state.unsafe_layers.insert(record.render_layer_id);
            }
        }
        for (&id, &layer) in &state.owned_layers {
            if state.records.get(&id).is_some_and(|r| r.clips_content)
                && !container_layers.contains(&layer)
            {
                if let Some(region) = scroll_region(id, ctx) {
                    state.scroll_regions.insert(id, (layer, region));
                }
            }
        }
        state.stats.rebuilds += 1;
        return;
    }

    ctx.prepare_scene_geometry();
    let mut affected = HashSet::new();
    for id in dirty {
        let Some(node) = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id)) else {
            continue;
        };
        let record = state.records.get_mut(&id).unwrap();
        state.stats.records_updated += 1;
        if record.layer == Layer::DontCare {
            continue;
        }
        let geometry = ctx.canvas_geometry_for_node(&node);
        if record.layer == Layer::Canvas {
            if let (Some(path), Some(old)) = (&record.path, &record.geometry) {
                let bounds = path_in_surface_bounds(path, old);
                affected.extend(ctx.compositing_nodes_intersecting(old.layer, &[bounds], true));
                if record.opacity > f64::EPSILON {
                    *state.canvas_counts.entry(old.layer).or_default() -= 1;
                }
            }
            record.path = borrow!(node.instance_node)
                .resolve_occlusion_path(&node)
                .filter(|path| OcclusionBox::new_from_path(path).is_some());
            record.opacity = borrow!(node.instance_node).resolve_coverage_opacity(&node);
            if let Some(path) = &record.path {
                let bounds = path_in_surface_bounds(path, &geometry);
                affected.extend(ctx.compositing_nodes_intersecting(
                    geometry.layer,
                    &[bounds],
                    true,
                ));
                if record.opacity > f64::EPSILON {
                    *state.canvas_counts.entry(geometry.layer).or_default() += 1;
                }
            }
        } else {
            affected.insert(id);
        }
        record.geometry = Some(geometry);
    }
    for id in scrolled {
        if let Some(region) = scroll_region(id, ctx) {
            let (layer, previous) = state.scroll_regions.get_mut(&id).unwrap();
            let previous = std::mem::replace(previous, region);
            affected.extend(ctx.compositing_nodes_intersecting(*layer, &[previous, region], true));
        }
    }
    ctx.set_canvas_drawable_layers(
        state
            .canvas_counts
            .iter()
            .filter_map(|(layer, count)| (*count > 0).then_some(*layer))
            .collect(),
    );
    for id in affected {
        update_indexed_mask(id, ctx, state);
    }
}

// Native view detachment is currently a macOS optimization. Only the already
// classified leaf-only, clipped scroll islands are eligible; containers and
// escaped content retain their existing presentation path.
pub(crate) fn update_native_culling(
    ctx: &RuntimeContext,
    policy: super::layer_tiling::ScrollerTilingPolicy,
) {
    let globals = ctx.globals();
    if globals.platform != pax_runtime_api::Platform::Native || !globals.os.is_macos() {
        return;
    }
    let mut state = ctx.occlusion_state.borrow_mut();
    if !std::mem::take(&mut state.culling_dirty) {
        return;
    }
    let mut warm = HashMap::<usize, HashSet<u32>>::new();
    for (&id, &(layer, _)) in &state.scroll_regions {
        let Some(surface) = ctx.get_scroller_surface_state(id) else {
            continue;
        };
        let Some(region) = scroll_region(id, ctx) else {
            continue;
        };
        let (pad_x, pad_y) = policy.prewarm_padding(
            region.width(),
            region.height(),
            surface.content_width > region.width() + 0.5,
            surface.content_height > region.height() + 0.5,
        );
        let region = Rect::new(
            region.x0 - pad_x,
            region.y0 - pad_y,
            region.x1 + pad_x,
            region.y1 + pad_y,
        );
        warm.insert(
            layer,
            ctx.compositing_nodes_intersecting(layer, &[region], true)
                .into_iter()
                .collect(),
        );
    }
    let mut patch = pax_message::NativeCullPatch::default();
    if std::mem::take(&mut state.culling_rebuild)
        || warm
            .keys()
            .any(|layer| !state.warm_native.contains_key(layer))
        || state
            .warm_native
            .keys()
            .any(|layer| !warm.contains_key(layer))
    {
        // Topology changes already traverse the scene. Ordinary scroll/animation
        // ticks below compare only the small warm sets, never all cold nodes.
        let culled: HashSet<_> = state
            .records
            .iter()
            .filter_map(|(&id, record)| {
                (record.geometry.as_ref().is_some_and(|g| g.is_native)
                    && warm
                        .get(&record.render_layer_id)
                        .is_some_and(|ids| !ids.contains(&id)))
                .then_some(id)
            })
            .collect();
        patch.cull.extend(culled.difference(&state.culled).copied());
        patch
            .restore
            .extend(state.culled.difference(&culled).copied());
        state.culled = culled;
    } else {
        for (&layer, ids) in &warm {
            let previous = &state.warm_native[&layer];
            patch.cull.extend(previous.difference(ids).copied());
            patch.restore.extend(ids.difference(previous).copied());
        }
        for id in &patch.cull {
            state.culled.insert(*id);
        }
        for id in &patch.restore {
            state.culled.remove(id);
        }
    }
    state.warm_native = warm;
    if !patch.cull.is_empty() || !patch.restore.is_empty() {
        patch.cull.sort_unstable();
        patch.restore.sort_unstable();
        ctx.enqueue_native_message(pax_message::NativeMessage::NativeCullUpdate(patch));
    }
}

fn scroll_region(id: u32, ctx: &RuntimeContext) -> Option<Rect> {
    let node = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id))?;
    let bounds = node.transform_and_bounds.get().bounds;
    let scroll = ctx
        .get_scroller_surface_scroll(id)
        .or_else(|| borrow!(node.instance_node).resolve_scroll_offset(&node))
        .unwrap_or_default();
    Some(Rect::new(
        scroll.0,
        scroll.1,
        scroll.0 + bounds.0,
        scroll.1 + bounds.1,
    ))
}

fn path_in_surface_bounds(path: &BezPath, geometry: &PreparedCanvasGeometry) -> Rect {
    (geometry.surface_transform * geometry.world_transform.inverse() * path.clone()).bounding_box()
}

fn update_indexed_mask(id: u32, ctx: &RuntimeContext, state: &mut OcclusionState) {
    let Some(record) = state.records.get(&id) else {
        return;
    };
    let Some(node) = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id)) else {
        return;
    };
    let geometry = ctx.canvas_geometry_for_node(&node);
    let tab = node.transform_and_bounds.get();
    let presentation = ctx.presentation_scroll_transform_for_node(&node);
    let native_bounds = OcclusionBox::new_from_transform_and_bounds_with_affine(tab, presentation);
    let inverse = Affine::from(tab.transform.inverse()) * presentation.inverse();
    let query_bounds = geometry.surface_transform.transform_rect_bbox(Rect::new(
        0.0,
        0.0,
        geometry.bounds.0,
        geometry.bounds.1,
    ));
    let candidates = if record.layer == Layer::Native {
        ctx.compositing_nodes_intersecting(geometry.layer, &[query_bounds], false)
    } else {
        Vec::new()
    };
    state.stats.masks_evaluated += 1;
    state.stats.mask_candidates += candidates.len() as u64;
    let mut entries = Vec::new();
    for candidate in candidates {
        let Some(coverage) = state.records.get(&candidate) else {
            continue;
        };
        if !(coverage.opacity > f64::EPSILON) {
            continue;
        }
        let Some(path) = &coverage.path else { continue };
        let Some(occluder) = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(candidate))
        else {
            continue;
        };
        if occluder.occlusion.get().z_index <= node.occlusion.get().z_index {
            continue;
        }
        let transform = ctx.presentation_scroll_transform_for_node(&occluder);
        let path = transform * path.clone();
        let Some(bounds) = OcclusionBox::new_from_path(&path) else {
            continue;
        };
        let clips: Vec<_> = coverage
            .clips
            .iter()
            .filter_map(|clip_id| {
                let clip = state.records.get(clip_id)?.clip.as_ref()?;
                let owner =
                    ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(*clip_id))?;
                Some(ctx.presentation_scroll_transform_for_node(&owner) * clip.clone())
            })
            .collect();
        if !clipped_coverage_bounds(bounds, &clips).is_some_and(|b| b.intersects(&native_bounds)) {
            continue;
        }
        entries.push(MaskPathPatch {
            path: bez_path_to_svg_path_data(&(inverse * path)),
            clips: clips
                .into_iter()
                .map(|clip| bez_path_to_svg_path_data(&(inverse * clip)))
                .collect(),
            opacity: Some(coverage.opacity),
        });
    }
    sort_mask_entries(&mut entries);
    emit_mask(&node, tab.bounds, entries, ctx);
}

fn sort_mask_entries(entries: &mut [MaskPathPatch]) {
    entries.sort_by(|lhs, rhs| {
        lhs.path
            .cmp(&rhs.path)
            .then_with(|| lhs.clips.cmp(&rhs.clips))
            .then_with(|| {
                lhs.opacity
                    .map(f64::to_bits)
                    .cmp(&rhs.opacity.map(f64::to_bits))
            })
    });
}

fn emit_mask(
    node: &ExpandedNode,
    size: (f64, f64),
    entries: Vec<MaskPathPatch>,
    ctx: &RuntimeContext,
) {
    let hash = if entries.is_empty() {
        0
    } else {
        hash_mask_entries(size, &entries)
    };
    if node.native_mask_hash.get() != hash {
        node.native_mask_hash.set(hash);
        ctx.enqueue_native_message(pax_message::NativeMessage::NativeMaskUpdate(
            NativeMaskPatch {
                id: node.id.to_u32(),
                size_x: size.0,
                size_y: size.1,
                entries,
            },
        ));
    }
}

/// Recompute z-order, native masks, and logical render-layer assignments for the tree.
fn rebuild_node_occlusion(
    root_node: &Rc<ExpandedNode>,
    ctx: &RuntimeContext,
    state: &mut OcclusionState,
) {
    #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
    let pass_start = std::time::Instant::now();

    let mut drawables = Vec::new();
    let mut z_index = 0;
    let mut next_layer_id = 1;
    let viewport = ctx.globals().viewport.get();
    let viewport_bounds = OcclusionBox::from_viewport(viewport.bounds.0, viewport.bounds.1);
    ctx.clear_layer_scroller_owners();
    update_node_occlusion_recursive(
        root_node,
        ctx,
        0,
        None,
        &[],
        viewport_bounds,
        viewport_bounds,
        Affine::IDENTITY,
        &mut z_index,
        &mut next_layer_id,
        &mut drawables,
        &[],
        state,
    );
    let canvas_drawable_layers = drawables
        .iter()
        .filter_map(|drawable| match drawable {
            DrawableInfo::Canvas { layer_id, .. } => Some(*layer_id),
            DrawableInfo::Native { .. } => None,
        })
        .collect::<HashSet<_>>();
    ctx.set_canvas_drawable_layers(canvas_drawable_layers);
    let _native_mask_stats = update_native_masks(&drawables, ctx);

    let new_layer_count = next_layer_id;
    if ctx.layer_count.get() != new_layer_count {
        ctx.layer_count.set(new_layer_count);
        ctx.enqueue_native_message(pax_message::NativeMessage::ShrinkLayersTo(
            new_layer_count as u32,
        ));
    }

    #[cfg(debug_assertions)]
    {
        let canvas_drawables = drawables
            .iter()
            .filter(|drawable| matches!(drawable, DrawableInfo::Canvas { .. }))
            .count();
        let native_drawables = drawables.len().saturating_sub(canvas_drawables);

        #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
        log::trace!(
            "occlusion pass: {}us, layers={}, scroller_layers={}, drawables={} canvas/{} native, native_masks={} updates/{} entries/{} nodes",
            pass_start.elapsed().as_micros(),
            new_layer_count,
            new_layer_count.saturating_sub(1),
            canvas_drawables,
            native_drawables,
            _native_mask_stats.updated_masks,
            _native_mask_stats.mask_entries,
            _native_mask_stats.native_nodes,
        );

        #[cfg(target_arch = "wasm32")]
        log::trace!(
            "occlusion pass: layers={}, scroller_layers={}, drawables={} canvas/{} native, native_masks={} updates/{} entries/{} nodes",
            new_layer_count,
            new_layer_count.saturating_sub(1),
            canvas_drawables,
            native_drawables,
            _native_mask_stats.updated_masks,
            _native_mask_stats.mask_entries,
            _native_mask_stats.native_nodes,
        );
    }
}

// Runtime is O(n^2) atm, but native punch-through work is grouped by logical render layer instead
// of by the old alternating vector/native layer stack.
fn update_node_occlusion_recursive(
    node: &Rc<ExpandedNode>,
    ctx: &RuntimeContext,
    current_layer_id: usize,
    active_container: Option<u32>,
    active_clips: &[BezPath],
    viewport_bounds: OcclusionBox,
    active_clip_bounds: OcclusionBox,
    active_scroll_transform: Affine,
    z_index: &mut i32,
    next_layer_id: &mut usize,
    drawables: &mut Vec<DrawableInfo>,
    active_clip_ids: &[u32],
    state: &mut OcclusionState,
) {
    fn clamp_offset(value: f64, content: f64, viewport: f64) -> f64 {
        if content <= viewport {
            return 0.0;
        }
        if !value.is_finite() {
            return 0.0;
        }
        value.max(0.0).min((content - viewport).max(0.0))
    }

    let instance_node = borrow!(node.instance_node);
    let world_clip = instance_node.resolve_effect_clip_path(node);
    let effect_clip_path = world_clip
        .clone()
        .map(|clip| active_scroll_transform * clip);
    let has_effect_clip = effect_clip_path.is_some();
    let scrolls_content = instance_node.scrolls_content(node);
    let scroll_transform = if scrolls_content {
        let root_delegates_to_page_scroll = ctx.get_root_scroller_id() == Some(node.id.to_u32())
            && ctx.get_visual_viewport_state().is_some();
        if root_delegates_to_page_scroll {
            Affine::IDENTITY
        } else {
            let (scroll_x, scroll_y) = if ctx.get_root_scroller_id() == Some(node.id.to_u32()) {
                if let Some(visual) = ctx.get_visual_viewport_state() {
                    let visual_x = visual.page_scroll_x + visual.offset_x;
                    let visual_y = visual.page_scroll_y + visual.offset_y;
                    if visual_x.is_finite() && visual_y.is_finite() {
                        if let Some(state) = ctx.get_scroller_surface_state(node.id.to_u32()) {
                            let viewport_width = if visual.width.is_finite() {
                                visual.width
                            } else {
                                state.viewport_width
                            };
                            let viewport_height = if visual.height.is_finite() {
                                visual.height
                            } else {
                                state.viewport_height
                            };
                            (
                                clamp_offset(visual_x, state.content_width, viewport_width),
                                clamp_offset(visual_y, state.content_height, viewport_height),
                            )
                        } else {
                            (visual_x, visual_y)
                        }
                    } else {
                        ctx.get_scroller_surface_scroll(node.id.to_u32())
                            .or_else(|| instance_node.resolve_scroll_offset(node))
                            .unwrap_or((0.0, 0.0))
                    }
                } else {
                    ctx.get_scroller_surface_scroll(node.id.to_u32())
                        .or_else(|| instance_node.resolve_scroll_offset(node))
                        .unwrap_or((0.0, 0.0))
                }
            } else {
                ctx.get_scroller_surface_scroll(node.id.to_u32())
                    .or_else(|| instance_node.resolve_scroll_offset(node))
                    .unwrap_or((0.0, 0.0))
            };
            if scroll_x.abs() > f64::EPSILON || scroll_y.abs() > f64::EPSILON {
                let world_transform = Affine::from(node.transform_and_bounds.get().transform);
                let inverse_world =
                    Affine::from(node.transform_and_bounds.get().transform.inverse());
                world_transform * Affine::translate((-scroll_x, -scroll_y)) * inverse_world
            } else {
                Affine::IDENTITY
            }
        }
    } else {
        Affine::IDENTITY
    };
    let allow_scroller_vector_layers = ctx.globals().browser_allows_scroller_vector_layers.get()
        && (ctx
            .globals()
            .browser_allows_nested_scroller_vector_layers
            .get()
            || active_container.is_none());
    let materializes_native_surface = instance_node.materializes_native_surface(node);
    let materializes_native_surface_before_children = materializes_native_surface
        && instance_node.materializes_native_surface_before_children(node);
    let layer = if materializes_native_surface {
        Layer::Native
    } else {
        instance_node.base().flags().layer
    };
    state.records.insert(
        node.id.to_u32(),
        OcclusionRecord {
            layer,
            render_layer_id: current_layer_id,
            scrolls: scrolls_content,
            clips_content: instance_node.clips_content(node),
            before_children: materializes_native_surface_before_children,
            unclippable: node
                .get_common_properties()
                .borrow()
                .unclippable
                .get()
                .unwrap_or(false),
            clip: world_clip,
            clips: active_clip_ids.to_vec(),
            path: None,
            opacity: 0.0,
            geometry: None,
        },
    );
    drop(instance_node);

    let descendant_layer_id = if scrolls_content && allow_scroller_vector_layers {
        let layer_id = *next_layer_id;
        *next_layer_id += 1;
        layer_id
    } else {
        current_layer_id
    };
    if scrolls_content && allow_scroller_vector_layers {
        ctx.register_layer_scroller_owner(descendant_layer_id, node.id);
        state
            .owned_layers
            .insert(node.id.to_u32(), descendant_layer_id);
    }
    let descendant_container = if scrolls_content {
        Some(node.id.to_u32())
    } else {
        has_effect_clip
            .then(|| node.id.to_u32())
            .or(active_container)
    };
    let descendant_scroll_transform = active_scroll_transform * scroll_transform;
    let presented_bounds = OcclusionBox::new_from_transform_and_bounds_with_affine(
        node.transform_and_bounds.get(),
        active_scroll_transform,
    );
    let mut descendant_clip_bounds = active_clip_bounds;
    if scrolls_content {
        descendant_clip_bounds = descendant_clip_bounds
            .intersect(&presented_bounds)
            .unwrap_or(descendant_clip_bounds);
    }
    if let Some(effect_clip_bounds) = effect_clip_path
        .as_ref()
        .and_then(OcclusionBox::new_from_path)
    {
        descendant_clip_bounds = descendant_clip_bounds
            .intersect(&effect_clip_bounds)
            .unwrap_or(descendant_clip_bounds);
    }
    let mut descendant_clip_ids = active_clip_ids.to_vec();
    let mut descendant_clips = active_clips.to_vec();
    if let Some(clip_path) = effect_clip_path.clone() {
        descendant_clips.push(clip_path);
        descendant_clip_ids.push(node.id.to_u32());
    }

    let presented_clip_bounds = if has_effect_clip || scrolls_content {
        Some(descendant_clip_bounds)
    } else {
        Some(active_clip_bounds)
    };

    if materializes_native_surface_before_children {
        let new_occlusion = Occlusion {
            render_layer_id: current_layer_id,
            z_index: *z_index,
            parent_frame: active_container,
        };

        if new_occlusion != node.occlusion.get() {
            ctx.invalidate_scene_geometry(node.id.to_u32());
            let previous_occlusion = node.occlusion.get();
            let prev_layer = previous_occlusion.render_layer_id;
            ctx.set_canvas_dirty(prev_layer);
            ctx.set_canvas_dirty(new_occlusion.render_layer_id);
            node.occlusion.set(new_occlusion);
        }

        drawables.push(DrawableInfo::Native {
            node: Rc::clone(node),
            layer,
            layer_id: current_layer_id,
            bounds: presented_bounds,
            presentation_transform: active_scroll_transform,
        });
        *z_index += 1;
    }

    for child in node.children.get().iter().rev() {
        let cp = child.get_common_properties();
        let cp = borrow!(cp);
        let unclippable = cp.unclippable.get().unwrap_or(false);
        let (child_container, child_clips, child_clip_bounds) = if unclippable {
            (None, Vec::new(), viewport_bounds)
        } else {
            (
                descendant_container,
                descendant_clips.clone(),
                descendant_clip_bounds,
            )
        };

        update_node_occlusion_recursive(
            child,
            ctx,
            descendant_layer_id,
            child_container,
            &child_clips,
            viewport_bounds,
            child_clip_bounds,
            descendant_scroll_transform,
            z_index,
            next_layer_id,
            drawables,
            if unclippable {
                &[]
            } else {
                &descendant_clip_ids
            },
            state,
        );
    }

    if materializes_native_surface_before_children {
        return;
    }

    if layer == Layer::DontCare && !has_effect_clip {
        return;
    }

    let new_occlusion = Occlusion {
        render_layer_id: current_layer_id,
        z_index: *z_index,
        parent_frame: active_container,
    };

    if scrolls_content {
        let content_layer_id = allow_scroller_vector_layers.then_some(descendant_layer_id as u32);
        let presentation_hash =
            hash_presentation_bounds(Some(presented_bounds), presented_clip_bounds);
        let presentation_changed = node.presentation_cache_hash.get() != presentation_hash;
        if node.browser_content_layer_id.get() != content_layer_id || presentation_changed {
            ctx.enqueue_native_message(pax_message::NativeMessage::ScrollerUpdate(ScrollerPatch {
                id: node.id.to_u32(),
                content_layer_id,
                presented_bounds: Some(presented_bounds.as_array()),
                presented_clip_bounds: presented_clip_bounds.map(|bounds| bounds.as_array()),
                ..Default::default()
            }));
            node.browser_content_layer_id.set(content_layer_id);
            node.presentation_cache_hash.set(presentation_hash);
        }
    }

    if has_effect_clip {
        let presentation_hash =
            hash_presentation_bounds(Some(presented_bounds), presented_clip_bounds);
        if node.presentation_cache_hash.get() != presentation_hash {
            ctx.enqueue_native_message(pax_message::NativeMessage::FrameUpdate(
                pax_message::FramePatch {
                    id: node.id.to_u32(),
                    presented_bounds: Some(presented_bounds.as_array()),
                    presented_clip_bounds: presented_clip_bounds.map(|bounds| bounds.as_array()),
                    ..Default::default()
                },
            ));
            node.presentation_cache_hash.set(presentation_hash);
        }
    }

    if new_occlusion != node.occlusion.get() {
        ctx.invalidate_scene_geometry(node.id.to_u32());
        let previous_occlusion = node.occlusion.get();
        let prev_layer = previous_occlusion.render_layer_id;
        if layer == Layer::Canvas && prev_layer != new_occlusion.render_layer_id {
            ctx.enqueue_canvas_node_removal(prev_layer, node.id.to_u32());
        }
        if layer == Layer::Canvas {
            ctx.mark_canvas_node_dirty(node.id);
        }
        ctx.set_canvas_dirty(prev_layer);
        ctx.set_canvas_dirty(new_occlusion.render_layer_id);
        node.occlusion.set(new_occlusion);
    }

    match layer {
        Layer::Canvas => {
            if let Some(coverage_path) = borrow!(node.instance_node).resolve_occlusion_path(node) {
                let record = state.records.get_mut(&node.id.to_u32()).unwrap();
                record.path =
                    OcclusionBox::new_from_path(&coverage_path).map(|_| coverage_path.clone());
                record.opacity = borrow!(node.instance_node).resolve_coverage_opacity(node);
                let coverage_path = active_scroll_transform * coverage_path;
                if let Some(bounds) = OcclusionBox::new_from_path(&coverage_path) {
                    let opacity = borrow!(node.instance_node).resolve_coverage_opacity(node);
                    if opacity > f64::EPSILON {
                        drawables.push(DrawableInfo::Canvas {
                            layer_id: current_layer_id,
                            entry: CoverageEntry {
                                bounds,
                                path: coverage_path,
                                clips: active_clips.to_vec(),
                                opacity,
                            },
                        });
                    }
                }
            }
        }
        Layer::Native | Layer::NativeNonOccluding => {
            drawables.push(DrawableInfo::Native {
                node: Rc::clone(node),
                layer,
                layer_id: current_layer_id,
                bounds: presented_bounds,
                presentation_transform: active_scroll_transform,
            });
        }
        Layer::DontCare => {}
    }

    *z_index += 1;
}

fn update_native_masks(drawables: &[DrawableInfo], ctx: &RuntimeContext) -> NativeMaskStats {
    let mut vector_above = HashMap::<usize, LayerCoverage>::new();
    #[cfg(debug_assertions)]
    let mut stats = NativeMaskStats::default();
    #[cfg(not(debug_assertions))]
    let stats = NativeMaskStats::default();

    for drawable in drawables.iter().rev() {
        match drawable {
            DrawableInfo::Canvas { layer_id, entry } => {
                vector_above
                    .entry(*layer_id)
                    .or_default()
                    .push(entry.clone());
            }
            DrawableInfo::Native {
                node,
                layer,
                layer_id,
                bounds,
                presentation_transform,
            } => {
                #[cfg(debug_assertions)]
                {
                    stats.native_nodes += 1;
                }
                let t_and_b = node.transform_and_bounds.get();
                let size = t_and_b.bounds;
                let layer_coverage = vector_above.get(layer_id);
                let mut entries = if *layer == Layer::Native {
                    let has_overlap = layer_coverage
                        .and_then(|coverage| coverage.bounds)
                        .is_some_and(|coverage_bounds| coverage_bounds.intersects(bounds));
                    if has_overlap {
                        let inverse = Affine::from(t_and_b.transform.inverse())
                            * presentation_transform.inverse();
                        layer_coverage
                            .into_iter()
                            .flat_map(|coverage| coverage.entries.iter())
                            .filter(|entry| entry.bounds.intersects(bounds))
                            .map(|entry| MaskPathPatch {
                                path: bez_path_to_svg_path_data(&(inverse * entry.path.clone())),
                                clips: entry
                                    .clips
                                    .iter()
                                    .map(|clip| {
                                        bez_path_to_svg_path_data(&(inverse * clip.clone()))
                                    })
                                    .collect(),
                                opacity: Some(entry.opacity),
                            })
                            .collect::<Vec<_>>()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                entries.sort_by(|lhs, rhs| {
                    lhs.path
                        .cmp(&rhs.path)
                        .then_with(|| lhs.clips.cmp(&rhs.clips))
                        .then_with(|| {
                            lhs.opacity
                                .map(f64::to_bits)
                                .cmp(&rhs.opacity.map(f64::to_bits))
                        })
                });

                let new_hash = if entries.is_empty() {
                    0
                } else {
                    hash_mask_entries(size, &entries)
                };
                if node.native_mask_hash.get() != new_hash {
                    node.native_mask_hash.set(new_hash);
                    #[cfg(debug_assertions)]
                    {
                        stats.updated_masks += 1;
                        stats.mask_entries += entries.len();
                    }
                    ctx.enqueue_native_message(pax_message::NativeMessage::NativeMaskUpdate(
                        NativeMaskPatch {
                            id: node.id.to_u32(),
                            size_x: size.0,
                            size_y: size.1,
                            entries,
                        },
                    ));
                }
            }
        }
    }

    stats
}

fn hash_mask_entries(size: (f64, f64), entries: &[MaskPathPatch]) -> u64 {
    let mut hasher = DefaultHasher::new();
    size.0.to_bits().hash(&mut hasher);
    size.1.to_bits().hash(&mut hasher);
    entries.hash(&mut hasher);
    hasher.finish()
}

fn hash_presentation_bounds(
    presented_bounds: Option<OcclusionBox>,
    presented_clip_bounds: Option<OcclusionBox>,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    presented_bounds
        .map(|bounds| bounds.as_array().map(f64::to_bits))
        .hash(&mut hasher);
    presented_clip_bounds
        .map(|bounds| bounds.as_array().map(f64::to_bits))
        .hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::{Rect, RoundedRect};

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> BezPath {
        Rect::new(x0, y0, x1, y1).to_path(0.1)
    }

    fn entry(path: BezPath, clips: Vec<BezPath>) -> CoverageEntry {
        CoverageEntry {
            bounds: OcclusionBox::new_from_path(&path).unwrap(),
            path,
            clips,
            opacity: 1.0,
        }
    }

    #[test]
    fn clipped_animation_never_becomes_an_occluder_below_its_frame() {
        let frame = rect(0.0, 0.0, 400.0, 420.0);
        let text = OcclusionBox::new_from_path(&rect(0.0, 532.0, 400.0, 784.0)).unwrap();
        for offset in (0..600).step_by(10) {
            let mut coverage = LayerCoverage::default();
            coverage.push(entry(
                rect(0.0, offset as f64, 400.0, offset as f64 + 140.0),
                vec![frame.clone()],
            ));
            assert!(!coverage
                .bounds
                .is_some_and(|bounds| bounds.intersects(&text)));
            assert!(coverage
                .entries
                .iter()
                .all(|entry| !entry.bounds.intersects(&text)));
        }
    }

    #[test]
    fn nested_disjoint_and_empty_clips_discard_coverage() {
        for clips in [
            vec![
                rect(0.0, 0.0, 100.0, 100.0),
                rect(200.0, 200.0, 300.0, 300.0),
            ],
            vec![BezPath::new()],
        ] {
            let mut coverage = LayerCoverage::default();
            coverage.push(entry(rect(0.0, 0.0, 400.0, 400.0), clips));
            assert!(coverage.bounds.is_none());
            assert!(coverage.entries.is_empty());
        }
    }

    #[test]
    fn partial_transformed_clips_keep_exact_mask_geometry() {
        let transform = Affine::translate((30.0, 80.0)) * Affine::rotate(0.2);
        let clip = transform * RoundedRect::new(0.0, 0.0, 100.0, 100.0, 20.0).to_path(0.1);
        let path = rect(-100.0, -100.0, 400.0, 400.0);
        let expected = OcclusionBox::new_from_path(&clip).unwrap();
        let mut coverage = LayerCoverage::default();
        coverage.push(entry(path.clone(), vec![clip.clone()]));
        assert_eq!(coverage.bounds.unwrap().as_array(), expected.as_array());
        assert_eq!(coverage.entries[0].path, path);
        assert_eq!(coverage.entries[0].clips, vec![clip]);
    }

    #[test]
    fn unclipped_occluders_keep_their_full_bounds() {
        let path = rect(-20.0, 40.0, 100.0, 200.0);
        let expected = OcclusionBox::new_from_path(&path).unwrap();
        let mut coverage = LayerCoverage::default();
        coverage.push(entry(path, vec![]));
        assert_eq!(coverage.bounds.unwrap().as_array(), expected.as_array());
    }
}
