use std::{borrow::Borrow, ops::Range};

use bezier_rs::{Bezier, BezierHandles, Identifier, Subpath, TValue};
use glam::{BVec2, DMat2, DVec2};
use pax_engine::log;

use crate::math::boolean_path_operations::bezier_rs_modifications::intersections;

mod bezier_rs_modifications;

/// An empty id type for use in tests
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct DesignerPathId;

impl Identifier for DesignerPathId {
    fn new() -> Self {
        Self
    }
}

#[derive(Clone)]
pub struct CompoundPath {
    pub subpaths: Vec<Subpath<DesignerPathId>>,
}

impl CompoundPath {
    pub fn new() -> Self {
        Self { subpaths: vec![] }
    }

    pub fn from_subpath(subpath: Subpath<DesignerPathId>) -> Self {
        Self {
            subpaths: vec![subpath],
        }
    }

    // TODO this needs verification:
    // is this a valid subpath (winding, non overlapping with other subpaths etc.)
    pub fn add_paths(&mut self, path: CompoundPath) {
        self.subpaths.extend(path.subpaths);
    }

    // TODO this needs verification:
    // is this a valid subpath (winding, non overlapping with other subpaths etc.)
    pub fn sub_paths(&mut self, path: CompoundPath) {
        self.subpaths
            .extend(path.subpaths.into_iter().map(|p| p.reverse()));
    }

    pub fn union(&self, other: &Self) -> Self {
        let self_segments: Vec<Vec<Bezier>> =
            self.subpaths.iter().map(|s| s.iter().collect()).collect();
        let other_segments: Vec<Vec<Bezier>> =
            other.subpaths.iter().map(|s| s.iter().collect()).collect();
        let mut all_intersections: Vec<(Intersection, Intersection)> = Vec::new();
        let mut self_subgraph_has_intersections: Vec<bool> = vec![false; self.subpaths.len()];
        let mut other_subgraph_has_intersections: Vec<bool> = vec![false; self.subpaths.len()];

        for (self_sub_ind, self_subpath) in self.subpaths.iter().enumerate() {
            for (other_sub_ind, other_subpath) in other.subpaths.iter().enumerate() {
                for (self_ind, self_seg) in self_subpath.iter().enumerate() {
                    for (other_ind, other_seg) in other_subpath.iter().enumerate() {
                        let segment_intersections =
                            intersections(&self_seg, &other_seg, Some(0.01), 0.1)
                                .into_iter()
                                .map(|[self_t, other_t]| {
                                    (
                                        Intersection {
                                            subgraph_ind: self_sub_ind,
                                            segment_index: self_ind,
                                            t: self_t,
                                        },
                                        Intersection {
                                            subgraph_ind: other_sub_ind,
                                            segment_index: other_ind,
                                            t: other_t,
                                        },
                                    )
                                });
                        if segment_intersections.len() > 0 {
                            self_subgraph_has_intersections[self_sub_ind] = true;
                            other_subgraph_has_intersections[other_sub_ind] = true;
                            all_intersections.extend(segment_intersections);
                        }
                    }
                }
            }
        }

        let intersections_len = all_intersections.len();
        let (self_intersections, other_intersections) =
            unzip_and_sort_with_cross_references(all_intersections);

        let self_path_data = PathIntersectionData {
            intersections: self_intersections,
            beziers: self_segments,
        };
        let other_path_data = PathIntersectionData {
            intersections: other_intersections,
            beziers: other_segments,
        };
        let mut intersection_visited = vec![false; intersections_len];

        if intersections_len % 2 != 0 {
            log::warn!("path intersection number should always be even");
            return Self::new();
        }

        // log::debug!("self_intersections: {:#?}", self_intersections);
        // log::debug!("other_intersections: {:#?}", other_intersections);
        let mut output_paths = vec![];
        while let Some(start_index) = (0..intersections_len).find(|ind| {
            if intersection_visited[*ind] {
                return false;
            }
            let (_, start_intersection) = self_path_data.intersections[*ind];
            const EPS: f64 = 1e-1;
            // TODO make this start point coice more intelligently
            let eps_ahead_of_start_point = self_path_data.beziers[start_intersection.subgraph_ind]
                [start_intersection.segment_index]
                .evaluate(TValue::Parametric(
                    (start_intersection.t + EPS).clamp(0.0, 1.0),
                ));
            log::debug!("after");
            let point_is_inside_other = other
                .subpaths
                .iter()
                .any(|s| s.point_inside(eps_ahead_of_start_point));
            !point_is_inside_other
        }) {
            log::debug!("tracing from {:?}", start_index);
            log::debug!("visited before: {:?}", intersection_visited);
            let path = trace_from(
                start_index,
                &self_path_data,
                &other_path_data,
                &mut intersection_visited,
            );
            log::debug!("visited after: {:?}", intersection_visited);
            output_paths.push(path);
        }

        Self {
            subpaths: output_paths
                .into_iter()
                .map(|p| Subpath::from_beziers(&p, true))
                .collect(),
        }
    }
}

