//! Shared, budgeted solid/support coverage for native and GPU consumers. Keys describe mounted identities; values contain
//! no nodes or subscriptions. Only the active version of a pair is retained.

use crate::{
    mesh3::{ConvexMesh, MeshProjection, ProjectedConvexMesh},
    DepthPlane,
};
use kurbo::{BezPath, PathEl};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

/// Mounted solid/support pair in one projection chart and compositing domain.
/// Node identities include mount epochs so recycled numeric IDs cannot alias.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RelationKey {
    /// Solid node ID and mount epoch.
    pub occluder: (u32, u64),
    /// Certified coplanar support owner ID and mount epoch.
    pub support: (u32, u64),
    /// Compositing domain; relations never cross its isolation boundary.
    pub domain: usize,
    /// Zero is the native/window chart; renderer leases own other chart IDs.
    pub chart: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        math::{Generic, Transform3},
        mesh3::{ConvexMesh, MeshProjection},
    };

    #[test]
    fn charts_share_one_budget_and_retire_without_touching_other_consumers() {
        let cache = Rc::new(RefCell::new(Relations::with_budget(2, 8192)));
        let native = RelationKey {
            occluder: (1, 7),
            support: (2, 9),
            domain: 3,
            chart: 0,
        };
        let mesh = ConvexMesh::sphere(12, 6);
        let projection =
            MeshProjection::new(Transform3::<Generic>::identity().coeffs(), false).unwrap();
        let solid = mesh.projected(projection, None);
        let path = cache
            .borrow_mut()
            .front_coverage(native, &solid, DepthPlane::default());
        let tile = RelationChart::new(cache.clone());
        let tile_key = RelationKey {
            chart: tile.id,
            ..native
        };
        let clipped = mesh.projected(projection, Some(kurbo::Rect::new(-0.5, -0.5, 0.5, 0.5)));
        let tile_path =
            cache
                .borrow_mut()
                .front_coverage(tile_key, &clipped, DepthPlane::default());
        assert_ne!(
            path, tile_path,
            "computational viewport is not a reusable window mask"
        );
        assert_eq!(cache.borrow().residency().0, 2);
        tile.remove_node(1);
        assert_eq!(cache.borrow().residency().0, 1);
        assert!(Rc::ptr_eq(
            &path,
            &cache
                .borrow_mut()
                .front_coverage(native, &solid, DepthPlane::default())
        ));
        cache
            .borrow_mut()
            .front_coverage(tile_key, &clipped, DepthPlane::default());
        drop(tile);
        assert_eq!(cache.borrow().residency().0, 1);
        cache.borrow_mut().remove(2);
        assert_eq!(cache.borrow().residency(), (0, 0));
    }

    #[test]
    fn active_relation_versions_share_coverage_and_obey_both_budgets() {
        let mesh = ConvexMesh::sphere(12, 6);
        let solid = mesh.projected(
            MeshProjection::new(Transform3::<Generic>::identity().coeffs(), false).unwrap(),
            None,
        );
        let key = RelationKey {
            occluder: (1, 1),
            support: (2, 1),
            domain: 0,
            chart: 0,
        };
        let mut cache = Relations::with_budget(2, 8192);
        let a = cache.front_coverage(key, &solid, DepthPlane::default());
        assert!(Rc::ptr_eq(
            &a,
            &cache.front_coverage(key, &solid, DepthPlane::default())
        ));
        let weak = Rc::downgrade(&a);
        drop(a);
        let changed = DepthPlane {
            presentation_layer: 0,
            coefficients: [0.1, 0., 0.2],
            backface: false,
        };
        let result = cache.front_coverage(key, &solid, changed);
        assert!(
            weak.upgrade().is_none(),
            "only the active pair revision is retained"
        );
        let expected = solid.front_contours(changed);
        assert_eq!(
            result
                .elements()
                .iter()
                .filter(|e| matches!(e, PathEl::MoveTo(_)))
                .count(),
            expected.len()
        );
        for support in 3..30 {
            cache.front_coverage(
                RelationKey {
                    support: (support, 1),
                    ..key
                },
                &solid,
                changed,
            );
            assert!(cache.residency().0 <= 2 && cache.residency().1 <= 8192);
        }
        assert!(cache.stats.evictions > 0);
        assert_eq!(
            cache.front_coverage(key, &solid, changed).as_ref(),
            result.as_ref(),
            "eviction changes work, never coverage"
        );
        cache.remove(1);
        assert_eq!(cache.residency(), (0, 0));
        let mut tiny = Relations::with_budget(2, 1);
        assert_eq!(
            tiny.front_coverage(key, &solid, changed).as_ref(),
            result.as_ref()
        );
        assert_eq!(tiny.residency(), (0, 0));
        assert_eq!(tiny.stats.oversized, 1);
    }
}

struct Relation {
    mesh: Weak<ConvexMesh>,
    projection: MeshProjection,
    viewport: Option<kurbo::Rect>,
    plane: [f64; 3],
    path: Rc<BezPath>,
    used: u64,
    bytes: usize,
}

/// Cumulative coverage work across all consumers of a relation store.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RelationStats {
    pub evaluations: u64,
    pub reuses: u64,
    pub evictions: u64,
    pub oversized: u64,
}

