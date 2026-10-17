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

#[cfg(test)]
#[path = "occlusion_reconcile_tests.rs"]
mod reconcile_tests;

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

    fn union(&self, other: &Self) -> Self {
        Self {
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
            x2: self.x2.max(other.x2),
            y2: self.y2.max(other.y2),
        }
    }
}

#[derive(Default)]
struct LayerCoverage {
    bounds: Option<OcclusionBox>,
    entries: Vec<(u32, OcclusionBox)>,
}

impl LayerCoverage {
    fn push(&mut self, id: u32, bounds: OcclusionBox) {
        self.bounds = Some(self.bounds.map_or(bounds, |existing| existing.union(&bounds)));
        self.entries.push((id, bounds));
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
        node_id: u32,
    },
    Native {
        node: Rc<ExpandedNode>,
        layer: Layer,
        layer_id: usize,
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
#[derive(Clone, Copy, Default, Debug, pax_message::serde::Serialize)]
#[serde(crate = "pax_message::serde")]
pub struct OcclusionStats {
    /// Reconciliation passes, including incremental passes.
    pub passes: u64,
    /// Full structural/presentation reconciliations.
    pub rebuilds: u64,
    /// Structural edits reconciled without replacing stable coverage records.
    pub structural_reconciliations: u64,
    /// Records reused by those structural reconciliations.
    pub records_retained: u64,
    /// Numeric paint-order changes sent without invalidating geometry.
    pub orders_updated: u64,
    /// Dirty/new records refreshed by localized coverage reconciliation.
    pub records_updated: u64,
    /// Native masks evaluated by localized reconciliation, including simple structural edits.
    pub masks_evaluated: u64,
    /// Canvas candidates examined for those masks.
    pub mask_candidates: u64,
    /// Cause of the most recent full reconciliation, for opt-in profiling.
    pub rebuild_reason: Option<&'static str>,
    /// Triggering expanded node, when the cause was not structural.
    pub rebuild_node: Option<u32>,
    /// Bounded origins of structural invalidation consumed by the last rebuild.
    /// Populated only when diagnostic attribution is enabled.
    pub rebuild_origins: [Option<OcclusionInvalidation>; 8],
}

#[derive(Clone, Copy, Debug, pax_message::serde::Serialize)]
#[serde(crate = "pax_message::serde")]
pub struct OcclusionInvalidation {
    pub file: &'static str,
    pub line: u32,
    pub node: Option<u32>,
    pub count: u32,
}

struct OcclusionRecord {
    instance: std::rc::Weak<dyn crate::rendering::InstanceNode>,
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
    pub(crate) diagnostics: bool,
    pub(crate) invalidations: [Option<OcclusionInvalidation>; 8],
    pub(crate) structural: bool,
    pub(crate) dirty: HashSet<u32>,
    pub(crate) scrolled: HashSet<u32>,
    records: HashMap<u32, OcclusionRecord>,
    // Preserve removed coverage until its former native overlaps are repaired.
    removed: Vec<OcclusionRecord>,
    // Exact inherited clip membership, rebuilt only when topology changes.
    // Animated clip geometry must not reconcile unrelated scroll islands.
    clip_dependents: HashMap<u32, Vec<u32>>,
    // Topology proof: these scrollers own clipped layers with no escaped or
    // shared-layer descendants. Scrolling changes exposure, not internal masks.
    isolated_scrollers: HashMap<u32, usize>,
    // Presentation bounds still move with ancestor scrolling.
    scroll_containers: HashMap<u32, Vec<u32>>,
    leaf_scroll_regions: HashSet<u32>,
    owned_layers: HashMap<u32, usize>,
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
        if let Some(record) = self.records.remove(&id) {
            self.removed.push(record);
        }
        self.clip_dependents.remove(&id);
        self.dirty.remove(&id);
        self.scrolled.remove(&id);
        self.isolated_scrollers.remove(&id);
        self.scroll_containers.remove(&id);
        self.leaf_scroll_regions.remove(&id);
        self.owned_layers.remove(&id);
    }
}

/// Reconcile structural changes, otherwise update only dirty geometry and overlapping masks.
pub fn update_node_occlusion(root_node: &Rc<ExpandedNode>, ctx: &RuntimeContext) {
    update_node_occlusion_observed(root_node, ctx, &mut |_| {});
}

pub(crate) fn update_node_occlusion_observed(
    root_node: &Rc<ExpandedNode>,
    ctx: &RuntimeContext,
    completed: &mut dyn FnMut(&'static str),
) {
    let mut state_guard = ctx.occlusion_state.borrow_mut();
    let state = &mut *state_guard;
    state.stats.passes += 1;
    state.culling_dirty = true;
    let mut dirty = std::mem::take(&mut state.dirty);
    let scrolled = std::mem::take(&mut state.scrolled);
    let structural = std::mem::take(&mut state.structural);
    let mut affected = HashSet::new();
    let reconciled = structural
        && !state.records.is_empty()
        && try_reconcile_structure(root_node, ctx, state, &mut dirty, &mut affected);
    completed("occlusion.topology");
    let rebuild = if (structural && !reconciled) || state.records.is_empty() {
        Some(("structure", None))
    } else {
        dirty
            .iter()
            .find_map(|id| {
                let Some(record) = state.records.get(id) else {
                    return Some(("new-record", Some(*id)));
                };
                let Some(node) = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(*id))
                else {
                    return Some(("missing-node", Some(*id)));
                };
                let instance = borrow!(node.instance_node);
                let layer = if instance.materializes_native_surface(&node) {
                    Layer::Native
                } else {
                    instance.base().flags().layer
                };
                // Scroller geometry and changed clip membership retain the
                // conservative path. Existing clip geometry is branch-local.
                let reason = if state.unsafe_layers.contains(&record.render_layer_id) {
                    "unsafe-layer"
                } else if record.scrolls || instance.scrolls_content(&node) {
                    "scroll-container"
                } else if record.clip.is_some()
                    != instance.resolve_effect_clip_path(&node).is_some()
                {
                    "clip-topology"
                } else if record.layer != layer
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
                {
                    "presentation-topology"
                } else {
                    return None;
                };
                Some((reason, Some(*id)))
            })
            .or_else(|| {
                scrolled.iter().find_map(|id| {
                    // Escaped/unclipped content and shared-layer scrolling still require
                    // conservative reconciliation. Nested clipped islands do not.
                    (!state.isolated_scrollers.contains_key(id))
                        .then_some(("unisolated-scroll", Some(*id)))
                })
            })
    };
    if let Some((reason, node)) = rebuild {
        state.stats.rebuild_origins = std::mem::take(&mut state.invalidations);
        state.stats.rebuild_reason = Some(reason);
        state.stats.rebuild_node = node;
        state.culling_rebuild = true;
        state.records.clear();
        state.removed.clear();
        state.clip_dependents.clear();
        state.canvas_counts.clear();
        state.isolated_scrollers.clear();
        state.scroll_containers.clear();
        state.leaf_scroll_regions.clear();
        state.owned_layers.clear();
        state.unsafe_layers.clear();
        rebuild_node_occlusion(root_node, ctx, state, completed);
        for (&id, record) in &state.records {
            for &clip in &record.clips {
                state.clip_dependents.entry(clip).or_default().push(id);
            }
        }
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
        for record in state.records.values() {
            if record.scrolls || record.clip.is_some() || record.unclippable {
                container_layers.insert(record.render_layer_id);
            }
        }
        let mut unsafe_branches = HashSet::new();
        for (&id, record) in &state.records {
            let is_container = record.scrolls || record.clip.is_some();
            let unsafe_branch = record.unclippable
                || (record.scrolls
                    && (!record.clips_content || !state.owned_layers.contains_key(&id)));
            if !is_container && !unsafe_branch {
                continue;
            }
            let Some(node) = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id)) else {
                continue;
            };
            let mut parent = node.render_parent_node();
            while let Some(ancestor) = parent {
                let ancestor_id = ancestor.id.to_u32();
                if state.owned_layers.contains_key(&ancestor_id) {
                    if is_container {
                        state
                            .scroll_containers
                            .entry(ancestor_id)
                            .or_default()
                            .push(id);
                    }
                    if unsafe_branch {
                        unsafe_branches.insert(ancestor_id);
                    }
                }
                parent = ancestor.render_parent_node();
            }
        }
        for (&id, &layer) in &state.owned_layers {
            if state.records.get(&id).is_some_and(|r| r.clips_content)
                && !unsafe_branches.contains(&id)
                && !state.unsafe_layers.contains(&layer)
            {
                if scroll_region(id, ctx).is_some() {
                    state.isolated_scrollers.insert(id, layer);
                    if !container_layers.contains(&layer) {
                        state.leaf_scroll_regions.insert(id);
                    }
                }
            }
        }
        state.stats.rebuilds += 1;
        completed("occlusion.index");
        return;
    }

    // Clip lineage and layer/z-order remain fixed on this path. Revisit only
    // records inheriting a dirty clip, including their old/new native overlaps.
    // The same index includes nested containers but excludes escaped content.
    let clips: Vec<_> = dirty
        .iter()
        .copied()
        .filter(|id| state.records[id].clip.is_some())
        .collect();
    let mut containers = HashSet::new();
    for id in clips {
        dirty.extend(
            state
                .clip_dependents
                .get(&id)
                .into_iter()
                .flatten()
                .copied(),
        );
    }
    for &id in &dirty {
        let record = state.records.get_mut(&id).unwrap();
        if record.clip.is_some() {
            let node = ctx
                .get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id))
                .unwrap();
            record.clip = borrow!(node.instance_node).resolve_effect_clip_path(&node);
        }
        if record.clip.is_some() || record.scrolls {
            containers.insert(id);
        }
    }
    ctx.prepare_scene_geometry();
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
        containers.insert(id);
        for &child in state.scroll_containers.get(&id).into_iter().flatten() {
            containers.insert(child);
        }
    }
    // Per-pass scratch only: each ancestor's presented geometry is resolved
    // once, even when hundreds of descendants share its clip chain.
    let mut presented = HashMap::new();
    for id in containers {
        update_container_presentation(id, ctx, state, &mut presented);
    }
    // All scroll-only changes reaching this path have isolated content layers.
    // Native containers enforce their shared clips; their cutouts are content-
    // local and do not depend on the sampled viewport or scroll position.
    ctx.set_canvas_drawable_layers(
        state
            .canvas_counts
            .iter()
            .filter_map(|(layer, count)| (*count > 0).then_some(*layer))
            .collect(),
    );
    completed("occlusion.coverage");
    for id in affected {
        update_indexed_mask(id, ctx, state);
    }
    completed("occlusion.masks");
}

