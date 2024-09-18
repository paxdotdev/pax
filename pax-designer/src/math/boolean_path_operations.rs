use std::{borrow::Borrow, ops::Range};

use bezier_rs::{Bezier, BezierHandles, Identifier, Subpath, TValue};
use glam::{BVec2, DMat2, DVec2};
use pax_engine::log;

pub trait SetOperations {
    fn union(&self, other: &Self) -> Self;
    // TODO more variants
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

impl<T: Identifier> SetOperations for Subpath<T> {
    fn union(&self, other: &Self) -> Self {
        let self_segments: Vec<_> = self.iter().collect();
        let other_segments: Vec<_> = other.iter().collect();
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
            return Self::new(vec![], true);
        }
        let Some((_, start_intersection)) = self_intersections.first() else {
            log::warn!("tried to combine non overlapping subpaths");
            return Self::new(vec![], true);
        };

        const EPS: f64 = 1e-1;
        // TODO make this start point coice more intelligently
        let eps_ahead_of_start_point = self_segments[start_intersection.segment_index]
            .evaluate(TValue::Parametric(start_intersection.t + EPS));
        // if we are inside when walking from first intersection, then start at index 0,
        // otherwise start at the next intersection (index 1)
        let mut index = (other.point_inside(eps_ahead_of_start_point)) as usize;
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
        Subpath::from_beziers(&union_path_segments, true)
    }
}

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

/// breaks down a list into two lists sorted by their respective T value, and with
/// and index that when looked up in the other vector returns the T that was originally
/// the other tuple pair value.

// --------------------- ALL BELLOW COPIED FROM bezier_rs (with minor signature changes) -------------------------------
// this was done to get the t values for both parametric curves being tested

pub fn intersections(
    slf: &Bezier,
    other: &Bezier,
    error: Option<f64>,
    minimum_separation: f64,
) -> Vec<[f64; 2]> {
    // TODO: Consider using the `intersections_between_vectors_of_curves` helper function here
    // Otherwise, use bounding box to determine intersections
    let mut intersection_t_values = unfiltered_intersections(slf, other, error);
    intersection_t_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

    intersection_t_values
        .iter()
        .fold(Vec::new(), |mut accumulator, t| {
            if !accumulator.is_empty() && {
                let a = accumulator.last().unwrap();
                (a[0] - t[0]).abs() + (a[1] - t[1]).abs()
            } < minimum_separation
            {
                accumulator.pop();
            }
            accumulator.push(*t);
            accumulator
        })
}

// TODO: Use an `impl Iterator` return type instead of a `Vec`
/// Returns a list of `t` values that correspond to intersection points between the current bezier curve and the provided one. The returned `t` values are with respect to the current bezier, not the provided parameter.
/// If the provided curve is linear, then zero intersection points will be returned along colinear segments.
/// - `error` - For intersections where the provided bezier is non-linear, `error` defines the threshold for bounding boxes to be considered an intersection point.
fn unfiltered_intersections(slf: &Bezier, other: &Bezier, error: Option<f64>) -> Vec<[f64; 2]> {
    let error = error.unwrap_or(0.5);
    if other.handles == BezierHandles::Linear {
        // Rotate the bezier and the line by the angle that the line makes with the x axis
        let line_directional_vector = other.end - other.start;
        let angle = line_directional_vector.angle_between(DVec2::new(0., 1.));
        let rotation_matrix = DMat2::from_angle(angle);
        let rotated_bezier = slf.apply_transformation(|point| rotation_matrix * point);

        // Translate the bezier such that the line becomes aligned on top of the x-axis
        let vertical_distance = (rotation_matrix * other.start).x;
        let translated_bezier = rotated_bezier.translate(DVec2::new(-vertical_distance, 0.));

        // Compute the roots of the resulting bezier curve
        let list_intersection_t = translated_bezier.find_tvalues_for_x(0.);

        // Calculate line's bounding box
        let [min_corner, max_corner] = other.bounding_box_of_anchors_and_handles();

        const MAX_ABSOLUTE_DIFFERENCE: f64 = 1e-3;
        return list_intersection_t
            // Accept the t value if it is approximately in [0, 1] and if the corresponding coordinates are within the range of the linear line
            .filter(|&t| {
                dvec2_approximately_in_range(
                    unrestricted_parametric_evaluate(slf, t),
                    min_corner,
                    max_corner,
                    MAX_ABSOLUTE_DIFFERENCE,
                )
                .all()
            })
            .map(|t1| {
                // MODIFIED: this needs to also return t values along the line segment
                let point = slf.evaluate(TValue::Parametric(t1));
                let t2 = (point - other.start).dot(line_directional_vector)
                    / line_directional_vector.length_squared();
                [t1.clamp(0., 1.), t2.clamp(0., 1.)]
            })
            .collect();
    }

    // TODO: Consider using the `intersections_between_vectors_of_curves` helper function here
    // Otherwise, use bounding box to determine intersections
    intersections_between_subcurves(slf, 0. ..1., other, 0. ..1., error)
}