/// Exact coverage cache shared by native masks and GPU computational charts.
///
/// The default ceiling is 2,048 entries / 4 MiB of charged record and path data.
/// Eviction or an oversized result changes work, never geometric correctness.
/// Values contain no live nodes; mesh ownership is weak. GPU resources and
/// caller-owned copies are separate from this auxiliary cache budget.
pub struct Relations {
    records: HashMap<RelationKey, Relation>,
    bytes: usize,
    clock: u64,
    max_entries: usize,
    max_bytes: usize,
    next_chart: u64,
    pub stats: RelationStats,
}

impl Default for Relations {
    fn default() -> Self {
        Self::with_budget(2048, 4 * 1024 * 1024)
    }
}

impl Relations {
    /// Current entry count and charged bytes, excluding allocator overhead.
    pub fn residency(&self) -> (usize, usize) {
        (self.records.len(), self.bytes)
    }
    /// Creates one budget shared by every chart attached to this store.
    pub fn with_budget(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            records: HashMap::new(),
            bytes: 0,
            clock: 0,
            max_entries,
            max_bytes,
            next_chart: 1,
            stats: RelationStats::default(),
        }
    }

    /// Returns projected solid coverage strictly in front of the support plane.
    /// Validates mesh identity, projection (including camera clips), computational
    /// viewport and plane coefficients before reusing the active pair revision.
    pub fn front_coverage(
        &mut self,
        key: RelationKey,
        solid: &Rc<ProjectedConvexMesh>,
        plane: DepthPlane,
    ) -> Rc<BezPath> {
        self.clock += 1;
        if let Some(old) = self.records.get_mut(&key) {
            if old.mesh.ptr_eq(&Rc::downgrade(&solid.mesh))
                && old.projection == solid.projection
                && old.viewport == solid.projection_viewport()
                && old.plane == plane.coefficients
            {
                self.stats.reuses += 1;
                old.used = self.clock;
                return old.path.clone();
            }
        }
        self.remove_key(key);
        self.stats.evaluations += 1;
        let mut path = BezPath::new();
        // Coverage retention belongs here, not in per-mesh or per-pose histories.
        for contour in solid.compute_front_contours(plane) {
            path.move_to((contour[0][0], contour[0][1]));
            for p in &contour[1..] {
                path.line_to((p[0], p[1]));
            }
            path.close_path();
        }
        let path = BezPath::from_vec(
            path.into_iter()
                .collect::<Vec<_>>()
                .into_boxed_slice()
                .into_vec(),
        );
        let bytes = std::mem::size_of::<Relation>()
            + std::mem::size_of::<RelationKey>()
            + path.elements().len() * std::mem::size_of::<PathEl>();
        let path = Rc::new(path);
        if bytes > self.max_bytes || self.max_entries == 0 {
            self.stats.oversized += 1;
            return path;
        }
        while self.records.len() >= self.max_entries || self.bytes + bytes > self.max_bytes {
            let key = *self
                .records
                .iter()
                .min_by_key(|(_, value)| value.used)
                .unwrap()
                .0;
            self.remove_key(key);
            self.stats.evictions += 1;
        }
        self.bytes += bytes;
        self.records.insert(
            key,
            Relation {
                mesh: Rc::downgrade(&solid.mesh),
                projection: solid.projection,
                viewport: solid.projection_viewport(),
                plane: plane.coefficients,
                path: path.clone(),
                used: self.clock,
                bytes,
            },
        );
        path
    }

    fn remove_key(&mut self, key: RelationKey) {
        if let Some(old) = self.records.remove(&key) {
            self.bytes -= old.bytes;
        }
    }

    /// Retires all dependencies on an unmounted node across every chart.
    pub fn remove(&mut self, node: u32) {
        self.retain(|key| key.occluder.0 != node && key.support.0 != node);
    }

    fn retain(&mut self, keep: impl Fn(&RelationKey) -> bool) {
        self.records.retain(|key, record| {
            if keep(key) {
                true
            } else {
                self.bytes -= record.bytes;
                false
            }
        });
    }
}

/// Mounted geometry identity and shared cache, carried by retained drawing programs.
/// Contains no reactive node or platform resource ownership.
#[derive(Clone)]
pub struct CoverageContext {
    /// Drawable ID and mount epoch.
    pub node: (u32, u64),
    /// Certified coplanar support owner ID and mount epoch.
    pub support: (u32, u64),
    /// Compositing domain containing the drawable.
    pub domain: usize,
    /// Runtime-wide coverage owner shared with native presentation.
    pub relations: Rc<RefCell<Relations>>,
}

/// A renderer's computational chart. Dropping a tile/capture retires its entries
/// without invalidating native/window coverage or other live tiles.
pub struct RelationChart {
    pub relations: Rc<RefCell<Relations>>,
    pub id: u64,
}

impl RelationChart {
    /// Allocates a chart ID unique within this store for the renderer's lifetime.
    pub fn new(relations: Rc<RefCell<Relations>>) -> Self {
        let id = {
            let mut cache = relations.borrow_mut();
            let id = cache.next_chart;
            cache.next_chart = id.checked_add(1).expect("visibility chart ID overflow");
            id
        };
        Self { relations, id }
    }

    /// Tile eviction removes only this chart's dependence on a mounted node.
    pub fn remove_node(&self, node: u32) {
        self.relations.borrow_mut().retain(|key| {
            key.chart != self.id || (key.occluder.0 != node && key.support.0 != node)
        });
    }
}

impl Drop for RelationChart {
    fn drop(&mut self) {
        self.relations
            .borrow_mut()
            .retain(|key| key.chart != self.id);
    }
}
