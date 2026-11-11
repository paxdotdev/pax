use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use kurbo::{Affine, BezPath, Rect, Shape};
use pax_message::{borrow, MaskPathPatch, NativeMaskPatch, ScrollerPatch};
use pax_runtime_api::Layer;
#[cfg(test)]
use pax_runtime_api::{bez_path_to_svg_path_data, Window};

use crate::scene_geometry::{CoverageBounds, PreparedCanvasGeometry};
#[cfg(test)]
use crate::{node_interface::NodeLocal, TransformAndBounds};
use crate::{ExpandedNode, RuntimeContext};

use super::expanded_node::Occlusion;

mod plan;
#[cfg(test)]
#[path = "occlusion_reconcile_tests.rs"]
mod reconcile_tests;
use plan::{Contribution, Mapping, PathInput};
pub(crate) use plan::{MaskOutput, MaskPlan, MaskPlans};

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

    #[cfg(test)]
    fn union(&self, other: &Self) -> Self {
        Self {
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
            x2: self.x2.max(other.x2),
            y2: self.y2.max(other.y2),
        }
    }

    fn for_node(node: &ExpandedNode, ctx: &RuntimeContext) -> Self {
        let (width, height) = node.transform_and_bounds.get().bounds;
        let viewport = ctx.globals().viewport.get().bounds;
        let rect = ctx
            .presented_projection_for_node(node)
            .and_then(|p| {
                p.coverage(
                    Rect::new(0., 0., width, height),
                    node.has_perspective_camera()
                        .then_some(Rect::new(0., 0., viewport.0, viewport.1)),
                )
            })
            .unwrap_or(Rect::ZERO);
        Self {
            x1: rect.x0,
            y1: rect.y0,
            x2: rect.x1,
            y2: rect.y1,
        }
    }

    #[cfg(test)]
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

#[cfg(test)]
#[derive(Clone)]
struct CoverageEntry {
    bounds: OcclusionBox,
    path: BezPath,
    clips: Vec<BezPath>,
    opacity: f64,
}

#[cfg(test)]
#[derive(Default)]
struct LayerCoverage {
    bounds: Option<OcclusionBox>,
    entries: Vec<CoverageEntry>,
}

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(all(test, debug_assertions))]
#[derive(Default)]
struct NativeMaskStats {
    native_nodes: usize,
    updated_masks: usize,
    mask_entries: usize,
}

#[cfg(all(test, not(debug_assertions)))]
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
    /// Expanded nodes inspected while reconciling structural edits.
    pub topology_nodes_visited: u64,
    /// Nodes copied from unchanged retained subtrees without inspecting components.
    pub topology_nodes_reused: u64,
    /// Numeric paint-order changes sent without invalidating geometry.
    pub orders_updated: u64,
    /// Dirty/new records refreshed by localized coverage reconciliation.
    pub records_updated: u64,
    /// Native masks evaluated after structural or incremental reconciliation.
    pub masks_evaluated: u64,
    /// Canvas candidates examined for those masks.
    pub mask_candidates: u64,
    /// Unchanged dependency plans reused before path mapping or serialization.
    pub mask_plan_reuses: u64,
    /// Mask contributions mapped after a geometry/chart dependency changed.
    pub mask_path_mappings: u64,
    /// SVG paths serialized, excluding wire-message cloning.
    pub mask_path_serializations: u64,
    /// Scroller lineage compositions after a dependency change.
    pub scroll_lineage_preparations: u64,
    /// Local/source clip shapes prepared after invalidation.
    pub clip_geometry_preparations: u64,
    /// Camera projections rebuilt after pose/view/scroll inputs changed.
    pub projection_preparations: u64,
    /// Exact solid/support front-coverage evaluations (zero without `3d`).
    pub solid_relation_evaluations: u64,
    /// Shared or unchanged solid/support coverage reused.
    pub solid_relation_reuses: u64,
    /// Resident auxiliary relation/plan entries across all presentation domains.
    pub visibility_cache_entries: usize,
    /// Conservatively charged bytes of retained derived coverage and mask plans.
    pub visibility_cache_bytes: usize,
    /// Cause of the most recent full reconciliation, for opt-in profiling.
    pub rebuild_reason: Option<&'static str>,
    /// Triggering expanded node, when the cause was not structural.
    pub rebuild_node: Option<u32>,
    /// Bounded origins of structural invalidation consumed by the last rebuild.
    /// Populated only when diagnostic attribution is enabled.
    pub rebuild_origins: [Option<OcclusionInvalidation>; 8],
    /// Up to eight slowest dirty-record updates in the latest pass, each at least
    /// 0.5 ms. Opt-in CPU wall times include synchronous coverage reads, not GPU
    /// work; full rebuilds are reported by the phase profile instead.
    pub coverage_slowest: [Option<OcclusionCoverageTiming>; 8],
}

/// Diagnostic attribution of an incremental coverage update to an engine node.
#[derive(Clone, Copy, Debug, pax_message::serde::Serialize)]
#[serde(crate = "pax_message::serde")]
pub struct OcclusionCoverageTiming {
    /// Session-local expanded node ID, accepted by `RuntimeContext::inspect_node`.
    pub node: u32,
    /// CPU wall-clock milliseconds spent updating this record.
    pub elapsed_ms: f64,
}

fn record_coverage_timing(stats: &mut OcclusionStats, node: u32, elapsed_ms: f64) {
    if elapsed_ms < 0.5 {
        return;
    }
    let slot = stats
        .coverage_slowest
        .iter_mut()
        .min_by(|a, b| {
            a.map_or(0., |v| v.elapsed_ms)
                .total_cmp(&b.map_or(0., |v| v.elapsed_ms))
        })
        .unwrap();
    if slot.is_none_or(|old| old.elapsed_ms < elapsed_ms) {
        *slot = Some(OcclusionCoverageTiming { node, elapsed_ms });
    }
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
    clip: Option<Rc<BezPath>>,
    clip_transform: Affine,
    clips: Vec<u32>,
    path: Option<Rc<BezPath>>,
    #[cfg(feature = "3d")]
    local_path: Option<Rc<BezPath>>,
    coverage_bounds: CoverageBounds,
    opacity: f64,
    geometry: Option<Rc<PreparedCanvasGeometry>>,
}

#[derive(Clone)]
struct TopologyBranch {
    id: u32,
    parent: Option<u32>,
    end: usize,
    order_start: usize,
    order_end: usize,
    layer_start: usize,
    layer_end: usize,
    layer: usize,
    container: Option<u32>,
}

// Traversal boundaries belong to the retained compositing records, not a second
// scene graph. They let a localized edit reuse siblings without evaluating their
// component properties. Dense global paint order is still renumbered.
#[derive(Default)]
struct OcclusionTopology {
    branches: Vec<TopologyBranch>,
    positions: HashMap<u32, usize>,
    order: Vec<(u32, Occlusion)>,
}

impl OcclusionTopology {
    fn begin(
        &mut self,
        id: u32,
        parent: Option<u32>,
        layer: usize,
        container: Option<u32>,
        next_layer: usize,
    ) -> usize {
        let index = self.branches.len();
        self.positions.insert(id, index);
        self.branches.push(TopologyBranch {
            id,
            parent,
            end: index + 1,
            order_start: self.order.len(),
            order_end: self.order.len(),
            layer_start: next_layer,
            layer_end: next_layer,
            layer,
            container,
        });
        index
    }

    fn end(&mut self, index: usize, next_layer: usize) {
        let end = self.branches.len();
        let branch = &mut self.branches[index];
        branch.end = end;
        branch.order_end = self.order.len();
        branch.layer_end = next_layer;
    }
}