/// NOTE: copied from bezier_rs (since not public)
/// Implementation of the algorithm to find curve intersections by iterating on bounding boxes.
/// - `self_original_t_interval` - Used to identify the `t` values of the original parent of `self` that the current iteration is representing.
/// - `other_original_t_interval` - Used to identify the `t` values of the original parent of `other` that the current iteration is representing.
pub(crate) fn intersections_between_subcurves(
    slf: &Bezier,
    slf_original_t_interval: Range<f64>,
    other: &Bezier,
    other_original_t_interval: Range<f64>,
    error: f64,
) -> Vec<[f64; 2]> {
    let bounding_box1 = slf.bounding_box();
    let bounding_box2 = other.bounding_box();

    // Get the `t` interval of the original parent of `self` and determine the middle `t` value
    let Range {
        start: self_start_t,
        end: self_end_t,
    } = slf_original_t_interval;
    let self_mid_t = (self_start_t + self_end_t) / 2.;

    // Get the `t` interval of the original parent of `other` and determine the middle `t` value
    let Range {
        start: other_start_t,
        end: other_end_t,
    } = other_original_t_interval;
    let other_mid_t = (other_start_t + other_end_t) / 2.;

    let error_threshold = DVec2::new(error, error);

    // Check if the bounding boxes overlap
    if do_rectangles_overlap(bounding_box1, bounding_box2) {
        // If bounding boxes are within the error threshold (i.e. are small enough), we have found an intersection
        if (bounding_box1[1] - bounding_box1[0])
            .cmplt(error_threshold)
            .all()
            && (bounding_box2[1] - bounding_box2[0])
                .cmplt(error_threshold)
                .all()
        {
            // Use the middle t value, return the corresponding `t` value for `self` and `other`
            return vec![[self_mid_t, other_mid_t]];
        }

        // Split curves in half and repeat with the combinations of the two halves of each curve
        let [split_1_a, split_1_b] = slf.split(TValue::Parametric(0.5));
        let [split_2_a, split_2_b] = other.split(TValue::Parametric(0.5));

        [
            intersections_between_subcurves(
                &split_1_a,
                self_start_t..self_mid_t,
                &split_2_a,
                other_start_t..other_mid_t,
                error,
            ),
            intersections_between_subcurves(
                &split_1_a,
                self_start_t..self_mid_t,
                &split_2_b,
                other_mid_t..other_end_t,
                error,
            ),
            intersections_between_subcurves(
                &split_1_b,
                self_mid_t..self_end_t,
                &split_2_a,
                other_start_t..other_mid_t,
                error,
            ),
            intersections_between_subcurves(
                &split_1_b,
                self_mid_t..self_end_t,
                &split_2_b,
                other_mid_t..other_end_t,
                error,
            ),
        ]
        .concat()
    } else {
        vec![]
    }
}

/// Determine if two rectangles have any overlap. The rectangles are represented by a pair of coordinates that designate the top left and bottom right corners (in a graphical coordinate system).
pub fn do_rectangles_overlap(rectangle1: [DVec2; 2], rectangle2: [DVec2; 2]) -> bool {
    let [bottom_left1, top_right1] = rectangle1;
    let [bottom_left2, top_right2] = rectangle2;

    top_right1.x >= bottom_left2.x
        && top_right2.x >= bottom_left1.x
        && top_right2.y >= bottom_left1.y
        && top_right1.y >= bottom_left2.y
}
/// Compare the two values in a `DVec2` independently with a provided max absolute value difference.
pub fn dvec2_compare(a: DVec2, b: DVec2, max_abs_diff: f64) -> BVec2 {
    BVec2::new(
        (a.x - b.x).abs() < max_abs_diff,
        (a.y - b.y).abs() < max_abs_diff,
    )
}
/// Determine if the values in a `DVec2` are within a given range independently by using a max absolute value difference comparison.
pub fn dvec2_approximately_in_range(
    point: DVec2,
    min_corner: DVec2,
    max_corner: DVec2,
    max_abs_diff: f64,
) -> BVec2 {
    (point.cmpge(min_corner) & point.cmple(max_corner))
        | dvec2_compare(point, min_corner, max_abs_diff)
        | dvec2_compare(point, max_corner, max_abs_diff)
}
/// Calculate the point on the curve based on the `t`-value provided.
pub fn unrestricted_parametric_evaluate(slf: &Bezier, t: f64) -> DVec2 {
    // Basis code based off of pseudocode found here: <https://pomax.github.io/bezierinfo/#explanation>.

    let t_squared = t * t;
    let one_minus_t = 1. - t;
    let squared_one_minus_t = one_minus_t * one_minus_t;

    match slf.handles {
        BezierHandles::Linear => slf.start.lerp(slf.end, t),
        BezierHandles::Quadratic { handle } => {
            squared_one_minus_t * slf.start + 2. * one_minus_t * t * handle + t_squared * slf.end
        }
        BezierHandles::Cubic {
            handle_start,
            handle_end,
        } => {
            let t_cubed = t_squared * t;
            let cubed_one_minus_t = squared_one_minus_t * one_minus_t;
            cubed_one_minus_t * slf.start
                + 3. * squared_one_minus_t * t * handle_start
                + 3. * one_minus_t * t_squared * handle_end
                + t_cubed * slf.end
        }
    }
}
