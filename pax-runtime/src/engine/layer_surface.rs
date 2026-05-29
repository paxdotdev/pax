use kurbo::Rect;
use std::collections::{HashMap, HashSet, VecDeque};

/// Logical and backing-pixel dimensions for one physical surface.
pub struct LayerSurfaceSize {
    pub logical_width: f32,
    pub logical_height: f32,
    pub surface_width: u32,
    pub surface_height: u32,
    pub dpr: [f32; 2],
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

#[derive(Clone, Copy, Debug)]
pub struct VisibleSurfaceEscape {
    pub previous_bounds: Rect,
    pub visible_bounds: Rect,
    pub visible_surfaces: usize,
    pub escaped_visible_surfaces: usize,
    pub max_gap_x: f64,
    pub max_gap_y: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum LayoutChangeKind {
    #[default]
    Unchanged,
    OriginOnly,
    Resized,
}

pub fn surface_intersects_coverage_bounds(
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

pub fn visible_surface_escape(
    previous_surface_bounds: &[Rect],
    current_surfaces: &[LayerSurfaceEntry],
) -> Option<VisibleSurfaceEscape> {
    let previous_bounds = union_bounds(previous_surface_bounds.iter().copied())?;
    let mut visible_bounds: Option<Rect> = None;
    let mut visible_surfaces = 0;
    let mut escaped_visible_surfaces = 0;
    let mut max_gap_x: f64 = 0.0;
    let mut max_gap_y: f64 = 0.0;

    for surface in current_surfaces
        .iter()
        .filter(|surface| surface.replay_priority <= 0)
    {
        visible_surfaces += 1;
        let bounds = surface_bounds(surface);
        visible_bounds = Some(match visible_bounds {
            Some(acc) => acc.union(bounds),
            None => bounds,
        });

        if !rect_contains(&previous_bounds, &bounds) {
            escaped_visible_surfaces += 1;
            let (gap_x, gap_y) = rect_escape_gap(&previous_bounds, &bounds);
            max_gap_x = max_gap_x.max(gap_x);
            max_gap_y = max_gap_y.max(gap_y);
        }
    }

    let visible_bounds = visible_bounds?;
    (escaped_visible_surfaces > 0).then_some(VisibleSurfaceEscape {
        previous_bounds,
        visible_bounds,
        visible_surfaces,
        escaped_visible_surfaces,
        max_gap_x,
        max_gap_y,
    })
}

pub fn replay_batches_by_priority(mut entries: Vec<(usize, i32)>) -> Vec<Vec<usize>> {
    if entries.is_empty() {
        return Vec::new();
    }
    entries.sort_by_key(|(index, priority)| (*priority, *index));

    // Keep visible tile replay isolated from warm pre-render work. Replaying a warm ring in the
    // same frame smooths fast scroll exposure, but on native it can also turn an otherwise small
    // scroll tick into a visible frame spike.
    let mut batches = Vec::new();
    let mut current_priority = entries[0].1;
    let mut current_batch = Vec::new();
    for (index, priority) in entries {
        if priority != current_priority {
            batches.push(current_batch);
            current_batch = Vec::new();
            current_priority = priority;
        }
        current_batch.push(index);
    }
    batches.push(current_batch);
    batches
}

/// Retargeted surface metadata used to choose replay order without involving backend resources.
#[derive(Clone, Copy, Debug)]
pub struct ReplayPriorityEntry {
    pub index: usize,
    pub priority: i32,
    previous_bounds: Rect,
    current_bounds: Rect,
}

impl ReplayPriorityEntry {
    pub fn new(index: usize, priority: i32, previous_bounds: Rect, current_bounds: Rect) -> Self {
        Self {
            index,
            priority,
            previous_bounds,
            current_bounds,
        }
    }
}

#[derive(Clone, Copy)]
enum ReplayDirectionAxis {
    X,
    Y,
}

const REPLAY_DIRECTION_EPSILON: f64 = 0.5;

/// Batch retargeted surfaces by planner priority, then by the leading row/column of travel.
pub fn replay_batches_by_directional_priority(entries: &[ReplayPriorityEntry]) -> Vec<Vec<usize>> {
    if entries.is_empty() {
        return Vec::new();
    }

    let Some((axis, direction_sign)) = replay_direction(&entries) else {
        return replay_batches_by_priority(
            entries
                .iter()
                .map(|entry| (entry.index, entry.priority))
                .collect(),
        );
    };

    let mut keyed_entries: Vec<_> = entries
        .iter()
        .map(|entry| {
            (
                entry.index,
                entry.priority,
                directional_lane_key(entry.current_bounds, axis, direction_sign),
            )
        })
        .collect();
    keyed_entries.sort_by_key(|(index, priority, lane)| (*priority, *lane, *index));

    let mut batches = Vec::new();
    let mut current_priority = keyed_entries[0].1;
    let mut current_lane = keyed_entries[0].2;
    let mut current_batch = Vec::new();
    for (index, priority, lane) in keyed_entries {
        if priority != current_priority || lane != current_lane {
            batches.push(current_batch);
            current_batch = Vec::new();
            current_priority = priority;
            current_lane = lane;
        }
        current_batch.push(index);
    }
    batches.push(current_batch);
    batches
}

fn replay_direction(entries: &[ReplayPriorityEntry]) -> Option<(ReplayDirectionAxis, f64)> {
    let (dx, dy) = entries.iter().fold((0.0, 0.0), |(dx, dy), entry| {
        (
            dx + rect_center_x(&entry.current_bounds) - rect_center_x(&entry.previous_bounds),
            dy + rect_center_y(&entry.current_bounds) - rect_center_y(&entry.previous_bounds),
        )
    });

    if dx.abs() < REPLAY_DIRECTION_EPSILON && dy.abs() < REPLAY_DIRECTION_EPSILON {
        return None;
    }

    if dx.abs() >= dy.abs() {
        Some((ReplayDirectionAxis::X, dx.signum()))
    } else {
        Some((ReplayDirectionAxis::Y, dy.signum()))
    }
}

fn directional_lane_key(bounds: Rect, axis: ReplayDirectionAxis, direction_sign: f64) -> i64 {
    let coord = match axis {
        ReplayDirectionAxis::X => rect_center_x(&bounds),
        ReplayDirectionAxis::Y => rect_center_y(&bounds),
    }
    .round() as i64;

    if direction_sign.is_sign_positive() {
        -coord
    } else {
        coord
    }
}

fn rect_center_x(bounds: &Rect) -> f64 {
    (bounds.x0 + bounds.x1) * 0.5
}

fn rect_center_y(bounds: &Rect) -> f64 {
    (bounds.y0 + bounds.y1) * 0.5
}

fn union_bounds(bounds: impl IntoIterator<Item = Rect>) -> Option<Rect> {
    bounds
        .into_iter()
        .filter(|bounds| rect_is_finite(bounds))
        .fold(None, |acc, bounds| {
            Some(match acc {
                Some(acc) => acc.union(bounds),
                None => bounds,
            })
        })
}

fn surface_bounds(surface: &LayerSurfaceEntry) -> Rect {
    let x0 = surface.origin_x as f64;
    let y0 = surface.origin_y as f64;
    Rect::new(
        x0,
        y0,
        x0 + surface.surface.logical_width as f64,
        y0 + surface.surface.logical_height as f64,
    )
}

fn rect_contains(outer: &Rect, inner: &Rect) -> bool {
    const EPSILON: f64 = 0.5;
    inner.x0 >= outer.x0 - EPSILON
        && inner.y0 >= outer.y0 - EPSILON
        && inner.x1 <= outer.x1 + EPSILON
        && inner.y1 <= outer.y1 + EPSILON
}

fn rect_escape_gap(outer: &Rect, inner: &Rect) -> (f64, f64) {
    let gap_x = (outer.x0 - inner.x0)
        .max(0.0)
        .max((inner.x1 - outer.x1).max(0.0));
    let gap_y = (outer.y0 - inner.y0)
        .max(0.0)
        .max((inner.y1 - outer.y1).max(0.0));
    (gap_x, gap_y)
}

fn rect_is_finite(bounds: &Rect) -> bool {
    bounds.x0.is_finite() && bounds.y0.is_finite() && bounds.x1.is_finite() && bounds.y1.is_finite()
}

/// Shared coordinator for physical-surface replay after a layer layout retarget.
///
/// This intentionally tracks renderer-agnostic surface indices and coverage bounds only. Backends
/// remain responsible for applying the selected indices to their own renderer objects.
#[derive(Default)]
pub struct SurfaceReplayCoordinator {
    targeted_replay_queues: Vec<VecDeque<Vec<usize>>>,
    targeted_replay_bounds: Vec<HashMap<usize, Vec<Rect>>>,
    canvas_node_coverage: Vec<HashMap<u32, Rect>>,
}

impl SurfaceReplayCoordinator {
    pub fn targeted_replay_scope(&self, layer: usize) -> Option<Vec<usize>> {
        self.targeted_replay_queues
            .get(layer)
            .and_then(|queue| queue.front().cloned())
    }

    pub fn targeted_or_all_indices(&self, layer: usize, renderer_count: usize) -> Vec<usize> {
        self.targeted_replay_scope(layer)
            .unwrap_or_else(|| (0..renderer_count).collect())
    }

    pub fn clear_targeted_replay_scope(&mut self, layer: usize) {
        if let Some(queue) = self.targeted_replay_queues.get_mut(layer) {
            queue.clear();
        }
        if let Some(bounds_by_surface) = self.targeted_replay_bounds.get_mut(layer) {
            bounds_by_surface.clear();
        }
    }

    pub fn set_targeted_replay_batches(
        &mut self,
        layer: usize,
        batches: Vec<Vec<usize>>,
        bounds_by_surface: HashMap<usize, Vec<Rect>>,
    ) {
        let mut queue = VecDeque::new();
        let mut scheduled_indices = HashSet::new();
        for mut batch in batches {
            batch.sort_unstable();
            batch.dedup();
            if !batch.is_empty() {
                scheduled_indices.extend(batch.iter().copied());
                queue.push_back(batch);
            }
        }
        if queue.is_empty() {
            return;
        }

        let mut preserved_indices = HashSet::new();
        if let Some(existing_queue) = self.targeted_replay_queues.get(layer) {
            for batch in existing_queue {
                let mut preserved_batch = Vec::new();
                for index in batch.iter().copied() {
                    if scheduled_indices.insert(index) {
                        preserved_indices.insert(index);
                        preserved_batch.push(index);
                    }
                }
                if !preserved_batch.is_empty() {
                    queue.push_back(preserved_batch);
                }
            }
        }

        let mut bounds_by_surface = bounds_by_surface;
        if let Some(existing_bounds) = self.targeted_replay_bounds.get(layer) {
            for index in preserved_indices {
                if let Some(bounds) = existing_bounds.get(&index) {
                    bounds_by_surface
                        .entry(index)
                        .or_insert_with(|| bounds.clone());
                }
            }
        }

        if self.targeted_replay_queues.len() <= layer {
            self.targeted_replay_queues
                .resize_with(layer + 1, VecDeque::new);
        }
        self.targeted_replay_queues[layer] = queue;

        if self.targeted_replay_bounds.len() <= layer {
            self.targeted_replay_bounds
                .resize_with(layer + 1, HashMap::new);
        }
        self.targeted_replay_bounds[layer] = bounds_by_surface;
    }

    pub fn advance_targeted_replay_queue(&mut self, layer: usize) -> bool {
        let (popped_batch, has_more) = {
            let Some(queue) = self.targeted_replay_queues.get_mut(layer) else {
                return false;
            };
            let popped_batch = queue.pop_front();
            (popped_batch, !queue.is_empty())
        };

        if let Some(popped_batch) = popped_batch {
            if let Some(bounds_by_surface) = self.targeted_replay_bounds.get_mut(layer) {
                for index in popped_batch {
                    bounds_by_surface.remove(&index);
                }
                if !has_more {
                    bounds_by_surface.clear();
                }
            }
        }

        has_more
    }

    pub fn remember_canvas_node_coverage(
        &mut self,
        layer: usize,
        node_id: u32,
        coverage_bounds: Rect,
    ) {
        if self.canvas_node_coverage.len() <= layer {
            self.canvas_node_coverage
                .resize_with(layer + 1, HashMap::new);
        }
        self.canvas_node_coverage[layer].insert(node_id, coverage_bounds);
    }

    pub fn forget_canvas_node_coverage(&mut self, layer: usize, node_id: u32) {
        if let Some(coverage) = self.canvas_node_coverage.get_mut(layer) {
            coverage.remove(&node_id);
        }
    }

    pub fn truncate_layers(&mut self, layer_count: usize) {
        self.targeted_replay_queues.truncate(layer_count);
        self.targeted_replay_bounds.truncate(layer_count);
        self.canvas_node_coverage.truncate(layer_count);
    }

    pub fn targeted_replay_surface_bounds(
        &self,
        layer: usize,
        mut fallback_bounds: impl FnMut(usize) -> Option<Rect>,
    ) -> Option<Vec<Rect>> {
        let indices = self.targeted_replay_scope(layer)?;
        let mut bounds = Vec::with_capacity(indices.len());
        let layer_replay_bounds = self.targeted_replay_bounds.get(layer);
        for index in indices {
            if let Some(surface_bounds) =
                layer_replay_bounds.and_then(|bounds_by_surface| bounds_by_surface.get(&index))
            {
                bounds.extend(surface_bounds.iter().copied());
                continue;
            }
            if let Some(surface_bounds) = fallback_bounds(index) {
                bounds.push(surface_bounds);
            }
        }
        (!bounds.is_empty()).then_some(bounds)
    }

    pub fn spatial_replay_node_ids(
        &self,
        layer: usize,
        fallback_bounds: impl FnMut(usize) -> Option<Rect>,
    ) -> Option<Vec<u32>> {
        let surface_bounds = self.targeted_replay_surface_bounds(layer, fallback_bounds)?;
        self.spatial_replay_node_ids_for_surface_bounds(layer, &surface_bounds)
    }

    pub fn spatial_replay_node_ids_for_surface_bounds(
        &self,
        layer: usize,
        surface_bounds: &[Rect],
    ) -> Option<Vec<u32>> {
        let layer_coverage = self.canvas_node_coverage.get(layer)?;
        if layer_coverage.is_empty() {
            return None;
        }

        let mut node_ids: Vec<_> = layer_coverage
            .iter()
            .filter_map(|(node_id, bounds)| {
                surface_bounds
                    .iter()
                    .any(|surface| {
                        surface_intersects_coverage_bounds(
                            bounds,
                            surface.x0,
                            surface.y0,
                            surface.width(),
                            surface.height(),
                        )
                    })
                    .then_some(*node_id)
            })
            .collect();
        node_ids.sort_unstable();
        if node_ids.is_empty() {
            return None;
        }
        Some(node_ids)
    }
}

#[cfg(test)]
mod replay_priority_tests {
    use super::{
        replay_batches_by_directional_priority, replay_batches_by_priority, visible_surface_escape,
        LayerSurfaceEntry, LayerSurfaceSize, ReplayPriorityEntry, SurfaceReplayCoordinator,
    };
    use kurbo::Rect;
    use std::collections::HashMap;

    #[test]
    fn visible_batch_excludes_warm_rings() {
        let batches = replay_batches_by_priority(vec![(3, 4), (0, 0), (2, 2), (1, 0)]);

        assert_eq!(batches[0], vec![0, 1]);
        assert_eq!(batches[1], vec![2]);
        assert_eq!(batches[2], vec![3]);
    }

    #[test]
    fn starts_with_nearest_warm_batch_when_no_visible_tiles() {
        let batches = replay_batches_by_priority(vec![(4, 5), (2, 3), (3, 3)]);

        assert_eq!(batches[0], vec![2, 3]);
        assert_eq!(batches[1], vec![4]);
    }

    #[test]
    fn visible_escape_detects_visible_tiles_outside_previous_warm_bounds() {
        let previous = vec![Rect::new(0.0, 0.0, 100.0, 300.0)];
        let current = vec![
            test_surface(0.0, 100.0, 100.0, 100.0, 0),
            test_surface(0.0, 300.0, 100.0, 100.0, 0),
            test_surface(0.0, 400.0, 100.0, 100.0, 1),
        ];

        let escape = visible_surface_escape(&previous, &current).expect("one visible tile escaped");

        assert_eq!(escape.visible_surfaces, 2);
        assert_eq!(escape.escaped_visible_surfaces, 1);
        assert_eq!(escape.max_gap_y, 100.0);
        assert_eq!(escape.max_gap_x, 0.0);
        assert_eq!(escape.visible_bounds, Rect::new(0.0, 100.0, 100.0, 400.0));
    }

    #[test]
    fn visible_escape_ignores_covered_visible_tiles() {
        let previous = vec![Rect::new(0.0, 0.0, 100.0, 300.0)];
        let current = vec![
            test_surface(0.0, 0.0, 100.0, 100.0, 0),
            test_surface(0.0, 100.0, 100.0, 100.0, 0),
        ];

        assert!(visible_surface_escape(&previous, &current).is_none());
    }

    #[test]
    fn directional_batches_start_with_leading_vertical_row() {
        let batches = replay_batches_by_directional_priority(&[
            ReplayPriorityEntry::new(
                0,
                0,
                Rect::new(0.0, 0.0, 100.0, 100.0),
                Rect::new(0.0, 100.0, 100.0, 200.0),
            ),
            ReplayPriorityEntry::new(
                1,
                0,
                Rect::new(100.0, 100.0, 200.0, 200.0),
                Rect::new(100.0, 200.0, 200.0, 300.0),
            ),
            ReplayPriorityEntry::new(
                2,
                0,
                Rect::new(200.0, -100.0, 300.0, 0.0),
                Rect::new(200.0, 0.0, 300.0, 100.0),
            ),
        ]);

        assert_eq!(batches, vec![vec![1], vec![0], vec![2]]);
    }

    #[test]
    fn directional_batches_keep_same_leading_row_together() {
        let batches = replay_batches_by_directional_priority(&[
            ReplayPriorityEntry::new(
                2,
                0,
                Rect::new(200.0, 100.0, 300.0, 200.0),
                Rect::new(200.0, 200.0, 300.0, 300.0),
            ),
            ReplayPriorityEntry::new(
                0,
                0,
                Rect::new(0.0, 100.0, 100.0, 200.0),
                Rect::new(0.0, 200.0, 100.0, 300.0),
            ),
            ReplayPriorityEntry::new(
                1,
                0,
                Rect::new(100.0, 0.0, 200.0, 100.0),
                Rect::new(100.0, 100.0, 200.0, 200.0),
            ),
        ]);

        assert_eq!(batches, vec![vec![0, 2], vec![1]]);
    }

    #[test]
    fn coordinator_advances_targeted_replay_batches() {
        let mut coordinator = SurfaceReplayCoordinator::default();
        coordinator.set_targeted_replay_batches(
            0,
            vec![vec![3, 1, 1], vec![2]],
            HashMap::from([
                (1, vec![Rect::new(0.0, 0.0, 10.0, 10.0)]),
                (2, vec![Rect::new(10.0, 0.0, 20.0, 10.0)]),
                (3, vec![Rect::new(20.0, 0.0, 30.0, 10.0)]),
            ]),
        );

        assert_eq!(coordinator.targeted_replay_scope(0), Some(vec![1, 3]));
        assert!(coordinator.advance_targeted_replay_queue(0));
        assert_eq!(coordinator.targeted_replay_scope(0), Some(vec![2]));
        let bounds = coordinator
            .targeted_replay_surface_bounds(0, |_| None)
            .expect("second replay bounds remain");
        assert_eq!(bounds, vec![Rect::new(10.0, 0.0, 20.0, 10.0)]);
        assert!(!coordinator.advance_targeted_replay_queue(0));
        assert_eq!(coordinator.targeted_replay_scope(0), None);
    }

    #[test]
    fn coordinator_preserves_unflushed_replay_batches_when_rescheduled() {
        let mut coordinator = SurfaceReplayCoordinator::default();
        coordinator.set_targeted_replay_batches(
            0,
            vec![vec![0], vec![1]],
            HashMap::from([
                (0, vec![Rect::new(0.0, 0.0, 10.0, 10.0)]),
                (1, vec![Rect::new(10.0, 0.0, 20.0, 10.0)]),
            ]),
        );
        coordinator.set_targeted_replay_batches(
            0,
            vec![vec![2]],
            HashMap::from([(2, vec![Rect::new(20.0, 0.0, 30.0, 10.0)])]),
        );

        assert_eq!(coordinator.targeted_replay_scope(0), Some(vec![2]));
        assert!(coordinator.advance_targeted_replay_queue(0));
        assert_eq!(coordinator.targeted_replay_scope(0), Some(vec![0]));
        let bounds = coordinator
            .targeted_replay_surface_bounds(0, |_| None)
            .expect("preserved replay bounds remain");
        assert_eq!(bounds, vec![Rect::new(0.0, 0.0, 10.0, 10.0)]);
        assert!(coordinator.advance_targeted_replay_queue(0));
        assert_eq!(coordinator.targeted_replay_scope(0), Some(vec![1]));
    }

    #[test]
    fn coordinator_selects_spatial_replay_nodes_by_surface_bounds() {
        let mut coordinator = SurfaceReplayCoordinator::default();
        coordinator.remember_canvas_node_coverage(0, 10, Rect::new(0.0, 0.0, 5.0, 5.0));
        coordinator.remember_canvas_node_coverage(0, 20, Rect::new(50.0, 50.0, 60.0, 60.0));
        coordinator.set_targeted_replay_batches(0, vec![vec![0]], HashMap::new());

        let node_ids = coordinator
            .spatial_replay_node_ids(0, |_| Some(Rect::new(0.0, 0.0, 10.0, 10.0)))
            .expect("spatial replay nodes");

        assert_eq!(node_ids, vec![10]);
    }

    #[test]
    fn coordinator_falls_back_when_targeted_replay_has_no_cached_coverage() {
        let mut coordinator = SurfaceReplayCoordinator::default();
        coordinator.remember_canvas_node_coverage(0, 10, Rect::new(0.0, 0.0, 5.0, 5.0));
        coordinator.set_targeted_replay_batches(0, vec![vec![0]], HashMap::new());

        let node_ids =
            coordinator.spatial_replay_node_ids(0, |_| Some(Rect::new(50.0, 50.0, 60.0, 60.0)));

        assert!(node_ids.is_none());
    }

    fn test_surface(
        origin_x: f32,
        origin_y: f32,
        logical_width: f32,
        logical_height: f32,
        replay_priority: i32,
    ) -> LayerSurfaceEntry {
        LayerSurfaceEntry {
            key: String::new(),
            host_signature: String::new(),
            origin_x,
            origin_y,
            replay_priority,
            surface: LayerSurfaceSize {
                logical_width,
                logical_height,
                surface_width: logical_width as u32,
                surface_height: logical_height as u32,
                dpr: [1.0, 1.0],
            },
        }
    }
}