impl OcclusionState {
    pub(crate) fn contains_node(&self, id: u32) -> bool {
        self.records.contains_key(&id)
    }
}

#[derive(Default)]
pub(crate) struct OcclusionState {
    pub(crate) diagnostics: bool,
    pub(crate) invalidations: [Option<OcclusionInvalidation>; 8],
    pub(crate) structural: bool,
    pub(crate) structural_nodes: HashSet<u32>,
    pub(crate) structural_unknown: bool,
    pub(crate) dirty: HashSet<u32>,
    pub(crate) scrolled: HashSet<u32>,
    records: HashMap<u32, OcclusionRecord>,
    topology: OcclusionTopology,
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
        // The live parent link disappears during unmount. Retain the old branch
        // origin until reconciliation has repaired its former overlaps/order.
        if let Some(&index) = self.topology.positions.get(&id) {
            if let Some(parent) = self.topology.branches[index].parent {
                self.structural_nodes.insert(parent);
            }
        }
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
    state.stats.coverage_slowest = [None; 8];
    state.culling_dirty = true;
    let mut dirty = std::mem::take(&mut state.dirty);
    let scrolled = std::mem::take(&mut state.scrolled);
    let structural = std::mem::take(&mut state.structural);
    // A structural origin can carry new presentation data without a separate
    // shape-property effect (for example its first native scroll state). Local
    // reconciliation must repair that data as well as its tree membership.
    dirty.extend(state.structural_nodes.iter().copied());
    let mut affected = HashSet::new();
    let reconciled = structural
        && !state.records.is_empty()
        && try_reconcile_structure(root_node, ctx, state, &mut dirty, &mut affected);
    state.structural_nodes.clear();
    state.structural_unknown = false;
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
                // An isolated scroller can move/resize like an existing clip:
                // preserve its content layer and repair the affected branch.
                // Changing ownership or losing an invertible coordinate frame
                // must still reclassify the scene conservatively.
                let reason = if state.unsafe_layers.contains(&record.render_layer_id) {
                    "unsafe-layer"
                } else if record.scrolls != instance.scrolls_content(&node)
                    || record.clips_content != instance.clips_content(&node)
                {
                    "clip-topology"
                } else if record.scrolls
                    && (!state.isolated_scrollers.contains_key(id)
                        || scroll_region(*id, ctx).is_none())
                {
                    "scroll-container"
                } else if record.clip.is_some() != node.world_clip_geometry.get().0.is_some() {
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
                    (!state.isolated_scrollers.contains_key(id)
                        || scroll_region(*id, ctx).is_none())
                    .then_some(("unisolated-scroll", Some(*id)))
                })
            })
    };
    completed("occlusion.coverage.validate");
    if let Some((reason, node)) = rebuild {
        state.stats.rebuild_origins = std::mem::take(&mut state.invalidations);
        state.stats.rebuild_reason = Some(reason);
        state.stats.rebuild_node = node;
        state.culling_rebuild = true;
        state.records.clear();
        state.topology = Default::default();
        state.removed.clear();
        ctx.clear_occlusion_bounds();
        state.clip_dependents.clear();
        state.canvas_counts.clear();
        state.isolated_scrollers.clear();
        state.scroll_containers.clear();
        state.leaf_scroll_regions.clear();
        state.owned_layers.clear();
        state.unsafe_layers.clear();
        rebuild_node_occlusion(root_node, ctx, state);
        refresh_container_membership(ctx, state);
        completed("occlusion.coverage.rebuild");
        ctx.prepare_scene_geometry();
        completed("occlusion.coverage.geometry");
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
                record.coverage_bounds = occlusion_bounds(&node, record, &geometry, ctx);
                ctx.set_occlusion_bounds(id, geometry.layer, record.coverage_bounds);
                record.geometry = Some(geometry);
            }
        }
        completed("occlusion.coverage.records");
        // Publish complete topology and geometry before constructing any masks.
        // Both initial/rebuilt and incremental scenes use the same candidate query;
        // a native leaf receives only its final geometric mask for this generation.
        ctx.set_canvas_drawable_layers(state.canvas_counts.keys().copied().collect());
        ctx.mark_lighting_topology_dirty();
        completed("occlusion.coverage");
        let native: Vec<_> = state
            .records
            .iter()
            .filter_map(|(&id, record)| {
                matches!(record.layer, Layer::Native | Layer::NativeNonOccluding).then_some(id)
            })
            .collect();
        for id in native {
            update_indexed_mask(id, ctx, state);
        }
        completed("occlusion.masks");
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
        for &child in state.clip_dependents.get(&id).into_iter().flatten() {
            let retained = ctx
                .get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(child))
                .filter(|node| node.local_compositing.get())
                .and_then(|node| node.retained_native_placement())
                .is_some_and(|(owner, _)| owner == id || state.records[&owner].clips.contains(&id));
            // An external clip is enforced at the native host boundary. It
            // cannot alter content-local coverage or cutouts inside that host.
            // Edits to clips *within* the chart still repair their dependents.
            if !retained {
                dirty.insert(child);
            }
            if state.records[&child].scrolls || state.records[&child].clip.is_some() {
                containers.insert(child);
            }
        }
    }
    for &id in &dirty {
        let record = state.records.get_mut(&id).unwrap();
        if record.clip.is_some() {
            let node = ctx
                .get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id))
                .unwrap();
            record.clip = node.world_clip_geometry.get().0;
            record.clip_transform =
                Affine::from(node.presentation_transform_and_bounds.get().transform);
        }
        if record.clip.is_some() || record.scrolls {
            containers.insert(id);
        }
    }
    completed("occlusion.coverage.clips");
    ctx.prepare_scene_geometry();
    completed("occlusion.coverage.geometry");
    for id in dirty {
        let Some(node) = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id)) else {
            continue;
        };
        let record = state.records.get_mut(&id).unwrap();
        state.stats.records_updated += 1;
        if record.layer == Layer::DontCare {
            continue;
        }
        let started = state.diagnostics.then(web_time::Instant::now);
        let geometry = ctx.canvas_geometry_for_node(&node);
        if record.layer == Layer::Canvas {
            if let Some(old) = &record.geometry {
                if let Some(bounds) = record.coverage_bounds.query_rect() {
                    affected.extend(ctx.compositing_nodes_intersecting(old.layer, &[bounds], true));
                }
                if record.path.is_some() && record.opacity > f64::EPSILON {
                    *state.canvas_counts.entry(old.layer).or_default() -= 1;
                }
            }
            let path = resolve_record_path(&node)
                .filter(|path| OcclusionBox::new_from_path(path).is_some());
            retain_path(&mut record.path, path);
            #[cfg(feature = "3d")]
            retain_local_path(record, &node);
            record.opacity = borrow!(node.instance_node).resolve_coverage_opacity(&node);
            record.coverage_bounds = occlusion_bounds(&node, record, &geometry, ctx);
            ctx.set_occlusion_bounds(id, geometry.layer, record.coverage_bounds);
            if let Some(bounds) = record.coverage_bounds.query_rect() {
                affected.extend(ctx.compositing_nodes_intersecting(
                    geometry.layer,
                    &[bounds],
                    true,
                ));
                if record.path.is_some() && record.opacity > f64::EPSILON {
                    *state.canvas_counts.entry(geometry.layer).or_default() += 1;
                }
            }
        } else {
            affected.insert(id);
        }
        record.geometry = Some(geometry);
        if let Some(started) = started {
            record_coverage_timing(
                &mut state.stats,
                id,
                started.elapsed().as_secs_f64() * 1000.,
            );
        }
    }
    completed("occlusion.coverage.records");
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
    completed("occlusion.coverage.containers");
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

