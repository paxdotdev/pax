//! Runtime-owned geometry preparation and spatial lookup, independent of canvas residency.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use kurbo::{Affine, BezPath, Rect, Shape};

pub(crate) mod canvas_placement;
pub(crate) mod input;
pub(crate) mod presentation;
#[cfg(test)]
mod bench;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, pax_message::serde::Serialize)]
#[serde(crate = "pax_message::serde")]
/// Cumulative input-query work; exact predicates remain distinct from broad-phase selection.
pub struct InputQueryStats {
    /// Topology reconciliations, including a change of queried subtree root.
    pub topology_rebuilds: u64,
    /// Conservative input footprints prepared after invalidation.
    pub bounds_prepared: u64,
    /// Queries visiting only candidates and their necessary ancestors.
    pub indexed_queries: u64,
    /// Uncached preparation while reactive or compositing work was pending.
    pub pending_queries: u64,
    /// Uncached preparation during incremental startup.
    pub startup_queries: u64,
    /// Broad-phase candidates, including unknown conservative bounds.
    pub candidates: u64,
    /// Nodes reaching exact hit evaluation, including clipping ancestors.
    pub exact_tests: u64,
    /// Exact tests for shapes without a finite conservative screen bound.
    pub unknown_tests: u64,
}

/// Conservative coverage of a geometry role. Missing knowledge is not empty paint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum CoverageBounds {
    Empty,
    Finite(Rect),
    Unbounded,
}

impl CoverageBounds {
    pub(crate) fn conservative(bounds: Option<Rect>) -> Self {
        match bounds {
            Some(bounds) if finite(bounds) => Self::Finite(bounds),
            _ => Self::Unbounded,
        }
    }

    pub(crate) fn query_rect(self) -> Option<Rect> {
        match self {
            Self::Empty => None,
            Self::Finite(bounds) => Some(bounds),
            Self::Unbounded => Some(Rect::new(
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::INFINITY,
            )),
        }
    }
}

/// Primitive-local geometry prepared before drawing or spatial queries.
///
/// An absent bound is conservatively unbounded. Primitives that draw outside their
/// layout rectangle must supply their paint bounds. Paths can retain their prepared
/// command geometry here so selection and drawing do not construct it twice.
#[derive(Clone, Default)]
pub struct CanvasGeometry {
    /// Conservative local paint bounds; `None` requires inclusion in every region query.
    pub local_bounds: Option<Rect>,
    /// Optional prepared path reused by the primitive during drawing.
    pub path: Option<Rc<BezPath>>,
}

impl CanvasGeometry {
    /// Use the layout rectangle for a bounded primitive.
    pub fn for_layout(bounds: (f64, f64)) -> Self {
        Self {
            local_bounds: Some(Rect::new(0.0, 0.0, bounds.0, bounds.1)),
            path: None,
        }
    }
}

/// Geometry shared by render selection, primitive drawing, and viewport observation.
/// Native surfaces use a separate bounds lookup in the same geometry service.
/// Observed layout-only targets and their ancestors share these records without
/// entering either rendering index.
///
/// Coverage is in the owning canvas's content coordinates. Affine native scrolling
/// changes only the queried region. Perspective scrolling also updates projected
/// records because surfaces at different depths move by different screen distances.
#[derive(Clone)]
pub struct PreparedCanvasGeometry {
    /// Camera-projected transform before native scroll offsets, shared with viewport observation.
    pub world_transform: Affine,
    /// Camera-projected geometry used to inspect depth and geometric coplanarity.
    /// Kept separate from canvas ownership and logical opacity scopes.
    #[cfg(feature = "3d")]
    pub spatial_geometry: crate::SpatialGeometry,
    /// Retained convex volume, in unscrolled world space.
    #[cfg(feature = "3d")]
    pub solid: Option<Rc<pax_runtime_api::mesh3::PosedConvexMesh>>,
    /// Camera-clipped volume in window coordinates. Orthographic scrolling is
    /// applied by consumers; perspective scrolling is already included.
    #[cfg(feature = "3d")]
    pub projected_solid: Option<Rc<pax_runtime_api::mesh3::ProjectedConvexMesh>>,
    /// Mesh-local projection into the owning canvas, preserving camera clip planes.
    #[cfg(feature = "3d")]
    pub solid_projection: Option<pax_runtime_api::mesh3::MeshProjection>,
    pub(crate) is_canvas: bool,
    pub(crate) is_native: bool,
    /// Owning logical canvas layer.
    pub layer: usize,
    /// Layout dimensions, kept separate from overflowing paint coverage.
    pub bounds: (f64, f64),
    /// Transform from primitive-local space to canvas content space.
    pub surface_transform: Affine,
    /// Projective local-to-surface mapping, including camera clipping.
    pub projection: Option<pax_runtime_api::math::ProjectedPlane>,
    /// Projective mapping into window coordinates, including ancestor scrolling.
    pub world_projection: Option<pax_runtime_api::math::ProjectedPlane>,
    /// Transformed paint coverage including the renderer's conservative culling pad.
    pub coverage_bounds: Option<Rect>,
    /// Primitive-local geometry backing this prepared record.
    pub local: CanvasGeometry,
}

