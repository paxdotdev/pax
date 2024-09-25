// --------------------- ALL BELLOW COPIED FROM bezier_rs (with minor signature changes) -------------------------------
// this was done to get the t values for both parametric curves being tested

use std::ops::Range;

use bezier_rs::{Bezier, BezierHandles, TValue};
use glam::{BVec2, DMat2, DVec2};
use pax_engine::log;

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
    // TODO: Consider using the `intersections_between_vectors_of_curves` helper function here
    // Otherwise, use bounding box to determine intersections
    intersections_between_subcurves(slf, 0. ..1., other, 0. ..1., error)
}

/// NOTE: copied from bezier_rs (since not public)
/// Implementation of the algorithm to find curve intersections by iterating on bounding boxes.
/// - `self_original_t_interval` - Used to identify the `t` values of the original parent of `self` that the current iteration is representing.
/// - `other_original_t_interval` - Used to identify the `t` values of the original parent of `other` that the current iteration is representing.
pub fn intersections_between_subcurves(
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
fn do_rectangles_overlap(rectangle1: [DVec2; 2], rectangle2: [DVec2; 2]) -> bool {
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
fn dvec2_approximately_in_range(
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
fn unrestricted_parametric_evaluate(slf: &Bezier, t: f64) -> DVec2 {
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