// Keep a full topology walk, but avoid resolving paint/clip geometry while proving
// that the existing presentation domains are unchanged. No retained state is
// mutated until the entire tree passes this conservative preflight.
fn try_reconcile_structure(
    root: &Rc<ExpandedNode>,
    ctx: &RuntimeContext,
    state: &mut OcclusionState,
    dirty: &mut HashSet<u32>,
    affected: &mut HashSet<u32>,
) -> bool {
    struct Walk {
        seen: HashSet<u32>,
        added: HashMap<u32, OcclusionRecord>,
        order: Vec<(Rc<ExpandedNode>, Occlusion)>,
        next_layer: usize,
    }
    fn complex(record: &OcclusionRecord) -> bool {
        record.scrolls || record.clip.is_some() || record.before_children || record.unclippable
    }
    fn visit(
        node: &Rc<ExpandedNode>,
        ctx: &RuntimeContext,
        state: &OcclusionState,
        dirty: &HashSet<u32>,
        layer_id: usize,
        container: Option<u32>,
        clips: &[u32],
        walk: &mut Walk,
    ) -> bool {
        let id = node.id.to_u32();
        if !walk.seen.insert(id) {
            return false;
        }
        let instance = borrow!(node.instance_node);
        let native = instance.materializes_native_surface(node);
        let layer = if native {
            Layer::Native
        } else {
            instance.base().flags().layer
        };
        let scrolls = instance.scrolls_content(node);
        let owns_layer = scrolls
            && ctx.globals().browser_allows_scroller_vector_layers.get()
            && (ctx
                .globals()
                .browser_allows_nested_scroller_vector_layers
                .get()
                || container.is_none());
        if owns_layer != state.owned_layers.contains_key(&id) {
            return false;
        }
        let before = native && instance.materializes_native_surface_before_children(node);
        let unclippable = node
            .get_common_properties()
            .borrow()
            .unclippable
            .get()
            .unwrap_or(false);
        let old = state.records.get(&id);
        let has_clip = if let Some(old) = old {
            if old.render_layer_id != layer_id
                || old.clips != clips
                || old.layer != layer
                || old.scrolls != scrolls
                || old.before_children != before
                || old.unclippable != unclippable
                || old.clips_content != instance.clips_content(node)
                || !old
                    .instance
                    .upgrade()
                    .is_some_and(|i| Rc::ptr_eq(&i, &instance))
                || (dirty.contains(&id)
                    && (complex(old) || state.unsafe_layers.contains(&layer_id)))
            {
                return false;
            }
            if dirty.contains(&id)
                && instance.resolve_effect_clip_path(node).is_some() != old.clip.is_some()
            {
                return false;
            }
            old.clip.is_some()
        } else {
            if scrolls
                || before
                || unclippable
                || state.unsafe_layers.contains(&layer_id)
                || instance.resolve_effect_clip_path(node).is_some()
            {
                return false;
            }
            walk.added.insert(
                id,
                OcclusionRecord {
                    instance: Rc::downgrade(&instance),
                    layer,
                    render_layer_id: layer_id,
                    scrolls: false,
                    clips_content: instance.clips_content(node),
                    before_children: false,
                    unclippable: false,
                    clip: None,
                    clips: clips.to_vec(),
                    path: None,
                    opacity: 0.0,
                    geometry: None,
                },
            );
            false
        };
        drop(instance);
        let child_layer = if let Some(&owned) = state.owned_layers.get(&id) {
            if owned != walk.next_layer {
                return false;
            }
            walk.next_layer += 1;
            owned
        } else {
            layer_id
        };
        let child_container = if scrolls || has_clip {
            Some(id)
        } else {
            container
        };
        let mut child_clips = clips.to_vec();
        if has_clip {
            child_clips.push(id);
        }
        if before {
            walk.order.push((
                Rc::clone(node),
                Occlusion {
                    render_layer_id: layer_id,
                    parent_frame: container,
                    z_index: walk.order.len() as i32,
                },
            ));
        }
        for child in node.children.get().iter().rev() {
            // Escaping branches use the existing full traversal, even when only
            // their siblings changed. Their mixed presentation domains are not
            // represented by this bounded reconciliation's clip lineage.
            if child
                .get_common_properties()
                .borrow()
                .unclippable
                .get()
                .unwrap_or(false)
                || !visit(
                    child,
                    ctx,
                    state,
                    dirty,
                    child_layer,
                    child_container,
                    &child_clips,
                    walk,
                )
            {
                return false;
            }
        }
        if !before && (layer != Layer::DontCare || has_clip) {
            walk.order.push((
                Rc::clone(node),
                Occlusion {
                    render_layer_id: layer_id,
                    parent_frame: container,
                    z_index: walk.order.len() as i32,
                },
            ));
        }
        true
    }
    if state
        .removed
        .iter()
        .any(|r| complex(r) || state.unsafe_layers.contains(&r.render_layer_id))
    {
        return false;
    }
    let mut walk = Walk {
        seen: HashSet::new(),
        added: HashMap::new(),
        order: Vec::new(),
        next_layer: 1,
    };
    if !visit(root, ctx, state, dirty, 0, None, &[], &mut walk)
        || walk.next_layer != ctx.layer_count.get()
        || state
            .records
            .iter()
            .any(|(id, r)| !walk.seen.contains(id) && complex(r))
    {
        return false;
    }

    // Renumbering after insertion/removal is not a relative-order change. For
    // real moves, repair overlaps only in the changed surviving order interval.
    let surviving: Vec<_> = walk
        .order
        .iter()
        .filter(|(n, _)| state.records.contains_key(&n.id.to_u32()))
        .map(|(n, _)| (n.occlusion.get().z_index, n.id.to_u32()))
        .collect();
    let mut previous = surviving.clone();
    previous.sort_unstable();
    let first = previous
        .iter()
        .zip(&surviving)
        .position(|(a, b)| a.1 != b.1);
    if let Some(first) = first {
        let last = previous
            .iter()
            .zip(&surviving)
            .rposition(|(a, b)| a.1 != b.1)
            .unwrap();
        for &(_, id) in &surviving[first..=last] {
            let record = &state.records[&id];
            if state.unsafe_layers.contains(&record.render_layer_id) {
                return false;
            }
        }
    }
    ctx.prepare_scene_geometry();
    if let Some(first) = first {
        let last = previous
            .iter()
            .zip(&surviving)
            .rposition(|(a, b)| a.1 != b.1)
            .unwrap();
        for &(_, id) in &surviving[first..=last] {
            let record = &state.records[&id];
            if record.layer == Layer::Canvas {
                include_old_overlaps(record, ctx, affected);
            } else if record.layer != Layer::DontCare {
                affected.insert(id);
            }
        }
    }
    let missing: Vec<_> = state
        .records
        .keys()
        .filter(|id| !walk.seen.contains(id))
        .copied()
        .collect();
    for id in missing {
        state.removed.push(state.records.remove(&id).unwrap());
    }
    for record in std::mem::take(&mut state.removed) {
        include_old_overlaps(&record, ctx, affected);
        if record.path.is_some() && record.opacity > f64::EPSILON {
            if let Some(geometry) = &record.geometry {
                *state.canvas_counts.entry(geometry.layer).or_default() -= 1;
            }
        }
    }
    for dependents in state.clip_dependents.values_mut() {
        dependents.retain(|id| walk.seen.contains(id));
    }
    state.stats.records_retained += state.records.len() as u64;
    for (id, record) in walk.added {
        for &clip in &record.clips {
            state.clip_dependents.entry(clip).or_default().push(id);
        }
        dirty.insert(id);
        state.records.insert(id, record);
    }
    dirty.retain(|id| walk.seen.contains(id));
    for (node, order) in walk.order {
        if assign_occlusion(&node, order, state.records[&node.id.to_u32()].layer, ctx) {
            state.stats.orders_updated += 1;
        }
    }
    state.stats.structural_reconciliations += 1;
    state.stats.rebuild_reason = Some("structure-reconciled");
    state.stats.rebuild_node = None;
    state.stats.rebuild_origins = std::mem::take(&mut state.invalidations);
    state.culling_rebuild = true;
    true
}

