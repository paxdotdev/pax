use kurbo::{
    stroke as kurbo_stroke, BezPath, Cap as KurboCap, Circle, Join as KurboJoin, PathEl, Point,
    Rect, Shape, Stroke as KurboStroke, StrokeOpts as KurboStrokeOpts,
};
use pax_runtime::api::{Stroke, StrokeCap};

pub fn stroke_width_pixels(stroke: &Stroke) -> f64 {
    stroke.width.get().expect_pixels().to_float()
}

pub fn stroked_outline_path(centerline: &BezPath, stroke: &Stroke) -> Option<BezPath> {
    let width = stroke_width_pixels(stroke);
    if width <= f64::EPSILON {
        return None;
    }

    let outline = kurbo_stroke(
        centerline.elements().iter().copied(),
        &KurboStroke::new(width)
            .with_join(KurboJoin::Miter)
            .with_miter_limit(4.0)
            .with_caps(to_kurbo_cap(stroke.cap.get())),
        &KurboStrokeOpts::default(),
        0.1,
    );

    if !outline.elements().is_empty() {
        Some(outline)
    } else {
        // kurbo intentionally produces no outline for a fully collapsed path.
        // Pax still needs cap-shaped coverage for zero-length lines so that
        // masking and hit testing remain consistent with rendering.
        degenerate_outline_path(centerline, stroke, width)
    }
}

fn to_kurbo_cap(cap: StrokeCap) -> KurboCap {
    match cap {
        StrokeCap::Butt => KurboCap::Butt,
        StrokeCap::Round => KurboCap::Round,
        StrokeCap::Square => KurboCap::Square,
    }
}

fn degenerate_outline_path(centerline: &BezPath, stroke: &Stroke, width: f64) -> Option<BezPath> {
    let center = collapsed_path_point(centerline)?;
    let half_width = width / 2.0;

    match stroke.cap.get() {
        StrokeCap::Butt => None,
        StrokeCap::Round => Some(Circle::new(center, half_width).to_path(0.1)),
        StrokeCap::Square => Some(
            Rect::new(
                center.x - half_width,
                center.y - half_width,
                center.x + half_width,
                center.y + half_width,
            )
            .to_path(0.1),
        ),
    }
}

fn collapsed_path_point(centerline: &BezPath) -> Option<Point> {
    let mut anchor = None;

    // Only treat the path as degenerate when every point-bearing element
    // collapses to the same location.
    for element in centerline.elements() {
        let points = match *element {
            PathEl::MoveTo(point) | PathEl::LineTo(point) => [Some(point), None, None],
            PathEl::QuadTo(point1, point2) => [Some(point1), Some(point2), None],
            PathEl::CurveTo(point1, point2, point3) => [Some(point1), Some(point2), Some(point3)],
            PathEl::ClosePath => [None, None, None],
        };

        for point in points.into_iter().flatten() {
            match anchor {
                Some(existing) if !same_point(existing, point) => return None,
                Some(_) => {}
                None => anchor = Some(point),
            }
        }
    }

    anchor
}

fn same_point(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() <= f64::EPSILON && (a.y - b.y).abs() <= f64::EPSILON
}
