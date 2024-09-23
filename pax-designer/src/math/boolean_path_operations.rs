use std::{
    borrow::Borrow,
    f64::consts::PI,
    iter,
    ops::{Mul, Range},
};

use bezier_rs::{Bezier, BezierHandles, Identifier, Subpath, TValue};
use glam::{BVec2, DMat2, DVec2};
use pax_engine::log;

use crate::math::boolean_path_operations::bezier_rs_modifications::intersections;

mod bezier_rs_modifications;
mod circular_range;
const EPS: f64 = 1e-1;

/// An empty id type for use in tests
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct DesignerPathId;

impl Identifier for DesignerPathId {
    fn new() -> Self {
        Self
    }
}

// TODO:
// - filter intersections by tangents
// - filter intersections by corner hits
// - limit maximum iteration count, and discard what has not been traced in an intelligent way in worst case
// - intersections_between_subcurves takes a lot of time in degenerate cases (curves really close): investigate/fix
//   relevant for this: https://math.stackexchange.com/questions/1616780/reliable-test-for-intersection-of-two-bezier-curves

// - support other operations (difference etc.)

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
        log::debug!("--- start union calc ---");
        let all_intersections = calculcate_all_intersections(&self, &other);
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
        // log::debug!("self_intersection_data: {:#?}", self_path_data);
        // log::debug!("other_intersection_data: {:#?}", other_path_data);
        let mut intersection_visited = vec![false; intersections_len];

        if intersections_len % 2 != 0 {
            log::warn!("path intersection number should always be even");
            // return Self::new();
        }

        let mut output_paths = vec![];
        while let Some(start_index) = find_next_entrypoint(
            &self_path_data,
            &other_path_data,
            &other,
            &intersection_visited,
        ) {
            // log::debug!("tracing from {:?}", start_index);
            // log::debug!("visited before: {:?}", intersection_visited);
            let subpath = trace_from(
                start_index,
                &self_path_data,
                &other_path_data,
                &mut intersection_visited,
            );
            // log::debug!("visited after: {:?}", intersection_visited);
            // log::debug!("path: {:#?}", path);
            if subpath.len() > 1 {
                let subpath = Subpath::from_beziers(&subpath, true);
                // skip this subpath if to short
                if subpath.length(Some(10)) > 2.0 * PI {
                    output_paths.push(subpath);
                } else {
                    log::warn!("subpath circumfrence to short");
                }
            } else {
                log::warn!("subpath length was: < 2, expected bezier segment count >= 2");
            }
        }

        // TODO this logic needs to be different for the different boolean operations,
        // and handle holes and non-holes differently (assuming UNION + non-holes below)
        // add all paths to output that were never crossed anywhere
        output_paths.extend(collect_subpaths_without_intersections(
            self,
            other,
            self_path_data.intersections.iter().map(|(_, i)| i),
        ));
        output_paths.extend(collect_subpaths_without_intersections(
            other,
            self,
            other_path_data.intersections.iter().map(|(_, i)| i),
        ));

        Self {
            subpaths: output_paths,
        }
    }

    pub fn contains_point(&self, point: DVec2) -> bool {
        self.subpaths
            .iter()
            .flat_map(|p| p.iter())
            .map(|bezier| bezier.winding(point))
            .sum::<i32>()
            != 0
    }
}

fn find_next_entrypoint(
    self_path: &PathIntersectionData,
    other_path: &PathIntersectionData,
    other: &CompoundPath,
    intersection_visited: &[bool],
) -> Option<usize> {
    let mut possibilities: Vec<_> = (0..intersection_visited.len())
        .filter(|ind| !intersection_visited[*ind])
        .map(|ind| {
            (
                ind,
                is_leaving_other_subpath(&self_path, &other_path, other, ind),
            )
        })
        .collect();
    possibilities.sort_unstable_by(|(_, f1), (_, f2)| f1.total_cmp(f2));
    possibilities.last().map(|(index, _)| *index)
}

fn is_leaving_other_subpath(
    self_path: &PathIntersectionData,
    other_path: &PathIntersectionData,
    other: &CompoundPath,
    self_intersection_index: usize,
) -> f64 {
    let (other_intersection_index, self_start_intersection) =
        self_path.intersections[self_intersection_index];
    let (_, other_start_intersection) = other_path.intersections[other_intersection_index];

    // TODO make this start point coice more intelligently
    let self_bezier = self_path.beziers[self_start_intersection.subpath_index]
        [self_start_intersection.segment_index];
    let other_bezier = other_path.beziers[other_start_intersection.subpath_index]
        [other_start_intersection.segment_index];
    let tangent_self = self_bezier.tangent(TValue::Parametric(self_start_intersection.t));
    let tangent_other = other_bezier.tangent(TValue::Parametric(other_start_intersection.t));

    let cross = tangent_self.perp_dot(tangent_other);
    cross
}

