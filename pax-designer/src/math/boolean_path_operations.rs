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

#[derive(Clone, Debug)]
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
        let all_intersections = calculcate_all_intersections(&self, &other);
        // log::debug!("all intersections: {:#?}", all_intersections);
        let intersections_len = all_intersections.len();

        let (self_intersections, other_intersections) =
            unzip_and_sort_with_cross_references(all_intersections);

        let self_path_data = PathIntersectionData {
            intersections: self_intersections,
            beziers: self.subpaths.iter().map(|s| s.iter().collect()).collect(),
        };
        let other_path_data = PathIntersectionData {
            intersections: other_intersections,
            beziers: other.subpaths.iter().map(|s| s.iter().collect()).collect(),
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
            let eps_ahead_of_start_point = self_path_data.beziers[start_intersection.subpath_index]
                [start_intersection.segment_index]
                .evaluate(TValue::Parametric(
                    (start_intersection.t + EPS).clamp(0.0, 1.0),
                ));
            let point_is_inside_other = other
                .subpaths
                .iter()
                .any(|s| s.point_inside(eps_ahead_of_start_point));
            !point_is_inside_other
        }) {
            // log::debug!("tracing from {:?}", start_index);
            // log::debug!("visited before: {:?}", intersection_visited);
            let path = trace_from(
                start_index,
                &self_path_data,
                &other_path_data,
                &mut intersection_visited,
            );
            // log::debug!("visited after: {:?}", intersection_visited);
            output_paths.push(Subpath::from_beziers(&path, true));
        }

        // TODO this logic needs to be different for the different boolean operations,
        // and handle holes and non-holes differently (assuming UNION + non-holes below)
        // add all paths to output that were never crossed anywhere
        output_paths.extend(collect_subpaths_without_intersections(
            self,
            self_path_data.intersections.iter().map(|(_, i)| i),
        ));
        output_paths.extend(collect_subpaths_without_intersections(
            other,
            other_path_data.intersections.iter().map(|(_, i)| i),
        ));

        Self {
            subpaths: output_paths,
        }
    }
}

fn calculcate_all_intersections(
    p1: &CompoundPath,
    p2: &CompoundPath,
) -> Vec<(Intersection, Intersection)> {
    let mut all_intersections: Vec<(Intersection, Intersection)> = Vec::new();
    for (p1_subpath_index, p1_subpath) in p1.subpaths.iter().enumerate() {
        for (p2_subpath_index, p2_subpath) in p2.subpaths.iter().enumerate() {
            for (p1_segment_index, p1_seg) in p1_subpath.iter().enumerate() {
                for (p2_segment_index, p2_seg) in p2_subpath.iter().enumerate() {
                    let segment_intersections = intersections(&p1_seg, &p2_seg, Some(0.001), 0.01)
                        .into_iter()
                        .map(|[p1_t, p2_t]| {
                            (
                                Intersection {
                                    subpath_index: p1_subpath_index,
                                    segment_index: p1_segment_index,
                                    t: p1_t,
                                },
                                Intersection {
                                    subpath_index: p2_subpath_index,
                                    segment_index: p2_segment_index,
                                    t: p2_t,
                                },
                            )
                        });
                    all_intersections.extend(segment_intersections);
                }
            }
        }
    }
    all_intersections
}

fn collect_subpaths_without_intersections<'a>(
    path: &'a CompoundPath,
    intersections: impl Iterator<Item = &'a Intersection>,
) -> impl Iterator<Item = Subpath<DesignerPathId>> + 'a {
    let mut subpaths_intersection_flags = vec![false; path.subpaths.len()];
    for intersection in intersections {
        subpaths_intersection_flags[intersection.subpath_index] = true;
    }
    subpaths_intersection_flags
        .into_iter()
        .zip(path.subpaths.iter())
        .filter_map(|(intersected, subgraph)| (!intersected).then_some(subgraph))
        .cloned()
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
        let subpath_index = curr_intersection.subpath_index;
        let (segment_ind_jump_pointer, next_intersection) = current
            .intersections
            .get(index + 1)
            .filter(|(_, i)| i.subpath_index == subpath_index)
            // find start
            .unwrap_or_else(|| {
                current
                    .intersections
                    .iter()
                    .find(|(_, i)| i.subpath_index == subpath_index)
                    .expect("current_intersection was taken from same list, and so should always exist at least one")
            });
        // log::debug!(
        //     "curr: {:#?}, next: {:#?}",
        //     curr_intersection,
        //     next_intersection
        // );
        if curr_intersection.segment_index == next_intersection.segment_index {
            // log::debug!("one segment");
            let segment = current.beziers[subpath_index][curr_intersection.segment_index];
            let part = segment.split(TValue::Parametric(curr_intersection.t))[1]
                .split(TValue::Parametric(next_intersection.t))[0];
            union_path_segments.push(part);
        } else {
            let first_segment = current.beziers[subpath_index][curr_intersection.segment_index];
            let [_before, on] = first_segment.split(TValue::Parametric(curr_intersection.t));
            union_path_segments.push(on);
            let mut count = 2;
            for seg_ind in circular_range(
                curr_intersection.segment_index + 1,
                next_intersection.segment_index,
                current.beziers[subpath_index].len(),
            ) {
                count += 1;
                let segment = current.beziers[subpath_index][seg_ind];
                union_path_segments.push(segment);
            }
            // log::debug!("adding {} segments", count);
            let last_segment = current.beziers[subpath_index][next_intersection.segment_index];
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
    subpath_index: usize,
    segment_index: usize,
    t: f64,
}

impl Eq for Intersection {}

impl Ord for Intersection {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.subpath_index.cmp(&other.subpath_index).then(
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