fn trace_from(
    mut index: usize,
    self_path: &PathIntersectionData,
    other_path: &PathIntersectionData,
    intersection_visited: &mut [bool],
) -> Vec<Bezier> {
    let mut current = self_path;
    let mut other = other_path;
    let mut union_path_segments = vec![];

    fn circular_range(start: usize, end: usize, len: usize) -> impl Iterator<Item = usize> {
        (0..len)
            .map(move |i| (start + i) % len)
            .take_while(move |&i| i != end)
    }

    loop {
        // a little hacky, fix
        let self_index = if std::ptr::eq(self_path, current) {
            index
        } else {
            other_path.intersections[index].0
        };
        if intersection_visited[self_index] {
            break;
        }
        intersection_visited[self_index] = true;

        let (_, curr_intersection) = current.intersections[index];
        let subpath_ind = curr_intersection.subgraph_ind;
        let (segment_ind_jump_pointer, next_intersection) = current
            .intersections
            .get(index + 1)
            .filter(|(_, i)| i.subgraph_ind == curr_intersection.subgraph_ind)
            // find start
            .unwrap_or_else(|| {
                current
                    .intersections
                    .iter()
                    .find(|(_, i)| i.subgraph_ind == curr_intersection.subgraph_ind)
                    .expect("current_intersection was taken from same list, and so should always exist at least one")
            });
        log::debug!(
            "curr: {:#?}, next: {:#?}",
            curr_intersection,
            next_intersection
        );
        if curr_intersection.segment_index == next_intersection.segment_index {
            log::debug!("one segment");
            let segment = current.beziers[subpath_ind][curr_intersection.segment_index];
            let part = segment.split(TValue::Parametric(curr_intersection.t))[1]
                .split(TValue::Parametric(next_intersection.t))[0];
            union_path_segments.push(part);
        } else {
            let first_segment = current.beziers[subpath_ind][curr_intersection.segment_index];
            let [_before, on] = first_segment.split(TValue::Parametric(curr_intersection.t));
            union_path_segments.push(on);
            let mut count = 2;
            for seg_ind in circular_range(
                curr_intersection.segment_index + 1,
                next_intersection.segment_index,
                current.beziers[subpath_ind].len(),
            ) {
                count += 1;
                let segment = current.beziers[subpath_ind][seg_ind];
                union_path_segments.push(segment);
            }
            log::debug!("adding {} segments", count);
            let last_segment = current.beziers[subpath_ind][next_intersection.segment_index];
            let [on, _after] = last_segment.split(TValue::Parametric(next_intersection.t));
            union_path_segments.push(on);
        }
        index = *segment_ind_jump_pointer;
        std::mem::swap(&mut current, &mut other);
    }
    union_path_segments
}

struct PathIntersectionData {
    intersections: Vec<(usize, Intersection)>,
    beziers: Vec<Vec<Bezier>>,
}

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
struct Intersection {
    subgraph_ind: usize,
    segment_index: usize,
    t: f64,
}

impl Eq for Intersection {}

impl Ord for Intersection {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.subgraph_ind.cmp(&other.subgraph_ind).then(
            self.segment_index
                .cmp(&other.segment_index)
                .then(self.t.total_cmp(&other.t)),
        )
    }
}

/// breaks down a list into two lists sorted by their respective T value, and with
/// and index that when looked up in the other vector returns the T that was originally
/// the other tuple pair value.
fn unzip_and_sort_with_cross_references<T: Ord>(
    data: impl IntoIterator<Item = (T, T)>,
) -> (Vec<(usize, T)>, Vec<(usize, T)>) {
    let (v1, v2): (Vec<_>, Vec<_>) = data.into_iter().unzip();
    let len = v1.len();

    let mut v1_with_ids: Vec<_> = v1.into_iter().enumerate().collect();
    let mut v2_with_ids: Vec<_> = v2.into_iter().enumerate().collect();

    v1_with_ids.sort_unstable_by(|(_, a), (_, b)| a.cmp(b));
    v2_with_ids.sort_unstable_by(|(_, a), (_, b)| a.cmp(b));

    let mut v1_index_map = vec![0; len];
    let mut v2_index_map = vec![0; len];

    for (new_index, (old_index, _)) in v1_with_ids.iter().enumerate() {
        v1_index_map[*old_index] = new_index;
    }

    for (new_index, (old_index, _)) in v2_with_ids.iter().enumerate() {
        v2_index_map[*old_index] = new_index;
    }

    for (index, _) in v1_with_ids.iter_mut() {
        *index = v2_index_map[*index];
    }

    for (index, _) in v2_with_ids.iter_mut() {
        *index = v1_index_map[*index];
    }

    (v1_with_ids, v2_with_ids)
}