// Reclassify retained membership without resolving paths or camera geometry.
// A transferred clip/container must follow its new presentation ancestry on
// subsequent scrolls, and its old ancestors must stop updating it.
fn refresh_container_membership(ctx: &RuntimeContext, state: &mut OcclusionState) {
    state.clip_dependents.clear();
    state.scroll_containers.clear();
    state.isolated_scrollers.clear();
    state.leaf_scroll_regions.clear();
    state.unsafe_layers.clear();
    let mut container_layers = HashSet::new();
    let mut unsafe_branches = HashSet::new();
    for (&id, record) in &state.records {
        for &clip in &record.clips {
            state.clip_dependents.entry(clip).or_default().push(id);
        }
        if record.unclippable || (record.scrolls && !state.owned_layers.contains_key(&id)) {
            state.unsafe_layers.insert(record.render_layer_id);
        }
        let is_container = record.scrolls || record.clip.is_some();
        if is_container || record.unclippable {
            container_layers.insert(record.render_layer_id);
        }
        let unsafe_branch = record.unclippable
            || (record.scrolls && (!record.clips_content || !state.owned_layers.contains_key(&id)));
        if !is_container && !unsafe_branch {
            continue;
        }
        let parent_of = |id: u32| {
            state
                .topology
                .positions
                .get(&id)
                .and_then(|&i| state.topology.branches[i].parent)
        };
        let mut parent = parent_of(id);
        while let Some(ancestor) = parent {
            if state.owned_layers.contains_key(&ancestor) {
                if is_container {
                    state
                        .scroll_containers
                        .entry(ancestor)
                        .or_default()
                        .push(id);
                }
                if unsafe_branch {
                    unsafe_branches.insert(ancestor);
                }
            }
            parent = parent_of(ancestor);
        }
    }
    for (&id, &layer) in &state.owned_layers {
        if state.records.get(&id).is_some_and(|r| r.clips_content)
            && !unsafe_branches.contains(&id)
            && !state.unsafe_layers.contains(&layer)
            && scroll_region(id, ctx).is_some()
        {
            state.isolated_scrollers.insert(id, layer);
            if !container_layers.contains(&layer) {
                state.leaf_scroll_regions.insert(id);
            }
        }
    }
    // Classification belongs to topology, not a per-frame subtree scan.
    let owners: HashMap<_, _> = state
        .isolated_scrollers
        .iter()
        .map(|(&id, &layer)| (layer, id))
        .collect();
    for (&id, record) in &state.records {
        if let Some(node) = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id)) {
            let eligible = owners.get(&record.render_layer_id).is_some_and(|&owner| {
                node.retained_native_placement()
                    .is_some_and(|(id, _)| id == owner)
                    && borrow!(node.instance_node).geometry_is_scroller_local()
            });
            if node.local_compositing.get() != eligible {
                node.local_compositing.set(eligible);
            }
        }
    }
}