fn include_old_overlaps(
    record: &OcclusionRecord,
    ctx: &RuntimeContext,
    affected: &mut HashSet<u32>,
) {
    if let (Some(path), Some(geometry)) = (&record.path, &record.geometry) {
        affected.extend(ctx.compositing_nodes_intersecting(
            geometry.layer,
            &[path_in_surface_bounds(path, geometry)],
            true,
        ));
    }
}

// Returns true for order-only changes. Native property subscribers still receive
// the new z value; only canvas content/geometry invalidation is avoided.
fn assign_occlusion(
    node: &Rc<ExpandedNode>,
    new: Occlusion,
    layer: Layer,
    ctx: &RuntimeContext,
) -> bool {
    let old = node.occlusion.get();
    if new == old {
        return false;
    }
    let order_only =
        new.render_layer_id == old.render_layer_id && new.parent_frame == old.parent_frame;
    if order_only {
        if layer == Layer::Canvas {
            ctx.queue_canvas_node_order(new.render_layer_id, node.id.to_u32(), new.z_index);
        }
    } else {
        ctx.invalidate_scene_geometry(node.id.to_u32());
        if layer == Layer::Canvas {
            if old.render_layer_id != new.render_layer_id {
                ctx.enqueue_canvas_node_removal(old.render_layer_id, node.id.to_u32());
            }
            ctx.mark_canvas_node_dirty(node.id);
        }
        ctx.set_canvas_dirty(old.render_layer_id);
        ctx.set_canvas_dirty(new.render_layer_id);
    }
    node.occlusion.set(new);
    order_only
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
    for (&id, &layer) in &state.isolated_scrollers {
        if !state.leaf_scroll_regions.contains(&id) {
            continue;
        }
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

type PresentedContainerGeometry = (OcclusionBox, Option<OcclusionBox>);

fn presented_container_geometry(
    id: u32,
    ctx: &RuntimeContext,
    state: &OcclusionState,
    presented: &mut HashMap<u32, PresentedContainerGeometry>,
) -> Option<PresentedContainerGeometry> {
    if let Some(geometry) = presented.get(&id) {
        return Some(*geometry);
    }
    let record = state.records.get(&id)?;
    let node = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id))?;
    let transform = ctx.presentation_scroll_transform_for_node(&node);
    let geometry = (
        OcclusionBox::new_from_transform_and_bounds_with_affine(
            node.transform_and_bounds.get(),
            transform,
        ),
        record
            .clip
            .as_ref()
            .and_then(|path| OcclusionBox::new_from_path(&(transform * path.clone()))),
    );
    presented.insert(id, geometry);
    Some(geometry)
}

