use std::{borrow::Borrow, ops::Range};

use bezier_rs::{Bezier, BezierHandles, Identifier, Subpath, TValue};
use glam::{BVec2, DMat2, DVec2};
use pax_engine::log;

use crate::math::boolean_path_operations::bezier_rs_modifications::intersections;

mod bezier_rs_modifications;
mod compound_path_graph;

/// An empty id type for use in tests
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct DesignerPathId;

impl Identifier for DesignerPathId {
    fn new() -> Self {
        Self
    }
}

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

    pub fn union(&self, other: &Self) -> Self {
        let self_subgraph = self.subpaths.first().unwrap();
        let other_subgraph = other.subpaths.first().unwrap();
        let self_segments: Vec<_> = self_subgraph.iter().collect();
        let other_segments: Vec<_> = other_subgraph.iter().collect();
        let mut all_intersections: Vec<(Intersection, Intersection)> = Vec::new();
        for (self_ind, self_seg) in self_segments.iter().enumerate() {
            for (other_ind, other_seg) in other_segments.iter().enumerate() {
                let segment_intersections = intersections(&self_seg, &other_seg, Some(0.01), 0.1)
                    .into_iter()
                    .map(|[self_t, other_t]| {
                        (
                            Intersection {
                                segment_index: self_ind,
                                t: self_t,
                            },
                            Intersection {
                                segment_index: other_ind,
                                t: other_t,
                            },
                        )
                    });
                all_intersections.extend(segment_intersections);
            }
        }

        let intersections_len = all_intersections.len();
        let (self_intersections, other_intersections) =
            unzip_and_sort_with_cross_references(all_intersections);

        if intersections_len % 2 != 0 {
            log::warn!("path intersection number should always be even");
            return Self::new();
        }
        let Some((_, start_intersection)) = self_intersections.first() else {
            log::warn!("tried to combine non overlapping subpaths");
            return Self::new();
        };

        const EPS: f64 = 1e-1;
        // TODO make this start point coice more intelligently
        let eps_ahead_of_start_point = self_segments[start_intersection.segment_index]
            .evaluate(TValue::Parametric(start_intersection.t + EPS));
        // if we are inside when walking from first intersection, then start at index 0,
        // otherwise start at the next intersection (index 1)
        let mut index = (other_subgraph.point_inside(eps_ahead_of_start_point)) as usize;
        let start_self_index = index;
        let start_other_index = self_intersections[index].0;

        let mut union_path_segments = vec![];

        log::debug!("self_intersections: {:#?}", self_intersections);
        log::debug!("other_intersections: {:#?}", other_intersections);
        let mut current_curve_intersections = &self_intersections;
        let mut other_curve_intersections = &other_intersections;
        let mut current_segments = &self_segments;
        let mut other_segments = &other_segments;
        let mut start_current_index = &start_self_index;
        let mut start_other_idnex = &start_other_index;

        fn circular_range(start: usize, end: usize, len: usize) -> impl Iterator<Item = usize> {
            (0..len)
                .map(move |i| (start + i) % len)
                .take_while(move |&i| i != end)
        }

        loop {
            let is_self = std::ptr::eq(current_curve_intersections, &self_intersections);
            log::debug!("processing ind: {:?} (is_self: {:?})", index, is_self);
            let (_, curr_intersection) = current_curve_intersections[index];
            let (segment_ind_jump, next_intersection) =
                current_curve_intersections[(index + 1) % intersections_len];
            log::debug!(
                "curr: {:#?}, next: {:#?}",
                curr_intersection,
                next_intersection
            );
            if curr_intersection.segment_index == next_intersection.segment_index {
                log::debug!("one segment");
                let segment = current_segments[curr_intersection.segment_index];
                let part = segment.split(TValue::Parametric(curr_intersection.t))[1]
                    .split(TValue::Parametric(next_intersection.t))[0];
                union_path_segments.push(part);
            } else {
                let first_segment = current_segments[curr_intersection.segment_index];
                let [_before, on] = first_segment.split(TValue::Parametric(curr_intersection.t));
                union_path_segments.push(on);
                let mut count = 2;
                for seg_ind in circular_range(
                    curr_intersection.segment_index + 1,
                    next_intersection.segment_index,
                    current_segments.len(),
                ) {
                    count += 1;
                    let segment = current_segments[seg_ind];
                    union_path_segments.push(segment);
                }
                log::debug!("adding {} segments", count);
                let last_segment = current_segments[next_intersection.segment_index];
                let [on, _after] = last_segment.split(TValue::Parametric(next_intersection.t));
                union_path_segments.push(on);
            }
            std::mem::swap(
                &mut current_curve_intersections,
                &mut other_curve_intersections,
            );
            std::mem::swap(&mut current_segments, &mut other_segments);
            std::mem::swap(&mut start_current_index, &mut start_other_idnex);
            index = segment_ind_jump;
            if index == *start_current_index {
                log::debug!("next index was {:?}, witch is start - returning", index);
                break;
            }
        }
        Self {
            subpaths: vec![Subpath::from_beziers(&union_path_segments, true)],
        }
    }
}

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
struct Intersection {
    segment_index: usize,
    t: f64,
}

impl Eq for Intersection {}

impl Ord for Intersection {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.segment_index
            .cmp(&other.segment_index)
            .then(self.t.total_cmp(&other.t))
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
