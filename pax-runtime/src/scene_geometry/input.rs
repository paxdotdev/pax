//! Input-only topology and immutable hit geometry, independent of paint bounds.
use super::{CoverageBounds, InputQueryStats, SpatialIndex};
use kurbo::{Affine, Point, Rect};
use pax_runtime_api::HitShape;
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub(crate) enum PreparedHit {
    Empty,
    Affine {
        shape: HitShape,
        inverse: Affine,
    },
    #[cfg(feature = "3d")]
    Projected {
        shape: HitShape,
        presentation: std::rc::Rc<crate::visibility::PreparedPresentation>,
    },
    #[cfg(feature = "3d")]
    Mesh {
        solid: std::rc::Rc<pax_runtime_api::mesh3::ProjectedConvexMesh>,
        inverse: Affine,
    },
}
impl PreparedHit {
    pub fn test(&self, point: Point) -> (bool, Option<f64>) {
        match self {
            Self::Empty => (false, None),
            Self::Affine { shape, inverse } => (shape.contains_local(*inverse * point), None),
            #[cfg(feature = "3d")]
            Self::Projected {
                shape,
                presentation,
            } => (
                presentation
                    .local_point(point)
                    .is_some_and(|p| shape.contains_local(p)),
                None,
            ),
            #[cfg(feature = "3d")]
            Self::Mesh { solid, inverse } => {
                let p = *inverse * point;
                let depth = solid.depth_at(p.x, p.y);
                (depth.is_some(), depth)
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct InputPlacement {
    pub epoch: u64,
    pub parent: Option<u32>,
    pub rank: usize,
    pub end: usize,
    pub scroll_parent: Option<u32>,
    pub scrolls: bool,
    // Perspective projection already incorporates scrolling before projection.
    // Affine shapes instead stay in unscrolled coordinates within their chart.
    pub window_space: bool,
    pub coverage: CoverageBounds,
    pub unknown: bool,
    pub hit: PreparedHit,
}

impl InputPlacement {
    fn chart(&self) -> Option<u32> {
        if self.window_space {
            None
        } else {
            self.scroll_parent
        }
    }

    pub fn test(&self, point: Point, scroll: Affine) -> (bool, Option<f64>) {
        self.hit.test(if self.window_space {
            point
        } else {
            scroll.inverse() * point
        })
    }
}

#[derive(Default)]
pub(crate) struct InputScene {
    pub root: Option<(u32, u64)>,
    pub renderer_is_gpu: bool,
    pub topology_dirty: bool,
    pub order: Vec<u32>,
    pub records: HashMap<u32, InputPlacement>,
    pub dirty: HashSet<u32>,
    pub scrollers: Vec<u32>,
    projected: HashSet<u32>,
    indexes: HashMap<Option<u32>, SpatialIndex>,
    pub stats: InputQueryStats,
}

impl InputScene {
    pub fn reset(&mut self, root: (u32, u64), renderer_is_gpu: bool) {
        self.root = Some(root);
        self.renderer_is_gpu = renderer_is_gpu;
        self.topology_dirty = false;
        self.order.clear();
        self.records.clear();
        self.dirty.clear();
        self.scrollers.clear();
        self.projected.clear();
        self.indexes.clear();
        self.stats.topology_rebuilds += 1;
    }

    pub fn invalidate(&mut self, id: u32) {
        if self.records.contains_key(&id) {
            self.dirty.insert(id);
        }
    }

    pub fn invalidate_scroll(&mut self, id: u32) {
        if let Some(record) = self.records.get(&id) {
            // Scroll changes a chart's query transform, not each affine shape.
            // Perspective descendants remain window-space and must be reprojected.
            self.dirty
                .extend(self.projected.iter().copied().filter(|id| {
                    let rank = self.records[id].rank;
                    rank > record.rank && rank < record.end
                }));
        }
    }

    pub fn publish(&mut self, id: u32, record: InputPlacement) {
        let changed = self
            .records
            .get(&id)
            .is_none_or(|old| old.coverage != record.coverage || old.chart() != record.chart());
        if changed {
            if let Some(old) = self.records.get(&id) {
                if let Some(index) = self.indexes.get_mut(&old.chart()) {
                    index.remove(id);
                }
            }
            let index = self.indexes.entry(record.chart()).or_default();
            match record.coverage {
                CoverageBounds::Empty => {}
                CoverageBounds::Finite(rect) => index.insert(id, Some(rect)),
                CoverageBounds::Unbounded => index.insert(id, None),
            }
        }
        if record.window_space {
            self.projected.insert(id);
        } else {
            self.projected.remove(&id);
        }
        self.stats.bounds_prepared += 1;
        self.records.insert(id, record);
    }

    pub fn candidates(&mut self, point: Point, scrolls: &HashMap<u32, Affine>) -> HashSet<u32> {
        let mut ids = HashSet::new();
        let mut local = HashSet::new();
        for (chart, index) in &self.indexes {
            let point = chart.map_or(point, |id| scrolls[&id].inverse() * point);
            let rect = Rect::new(point.x, point.y, point.x, point.y);
            local.clear();
            index.candidates(rect, &mut local);
            ids.extend(local.iter().copied().filter(|id| {
                self.records[id]
                    .coverage
                    .query_rect()
                    .is_some_and(|bounds| super::intersects(Some(bounds), rect))
            }));
        }
        self.stats.candidates += ids.len() as u64;
        ids
    }

    pub fn remove(&mut self, id: u32) {
        if self.records.contains_key(&id) {
            self.topology_dirty = true;
        }
        if let Some(old) = self.records.remove(&id) {
            if let Some(index) = self.indexes.get_mut(&old.chart()) {
                index.remove(id);
            }
        }
        self.projected.remove(&id);
        self.dirty.remove(&id);
    }
}