// Reuse unchanged subtree boundaries; inspect edited branches without resolving
// paint geometry. No retained state is mutated until the preflight succeeds.
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
        migrated: HashMap<u32, (usize, Vec<u32>)>,
        topology: OcclusionTopology,
        next_layer: usize,
        inspected: u64,
        reused: u64,
    }
    // Follow both old and current ancestry: reparenting must open the source and
    // destination branches. Projection's render parents, not lexical parents,
    // define this topology. Unknown notifications retain the full preflight.
    let mut changed = HashSet::new();
    let mut local = !state.structural_unknown
        && !state.topology.branches.is_empty()
        && state.unsafe_layers.is_empty();
    if local {
        for &id in state.structural_nodes.iter().chain(dirty.iter()) {
            let mut reaches_root = false;
            let mut old = Some(id);
            while let Some(id) = old {
                reaches_root |= id == root.id.to_u32();
                changed.insert(id);
                old = state
                    .topology
                    .positions
                    .get(&id)
                    .and_then(|&i| state.topology.branches[i].parent);
            }
            let mut node = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id));
            while let Some(current) = node {
                reaches_root |= current.id == root.id;
                changed.insert(current.id.to_u32());
                node = current.render_parent_node();
            }
            if !reaches_root {
                // A detached/replaced origin is not proof that every retained
                // branch is unchanged. Keep the conservative preflight.
                local = false;
                break;
            }
        }
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
        parent: Option<u32>,
        changed: Option<&HashSet<u32>>,
        walk: &mut Walk,
    ) -> bool {
        let id = node.id.to_u32();
        if changed.is_some_and(|changed| !changed.contains(&id)) {
            if let Some(&index) = state.topology.positions.get(&id) {
                let branch = &state.topology.branches[index];
                if branch.layer == layer_id
                    && branch.container == container
                    && branch.parent == parent
                    && branch.layer_start == walk.next_layer
                    && state.records.get(&id).is_some_and(|r| r.clips == clips)
                {
                    let new_start = walk.topology.branches.len();
                    let new_order_start = walk.topology.order.len();
                    for old in &state.topology.branches[index..branch.end] {
                        if !walk.seen.insert(old.id) || !state.records.contains_key(&old.id) {
                            return false;
                        }
                        let mut retained = old.clone();
                        retained.end = new_start + old.end - index;
                        retained.order_start =
                            new_order_start + old.order_start - branch.order_start;
                        retained.order_end = new_order_start + old.order_end - branch.order_start;
                        walk.topology
                            .positions
                            .insert(old.id, walk.topology.branches.len());
                        walk.topology.branches.push(retained);
                    }
                    for &(id, mut order) in
                        &state.topology.order[branch.order_start..branch.order_end]
                    {
                        order.z_index = walk.topology.order.len() as i32;
                        walk.topology.order.push((id, order));
                    }
                    walk.next_layer = branch.layer_end;
                    walk.reused += (branch.end - index) as u64;
                    return true;
                }
                // Changed inherited ownership is a reason to visit this branch,
                // not to throw away unrelated retained coverage.
            }
        }
        if !walk.seen.insert(id) {
            return false;
        }
        walk.inspected += 1;
        let index = walk
            .topology
            .begin(id, parent, layer_id, container, walk.next_layer);
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
            if old.layer != layer
                || old.scrolls != scrolls
                || old.before_children != before
                || old.unclippable != unclippable
                || old.clips_content != instance.clips_content(node)
                || !old
                    .instance
                    .upgrade()
                    .is_some_and(|i| Rc::ptr_eq(&i, &instance))
                || (dirty.contains(&id) && state.unsafe_layers.contains(&layer_id))
            {
                return false;
            }
            if dirty.contains(&id) {
                let has_clip = if node.has_perspective_camera() {
                    ctx.presented_clip_path(node).is_some()
                } else {
                    instance.resolve_effect_clip_path(node).is_some()
                };
                if has_clip != old.clip.is_some() {
                    return false;
                }
            }
            let old_parent = state.topology.positions.get(&id).map(|&i| {
                let branch = &state.topology.branches[i];
                (branch.parent, branch.container)
            });
            if old.render_layer_id != layer_id
                || old.clips != clips
                || old_parent != Some((parent, container))
            {
                walk.migrated.insert(id, (layer_id, clips.to_vec()));
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
                    clip_transform: Affine::IDENTITY,
                    clips: clips.to_vec(),
                    path: None,
                    #[cfg(feature = "3d")]
                    local_path: None,
                    coverage_bounds: CoverageBounds::Empty,
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
            walk.topology.order.push((
                id,
                Occlusion {
                    render_layer_id: layer_id,
                    parent_frame: container,
                    z_index: walk.topology.order.len() as i32,
                },
            ));
        }
        for child in node.presentation_children(ctx).iter().rev() {
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
                    Some(id),
                    changed,
                    walk,
                )
            {
                return false;
            }
        }
        if !before && (layer != Layer::DontCare || has_clip) {
            walk.topology.order.push((
                id,
                Occlusion {
                    render_layer_id: layer_id,
                    parent_frame: container,
                    z_index: walk.topology.order.len() as i32,
                },
            ));
        }
        walk.topology.end(index, walk.next_layer);
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
        migrated: HashMap::new(),
        topology: Default::default(),
        next_layer: 1,
        inspected: 0,
        reused: 0,
    };
    if !visit(
        root,
        ctx,
        state,
        dirty,
        0,
        None,
        &[],
        None,
        local.then_some(&changed),
        &mut walk,
    ) || walk.next_layer != ctx.layer_count.get()
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
        .topology
        .order
        .iter()
        .filter(|(id, _)| state.records.contains_key(id))
        .map(|(id, _)| *id)
        .collect();
    let previous: Vec<_> = state
        .topology
        .order
        .iter()
        .filter(|(id, _)| walk.seen.contains(id) && state.records.contains_key(id))
        .map(|(id, _)| *id)
        .collect();
    if previous.len() != surviving.len() {
        return false;
    }
    let first = previous.iter().zip(&surviving).position(|(a, b)| a != b);
    if let Some(first) = first {
        let last = previous
            .iter()
            .zip(&surviving)
            .rposition(|(a, b)| a != b)
            .unwrap();
        for &id in &surviving[first..=last] {
            let record = &state.records[&id];
            if state.unsafe_layers.contains(&record.render_layer_id) {
                return false;
            }
        }
    }
    ctx.prepare_scene_geometry();
    for &id in walk.migrated.keys() {
        let record = &state.records[&id];
        include_old_overlaps(record, ctx, affected);
        if matches!(record.layer, Layer::Native | Layer::NativeNonOccluding) {
            affected.insert(id);
        }
    }
    if let Some(first) = first {
        let last = previous
            .iter()
            .zip(&surviving)
            .rposition(|(a, b)| a != b)
            .unwrap();
        for &id in &surviving[first..=last] {
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
    let added_ids: HashSet<_> = walk.added.keys().copied().collect();
    for (id, record) in walk.added {
        for &clip in &record.clips {
            state.clip_dependents.entry(clip).or_default().push(id);
        }
        dirty.insert(id);
        state.records.insert(id, record);
    }
    let migrated = !walk.migrated.is_empty();
    for (id, (layer, clips)) in walk.migrated {
        let record = state.records.get_mut(&id).unwrap();
        // Keep the old geometry/coverage until the ordinary dirty-record path
        // repairs both sides of the move, including canvas counts per layer.
        record.render_layer_id = layer;
        record.clips = clips;
        dirty.insert(id);
    }
    dirty.retain(|id| walk.seen.contains(id));
    for &(id, order) in &walk.topology.order {
        // Avoid property reads/writes and scene lookup for an unchanged order.
        let unchanged = !added_ids.contains(&id)
            && state.topology.positions.get(&id).is_some_and(|&index| {
                let branch = &state.topology.branches[index];
                let old_index = if state.records[&id].before_children {
                    branch.order_start
                } else {
                    branch.order_end - 1
                };
                state
                    .topology
                    .order
                    .get(old_index)
                    .is_some_and(|old| old == &(id, order))
            });
        if unchanged {
            continue;
        }
        let Some(node) = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id)) else {
            continue;
        };
        if assign_occlusion(&node, order, state.records[&id].layer, ctx) {
            state.stats.orders_updated += 1;
        }
    }
    state.stats.topology_nodes_visited += walk.inspected;
    state.stats.topology_nodes_reused += walk.reused;
    state.topology = walk.topology;
    if migrated {
        refresh_container_membership(ctx, state);
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
    if let (Some(bounds), Some(geometry)) = (record.coverage_bounds.query_rect(), &record.geometry)
    {
        affected.extend(ctx.compositing_nodes_intersecting(geometry.layer, &[bounds], true));
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
    node.canvas_owner.set_if_neq(
        ctx.get_layer_scroller_owner(new.render_layer_id)
            .map(|owner| owner.to_u32()),
    );
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
        OcclusionBox::for_node(&node, ctx),
        if node.has_perspective_camera() {
            ctx.presented_clip_path(&node)
                .as_ref()
                .and_then(OcclusionBox::new_from_path)
        } else {
            record.clip.as_ref().and_then(|path| {
                OcclusionBox::new_from_path(
                    &(transform
                        * retained_pose_delta(&node, record.clip_transform)
                        * path.as_ref().clone()),
                )
            })
        },
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
    // A native Scroller can constrain descendants without supplying a vector
    // clip path. Follow container ownership, not only the mask-path lineage.
    let mut lineage = Vec::new();
    let mut container = Some(id);
    while let Some(id) = container {
        lineage.push(id);
        container = state
            .topology
            .positions
            .get(&id)
            .and_then(|&i| state.topology.branches[i].container);
    }
    for clip_id in lineage.into_iter().rev() {
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
    let hash = hash_node_presentation(node.as_ref(), ctx, Some(bounds), Some(clipped));
    if node.presentation_cache_hash.get() == hash {
        return;
    }
    if record.scrolls {
        ctx.enqueue_native_message(pax_message::NativeMessage::ScrollerUpdate(ScrollerPatch {
            id,
            content_layer_id: state.owned_layers.get(&id).map(|layer| *layer as u32),
            presentation_projection: Some(if node.has_perspective_camera() {
                ctx.presented_projection_for_node(&node)
                    .map(|p| p.matrix().coeffs().to_vec())
                    .unwrap_or_default()
            } else {
                vec![]
            }),
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
    let bounds = node.presentation_transform_and_bounds.get().bounds;
    let scroll = ctx
        .get_scroller_surface_scroll(id)
        .or_else(|| borrow!(node.instance_node).resolve_scroll_offset(&node))
        .unwrap_or_default();
    // Native mask rejection uses presentation-space AABBs. Under rotation,
    // their conservative overlap can include leaves just outside the local
    // viewport. Query that same envelope rather than dropping those leaves.
    let transform = ctx.presentation_scroll_transform_for_node(&node)
        * Affine::from(node.presentation_transform_and_bounds.get().transform);
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

fn occlusion_bounds(
    node: &ExpandedNode,
    record: &OcclusionRecord,
    geometry: &PreparedCanvasGeometry,
    ctx: &RuntimeContext,
) -> CoverageBounds {
    if record.layer != Layer::Canvas || node.is_render_source() {
        return CoverageBounds::Empty;
    }
    if node.has_perspective_camera() {
        #[cfg(feature = "3d")]
        if geometry.projected_solid.is_some() {
            return CoverageBounds::conservative(geometry.coverage_bounds);
        }
        // Custom hooks remain conservative unless they explicitly supply bounds
        // and coverage invalidation. Their exact callback stays live in queries.
        let bounds = if let Some(path) = cached_local_occlusion_path(node) {
            let Some(path) = path else {
                return CoverageBounds::Empty;
            };
            Some(path.bounding_box())
        } else {
            borrow!(node.instance_node).resolve_local_occlusion_bounds(node)
        };
        let Some(bounds) = bounds else {
            return CoverageBounds::Unbounded;
        };
        let Some(projection) = geometry.projection else {
            return CoverageBounds::Unbounded;
        };
        for point in [
            bounds.origin(),
            kurbo::Point::new(bounds.x1, bounds.y0),
            kurbo::Point::new(bounds.x1, bounds.y1),
            kurbo::Point::new(bounds.x0, bounds.y1),
        ] {
            // A projective horizon needs conservative inclusion, not a finite
            // rectangle made by dropping its undividable vertices.
            if projection.mapping().homogeneous(point)[2] <= 0. {
                return CoverageBounds::Unbounded;
            }
        }
        return CoverageBounds::conservative(projection.coverage(bounds, None));
    }
    let Some(path) = &record.path else {
        return CoverageBounds::Empty;
    };
    // Use the same presented-to-canvas chart for every contributor. The canvas
    // owner's scroll cancels out, while independently scrolling descendants in
    // a shared canvas retain their actual displacement.
    CoverageBounds::conservative(
        ctx.window_to_canvas_mapping(geometry.layer)
            .and_then(|mapping| mapping.as_affine())
            .map(|mapping| {
                (mapping * ctx.presentation_scroll_transform_for_node(node) * path.as_ref().clone())
                    .bounding_box()
            }),
    )
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

// Retained paths keep the pose at which they were prepared. An external or
// changed-neighbor query composes their current placement only when needed;
// host motion alone does not remap every path in the island.
fn retained_pose_delta(node: &ExpandedNode, prepared: Affine) -> Affine {
    if node.local_compositing.get() && node.retained_native_placement().is_some() {
        let current = Affine::from(node.presentation_transform_and_bounds.get().transform);
        if current != prepared {
            return current * prepared.inverse();
        }
    }
    Affine::IDENTITY
}

fn update_indexed_mask(id: u32, ctx: &RuntimeContext, state: &mut OcclusionState) {
    let Some(record) = state.records.get(&id) else {
        return;
    };
    let Some(node) = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id)) else {
        return;
    };
    #[cfg(feature = "3d")]
    if node.has_perspective_camera() {
        update_projected_mask(&node, ctx, state);
        return;
    }
    let geometry = ctx.canvas_geometry_for_node(&node);
    let tab = node.presentation_transform_and_bounds.get();
    let mixed = state.unsafe_layers.contains(&record.render_layer_id);
    let presentation_for = |node: &ExpandedNode| {
        if mixed {
            ctx.presentation_scroll_transform_for_node(node)
        } else {
            Affine::IDENTITY
        }
    };
    let presentation = presentation_for(&node);
    let Some(inverse) = native_mask_inverse(Affine::from(tab.transform), presentation) else {
        emit_empty_mask(&node, tab.bounds, ctx);
        return;
    };
    let query_bounds = CoverageBounds::conservative(
        ctx.window_to_canvas_mapping(geometry.layer)
            .and_then(|mapping| mapping.as_affine())
            .map(|mapping| {
                (mapping
                    * ctx.presentation_scroll_transform_for_node(&node)
                    * Affine::from(tab.transform))
                .transform_rect_bbox(Rect::new(0., 0., tab.bounds.0, tab.bounds.1))
            }),
    )
    .query_rect()
    .unwrap();
    let candidates = if record.layer == Layer::Native {
        ctx.compositing_nodes_intersecting(geometry.layer, &[query_bounds], false)
    } else {
        Vec::new()
    };
    state.stats.masks_evaluated += 1;
    state.stats.mask_candidates += candidates.len() as u64;
    let bounds = (presentation * Affine::from(tab.transform)).transform_rect_bbox(Rect::new(
        0.,
        0.,
        tab.bounds.0,
        tab.bounds.1,
    ));
    let mut contributions = Vec::new();
    for candidate in candidates {
        let Some(coverage) = state.records.get(&candidate) else {
            continue;
        };
        if !(coverage.opacity > f64::EPSILON) {
            continue;
        }
        let Some(path) = &coverage.path else {
            continue;
        };
        let Some(occluder) = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(candidate))
        else {
            continue;
        };
        let layer_order = occluder
            .presentation_layer
            .get()
            .cmp(&node.presentation_layer.get());
        if layer_order.is_lt() {
            continue;
        }
        let transform = presentation_for(&occluder);
        #[cfg(feature = "3d")]
        let (front, back, solid) = {
            let presented = transform
                * Affine::from(occluder.presentation_transform_and_bounds.get().transform);
            (
                ctx.presented_depth_plane(&occluder, presented),
                ctx.presented_depth_plane(&node, presentation * Affine::from(tab.transform)),
                ctx.canvas_geometry_for_node(&occluder)
                    .projected_solid
                    .clone()
                    .filter(|_| layer_order.is_eq()),
            )
        };
        #[cfg(feature = "3d")]
        let spatial =
            ctx.globals().renderer_is_gpu && (solid.is_some() || front.is_some() || back.is_some());
        #[cfg(not(feature = "3d"))]
        let spatial = false;
        if layer_order.is_eq()
            && !spatial
            && occluder.occlusion.get().z_index <= node.occlusion.get().z_index
        {
            continue;
        }
        let path = path.clone();
        #[cfg(feature = "3d")]
        let path = if let Some(solid) = &solid {
            let (support, mut plane) = ctx.support_depth(&node);
            let [a, b, c] = plane.coefficients;
            let t = transform.as_coeffs();
            plane.coefficients = [
                a * t[0] + b * t[1],
                a * t[2] + b * t[3],
                a * t[4] + b * t[5] + c - ctx.presentation_depth_offset(&occluder),
            ];
            ctx.solid_front_coverage(&occluder, support, geometry.layer, solid, plane)
        } else {
            path
        };
        #[cfg(feature = "3d")]
        let depth = (spatial && solid.is_none()).then(|| {
            (
                front.unwrap_or_default(),
                back.unwrap_or_default(),
                occluder.occlusion.get().z_index,
                node.occlusion.get().z_index,
            )
        });
        #[cfg(not(feature = "3d"))]
        let depth = None;
        #[cfg(feature = "3d")]
        let opacity = if spatial {
            conditional_native_coverage_opacity(
                coverage.opacity,
                occluder.computed_opacity.get(),
                &occluder.computed_opacity_scopes.get(),
                &node.computed_opacity_scopes.get(),
            )
        } else {
            coverage.opacity
        };
        #[cfg(not(feature = "3d"))]
        let opacity = coverage.opacity;
        contributions.push(Contribution {
            owner: (candidate, occluder.mount_epoch.get()),
            path: PathInput {
                path,
                mapping: Mapping::Affine(
                    transform
                        * coverage.geometry.as_ref().map_or(Affine::IDENTITY, |g| {
                            retained_pose_delta(&occluder, g.world_transform)
                        }),
                ),
            },
            clips: mask_clip_inputs(&coverage.clips, false, mixed, ctx, state)
                .into_iter()
                .filter(|((id, _), _)| !record.clips.contains(id))
                .collect(),
            depth,
            opacity,
        });
    }
    emit_plan(
        &node,
        MaskPlan {
            size: tab.bounds,
            bounds,
            inverse: Mapping::Affine(inverse),
            camera: None,
            contributions,
        },
        ctx,
    );
}

fn cached_local_occlusion_path(node: &ExpandedNode) -> Option<Option<Rc<BezPath>>> {
    let instance = borrow!(node.instance_node);
    (instance.local_occlusion_path_is_retained()
        && instance.local_occlusion_path_is_placement_independent())
    .then(|| node.local_occlusion_geometry.get().0)
}

fn resolve_record_path(node: &ExpandedNode) -> Option<BezPath> {
    if let Some(local) = cached_local_occlusion_path(node) {
        local.and_then(|path| Some(node.presentation_projection()?.project_path(&path, None)))
    } else {
        borrow!(node.instance_node).resolve_occlusion_path(node)
    }
}

fn retain_path(slot: &mut Option<Rc<BezPath>>, value: Option<BezPath>) {
    if slot.as_deref() != value.as_ref() {
        *slot = value.map(compact_path);
    }
}

fn compact_path(path: BezPath) -> Rc<BezPath> {
    Rc::new(BezPath::from_vec(
        path.into_iter()
            .collect::<Vec<_>>()
            .into_boxed_slice()
            .into_vec(),
    ))
}

#[cfg(feature = "3d")]
fn retain_local_path(record: &mut OcclusionRecord, node: &ExpandedNode) {
    let instance = borrow!(node.instance_node);
    if node.has_perspective_camera() && instance.local_occlusion_path_is_retained() {
        if let Some(path) = cached_local_occlusion_path(node) {
            record.local_path = path;
        } else {
            retain_path(
                &mut record.local_path,
                instance.resolve_local_occlusion_path(node),
            );
        }
    } else {
        record.local_path = None;
    }
}

fn mask_clip_inputs(
    ids: &[u32],
    projected: bool,
    include_scroll: bool,
    ctx: &RuntimeContext,
    state: &OcclusionState,
) -> Vec<((u32, u64), PathInput)> {
    ids.iter()
        .filter_map(|id| {
            let owner = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(*id))?;
            let path = if projected {
                if let Some(local) = owner.local_clip_geometry.get().0 {
                    PathInput {
                        path: local,
                        mapping: Mapping::Projected(
                            ctx.presented_projection_for_node(&owner)?,
                            None,
                        ),
                    }
                } else {
                    let world = owner.world_clip_geometry.get().0?;
                    let mapping = ctx.presented_projection_for_node(&owner)?.mapping()
                        * owner.presentation_projection()?.mapping().inverse()?;
                    PathInput {
                        path: world,
                        mapping: Mapping::Planar(mapping),
                    }
                }
            } else {
                PathInput {
                    path: state.records.get(id)?.clip.clone()?,
                    mapping: Mapping::Affine(
                        (if include_scroll {
                            ctx.presentation_scroll_transform_for_node(&owner)
                        } else {
                            Affine::IDENTITY
                        }) * retained_pose_delta(&owner, state.records.get(id)?.clip_transform),
                    ),
                }
            };
            Some(((*id, owner.mount_epoch.get()), path))
        })
        .collect()
}

fn emit_plan(node: &ExpandedNode, plan: MaskPlan, ctx: &RuntimeContext) {
    let size = plan.size;
    let output = ctx.prepare_mask_plan(node, plan);
    if node.native_mask_hash.get() != output.hash {
        node.native_mask_hash.set(output.hash);
        ctx.enqueue_native_message(pax_message::NativeMessage::NativeMaskUpdate(
            NativeMaskPatch {
                id: node.id.to_u32(),
                size_x: size.0,
                size_y: size.1,
                entries: output.entries.as_ref().clone(),
            },
        ));
    }
}

#[cfg(feature = "3d")]
fn update_projected_mask(node: &ExpandedNode, ctx: &RuntimeContext, state: &mut OcclusionState) {
    let geometry = ctx.canvas_geometry_for_node(node);
    let size = geometry.bounds;
    let presentation = ctx.prepared_presentation(node);
    let Some(back) = presentation.projection() else {
        return;
    };
    let Some(inverse) = presentation.inverse_mapping() else {
        emit_empty_mask(node, size, ctx);
        return;
    };
    let bounds = Rect::new(0., 0., size.0, size.1);
    // Native controls need no canvas paint bounds. Query their projected layout
    // rectangle, using the same coordinates as the native spatial index.
    let query = geometry.native_bounds().unwrap_or(Rect::ZERO);
    let candidates = if state
        .records
        .get(&node.id.to_u32())
        .is_some_and(|r| r.layer == Layer::Native)
    {
        ctx.compositing_nodes_intersecting(geometry.layer, &[query], false)
    } else {
        vec![]
    };
    state.stats.masks_evaluated += 1;
    state.stats.mask_candidates += candidates.len() as u64;
    let mut contributions = Vec::new();
    let back_bounds = back.coverage(bounds, None).unwrap_or(Rect::ZERO);
    for id in candidates {
        let Some(record) = state.records.get(&id) else {
            continue;
        };
        if record.opacity <= f64::EPSILON {
            continue;
        }
        let Some(front_node) = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id))
        else {
            continue;
        };
        let layer_order = front_node
            .presentation_layer
            .get()
            .cmp(&node.presentation_layer.get());
        if layer_order.is_lt() {
            continue;
        }
        let Some(back_depth) = ctx.depth_plane_in_presentation(node, presentation.depth_plane())
        else {
            continue;
        };
        let solid = ctx
            .canvas_geometry_for_node(&front_node)
            .projected_solid
            .clone();
        let (path, depth) = if layer_order.is_gt() && solid.is_some() {
            let Some(path) = record.path.clone() else {
                continue;
            };
            (
                PathInput {
                    path,
                    mapping: Mapping::Affine(Affine::IDENTITY),
                },
                None,
            )
        } else if let Some(solid) = solid {
            let (support, plane) = ctx.support_depth(node);
            (
                PathInput {
                    path: ctx.solid_front_coverage(
                        &front_node,
                        support,
                        geometry.layer,
                        &solid,
                        plane,
                    ),
                    mapping: Mapping::Affine(Affine::IDENTITY),
                },
                None,
            )
        } else {
            let front_presentation = ctx.prepared_presentation(&front_node);
            let Some(front) = front_presentation.projection() else {
                continue;
            };
            // Custom hooks keep their live callback convention unless they opt into
            // invalidation-driven retention independently of their bounds.
            let local = if borrow!(front_node.instance_node).local_occlusion_path_is_retained() {
                record.local_path.clone()
            } else {
                borrow!(front_node.instance_node)
                    .resolve_local_occlusion_path(&front_node)
                    .map(compact_path)
            };
            let Some(local) = local else {
                continue;
            };
            let Some(front_depth) =
                ctx.depth_plane_in_presentation(&front_node, front_presentation.depth_plane())
            else {
                continue;
            };
            (
                PathInput {
                    path: local,
                    mapping: Mapping::Projected(front, Some(back_bounds)),
                },
                Some((
                    front_depth,
                    back_depth,
                    front_node.occlusion.get().z_index,
                    node.occlusion.get().z_index,
                )),
            )
        };
        contributions.push(Contribution {
            owner: (id, front_node.mount_epoch.get()),
            path,
            clips: mask_clip_inputs(&record.clips, true, true, ctx, state),
            depth,
            opacity: conditional_native_coverage_opacity(
                record.opacity,
                front_node.computed_opacity.get(),
                &front_node.computed_opacity_scopes.get(),
                &node.computed_opacity_scopes.get(),
            ),
        });
    }
    let inverse =
        pax_runtime_api::math::ProjectedPlane::affine(Affine::IDENTITY).post_mapping(inverse);
    emit_plan(
        node,
        MaskPlan {
            size,
            bounds: back_bounds,
            inverse: Mapping::Projected(inverse, Some(bounds.inflate(1., 1.))),
            camera: Some(Some(back)),
            contributions,
        },
        ctx,
    );
}

#[cfg(test)]
fn project_mask_entry(
    path: &BezPath,
    clips: &[BezPath],
    inverse: pax_runtime_api::math::ProjectedPlane,
    bounds: Rect,
    opacity: f64,
) -> Option<MaskPathPatch> {
    // This is a computational viewport, not an authored clip. Keep its edges
    // outside the control's mask texture so antialiasing cannot mix a fully
    // covered border with the white mask backdrop. Authored boundaries are intact.
    let raster_bounds = bounds.inflate(1., 1.);
    let project = |path: &BezPath| {
        let path = inverse.project_path(path, Some(raster_bounds));
        (!path.elements().is_empty()).then(|| bez_path_to_svg_path_data(&path))
    };
    // Every clip is an intersection. If projection leaves any one empty, this
    // paint cannot occlude the native leaf; omit it on every chassis.
    Some(MaskPathPatch {
        path: project(path)?,
        clips: clips.iter().map(project).collect::<Option<Vec<_>>>()?,
        opacity: Some(opacity),
    })
}

#[cfg(test)]
fn camera_mask_entry(node: &ExpandedNode, ctx: &RuntimeContext) -> Option<MaskPathPatch> {
    if !node.has_perspective_camera() {
        return None;
    }
    let bounds = node.transform_and_bounds.get().bounds;
    let rect = Rect::new(0., 0., bounds.0, bounds.1);
    let corners = vec![
        rect.origin(),
        kurbo::Point::new(rect.x1, rect.y0),
        kurbo::Point::new(rect.x1, rect.y1),
        kurbo::Point::new(rect.x0, rect.y1),
    ];
    let visible = ctx
        .presented_projection_for_node(node)
        .map(|p| p.clip_polygon(corners.clone(), None))
        .unwrap_or_default();
    if visible == corners {
        return None;
    }
    let mut path = rect.to_path(0.1);
    if let Some(last) = visible.last() {
        // Opposite winding removes the visible polygon from this opaque cover.
        path.move_to(*last);
        for p in visible.iter().rev().skip(1) {
            path.line_to(*p);
        }
        path.close_path();
    }
    Some(MaskPathPatch {
        path: bez_path_to_svg_path_data(&path),
        clips: vec![],
        opacity: Some(1.),
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

// A native leaf already carries its ancestor opacity. Condition its cutout on
// those shared ancestors being present, or hidden siblings ghost through a fade.
// Independent native/canvas transparency still uses the platform approximation.
#[cfg(feature = "3d")]
fn conditional_native_coverage_opacity(
    coverage: f64,
    total: f64,
    occluder: &[pax_runtime_api::OpacityScope],
    native: &[pax_runtime_api::OpacityScope],
) -> f64 {
    if total <= f64::EPSILON {
        return 0.;
    }
    let paint = (coverage / total).clamp(0., 1.);
    occluder
        .iter()
        .filter(|scope| !native.iter().any(|other| other.node_id == scope.node_id))
        .fold(paint, |alpha, scope| alpha * scope.opacity as f64)
        .clamp(0., 1.)
}

fn emit_empty_mask(node: &ExpandedNode, size: (f64, f64), ctx: &RuntimeContext) {
    emit_plan(
        node,
        MaskPlan {
            size,
            bounds: Rect::ZERO,
            inverse: Mapping::Affine(Affine::IDENTITY),
            camera: node
                .has_perspective_camera()
                .then(|| ctx.presented_projection_for_node(node)),
            contributions: Vec::new(),
        },
        ctx,
    );
}

/// Resolve z-order, logical layers, clip membership and coverage before mask queries.
fn rebuild_node_occlusion(
    root_node: &Rc<ExpandedNode>,
    ctx: &RuntimeContext,
    state: &mut OcclusionState,
) {
    #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
    let pass_start = std::time::Instant::now();

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
        &[],
        None,
        state,
    );

    let new_layer_count = next_layer_id;
    if ctx.layer_count.get() != new_layer_count {
        ctx.layer_count.set(new_layer_count);
        ctx.enqueue_native_message(pax_message::NativeMessage::ShrinkLayersTo(
            new_layer_count as u32,
        ));
    }

    #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
    log::trace!(
        "occlusion topology: {}us, layers={}, records={}",
        pass_start.elapsed().as_micros(),
        new_layer_count,
        state.records.len(),
    );
}

// Resolve topology and coverage records before querying masks from the spatial index.
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
    active_clip_ids: &[u32],
    parent: Option<u32>,
    state: &mut OcclusionState,
) {
    let topology_index = state.topology.begin(
        node.id.to_u32(),
        parent,
        current_layer_id,
        active_container,
        *next_layer_id,
    );
    let instance_node = borrow!(node.instance_node);
    let world_clip = node.world_clip_geometry.get().0;
    let effect_clip_path = if node.has_perspective_camera() {
        ctx.presented_clip_path(node)
    } else {
        world_clip
            .clone()
            .map(|clip| active_scroll_transform * clip.as_ref().clone())
    };
    let has_effect_clip = effect_clip_path.is_some();
    let scrolls_content = instance_node.scrolls_content(node);
    let scroll_transform = if scrolls_content {
        ctx.scroller_content_presentation_transform(node)
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
            clip_transform: Affine::from(node.presentation_transform_and_bounds.get().transform),
            clips: active_clip_ids.to_vec(),
            path: None,
            #[cfg(feature = "3d")]
            local_path: None,
            coverage_bounds: CoverageBounds::Empty,
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
    let presented_bounds = OcclusionBox::for_node(node, ctx);
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
        state.topology.order.push((node.id.to_u32(), new_occlusion));

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
            if unclippable {
                &[]
            } else {
                &descendant_clip_ids
            },
            Some(node.id.to_u32()),
            state,
        );
    }

    if materializes_native_surface_before_children {
        state.topology.end(topology_index, *next_layer_id);
        return;
    }

    if layer == Layer::DontCare && !has_effect_clip {
        state.topology.end(topology_index, *next_layer_id);
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
            hash_node_presentation(node, ctx, Some(presented_bounds), presented_clip_bounds);
        let presentation_changed = node.presentation_cache_hash.get() != presentation_hash;
        if node.browser_content_layer_id.get() != content_layer_id || presentation_changed {
            ctx.enqueue_native_message(pax_message::NativeMessage::ScrollerUpdate(ScrollerPatch {
                id: node.id.to_u32(),
                content_layer_id,
                presentation_projection: Some(if node.has_perspective_camera() {
                    ctx.presented_projection_for_node(node)
                        .map(|p| p.matrix().coeffs().to_vec())
                        .unwrap_or_default()
                } else {
                    vec![]
                }),
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
            hash_node_presentation(node, ctx, Some(presented_bounds), presented_clip_bounds);
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
    state.topology.order.push((node.id.to_u32(), new_occlusion));
    state.topology.end(topology_index, *next_layer_id);

    match layer {
        Layer::Canvas => {
            #[cfg(feature = "3d")]
            retain_local_path(state.records.get_mut(&node.id.to_u32()).unwrap(), node);
            if let Some(coverage_path) = resolve_record_path(node) {
                let record = state.records.get_mut(&node.id.to_u32()).unwrap();
                retain_path(
                    &mut record.path,
                    OcclusionBox::new_from_path(&coverage_path).map(|_| coverage_path),
                );
                record.opacity = borrow!(node.instance_node).resolve_coverage_opacity(node);
            }
        }
        Layer::Native | Layer::NativeNonOccluding => {}
        Layer::DontCare => {}
    }

    *z_index += 1;
}

// Retained only as a differential oracle for the pre-indexed affine pass.
#[cfg(test)]
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
                let t_and_b = node.presentation_transform_and_bounds.get();
                let size = t_and_b.bounds;
                let layer_coverage = vector_above.get(layer_id);
                let mut entries = if *layer == Layer::Native {
                    let has_overlap = layer_coverage
                        .and_then(|coverage| coverage.bounds)
                        .is_some_and(|coverage_bounds| coverage_bounds.intersects(bounds));
                    if let Some(inverse) = has_overlap
                        .then(|| {
                            native_mask_inverse(
                                Affine::from(t_and_b.transform),
                                *presentation_transform,
                            )
                        })
                        .flatten()
                    {
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

                if let Some(clip) = camera_mask_entry(node, ctx) {
                    entries.push(clip);
                }
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

fn hash_node_presentation(
    node: &ExpandedNode,
    ctx: &RuntimeContext,
    bounds: Option<OcclusionBox>,
    clips: Option<OcclusionBox>,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    hash_presentation_bounds(bounds, clips).hash(&mut hasher);
    node.has_perspective_camera().hash(&mut hasher);
    if node.has_perspective_camera() {
        ctx.presented_projection_for_node(node)
            .map(|p| p.matrix().coeffs().map(f64::to_bits))
            .hash(&mut hasher);
    }
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

// Compare the migrated structural path with the former affine pass using the
// same mounted records. This oracle is never present in a production binary.
#[cfg(all(test, feature = "3d"))]
pub(crate) fn assert_affine_masks_match_legacy(ctx: &RuntimeContext) {
    let state = ctx.occlusion_state.borrow();
    let mut ordered: Vec<_> = state
        .records
        .iter()
        .filter_map(|(&id, record)| {
            let node = ctx.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(id))?;
            Some((node.occlusion.get().z_index, node, record))
        })
        .collect();
    ordered.sort_by_key(|(z, _, _)| *z);
    let mut drawables = Vec::new();
    let mut expected = Vec::new();
    for (_, node, record) in ordered {
        assert!(!node.has_perspective_camera());
        let presentation = ctx.presentation_scroll_transform_for_node(&node);
        match record.layer {
            Layer::Canvas => {
                if let Some(path) = record
                    .path
                    .as_ref()
                    .filter(|_| record.opacity > f64::EPSILON)
                {
                    let path = presentation * path.as_ref().clone();
                    if let Some(bounds) = OcclusionBox::new_from_path(&path) {
                        drawables.push(DrawableInfo::Canvas {
                            layer_id: record.render_layer_id,
                            entry: CoverageEntry {
                                bounds,
                                path,
                                opacity: record.opacity,
                                clips: record
                                    .clips
                                    .iter()
                                    .filter_map(|id| {
                                        let clip = state.records.get(id)?.clip.as_ref()?;
                                        let owner = ctx.get_expanded_node_by_eid(
                                            crate::ExpandedNodeIdentifier(*id),
                                        )?;
                                        Some(
                                            ctx.presentation_scroll_transform_for_node(&owner)
                                                * clip.as_ref().clone(),
                                        )
                                    })
                                    .collect(),
                            },
                        });
                    }
                }
            }
            Layer::Native | Layer::NativeNonOccluding => {
                expected.push((node.clone(), node.native_mask_hash.get()));
                drawables.push(DrawableInfo::Native {
                    layer: record.layer,
                    layer_id: record.render_layer_id,
                    bounds: OcclusionBox::new_from_transform_and_bounds_with_affine(
                        node.presentation_transform_and_bounds.get(),
                        presentation,
                    ),
                    presentation_transform: presentation,
                    node,
                });
            }
            Layer::DontCare => {}
        }
    }
    drop(state);
    update_native_masks(&drawables, ctx);
    for (node, hash) in expected {
        assert_eq!(
            node.native_mask_hash.get(),
            hash,
            "native {:?} differs from affine oracle",
            node.id
        );
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "3d")]
    #[test]
    fn conditional_native_mask_cancels_shared_fade_only() {
        use pax_runtime_api::OpacityScope;
        let shared = OpacityScope {
            node_id: 1,
            opacity: 0.37,
        };
        let private = OpacityScope {
            node_id: 2,
            opacity: 0.5,
        };
        assert_eq!(
            super::conditional_native_coverage_opacity(0.37, 0.37, &[shared], &[shared]),
            1.
        );
        assert_eq!(
            super::conditional_native_coverage_opacity(0.185, 0.185, &[shared, private], &[shared]),
            0.5
        );
        assert_eq!(
            super::conditional_native_coverage_opacity(
                0.0925,
                0.185,
                &[shared, private],
                &[shared]
            ),
            0.25
        );
        assert_eq!(
            super::conditional_native_coverage_opacity(0., 0., &[shared], &[shared]),
            0.
        );
    }
    use super::*;
    use kurbo::{Rect, RoundedRect};

    #[test]
    fn projected_native_mask_preserves_coverage_past_control_edges() {
        use pax_runtime_api::math::ProjectedPlane;
        let bounds = Rect::new(0., 0., 100., 28.);
        let paint = Rect::new(40., -20., 200., 60.).to_path(0.1);
        let clip = Rect::new(-100., -100., 300., 300.).to_path(0.1);
        let entry = project_mask_entry(
            &paint,
            &[clip],
            ProjectedPlane::affine(Affine::IDENTITY),
            bounds,
            1.,
        )
        .unwrap();
        let path = BezPath::from_svg(&entry.path).unwrap();
        let clip = BezPath::from_svg(&entry.clips[0]).unwrap();
        // Samples just outside the control stay black through texture filtering;
        // the authored boundary at x=40 must not expand into visible content.
        for p in [(60., -0.5), (60., 28.5), (100.5, 14.)] {
            assert!(path.contains(p.into()) && clip.contains(p.into()));
        }
        assert!(!path.contains((39.5, 14.).into()));
    }

    #[test]
    fn moving_projected_clips_preserve_native_visibility_through_empty_intersections() {
        use pax_runtime_api::math::{PlanarProjection, ProjectedPlane};

        let mapping = PlanarProjection([1., 0., 0.002, 0.1, 1., 0.001, 50., 30., 1.]);
        let inverse =
            ProjectedPlane::affine(Affine::IDENTITY).post_mapping(mapping.inverse().unwrap());
        let bounds = Rect::new(0., 0., 120., 60.);
        let paint = Rect::new(0., 0., 400., 300.).to_path(0.1);
        let mut empty_frames = 0;
        let mut covered_frames = 0;
        // The paint remains over the control while an ancestor clip sweeps
        // across it and back. Empty intersections must never become full paint.
        for frame in (0..80).chain((0..80).rev()) {
            let x = frame as f64 * 5. - 200.;
            let clip = Rect::new(x, 0., x + 45., 300.);
            let entry = project_mask_entry(&paint, &[clip.to_path(0.1)], inverse, bounds, 1.);
            if entry.is_some() {
                covered_frames += 1;
            } else {
                empty_frames += 1;
            }
            for local_x in [13., 37., 61., 89., 113.] {
                for local_y in [11., 29., 47.] {
                    let local = kurbo::Point::new(local_x, local_y);
                    let screen = mapping.project(local).unwrap();
                    let expected = clip.contains(screen);
                    let actual = entry.as_ref().is_some_and(|entry| {
                        assert!(!entry.path.is_empty());
                        assert!(entry.clips.iter().all(|clip| !clip.is_empty()));
                        BezPath::from_svg(&entry.path).unwrap().contains(local)
                            && entry
                                .clips
                                .iter()
                                .all(|clip| BezPath::from_svg(clip).unwrap().contains(local))
                    });
                    assert_eq!(actual, expected, "frame {frame}, point {local:?}");
                }
            }
        }
        assert!(empty_frames > 0 && covered_frames > 0);
        assert!(project_mask_entry(&paint, &[BezPath::new()], inverse, bounds, 1.).is_none());
    }

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
            component_camera: None,
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
            root.presentation_transform_and_bounds
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