fn update_container_presentation(
    id: u32,
    ctx: &RuntimeContext,
    state: &OcclusionState,
    presented: &mut HashMap<u32, PresentedContainerGeometry>,
) {
    let Some(record) = state.records.get(&id) else {
        return;
    };
    if record.before_children {
        return;
    }
    let Some(node) = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id)) else {
        return;
    };
    let Some((bounds, _)) = presented_container_geometry(id, ctx, state, presented) else {
        return;
    };
    let viewport = ctx.globals().viewport.get().bounds;
    let mut clipped = OcclusionBox::from_viewport(viewport.0, viewport.1);
    // Match full reconciliation's conservative clip bounds. Exact paths still
    // belong to Frame/Scroller and native masks; no bbox replaces those masks.
    for clip_id in record.clips.iter().copied().chain(std::iter::once(id)) {
        let Some(owner_record) = state.records.get(&clip_id) else {
            continue;
        };
        let Some((bounds, effect_bounds)) =
            presented_container_geometry(clip_id, ctx, state, presented)
        else {
            continue;
        };
        if owner_record.scrolls {
            clipped = clipped.intersect(&bounds).unwrap_or(clipped);
        }
        if let Some(bounds) = effect_bounds {
            clipped = clipped.intersect(&bounds).unwrap_or(clipped);
        }
    }
    let hash = hash_presentation_bounds(Some(bounds), Some(clipped));
    if node.presentation_cache_hash.get() == hash {
        return;
    }
    if record.scrolls {
        ctx.enqueue_native_message(pax_message::NativeMessage::ScrollerUpdate(ScrollerPatch {
            id,
            content_layer_id: state.owned_layers.get(&id).map(|layer| *layer as u32),
            presented_bounds: Some(bounds.as_array()),
            presented_clip_bounds: Some(clipped.as_array()),
            ..Default::default()
        }));
    } else if record.clip.is_some() {
        ctx.enqueue_native_message(pax_message::NativeMessage::FrameUpdate(
            pax_message::FramePatch {
                id,
                presented_bounds: Some(bounds.as_array()),
                presented_clip_bounds: Some(clipped.as_array()),
                ..Default::default()
            },
        ));
    }
    node.presentation_cache_hash.set(hash);
}