impl PreparedCanvasGeometry {
    pub(crate) fn native_bounds(&self) -> Option<Rect> {
        let bounds = Rect::new(0., 0., self.bounds.0, self.bounds.1);
        match self.projection {
            Some(projection) => projection.coverage(bounds, None),
            None => Some(self.surface_transform.transform_rect_bbox(bounds)),
        }
    }

    pub(crate) fn new(
        layer: usize,
        bounds: (f64, f64),
        surface_transform: Affine,
        local: CanvasGeometry,
    ) -> Self {
        // Preserve the renderer's existing conservative coverage policy. This is
        // paint coverage, not the target's layout or viewport-intersection bounds.
        const TILE_CULL_BOUNDS_PAD: f64 = 64.0;
        let coverage_bounds = local.local_bounds.map(|bounds| {
            (surface_transform * bounds.to_path(0.1))
                .bounding_box()
                .inflate(TILE_CULL_BOUNDS_PAD, TILE_CULL_BOUNDS_PAD)
        });
        Self {
            world_transform: surface_transform,
            #[cfg(feature = "3d")]
            spatial_geometry: crate::SpatialGeometry::from(crate::TransformAndBounds {
                transform: pax_runtime_api::math::Transform2::new(surface_transform.as_coeffs()),
                bounds,
            }),
            #[cfg(feature = "3d")]
            solid: None,
            #[cfg(feature = "3d")]
            projected_solid: None,
            #[cfg(feature = "3d")]
            solid_projection: None,
            is_canvas: true,
            is_native: false,
            layer,
            bounds,
            surface_transform,
            projection: None,
            world_projection: None,
            coverage_bounds,
            local,
        }
    }
}

/// Cumulative work counters for verifying geometry reuse and spatial-query scaling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneGeometryStats {
    /// Prepared record updates, excluding clean-record cache hits.
    pub preparations: u64,
    /// Calls to a primitive's local paint-geometry preparation hook.
    pub local_preparations: u64,
    /// Placement updates reusing unchanged surface-local paint coverage.
    pub surface_coverage_reuses: u64,
    /// Insertions or changes to indexed coverage/layer ownership.
    pub index_updates: u64,
    /// Region-query requests.
    pub queries: u64,
    /// Distinct candidates reaching final rectangle tests, accumulated across queries.
    pub candidate_tests: u64,
}

#[derive(Default)]
pub(crate) struct SceneGeometry {
    pub(crate) input: input::InputScene,
    pub(crate) presentation_work: Rc<presentation::PreparationWork>,
    pub(crate) mask_plans: crate::engine::occlusion::MaskPlans,
    #[cfg(feature = "3d")]
    pub(crate) presentations: crate::visibility::Presentations,
    #[cfg(feature = "3d")]
    pub(crate) relations: Rc<std::cell::RefCell<crate::visibility::Relations>>,
    records: HashMap<u32, Rc<PreparedCanvasGeometry>>,
    dirty: HashSet<u32>,
    content_dirty: HashSet<u32>,
    pending: Vec<u32>,
    layers: HashMap<usize, SpatialIndex>,
    native_layers: HashMap<usize, SpatialIndex>,
    // Occlusion shapes can deliberately exceed current paint bounds (for example
    // stable coverage during a draw animation). They need their own broad phase.
    occlusion_layers: HashMap<usize, SpatialIndex>,
    occlusion_bounds: HashMap<u32, (usize, CoverageBounds)>,
    #[cfg(feature = "3d")]
    spatial_nodes: HashSet<u32>,
    pub stats: SceneGeometryStats,
}