fn calculcate_all_intersections(
    p1: &CompoundPath,
    p2: &CompoundPath,
) -> Vec<(Intersection, Intersection)> {
    let mut all_intersections: Vec<(Intersection, Intersection)> = Vec::new();
    for (p1_subpath_index, p1_subpath) in p1.subpaths.iter().enumerate() {
        for (p2_subpath_index, p2_subpath) in p2.subpaths.iter().enumerate() {
            let path_start_index = all_intersections.len();
            for (p1_segment_index, p1_seg) in p1_subpath.iter().enumerate() {
                for (p2_segment_index, p2_seg) in p2_subpath.iter().enumerate() {
                    let segment_intersections =
                        intersections(&p1_seg, &p2_seg, Some(EPS / 20.0), EPS)
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
            if all_intersections.len() % 2 != 0 {
                let index = all_intersections.iter().skip(path_start_index).position(
                    |(intersec_1, intersec_2)| {
                        let p1_tangent = p1_subpath
                            .get_segment(intersec_1.segment_index)
                            .unwrap()
                            .tangent(TValue::Parametric(intersec_1.t));
                        let p2_tangent = p2_subpath
                            .get_segment(intersec_2.segment_index)
                            .unwrap()
                            .tangent(TValue::Parametric(intersec_2.t));
                        let tangential = p1_tangent.dot(p2_tangent).abs() > 0.999;
                        tangential
                    },
                );
                if let Some(index) = index {
                    all_intersections.swap_remove(path_start_index + index);
                } else {
                    log::warn!("uneven intersections but none where tangential");
                }
            }
        }
    }
    all_intersections
}

fn collect_subpaths_without_intersections<'a>(
    path: &'a CompoundPath,
    other: &'a CompoundPath,
    path_intersections: impl Iterator<Item = &'a Intersection>,
) -> impl Iterator<Item = Subpath<DesignerPathId>> + 'a {
    let mut subpaths_intersection_flags = vec![false; path.subpaths.len()];
    for intersection in path_intersections {
        subpaths_intersection_flags[intersection.subpath_index] = true;
    }
    subpaths_intersection_flags
        .into_iter()
        .zip(path.subpaths.iter())
        .filter_map(|(intersected, subgraph)| (!intersected).then_some(subgraph))
        // TODO generalize for different path ops (this is for union)
        .filter(|subpath| !other.contains_point(subpath.get_segment(0).unwrap().start))
        .cloned()
}

fn trace_from(
    start_index: usize,
    self_path: &PathIntersectionData,
    other_path: &PathIntersectionData,
    intersection_visited: &mut [bool],
) -> Vec<Bezier> {
    let mut current_path = self_path;
    let mut other_path = other_path;
    let mut union_path_segments = Vec::new();
    let mut current_index = start_index;

    loop {
        let is_self = std::ptr::eq(self_path, current_path);
        let self_index = if is_self {
            current_index
        } else {
            current_path.intersections[current_index].0
        };
        // log::debug!("is_self: {:?}", is_self);
        if intersection_visited[self_index] {
            // log::debug!("stopped at: {:?} ({})", self_index, current_index);
            break;
        }
        // log::debug!("visiting: {:?} ({})", self_index, current_index);
        intersection_visited[self_index] = true;

        let (_, curr_intersection) = current_path.intersections[current_index];
        let subpath_index = curr_intersection.subpath_index;

        let (segment_ind_jump_pointer, next_intersection) =
            find_next_intersection(current_path, subpath_index, current_index);

        union_path_segments.extend(beziers_between_intersections(
            &current_path.beziers[subpath_index],
            &curr_intersection,
            &next_intersection,
        ));

        current_index = *segment_ind_jump_pointer;
        std::mem::swap(&mut current_path, &mut other_path);
    }

    union_path_segments
}

fn find_next_intersection(
    path: &PathIntersectionData,
    subpath_index: usize,
    current_index: usize,
) -> &(usize, Intersection) {
    let out =  path.intersections
        .get(current_index + 1)
        .filter(|(_, i)| i.subpath_index == subpath_index)
        .unwrap_or_else(|| {
            path.intersections
                .iter()
                .enumerate()
                .find(|(ind, (_, i))| i.subpath_index == subpath_index && *ind != current_index)
                .map(|(_, val)| val)
                .expect("current_intersection was taken from same list, and so should always exist at least one")
        });
    out
}

fn beziers_between_intersections(
    subpath: &[Bezier],
    curr_intersection: &Intersection,
    next_intersection: &Intersection,
) -> Vec<Bezier> {
    // log::debug!(
    //     "computing beziers between curr inters: {:?}, next_inters: {:?}",
    //     curr_intersection,
    //     next_intersection
    // );
    let mut path_segments = vec![];
    for seg_ind in circular_range::circular_range(
        curr_intersection.segment_index,
        next_intersection.segment_index,
        subpath.len(),
        curr_intersection.t > next_intersection.t,
    ) {
        path_segments.push(subpath[seg_ind]);
    }
    let first = path_segments.first_mut().expect("at least one");
    if curr_intersection.t > 1.0 {
        log::warn!("start split: {:?}", curr_intersection.t);
    }
    *first = first.split(TValue::Parametric(curr_intersection.t.clamp(0.0, 1.0)))[1];
    let end_split_t = if path_segments.len() == 1 {
        let adjusted_t = (next_intersection.t - curr_intersection.t) / (1.0 - curr_intersection.t);
        adjusted_t
    } else {
        next_intersection.t
    };
    if end_split_t > 1.0 {
        log::warn!("end split: {:?}", end_split_t);
    }
    let last = path_segments.last_mut().expect("at least one");
    *last = last.split(TValue::Parametric(end_split_t.clamp(0.0, 1.0)))[0];
    // log::debug!("output segments: {:#?}", path_segments);
    path_segments
}

#[derive(Debug)]
struct PathIntersectionData {
    intersections: Vec<(usize, Intersection)>,
    // intersection_markings: Vec<EntryOrExit>,
    beziers: Vec<Vec<Bezier>>,
}

// pub enum EntryOrExit {
//     Entry,
//     Exit,
// }

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
