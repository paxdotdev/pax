use kurbo::{BezPath, PathEl, Point};

use super::PathSmoothing;

const EPSILON: f64 = 1e-6;

#[derive(Clone, Copy)]
struct SmoothingParams {
    angle_cut_degrees: f64,
    passes: usize,
}

/// Converts long polyline runs in `path` to cubic Bezier segments.
///
/// Existing quadratic and cubic segments are preserved. Sharp corners split a
/// line run so smoothing does not round deliberate angular forms.
pub fn smooth_bez_path(path: &BezPath, smoothing: PathSmoothing) -> BezPath {
    let Some(params) = smoothing_params(smoothing) else {
        return path.clone();
    };

    let mut output = BezPath::new();
    let mut run = Vec::new();
    let mut contour_start = None;

    for element in path.elements() {
        match *element {
            PathEl::MoveTo(point) => {
                flush_run(&mut output, &mut run, params);
                output.move_to(point);
                run.clear();
                run.push(point);
                contour_start = Some(point);
            }
            PathEl::LineTo(point) => {
                if run.is_empty() {
                    output.line_to(point);
                } else {
                    push_distinct(&mut run, point);
                }
            }
            PathEl::QuadTo(control, point) => {
                flush_run(&mut output, &mut run, params);
                output.quad_to(control, point);
                run.clear();
                run.push(point);
            }
            PathEl::CurveTo(control_1, control_2, point) => {
                flush_run(&mut output, &mut run, params);
                output.curve_to(control_1, control_2, point);
                run.clear();
                run.push(point);
            }
            PathEl::ClosePath => {
                if let (Some(start), Some(last)) = (contour_start, run.last().copied()) {
                    if distance(last, start) > EPSILON {
                        run.push(start);
                    }
                }
                flush_run(&mut output, &mut run, params);
                output.close_path();
                run.clear();
                contour_start = None;
            }
        }
    }

    flush_run(&mut output, &mut run, params);
    output
}

fn smoothing_params(smoothing: PathSmoothing) -> Option<SmoothingParams> {
    match smoothing {
        PathSmoothing::None => None,
        PathSmoothing::Light => Some(SmoothingParams {
            angle_cut_degrees: 130.0,
            passes: 1,
        }),
        PathSmoothing::Strong => Some(SmoothingParams {
            angle_cut_degrees: 95.0,
            passes: 2,
        }),
    }
}

fn flush_run(output: &mut BezPath, run: &mut Vec<Point>, params: SmoothingParams) {
    let clean = clean_points(run);
    if clean.len() < 2 {
        run.clear();
        return;
    }

    let mut cuts = vec![0usize];
    for index in 1..clean.len() - 1 {
        if angle_degrees(clean[index - 1], clean[index], clean[index + 1])
            <= params.angle_cut_degrees
        {
            cuts.push(index);
        }
    }
    cuts.push(clean.len() - 1);

    for pair in cuts.windows(2) {
        let start = pair[0];
        let end = pair[1];
        if end <= start {
            continue;
        }
        let subrun = smooth_points(&clean[start..=end], params.passes);
        emit_subrun(output, &subrun);
    }

    run.clear();
}

fn emit_subrun(output: &mut BezPath, points: &[Point]) {
    if points.len() < 2 {
        return;
    }
    if points.len() == 2 {
        output.line_to(points[1]);
        return;
    }

    for index in 0..points.len() - 1 {
        let p0 = if index == 0 {
            points[index]
        } else {
            points[index - 1]
        };
        let p1 = points[index];
        let p2 = points[index + 1];
        let p3 = if index + 2 < points.len() {
            points[index + 2]
        } else {
            points[index + 1]
        };

        let control_1 = p1 + (p2 - p0) * (1.0 / 6.0);
        let control_2 = p2 - (p3 - p1) * (1.0 / 6.0);
        output.curve_to(control_1, control_2, p2);
    }
}

fn smooth_points(points: &[Point], passes: usize) -> Vec<Point> {
    if points.len() < 4 || passes == 0 {
        return points.to_vec();
    }

    let mut output = points.to_vec();
    for _ in 0..passes {
        let mut next = Vec::with_capacity(output.len());
        next.push(output[0]);
        for index in 1..output.len() - 1 {
            let point = Point::new(
                output[index - 1].x * 0.25 + output[index].x * 0.5 + output[index + 1].x * 0.25,
                output[index - 1].y * 0.25 + output[index].y * 0.5 + output[index + 1].y * 0.25,
            );
            next.push(point);
        }
        next.push(output[output.len() - 1]);
        output = next;
    }
    output
}

fn clean_points(points: &[Point]) -> Vec<Point> {
    let mut clean = Vec::with_capacity(points.len());
    for point in points {
        push_distinct(&mut clean, *point);
    }
    clean
}

fn push_distinct(points: &mut Vec<Point>, point: Point) {
    if points
        .last()
        .map(|last| distance(*last, point) > EPSILON)
        .unwrap_or(true)
    {
        points.push(point);
    }
}

fn angle_degrees(previous: Point, current: Point, next: Point) -> f64 {
    let vector_1 = previous - current;
    let vector_2 = next - current;
    let length_1 = vector_1.hypot();
    let length_2 = vector_2.hypot();
    if length_1 < EPSILON || length_2 < EPSILON {
        return 180.0;
    }
    let dot = ((vector_1.x * vector_2.x) + (vector_1.y * vector_2.y)) / (length_1 * length_2);
    dot.clamp(-1.0, 1.0).acos().to_degrees()
}

fn distance(a: Point, b: Point) -> f64 {
    (a - b).hypot()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_preserves_path() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((50.0, 20.0));
        path.line_to((100.0, 0.0));

        assert_eq!(smooth_bez_path(&path, PathSmoothing::None), path);
    }

    #[test]
    fn smoothing_converts_gentle_line_run_to_curves() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((25.0, 10.0));
        path.line_to((50.0, 12.0));
        path.line_to((75.0, 10.0));
        path.line_to((100.0, 0.0));

        let smoothed = smooth_bez_path(&path, PathSmoothing::Light);

        assert!(smoothed
            .elements()
            .iter()
            .any(|element| matches!(element, PathEl::CurveTo(..))));
    }

    #[test]
    fn smoothing_preserves_sharp_corners_as_lines() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((50.0, 0.0));
        path.line_to((50.0, 50.0));

        let smoothed = smooth_bez_path(&path, PathSmoothing::Light);

        assert!(smoothed
            .elements()
            .iter()
            .any(|element| matches!(element, PathEl::LineTo(..))));
    }
}