impl SceneGeometry {
    #[cfg(feature = "3d")]
    pub(crate) fn may_have_spatial_nodes(&self) -> bool {
        !self.spatial_nodes.is_empty() || !self.dirty.is_empty()
    }

    pub(crate) fn is_settled(&self) -> bool {
        self.dirty.is_empty()
    }

    pub fn invalidate(&mut self, id: u32) {
        if self.dirty.insert(id) {
            self.pending.push(id);
        }
    }

    pub fn invalidate_content(&mut self, id: u32) {
        self.content_dirty.insert(id);
        self.invalidate(id);
    }

    pub fn retained_paint(&self, id: u32) -> Option<Rc<PreparedCanvasGeometry>> {
        (!self.content_dirty.contains(&id))
            .then(|| self.records.get(&id).cloned())
            .flatten()
    }

    pub(crate) fn invalidate_existing(&mut self, id: u32) {
        if self.records.contains_key(&id) {
            self.invalidate(id);
        }
    }

    pub fn take_dirty(&mut self) -> Vec<u32> {
        // Walk queued IDs, not HashSet capacity left over from a large initial mount.
        let dirty = &self.dirty;
        self.pending
            .drain(..)
            .filter(|id| dirty.contains(id))
            .collect()
    }

    pub fn get(&self, id: u32) -> Option<Rc<PreparedCanvasGeometry>> {
        (!self.dirty.contains(&id))
            .then(|| self.records.get(&id).cloned())
            .flatten()
    }

    pub fn insert(
        &mut self,
        id: u32,
        geometry: PreparedCanvasGeometry,
    ) -> Rc<PreparedCanvasGeometry> {
        self.dirty.remove(&id);
        self.content_dirty.remove(&id);
        self.stats.preparations += 1;
        #[cfg(feature = "3d")]
        {
            if geometry.solid.is_none()
                && geometry
                    .spatial_geometry
                    .world_transform()
                    .try_as_2d()
                    .is_some()
            {
                self.spatial_nodes.remove(&id);
            } else {
                self.spatial_nodes.insert(id);
            }
        }
        let changed = self.records.get(&id).is_none_or(|old| {
            old.layer != geometry.layer
                || old.coverage_bounds != geometry.coverage_bounds
                || old.is_canvas != geometry.is_canvas
                || old.is_native != geometry.is_native
                || (geometry.is_native
                    && (old.bounds != geometry.bounds
                        || old.surface_transform != geometry.surface_transform
                        || old.projection != geometry.projection))
        });
        if changed {
            if let Some(old) = self.records.get(&id) {
                if let Some(index) = self.native_layers.get_mut(&old.layer) {
                    index.remove(id);
                    if index.is_empty() {
                        self.native_layers.remove(&old.layer);
                    }
                }
                if let Some(index) = self.layers.get_mut(&old.layer) {
                    index.remove(id);
                    if index.is_empty() {
                        self.layers.remove(&old.layer);
                    }
                }
            }
            if geometry.is_canvas {
                self.layers
                    .entry(geometry.layer)
                    .or_default()
                    .insert(id, geometry.coverage_bounds);
            }
            if geometry.is_native {
                self.native_layers
                    .entry(geometry.layer)
                    .or_default()
                    .insert(id, geometry.native_bounds());
            }
            self.stats.index_updates += 1;
        }
        let geometry = Rc::new(geometry);
        self.records.insert(id, geometry.clone());
        geometry
    }

