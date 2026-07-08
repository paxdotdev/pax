use kurbo::{BezPath, CubicBez, Line, ParamCurve, ParamCurveArclen, PathSeg, QuadBez};

const PATH_TRIM_ACCURACY: f64 = 0.1;

#[derive(Clone, Copy)]
struct TrimSegment {
    segment: PathSeg,
    contour_index: usize,
    closes_contour: bool,
}

#[derive(Clone, Copy)]
struct TrimContour {
    first_segment_index: Option<usize>,
    last_segment_index: Option<usize>,
    is_closed: bool,
}

/// Returns the sub-path visible between normalized `draw_start` and `draw_end`.
pub fn trim_bez_path(path: &BezPath, draw_start: f64, draw_end: f64) -> BezPath {
    let start = draw_start.clamp(0.0, 1.0);
    let end = draw_end.clamp(0.0, 1.0);
    if start <= f64::EPSILON && end >= 1.0 - f64::EPSILON {
        return path.clone();
    }
    if start >= end {
        return BezPath::new();
    }

    let (segments, contours) = collect_trim_segments(path);
    let lengths = segments
        .iter()
        .map(|record| record.segment.arclen(PATH_TRIM_ACCURACY).max(0.0))
        .collect::<Vec<_>>();
    let total_length = lengths.iter().sum::<f64>();
    if total_length <= f64::EPSILON {
        return BezPath::new();
    }

    let trim_start = start * total_length;
    let trim_end = end * total_length;
    let mut trimmed = BezPath::new();
    let mut current_point = None;
    let mut cursor = 0.0;
    let mut segment_starts = Vec::with_capacity(lengths.len());
    let mut segment_ends = Vec::with_capacity(lengths.len());

    for segment_length in &lengths {
        segment_starts.push(cursor);
        cursor += segment_length;
        segment_ends.push(cursor);
    }

    let mut fully_visible_closed_contours = vec![false; contours.len()];
    for (contour_index, contour) in contours.iter().enumerate() {
        if !contour.is_closed {
            continue;
        }
        let (Some(first_segment_index), Some(last_segment_index)) =
            (contour.first_segment_index, contour.last_segment_index)
        else {
            continue;
        };
        let contour_start = segment_starts[first_segment_index];
        let contour_end = segment_ends[last_segment_index];
        fully_visible_closed_contours[contour_index] =
            trim_start <= contour_start + f64::EPSILON && trim_end >= contour_end - f64::EPSILON;
    }

    for (index, (record, segment_length)) in segments.into_iter().zip(lengths).enumerate() {
        let segment_start = segment_starts[index];
        let segment_end = segment_ends[index];
        if segment_length <= f64::EPSILON || segment_end <= trim_start + f64::EPSILON {
            continue;
        }
        if segment_start >= trim_end - f64::EPSILON {
            break;
        }

        let local_start = (trim_start - segment_start).clamp(0.0, segment_length);
        let local_end = (trim_end - segment_start).clamp(0.0, segment_length);
        if local_start >= local_end {
            continue;
        }

        let whole_segment_visible =
            local_start <= f64::EPSILON && local_end >= segment_length - f64::EPSILON;
        let visible_segment = if whole_segment_visible {
            record.segment
        } else {
            let t0 = if local_start <= f64::EPSILON {
                0.0
            } else {
                record.segment.inv_arclen(local_start, PATH_TRIM_ACCURACY)
            };
            let t1 = if local_end >= segment_length - f64::EPSILON {
                1.0
            } else {
                record.segment.inv_arclen(local_end, PATH_TRIM_ACCURACY)
            };
            if t0 >= t1 {
                continue;
            }
            record.segment.subsegment(t0..t1)
        };

        append_segment(&mut trimmed, &mut current_point, visible_segment);
        if whole_segment_visible
            && record.closes_contour
            && fully_visible_closed_contours[record.contour_index]
        {
            trimmed.close_path();
            current_point = None;
        }
    }

    trimmed
}

fn collect_trim_segments(path: &BezPath) -> (Vec<TrimSegment>, Vec<TrimContour>) {
    let mut segments = Vec::new();
    let mut contours = Vec::new();
    let mut current_contour = None;
    let mut contour_start = None;
    let mut current_point = None;

    for element in path.iter() {
        match element {
            kurbo::PathEl::MoveTo(point) => {
                current_contour = Some(contours.len());
                contours.push(TrimContour {
                    first_segment_index: None,
                    last_segment_index: None,
                    is_closed: false,
                });
                contour_start = Some(point);
                current_point = Some(point);
            }
            kurbo::PathEl::LineTo(point) => {
                let (Some(from), Some(contour_index)) = (current_point, current_contour) else {
                    continue;
                };
                push_trim_segment(
                    &mut segments,
                    &mut contours,
                    contour_index,
                    PathSeg::Line(Line::new(from, point)),
                    false,
                );
                current_point = Some(point);
            }
            kurbo::PathEl::QuadTo(control, point) => {
                let (Some(from), Some(contour_index)) = (current_point, current_contour) else {
                    continue;
                };
                push_trim_segment(
                    &mut segments,
                    &mut contours,
                    contour_index,
                    PathSeg::Quad(QuadBez::new(from, control, point)),
                    false,
                );
                current_point = Some(point);
            }
            kurbo::PathEl::CurveTo(control_1, control_2, point) => {
                let (Some(from), Some(contour_index)) = (current_point, current_contour) else {
                    continue;
                };
                push_trim_segment(
                    &mut segments,
                    &mut contours,
                    contour_index,
                    PathSeg::Cubic(CubicBez::new(from, control_1, control_2, point)),
                    false,
                );
                current_point = Some(point);
            }
            kurbo::PathEl::ClosePath => {
                let (Some(from), Some(start), Some(contour_index)) =
                    (current_point, contour_start, current_contour)
                else {
                    continue;
                };
                contours[contour_index].is_closed = true;
                if same_kurbo_point(from, start) {
                    if let Some(last_segment_index) = contours[contour_index].last_segment_index {
                        segments[last_segment_index].closes_contour = true;
                    }
                } else {
                    push_trim_segment(
                        &mut segments,
                        &mut contours,
                        contour_index,
                        PathSeg::Line(Line::new(from, start)),
                        true,
                    );
                    current_point = Some(start);
                }
            }
        }
    }

    (segments, contours)
}

fn push_trim_segment(
    segments: &mut Vec<TrimSegment>,
    contours: &mut [TrimContour],
    contour_index: usize,
    segment: PathSeg,
    closes_contour: bool,
) {
    let segment_index = segments.len();
    let contour = &mut contours[contour_index];
    if contour.first_segment_index.is_none() {
        contour.first_segment_index = Some(segment_index);
    }
    contour.last_segment_index = Some(segment_index);
    segments.push(TrimSegment {
        segment,
        contour_index,
        closes_contour,
    });
}

fn append_segment(path: &mut BezPath, current_point: &mut Option<kurbo::Point>, segment: PathSeg) {
    let start = segment.start();
    if current_point
        .map(|point| !same_kurbo_point(point, start))
        .unwrap_or(true)
    {
        path.move_to(start);
    }
    path.push(segment.as_path_el());
    *current_point = Some(segment.end());
}

fn same_kurbo_point(a: kurbo::Point, b: kurbo::Point) -> bool {
    (a.x - b.x).abs() <= f64::EPSILON && (a.y - b.y).abs() <= f64::EPSILON
}