fn scroll_region(id: u32, ctx: &RuntimeContext) -> Option<Rect> {
    let node = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id))?;
    let bounds = node.transform_and_bounds.get().bounds;
    let scroll = ctx
        .get_scroller_surface_scroll(id)
        .or_else(|| borrow!(node.instance_node).resolve_scroll_offset(&node))
        .unwrap_or_default();
    // Native mask rejection uses presentation-space AABBs. Under rotation,
    // their conservative overlap can include leaves just outside the local
    // viewport. Query that same envelope rather than dropping those leaves.
    let transform = ctx.presentation_scroll_transform_for_node(&node)
        * Affine::from(node.transform_and_bounds.get().transform);
    if !transform
        .inverse()
        .as_coeffs()
        .iter()
        .all(|v| v.is_finite())
    {
        return None;
    }
    let region = transform.inverse().transform_rect_bbox(
        transform.transform_rect_bbox(Rect::new(0.0, 0.0, bounds.0, bounds.1)),
    );
    Some(region + kurbo::Vec2::new(scroll.0, scroll.1))
}

fn path_in_surface_bounds(path: &BezPath, geometry: &PreparedCanvasGeometry) -> Rect {
    (geometry.surface_transform * geometry.world_transform.inverse() * path.clone()).bounding_box()
}