    pub fn remove(&mut self, id: u32) {
        self.content_dirty.remove(&id);
        self.input.remove(id);
        self.mask_plans.remove(id);
        self.set_occlusion_bounds(id, 0, CoverageBounds::Empty);
        #[cfg(feature = "3d")]
        self.presentations.remove(id);
        #[cfg(feature = "3d")]
        self.relations.borrow_mut().remove(id);
        self.remove_geometry(id);
    }

    // A layout-only support can lose its observation/paint record while staying
    // mounted. Retire dependent visibility only on actual mount retirement.
    pub fn remove_geometry(&mut self, id: u32) {
        self.content_dirty.remove(&id);
        #[cfg(feature = "3d")]
        self.spatial_nodes.remove(&id);
        self.dirty.remove(&id);
        if let Some(old) = self.records.remove(&id) {
            if let Some(index) = self.native_layers.get_mut(&old.layer) {
                index.remove(id);
                if index.is_empty() {
                    self.native_layers.remove(&old.layer);
                }
            }
            if let Some(index) = self.layers.get_mut(&old.layer) {
                index.remove(id);
                if index.is_empty() {
                    self.layers.remove(&old.layer);
                }
            }
        }
    }

    pub fn query(&mut self, layer: usize, regions: &[Rect]) -> Option<Vec<u32>> {
        let ids = self.query_exact(layer, regions, false);
        // Canvas replay retains its historical full-replay fallback. Compositing
        // uses the exact query so an empty result means no affected elements.
        (!ids.is_empty()).then_some(ids)
    }

    pub(crate) fn set_occlusion_bounds(&mut self, id: u32, layer: usize, bounds: CoverageBounds) {
        if self.occlusion_bounds.get(&id) == Some(&(layer, bounds)) {
            return;
        }
        if let Some((old_layer, _)) = self.occlusion_bounds.remove(&id) {
            if let Some(index) = self.occlusion_layers.get_mut(&old_layer) {
                index.remove(id);
                if index.is_empty() {
                    self.occlusion_layers.remove(&old_layer);
                }
            }
        }
        if bounds != CoverageBounds::Empty {
            self.occlusion_layers.entry(layer).or_default().insert(
                id,
                match bounds {
                    CoverageBounds::Finite(bounds) => Some(bounds),
                    _ => None,
                },
            );
            self.occlusion_bounds.insert(id, (layer, bounds));
        }
    }

    pub(crate) fn query_occlusion(&mut self, layer: usize, regions: &[Rect]) -> Vec<u32> {
        Self::query_index(&self.occlusion_layers, &mut self.stats, layer, regions)
    }

    pub(crate) fn clear_occlusion_bounds(&mut self) {
        self.occlusion_layers.clear();
        self.occlusion_bounds.clear();
    }

    pub(crate) fn query_exact(&mut self, layer: usize, regions: &[Rect], native: bool) -> Vec<u32> {
        let indices = if native {
            &self.native_layers
        } else {
            &self.layers
        };
        Self::query_index(indices, &mut self.stats, layer, regions)
    }

    fn query_index(
        indices: &HashMap<usize, SpatialIndex>,
        stats: &mut SceneGeometryStats,
        layer: usize,
        regions: &[Rect],
    ) -> Vec<u32> {
        stats.queries += 1;
        let Some(index) = indices.get(&layer) else {
            return Vec::new();
        };
        let mut candidates = HashSet::new();
        for region in regions {
            index.candidates(*region, &mut candidates);
        }
        stats.candidate_tests += candidates.len() as u64;
        let mut ids: Vec<_> = candidates
            .into_iter()
            .filter(|id| {
                regions
                    .iter()
                    .any(|region| intersects(index.bounds[id], *region))
            })
            .collect();
        ids.sort_unstable();
        ids
    }
}

fn finite(rect: Rect) -> bool {
    [rect.x0, rect.y0, rect.x1, rect.y1]
        .iter()
        .all(|x| x.is_finite())
}

fn intersects(bounds: Option<Rect>, region: Rect) -> bool {
    let Some(bounds) = bounds.filter(|bounds| finite(*bounds)) else {
        return true;
    };
    !finite(region)
        || (bounds.x1 >= region.x0
            && bounds.x0 <= region.x1
            && bounds.y1 >= region.y0
            && bounds.y0 <= region.y1)
}

