//! Runtime-owned geometry preparation and spatial lookup, independent of canvas residency.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use kurbo::{Affine, BezPath, Rect, Shape};

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

/// Geometry shared by render candidate selection and primitive drawing.
///
/// Coverage is in the owning canvas's content coordinates, so native scrolling
/// changes the queried region without invalidating every descendant record.
pub struct PreparedCanvasGeometry {
    /// Owning logical canvas layer.
    pub layer: usize,
    /// Layout dimensions, kept separate from overflowing paint coverage.
    pub bounds: (f64, f64),
    /// Transform from primitive-local space to canvas content space.
    pub surface_transform: Affine,
    /// Transformed paint coverage including the renderer's conservative culling pad.
    pub coverage_bounds: Option<Rect>,
    /// Primitive-local geometry backing this prepared record.
    pub local: CanvasGeometry,
}

impl PreparedCanvasGeometry {
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
            layer,
            bounds,
            surface_transform,
            coverage_bounds,
            local,
        }
    }
}

/// Cumulative work counters for verifying geometry reuse and spatial-query scaling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneGeometryStats {
    /// Primitive geometry preparations, excluding cache hits.
    pub preparations: u64,
    /// Insertions or changes to indexed coverage/layer ownership.
    pub index_updates: u64,
    /// Region-query requests.
    pub queries: u64,
    /// Distinct candidates reaching final rectangle tests, accumulated across queries.
    pub candidate_tests: u64,
}

#[derive(Default)]
pub(crate) struct SceneGeometry {
    records: HashMap<u32, Rc<PreparedCanvasGeometry>>,
    dirty: HashSet<u32>,
    pending: Vec<u32>,
    layers: HashMap<usize, SpatialIndex>,
    pub stats: SceneGeometryStats,
}

impl SceneGeometry {
    pub fn invalidate(&mut self, id: u32) {
        if self.dirty.insert(id) {
            self.pending.push(id);
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
        self.stats.preparations += 1;
        let changed = self.records.get(&id).is_none_or(|old| {
            old.layer != geometry.layer || old.coverage_bounds != geometry.coverage_bounds
        });
        if changed {
            if let Some(old) = self.records.get(&id) {
                if let Some(index) = self.layers.get_mut(&old.layer) {
                    index.remove(id);
                    if index.is_empty() {
                        self.layers.remove(&old.layer);
                    }
                }
            }
            self.layers
                .entry(geometry.layer)
                .or_default()
                .insert(id, geometry.coverage_bounds);
            self.stats.index_updates += 1;
        }
        let geometry = Rc::new(geometry);
        self.records.insert(id, geometry.clone());
        geometry
    }

    pub fn remove(&mut self, id: u32) {
        self.dirty.remove(&id);
        if let Some(old) = self.records.remove(&id) {
            if let Some(index) = self.layers.get_mut(&old.layer) {
                index.remove(id);
                if index.is_empty() {
                    self.layers.remove(&old.layer);
                }
            }
        }
    }

    pub fn query(&mut self, layer: usize, regions: &[Rect]) -> Option<Vec<u32>> {
        self.stats.queries += 1;
        let index = self.layers.get(&layer)?;
        let mut candidates = HashSet::new();
        for region in regions {
            index.candidates(*region, &mut candidates);
        }
        self.stats.candidate_tests += candidates.len() as u64;
        let mut ids: Vec<_> = candidates
            .into_iter()
            .filter(|id| {
                let bounds = index.bounds[id];
                regions.iter().any(|region| intersects(bounds, *region))
            })
            .collect();
        ids.sort_unstable();
        // Keep the existing full-replay fallback for empty/unknown layer coverage.
        (!ids.is_empty()).then_some(ids)
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
struct SpatialIndex {
    bounds: HashMap<u32, Option<Rect>>,
    levels: BTreeMap<u32, Columns>,
    unbounded: HashSet<u32>,
}

impl SpatialIndex {
    fn is_empty(&self) -> bool {
        self.bounds.is_empty()
    }

    fn insert(&mut self, id: u32, bounds: Option<Rect>) {
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

    fn remove(&mut self, id: u32) {
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

    fn candidates(&self, region: Rect, result: &mut HashSet<u32>) {
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

    fn geometry(layer: usize, bounds: Option<Rect>) -> PreparedCanvasGeometry {
        PreparedCanvasGeometry {
            layer,
            bounds: (10.0, 10.0),
            surface_transform: Affine::IDENTITY,
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