fn native_mask_inverse(world: Affine, presentation: Affine) -> Option<Affine> {
    // A collapsed native surface paints no area and has no local inverse.
    // Clear its previous mask until it expands instead of serializing NaN/inf
    // SVG coordinates. Do not reject small, still-invertible entrance scales.
    let transform = presentation * world;
    let inverse = transform.inverse();
    (transform.as_coeffs().iter().all(|v| v.is_finite())
        && inverse.as_coeffs().iter().all(|v| v.is_finite()))
    .then_some(inverse)
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
        let Some(occluder) = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(candidate))
        else {
            continue;
        };
        if occluder.occlusion.get().z_index <= node.occlusion.get().z_index {
            continue;
        }
        if let Some(entry) = native_mask_entry(&node, &occluder, coverage, ctx, state) {
            entries.push(entry);
        }
    }
    sort_mask_entries(&mut entries);
    emit_mask(&node, tab.bounds, entries, ctx);
}

fn native_mask_entry(
    native: &ExpandedNode,
    occluder: &ExpandedNode,
    coverage: &OcclusionRecord,
    ctx: &RuntimeContext,
    state: &OcclusionState,
) -> Option<MaskPathPatch> {
    if !(coverage.opacity > f64::EPSILON) {
        return None;
    }
    let native_record = &state.records[&native.id.to_u32()];
    let mixed = state.unsafe_layers.contains(&native_record.render_layer_id);
    let presentation = |node: &ExpandedNode| {
        if mixed {
            ctx.presentation_scroll_transform_for_node(node)
        } else {
            Affine::IDENTITY
        }
    };
    // In one scroll domain, P cancels from (P * N)^-1 * (P * C).
    // Do not introduce sampled scroll offsets (or rounding noise) into the
    // native-local cutout. Mixed/escaped domains retain relative presentation.
    let tab = native.transform_and_bounds.get();
    let native_presentation = presentation(native);
    let native_bounds =
        OcclusionBox::new_from_transform_and_bounds_with_affine(tab, native_presentation);
    let inverse = native_mask_inverse(Affine::from(tab.transform), native_presentation)?;
    let path = presentation(occluder) * coverage.path.as_ref()?.clone();
    let bounds = OcclusionBox::new_from_path(&path)?;
    let clips: Vec<_> = coverage
        .clips
        .iter()
        // Shared ancestors already clip the native element itself. Baking them
        // into its cutout would make a content-local mask depend on the viewport.
        .filter(|id| !native_record.clips.contains(id))
        .filter_map(|id| {
            let clip = state.records.get(id)?.clip.as_ref()?;
            let owner = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(*id))?;
            Some(presentation(&owner) * clip.clone())
        })
        .collect();
    if !clipped_coverage_bounds(bounds, &clips).is_some_and(|b| b.intersects(&native_bounds)) {
        return None;
    }
    Some(MaskPathPatch {
        path: bez_path_to_svg_path_data(&(inverse * path)),
        clips: clips
            .into_iter()
            .map(|clip| bez_path_to_svg_path_data(&(inverse * clip)))
            .collect(),
        opacity: Some(coverage.opacity),
    })
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
    completed: &mut dyn FnMut(&'static str),
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
        viewport_bounds,
        viewport_bounds,
        Affine::IDENTITY,
        &mut z_index,
        &mut next_layer_id,
        &mut drawables,
        &[],
        state,
    );
    // A rebuild can change layer ownership even without a lifecycle edit (for
    // example, switching native/scroller topology). Keep lighting's recipient
    // dependency snapshot aligned without invalidating stable light slots.
    ctx.mark_lighting_topology_dirty();
    let canvas_drawable_layers = drawables
        .iter()
        .filter_map(|drawable| match drawable {
            DrawableInfo::Canvas { layer_id, .. } => Some(*layer_id),
            DrawableInfo::Native { .. } => None,
        })
        .collect::<HashSet<_>>();
    ctx.set_canvas_drawable_layers(canvas_drawable_layers);
    completed("occlusion.coverage");
    // Classify mixed presentation domains before constructing masks. Other
    // layers can cancel shared native scroll transforms algebraically.
    for (&id, record) in &state.records {
        if record.unclippable || (record.scrolls && !state.owned_layers.contains_key(&id)) {
            state.unsafe_layers.insert(record.render_layer_id);
        }
    }
    let _native_mask_stats = update_native_masks(&drawables, ctx, state);
    completed("occlusion.masks");

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
            instance: Rc::downgrade(&instance_node),
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
    if effect_clip_path.is_some() {
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

        if assign_occlusion(node, new_occlusion, layer, ctx) {
            state.stats.orders_updated += 1;
        }

        drawables.push(DrawableInfo::Native {
            node: Rc::clone(node),
            layer,
            layer_id: current_layer_id,
        });
        *z_index += 1;
    }

    for child in node.presentation_children(ctx).iter().rev() {
        let cp = child.get_common_properties();
        let cp = borrow!(cp);
        let unclippable = cp.unclippable.get().unwrap_or(false);
        let (child_container, child_clip_bounds) = if unclippable {
            (None, viewport_bounds)
        } else {
            (descendant_container, descendant_clip_bounds)
        };

        update_node_occlusion_recursive(
            child,
            ctx,
            descendant_layer_id,
            child_container,
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

    if assign_occlusion(node, new_occlusion, layer, ctx) {
        state.stats.orders_updated += 1;
    }

    match layer {
        Layer::Canvas => {
            if let Some(coverage_path) = borrow!(node.instance_node).resolve_occlusion_path(node) {
                let record = state.records.get_mut(&node.id.to_u32()).unwrap();
                record.path =
                    OcclusionBox::new_from_path(&coverage_path).map(|_| coverage_path.clone());
                record.opacity = borrow!(node.instance_node).resolve_coverage_opacity(node);
                if record.path.is_some() && record.opacity > f64::EPSILON {
                    drawables.push(DrawableInfo::Canvas {
                        layer_id: current_layer_id,
                        node_id: node.id.to_u32(),
                    });
                }
            }
        }
        Layer::Native | Layer::NativeNonOccluding => {
            drawables.push(DrawableInfo::Native {
                node: Rc::clone(node),
                layer,
                layer_id: current_layer_id,
            });
        }
        Layer::DontCare => {}
    }

    *z_index += 1;
}

fn update_native_masks(
    drawables: &[DrawableInfo],
    ctx: &RuntimeContext,
    state: &OcclusionState,
) -> NativeMaskStats {
    let mut vector_above = HashMap::<usize, LayerCoverage>::new();
    #[cfg(debug_assertions)]
    let mut stats = NativeMaskStats::default();
    #[cfg(not(debug_assertions))]
    let stats = NativeMaskStats::default();

    for drawable in drawables.iter().rev() {
        match drawable {
            DrawableInfo::Canvas { layer_id, node_id } => {
                let coverage = &state.records[node_id];
                let mut path = coverage.path.as_ref().unwrap().clone();
                if state.unsafe_layers.contains(layer_id) {
                    let node = ctx
                        .get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(*node_id))
                        .unwrap();
                    path = ctx.presentation_scroll_transform_for_node(&node) * path;
                }
                let Some(bounds) = OcclusionBox::new_from_path(&path) else {
                    continue;
                };
                vector_above
                    .entry(*layer_id)
                    .or_default()
                    .push(*node_id, bounds);
            }
            DrawableInfo::Native {
                node,
                layer,
                layer_id,
            } => {
                #[cfg(debug_assertions)]
                {
                    stats.native_nodes += 1;
                }
                let t_and_b = node.transform_and_bounds.get();
                let size = t_and_b.bounds;
                let presentation = if state.unsafe_layers.contains(layer_id) {
                    ctx.presentation_scroll_transform_for_node(node)
                } else {
                    Affine::IDENTITY
                };
                let bounds =
                    OcclusionBox::new_from_transform_and_bounds_with_affine(t_and_b, presentation);
                let mut entries = if *layer == Layer::Native {
                    vector_above
                        .get(layer_id)
                        .filter(|coverage| coverage.bounds.is_some_and(|b| b.intersects(&bounds)))
                        .into_iter()
                        .flat_map(|coverage| coverage.entries.iter())
                        .filter(|(_, b)| b.intersects(&bounds))
                        .filter_map(|(id, _)| {
                            let occluder =
                                ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(*id))?;
                            native_mask_entry(node, &occluder, &state.records[id], ctx, state)
                        })
                        .collect::<Vec<_>>()
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

    #[test]
    fn collapsed_native_surfaces_have_no_local_mask_until_they_expand() {
        let presentation = Affine::translate((0.0, -400.0));
        for scale in [(0.0, 0.0), (0.0, 1.0), (1.0, 0.0)] {
            let world = Affine::translate((200.0, 500.0))
                * Affine::rotate(0.3)
                * Affine::scale_non_uniform(scale.0, scale.1);
            assert!(native_mask_inverse(world, presentation).is_none());
        }
        // Small but invertible transforms must not lose their masks to an
        // arbitrary determinant threshold during an entrance animation.
        for scale in [1e-8, 0.5, 1.0, -1.0] {
            let world = Affine::translate((200.0, 500.0)) * Affine::scale(scale);
            let inverse = native_mask_inverse(world, presentation).unwrap();
            let point = kurbo::Point::new(20.0, 30.0);
            let restored = inverse * (presentation * world * point);
            assert!((restored - point).hypot() < 1e-4);
        }
    }

    #[test]
    fn collapsing_native_surface_clears_old_mask_and_expansion_restores_it() {
        use crate::{ComponentInstance, InstanceNode, InstantiationArgs};
        use pax_runtime_api::{LocalProperty, Platform, OS};

        let mut engine = crate::test_support::empty_engine(
            (400.0, 400.0),
            Platform::Web,
            OS::Mac,
            Box::new(|| 0),
            Default::default(),
        );
        let root = engine.mount_root_component(ComponentInstance::instantiate(InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Default,
            prototypical_properties: crate::PropertiesInit::Factory(Box::new(|_, _| {
                Some(Rc::new(std::cell::RefCell::new(
                    pax_runtime_api::pax_value::PaxAny::Builtin(
                        pax_runtime_api::PaxValue::default(),
                    ),
                )))
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
        }));
        let ctx = &engine.runtime_context;
        let _graph = ctx.property_graph.enter();
        ctx.take_native_messages();
        for (scale, expected_entries) in [(1.0, 1), (0.0, 0), (1.0, 1)] {
            let tab = TransformAndBounds {
                transform: pax_runtime_api::math::Transform2::new(
                    (Affine::translate((50.0, 50.0)) * Affine::scale(scale)).as_coeffs(),
                ),
                bounds: (100.0, 100.0),
            };
            root.transform_and_bounds
                .replace_with(LocalProperty::new(tab));
            let drawables = [
                DrawableInfo::Native {
                    node: root.clone(),
                    layer: Layer::Native,
                    layer_id: 0,
                    bounds: OcclusionBox::new_from_transform_and_bounds_with_affine(
                        tab,
                        Affine::IDENTITY,
                    ),
                    presentation_transform: Affine::IDENTITY,
                },
                DrawableInfo::Canvas {
                    layer_id: 0,
                    entry: entry(rect(0.0, 0.0, 200.0, 200.0), vec![]),
                },
            ];
            update_native_masks(&drawables, ctx);
            let masks: Vec<_> = ctx
                .take_native_messages()
                .into_iter()
                .filter_map(|message| match message {
                    pax_message::NativeMessage::NativeMaskUpdate(patch) => Some(patch),
                    _ => None,
                })
                .collect();
            assert_eq!(
                masks.len(),
                1,
                "each scale transition updates the existing mask"
            );
            assert_eq!(masks[0].entries.len(), expected_entries);
            assert!(masks[0]
                .entries
                .iter()
                .all(|entry| { !entry.path.contains("NaN") && !entry.path.contains("inf") }));
        }
    }

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> BezPath {
        Rect::new(x0, y0, x1, y1).to_path(0.1)
    }

    #[test]
    fn clipped_animation_never_becomes_an_occluder_below_its_frame() {
        let frame = rect(0.0, 0.0, 400.0, 420.0);
        let text = OcclusionBox::new_from_path(&rect(0.0, 532.0, 400.0, 784.0)).unwrap();
        for offset in (0..600).step_by(10) {
            let bounds = OcclusionBox::new_from_path(&rect(
                0.0,
                offset as f64,
                400.0,
                offset as f64 + 140.0,
            ))
            .unwrap();
            assert!(!clipped_coverage_bounds(bounds, &[frame.clone()])
                .is_some_and(|b| b.intersects(&text)));
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
            let bounds = OcclusionBox::new_from_path(&rect(0.0, 0.0, 400.0, 400.0)).unwrap();
            assert!(clipped_coverage_bounds(bounds, &clips).is_none());
        }
    }

    #[test]
    fn partial_transformed_clips_keep_conservative_bounds() {
        let transform = Affine::translate((30.0, 80.0)) * Affine::rotate(0.2);
        let clip = transform * RoundedRect::new(0.0, 0.0, 100.0, 100.0, 20.0).to_path(0.1);
        let path = rect(-100.0, -100.0, 400.0, 400.0);
        let expected = OcclusionBox::new_from_path(&clip).unwrap();
        let bounds = OcclusionBox::new_from_path(&path).unwrap();
        assert_eq!(
            clipped_coverage_bounds(bounds, &[clip]).unwrap().as_array(),
            expected.as_array()
        );
    }

    #[test]
    fn unclipped_occluders_keep_their_full_bounds() {
        let path = rect(-20.0, 40.0, 100.0, 200.0);
        let expected = OcclusionBox::new_from_path(&path).unwrap();
        assert_eq!(
            clipped_coverage_bounds(expected, &[]).unwrap().as_array(),
            expected.as_array()
        );
    }
}