// A size-tiered sparse grid bounds insertion to at most four cells per rectangle.
// Ordered columns/rows query only occupied coordinates; neither long lists nor
// huge sparse scroll regions require enumerating every intervening cell.
type Columns = BTreeMap<i64, BTreeMap<i64, HashSet<u32>>>;

#[derive(Clone, Copy)]
struct CellRange {
    level: u32,
    x0: i64,
    x1: i64,
    y0: i64,
    y1: i64,
}

impl CellRange {
    fn for_bounds(bounds: Rect) -> Option<Self> {
        if !finite(bounds) || bounds.width() < 0.0 || bounds.height() < 0.0 {
            return None;
        }
        let span = bounds.width().max(bounds.height()).max(256.0);
        if !span.is_finite() {
            return None;
        }
        let level = (span / 256.0).log2().ceil().max(0.0) as u32;
        let range = Self::at_level(bounds, level);
        // Extreme coordinates can saturate integer conversion; conservative
        // unbounded fallback also protects against floating-point tier rounding.
        ((range.x1 as i128 - range.x0 as i128) <= 1 && (range.y1 as i128 - range.y0 as i128) <= 1)
            .then_some(range)
    }

    fn at_level(bounds: Rect, level: u32) -> Self {
        let size = 256.0 * 2.0f64.powi(level as i32);
        Self {
            level,
            x0: (bounds.x0 / size).floor() as i64,
            x1: (bounds.x1 / size).floor() as i64,
            y0: (bounds.y0 / size).floor() as i64,
            y1: (bounds.y1 / size).floor() as i64,
        }
    }
}

#[derive(Default)]
pub(crate) struct SpatialIndex {
    bounds: HashMap<u32, Option<Rect>>,
    levels: BTreeMap<u32, Columns>,
    unbounded: HashSet<u32>,
}

impl SpatialIndex {
    fn is_empty(&self) -> bool {
        self.bounds.is_empty()
    }

    pub(crate) fn insert(&mut self, id: u32, bounds: Option<Rect>) {
        self.bounds.insert(id, bounds);
        let Some(range) = bounds.and_then(CellRange::for_bounds) else {
            self.unbounded.insert(id);
            return;
        };
        let columns = self.levels.entry(range.level).or_default();
        for x in range.x0..=range.x1 {
            let rows = columns.entry(x).or_default();
            for y in range.y0..=range.y1 {
                rows.entry(y).or_default().insert(id);
            }
        }
    }

    pub(crate) fn remove(&mut self, id: u32) {
        let Some(bounds) = self.bounds.remove(&id) else {
            return;
        };
        self.unbounded.remove(&id);
        let Some(range) = bounds.and_then(CellRange::for_bounds) else {
            return;
        };
        let Some(columns) = self.levels.get_mut(&range.level) else {
            return;
        };
        for x in range.x0..=range.x1 {
            if let Some(rows) = columns.get_mut(&x) {
                for y in range.y0..=range.y1 {
                    if let Some(ids) = rows.get_mut(&y) {
                        ids.remove(&id);
                        if ids.is_empty() {
                            rows.remove(&y);
                        }
                    }
                }
                if rows.is_empty() {
                    columns.remove(&x);
                }
            }
        }
        if columns.is_empty() {
            self.levels.remove(&range.level);
        }
    }

    pub(crate) fn candidates(&self, region: Rect, result: &mut HashSet<u32>) {
        if !finite(region) {
            result.extend(self.bounds.keys().copied());
            return;
        }
        if region.width() < 0.0 || region.height() < 0.0 {
            return;
        }
        result.extend(self.unbounded.iter().copied());
        for (&level, columns) in &self.levels {
            let range = CellRange::at_level(region, level);
            for (_, rows) in columns.range(range.x0..=range.x1) {
                for (_, ids) in rows.range(range.y0..=range.y1) {
                    result.extend(ids);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn occlusion_bounds_are_independent_of_paint_and_retire_with_membership() {
        let mut scene = SceneGeometry::default();
        assert_eq!(
            CoverageBounds::conservative(None),
            CoverageBounds::Unbounded
        );
        assert_eq!(
            CoverageBounds::conservative(Some(Rect::new(0., 0., f64::INFINITY, 1.))),
            CoverageBounds::Unbounded
        );
        scene.insert(1, geometry(0, Some(Rect::new(0., 0., 5., 5.))));
        scene.set_occlusion_bounds(1, 0, CoverageBounds::Finite(Rect::new(0., 0., 200., 100.)));
        let region = Rect::new(150., 20., 170., 40.);
        assert!(scene.query_exact(0, &[region], false).is_empty());
        assert_eq!(scene.query_occlusion(0, &[region]), [1]);
        scene.set_occlusion_bounds(2, 0, CoverageBounds::Unbounded);
        scene.set_occlusion_bounds(3, 0, CoverageBounds::Empty);
        assert_eq!(scene.query_occlusion(0, &[region]), [1, 2]);
        scene.set_occlusion_bounds(1, 7, CoverageBounds::Finite(region));
        assert_eq!(scene.query_occlusion(0, &[region]), [2]);
        assert_eq!(scene.query_occlusion(7, &[region]), [1]);
        scene.remove(1);
        scene.set_occlusion_bounds(2, 0, CoverageBounds::Empty);
        assert!(scene.occlusion_layers.is_empty());
        assert!(scene.occlusion_bounds.is_empty());
    }

    fn geometry(layer: usize, bounds: Option<Rect>) -> PreparedCanvasGeometry {
        PreparedCanvasGeometry {
            world_transform: Affine::IDENTITY,
            #[cfg(feature = "3d")]
            spatial_geometry: crate::SpatialGeometry::default(),
            #[cfg(feature = "3d")]
            solid: None,
            #[cfg(feature = "3d")]
            projected_solid: None,
            #[cfg(feature = "3d")]
            solid_projection: None,
            is_canvas: true,
            is_native: false,
            layer,
            bounds: (10.0, 10.0),
            surface_transform: Affine::IDENTITY,
            projection: None,
            world_projection: None,
            coverage_bounds: bounds,
            local: CanvasGeometry::default(),
        }
    }

    #[test]
    fn updates_moves_layer_changes_and_removal_replace_old_coverage() {
        let mut scene = SceneGeometry::default();
        let a = Rect::new(0.0, 0.0, 100.0, 100.0);
        let b = Rect::new(4000.0, 0.0, 4100.0, 100.0);
        scene.insert(1, geometry(0, Some(a)));
        scene.insert(2, geometry(0, Some(b)));
        assert_eq!(scene.query(0, &[a]), Some(vec![1]));
        scene.insert(1, geometry(0, Some(a)));
        assert_eq!(
            scene.stats.index_updates, 2,
            "unchanged coverage keeps its index entries"
        );
        scene.insert(1, geometry(0, Some(b)));
        assert_eq!(scene.query(0, &[a]), None);
        assert_eq!(scene.query(0, &[b, b]), Some(vec![1, 2]));
        scene.insert(1, geometry(5, Some(a)));
        assert_eq!(scene.query(0, &[b]), Some(vec![2]));
        assert_eq!(scene.query(5, &[a]), Some(vec![1]));
        scene.invalidate(1);
        assert!(scene.get(1).is_none());
        scene.remove(1);
        assert!(scene.take_dirty().is_empty());
        assert!(!scene.layers.contains_key(&5));
        scene.remove(2);
        assert!(scene.layers.is_empty());
    }

    #[test]
    fn spatial_queries_match_linear_oracle_across_sizes_and_mutations() {
        let mut scene = SceneGeometry::default();
        let mut seed = 13891u64;
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            (seed >> 32) as u32
        };
        for id in 0..2000 {
            let x = next() as f64 % 40000.0 - 20000.0;
            let y = next() as f64 % 40000.0 - 20000.0;
            let size = 2.0f64.powi((next() % 18) as i32);
            scene.insert(
                id,
                geometry(
                    id as usize % 3,
                    Some(Rect::new(x, y, x + size, y + size / 3.0)),
                ),
            );
        }
        scene.insert(2000, geometry(0, None));
        scene.insert(
            2001,
            geometry(0, Some(Rect::new(f64::NAN, 0.0, 10.0, 10.0))),
        );
        scene.insert(
            2002,
            geometry(0, Some(Rect::new(-f64::MAX, -f64::MAX, f64::MAX, f64::MAX))),
        );
        for phase in 0..2 {
            for _ in 0..150 {
                let x = next() as f64 % 50000.0 - 25000.0;
                let y = next() as f64 % 50000.0 - 25000.0;
                let regions = [
                    Rect::new(x, y, x + 800.0, y + 900.0),
                    Rect::new(-50.0, -50.0, 0.0, 0.0),
                ];
                for layer in 0..3 {
                    let mut expected: Vec<_> = scene
                        .records
                        .iter()
                        .filter_map(|(&id, g)| {
                            (g.layer == layer
                                && regions.iter().any(|r| intersects(g.coverage_bounds, *r)))
                            .then_some(id)
                        })
                        .collect();
                    expected.sort_unstable();
                    assert_eq!(scene.query(layer, &regions).unwrap_or_default(), expected);
                }
            }
            if phase == 0 {
                for id in (0..2000).step_by(3) {
                    scene.remove(id);
                }
                for id in (1..2000).step_by(3) {
                    scene.insert(id, geometry(2, Some(Rect::new(-256.0, -256.0, 0.0, 0.0))));
                }
            }
        }
    }

    #[test]
    fn edge_contact_unknown_bounds_and_extreme_queries_are_conservative() {
        let mut scene = SceneGeometry::default();
        scene.insert(1, geometry(0, Some(Rect::new(-256.0, -256.0, 0.0, 0.0))));
        scene.insert(2, geometry(0, None));
        scene.insert(
            3,
            geometry(0, Some(Rect::new(1.0e200, 1.0e200, 1.0e200, 1.0e200))),
        );
        assert_eq!(
            scene.query(0, &[Rect::new(0.0, 0.0, 1.0, 1.0)]),
            Some(vec![1, 2])
        );
        assert_eq!(
            scene.query(0, &[Rect::new(-f64::MAX, -f64::MAX, f64::MAX, f64::MAX)]),
            Some(vec![1, 2, 3])
        );
        assert_eq!(
            scene.query(0, &[Rect::new(f64::NEG_INFINITY, 0.0, f64::INFINITY, 1.0)]),
            Some(vec![1, 2, 3])
        );
    }

    #[test]
    fn fixed_density_queries_do_not_scan_the_collection() {
        for horizontal in [false, true] {
            let mut costs = Vec::new();
            for count in [1000, 10000] {
                let mut scene = SceneGeometry::default();
                for id in 0..count {
                    let offset = id as f64 * 200.0;
                    let bounds = if horizontal {
                        Rect::new(offset, 0.0, offset + 100.0, 100.0)
                    } else {
                        Rect::new(0.0, offset, 100.0, offset + 100.0)
                    };
                    scene.insert(id, geometry(0, Some(bounds)));
                }
                let region = if horizontal {
                    Rect::new(100000.0, 0.0, 101000.0, 100.0)
                } else {
                    Rect::new(0.0, 100000.0, 100.0, 101000.0)
                };
                assert_eq!(scene.query(0, &[region]).unwrap().len(), 6);
                costs.push(scene.stats.candidate_tests);
                assert_eq!(scene.stats.preparations, count as u64);
            }
            assert_eq!(costs[0], costs[1]);
            assert!(
                costs[1] < 12,
                "only nearby buckets should reach detailed tests: {costs:?}"
            );
        }
    }

    #[test]
    fn preparation_preserves_canvas_coverage_padding_and_overflow() {
        let local = CanvasGeometry {
            local_bounds: Some(Rect::new(-25.0, -10.0, 125.0, 110.0)),
            path: None,
        };
        let prepared =
            PreparedCanvasGeometry::new(2, (100.0, 100.0), Affine::translate((10.0, 20.0)), local);
        assert_eq!(prepared.bounds, (100.0, 100.0));
        assert_eq!(
            prepared.coverage_bounds,
            Some(Rect::new(-79.0, -54.0, 199.0, 194.0))
        );
    }
}
