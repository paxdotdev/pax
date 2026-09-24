//! Viewport-bounded plotting. Geometry stays in the Scroller's world coordinate space.

use crate::expression::{Expression, Range};

pub const WORLD_LIMIT: f64 = 256.0;
pub const DEFAULT_SCALE: f64 = 16.0;
pub const MIN_ZOOM: i32 = -2;
pub const MAX_ZOOM: i32 = 4;
pub const MAX_POINTS: usize = 4096;
pub const WORK_LIMIT: usize = 250_000;

pub fn zoom_scale(level: i32) -> f64 {
    DEFAULT_SCALE * 2.0_f64.powi(level.clamp(MIN_ZOOM, MAX_ZOOM))
}

#[derive(Clone, Copy, Debug)]
pub struct View {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
    pub scale: f64,
}

impl View {
    pub fn extent(self) -> f64 {
        WORLD_LIMIT * 2.0 * self.scale
    }
    /// Resize or zoom around the current world center, constrained by the finite plane.
    pub fn reframe(self, width: f64, height: f64, scale: f64) -> Self {
        let cx = self.world_x(self.width * 0.5);
        let cy = self.world_y(self.height * 0.5);
        let extent = WORLD_LIMIT * 2.0 * scale;
        Self {
            left: ((cx + WORLD_LIMIT) * scale - width * 0.5).clamp(0., (extent - width).max(0.)),
            top: ((WORLD_LIMIT - cy) * scale - height * 0.5).clamp(0., (extent - height).max(0.)),
            width,
            height,
            scale,
        }
    }
    pub fn world_x(self, px: f64) -> f64 {
        (self.left + px) / self.scale - WORLD_LIMIT
    }
    pub fn world_y(self, py: f64) -> f64 {
        WORLD_LIMIT - (self.top + py) / self.scale
    }
}

#[derive(Default, Debug)]
pub struct Plot {
    // Each accepted segment is independent so rejected intervals can never acquire a join.
    pub segments: Vec<[(f64, f64); 2]>,
    pub limited: bool,
    pub enveloped: bool,
    pub work: usize,
}

pub fn plot(expression: &Expression, view: View) -> Plot {
    plot_mode(expression, view, false)
}

/// Polar graphs cover one revolution in radians; negative radii are preserved.
pub fn plot_mode(expression: &Expression, view: View, polar: bool) -> Plot {
    let mut output = Plot::default();
    if ![view.left, view.top, view.width, view.height, view.scale]
        .iter()
        .all(|v| v.is_finite())
        || view.width <= 0.
        || view.height <= 0.
        || view.scale <= 0.
    {
        return output;
    }
    let mut deferred = Vec::new();
    let columns = if polar {
        256
    } else {
        (view.width / 2.).ceil() as usize
    }
    .clamp(1, MAX_POINTS / 2);
    for i in 0..columns {
        // Reserve work AND geometry for every remaining interval. A dense region
        // must never spend the budget belonging to the other side of the LCD.
        let allowance = (WORK_LIMIT - output.work) / (columns - i);
        let slots = (MAX_POINTS / 2 - output.segments.len()) / (columns - i);
        let (a, b) = if polar {
            let step = std::f64::consts::TAU / columns as f64;
            (i as f64 * step, (i + 1) as f64 * step)
        } else {
            (
                view.world_x(view.width * i as f64 / columns as f64),
                view.world_x(view.width * (i + 1) as f64 / columns as f64),
            )
        };
        let mut sampler = Sampler {
            expression,
            view,
            polar,
            budget: allowance,
            output: &mut output,
            deferred: polar.then_some(&mut deferred),
        };
        sampler.interval(a, b, slots, 0);
        let used = allowance - sampler.budget;
        output.work += used;
    }
    // Angular slices vary greatly in visible complexity. Reuse the work and
    // geometry left by culled or simple slices before declaring a detail limit.
    // One bounded retry preserves the initial pass's coverage and hard caps.
    for (i, &(a, b, depth)) in deferred.iter().enumerate() {
        let remaining = deferred.len() - i;
        let allowance = (WORK_LIMIT - output.work) / remaining;
        let slots = (MAX_POINTS / 2 - output.segments.len()) / remaining;
        let mut sampler = Sampler {
            expression,
            view,
            polar,
            budget: allowance,
            output: &mut output,
            deferred: None,
        };
        sampler.interval(a, b, slots, depth);
        output.work += allowance - sampler.budget;
    }
    output
}

struct Sampler<'a> {
    expression: &'a Expression,
    view: View,
    polar: bool,
    budget: usize,
    output: &'a mut Plot,
    deferred: Option<&'a mut Vec<(f64, f64, usize)>>,
}

impl Sampler<'_> {
    fn bounds(&mut self, a: f64, b: f64) -> Option<(Range, Range, Range)> {
        let r = self
            .expression
            .range(Range { lo: a, hi: b }, &mut self.budget)?;
        if self.polar {
            let (x, y) = polar_bounds(r, a, b)?;
            Some((x, y, r))
        } else {
            Some((Range { lo: a, hi: b }, r, r))
        }
    }
    fn sample(&mut self, t: f64) -> Option<(f64, f64)> {
        let r = self.expression.sample(t, &mut self.budget)?;
        Some(if self.polar {
            (r * t.cos(), r * t.sin())
        } else {
            (t, r)
        })
    }
    fn interval(&mut self, a: f64, b: f64, slots: usize, depth: usize) {
        if slots == 0 || self.budget == 0 {
            if !self.defer(a, b, depth) {
                self.output.limited = true;
            }
            return;
        }
        let bounds = self.bounds(a, b);
        if let Some((x, y, radius)) = bounds {
            // Cull before point sampling or refinement, using conservative bounds.
            if x.lo > self.view.world_x(self.view.width)
                || x.hi < self.view.world_x(0.)
                || y.lo > self.view.world_y(0.)
                || y.hi < self.view.world_y(self.view.height)
            {
                return;
            }
            let pa = self.sample(a);
            let pb = self.sample(b);
            if let (Some(pa), Some(pb)) = (pa, pb) {
                let tolerance = 0.5 / self.view.scale;
                let straight = |range: Range, a: f64, b: f64, mid: f64| {
                    (mid - (a * 0.5 + b * 0.5)).abs() <= tolerance
                        && range.lo >= a.min(b) - tolerance
                        && range.hi <= a.max(b) + tolerance
                };
                let resolved = if self.polar {
                    self.polar_flat(radius, a, b, pa, pb, tolerance)
                } else {
                    self.sample(a + (b - a) * 0.5).is_some_and(|pm| {
                        straight(x, pa.0, pb.0, pm.0) && straight(y, pa.1, pb.1, pm.1)
                    })
                };
                if resolved {
                    if let Some(segment) = clip_segment(pa.0, pa.1, pb.0, pb.1, self.view) {
                        self.output.segments.push(segment);
                    }
                    return;
                }
            }
        }
        let pixel_width = if self.polar {
            bounds.map_or(f64::INFINITY, |(x, y, _)| {
                (x.hi - x.lo).max(y.hi - y.lo) * self.view.scale
            })
        } else {
            (b - a) * self.view.scale
        };
        if slots < 2 || self.budget == 0 || depth >= 12 || pixel_width <= 0.75 {
            if depth < 12 && pixel_width > 0.75 && self.defer(a, b, depth) {
                return;
            }
            if let Some((x, y, _)) = bounds {
                self.output.limited = true;
                // An envelope is honest about unresolved subpixel oscillation.
                // Never draw one across an undefined interval or a broad polar box.
                let segment = if !self.polar || (x.hi - x.lo) * self.view.scale <= 1. {
                    let mid = x.lo * 0.5 + x.hi * 0.5;
                    clip_segment(mid, y.lo, mid, y.hi, self.view)
                } else if (y.hi - y.lo) * self.view.scale <= 1. {
                    let mid = y.lo * 0.5 + y.hi * 0.5;
                    clip_segment(x.lo, mid, x.hi, mid, self.view)
                } else {
                    None
                };
                if let Some(segment) = segment {
                    self.output.enveloped = true;
                    self.output.segments.push(segment);
                }
            } else if self.budget == 0 {
                self.output.limited = true;
            }
            return;
        }
        let mid = a + (b - a) * 0.5;
        let right_budget = self.budget / 2;
        self.budget -= right_budget;
        let before = self.output.segments.len();
        self.interval(a, mid, slots / 2, depth + 1);
        self.budget += right_budget;
        self.interval(
            mid,
            b,
            slots - (self.output.segments.len() - before),
            depth + 1,
        );
    }

    fn defer(&mut self, a: f64, b: f64, depth: usize) -> bool {
        if let Some(pending) = &mut self.deferred {
            if pending.len() < MAX_POINTS / 2 {
                pending.push((a, b, depth));
                return true;
            }
        }
        false
    }

    fn polar_flat(
        &mut self,
        radius: Range,
        a: f64,
        b: f64,
        pa: (f64, f64),
        pb: (f64, f64),
        tolerance: f64,
    ) -> bool {
        let dx = pb.0 - pa.0;
        let dy = pb.1 - pa.1;
        let length = dx.hypot(dy);
        if !length.is_finite() || length == 0. {
            return false;
        }
        let ux = dx / length;
        let uy = dy / length;
        let angle = dy.atan2(dx);
        let origin_along = pa.0 * ux + pa.1 * uy;
        let origin_across = -pa.0 * uy + pa.1 * ux;
        // Measure geometric deviation from the chord, independently of how
        // quickly the curve travels along it. Enclose the entire interval so
        // a loop cannot disappear just because its midpoint lands on the chord.
        let fits = |radius, a, b| {
            let Some((along, across)) = polar_bounds(radius, a - angle, b - angle) else {
                return false;
            };
            along.lo >= origin_along - tolerance
                && along.hi <= origin_along + length + tolerance
                && across.lo >= origin_across - tolerance
                && across.hi <= origin_across + tolerance
        };
        if fits(radius, a, b) {
            return true;
        }
        let Some(mid) = self.sample(a + (b - a) * 0.5) else {
            return false;
        };
        let along = (mid.0 - pa.0) * ux + (mid.1 - pa.1) * uy;
        let across = -(mid.0 - pa.0) * uy + (mid.1 - pa.1) * ux;
        if across.abs() > tolerance || along < -tolerance || along > length + tolerance {
            return false;
        }
        // Tighten conservative bounds without emitting more geometry. Interval
        // products lose the correlation between radius and angle; smaller
        // enclosures can certify a long chord that the broad box rejects.
        for i in 0..4 {
            let lo = a + (b - a) * i as f64 / 4.;
            let hi = a + (b - a) * (i + 1) as f64 / 4.;
            let Some(radius) = self.expression.range(Range { lo, hi }, &mut self.budget) else {
                return false;
            };
            if !fits(radius, lo, hi) {
                return false;
            }
        }
        true
    }
}

fn polar_bounds(radius: Range, a: f64, b: f64) -> Option<(Range, Range)> {
    let cosine = Range::new(
        a + std::f64::consts::FRAC_PI_2,
        b + std::f64::consts::FRAC_PI_2,
    )?
    .sin()?;
    Some((
        radius.multiply(cosine)?,
        radius.multiply(Range::new(a, b)?.sin()?)?,
    ))
}

fn clip_segment(x0: f64, y0: f64, x1: f64, y1: f64, v: View) -> Option<[(f64, f64); 2]> {
    let mut points = [[x0, y0], [x1, y1]];
    // Clip y first, preserving the exact boundary coordinate. Reconstructing both
    // coordinates from a rounded t loses the visible span of e.g. 1e200*x.
    for (axis, low, high) in [
        (1, v.world_y(v.height), v.world_y(0.)),
        (0, v.world_x(0.), v.world_x(v.width)),
    ] {
        for (boundary, lower) in [(low, true), (high, false)] {
            let outside = |p: [f64; 2]| {
                if lower {
                    p[axis] < boundary
                } else {
                    p[axis] > boundary
                }
            };
            let a = outside(points[0]);
            let b = outside(points[1]);
            if a && b {
                return None;
            }
            if a != b {
                let magnitude = points[0][axis].abs().max(points[1][axis].abs()).max(1.);
                let t = ((boundary / magnitude - points[0][axis] / magnitude)
                    / (points[1][axis] / magnitude - points[0][axis] / magnitude))
                    .clamp(0., 1.);
                let other = 1 - axis;
                let at = if a { 0 } else { 1 };
                points[at][other] = points[0][other] * (1. - t) + points[1][other] * t;
                points[at][axis] = boundary;
            }
        }
    }
    let points = points.map(|[x, y]| {
        (
            ((x + WORLD_LIMIT) * v.scale - v.left).clamp(0., v.width),
            ((WORLD_LIMIT - y) * v.scale - v.top).clamp(0., v.height),
        )
    });
    points
        .iter()
        .all(|(x, y)| x.is_finite() && y.is_finite())
        .then_some(points)
}

pub fn tick_step(scale: f64) -> f64 {
    let target = 54.0 / scale;
    let decade = 10.0_f64.powf(target.log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|m| m * decade)
        .find(|s| *s >= target)
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn view() -> View {
        View {
            left: 246. * 16.,
            top: 250. * 16.,
            width: 320.,
            height: 192.,
            scale: 16.,
        }
    }
    #[test]
    fn oscillation_never_starves_the_right_side_of_the_viewport() {
        for width in [240., 360., 960., 1200.] {
            for scale in [4., 8., 16.] {
                let v = View {
                    left: (256. - 0.2) * scale,
                    top: 250. * scale,
                    width,
                    height: 12. * scale,
                    scale,
                };
                for source in ["sin(x^2)", "sin(100000*x)"] {
                    let p = plot(&Expression::parse(source, true, None).unwrap(), v);
                    assert!(p.limited);
                    assert!(p.work <= WORK_LIMIT && p.segments.len() * 2 <= MAX_POINTS);
                    for quarter in 0..4 {
                        assert!(
                            p.segments
                                .iter()
                                .any(|s| s[0].0 >= width * quarter as f64 / 4.
                                    && s[0].0 < width * (quarter + 1) as f64 / 4.),
                            "{source} {width} {scale} quarter {quarter}"
                        );
                    }
                    assert!(p.segments.iter().any(|s| s[1].0 >= width - 2.));
                }
            }
        }
    }
    #[test]
    fn enormous_steep_lines_keep_their_visible_height() {
        let p = plot(&Expression::parse("1e200*x", true, None).unwrap(), view());
        assert!(p
            .segments
            .iter()
            .any(|s| (s[1].1 - s[0].1).abs() > view().height * 0.4));
    }
    #[test]
    fn offscreen_curves_are_culled_before_point_sampling() {
        let e = Expression::parse("1000+sin(x^2)", true, None).unwrap();
        let p = plot(&e, view());
        assert!(p.segments.is_empty());
        assert!(!p.limited);
        assert!(p.work < 2000);
    }
    #[test]
    fn polar_circles_roses_and_domains_stay_bounded() {
        let v = view();
        for source in ["4", "-4", "4*cos(3*θ)", "1/cos(θ)", "sqrt(cos(θ))", "1e200"] {
            let e = Expression::parse_polar(source, None).unwrap();
            let p = plot_mode(&e, v, true);
            assert!(p.work <= WORK_LIMIT && p.segments.len() * 2 <= MAX_POINTS);
            for segment in &p.segments {
                for &(x, y) in segment {
                    assert!(x.is_finite() && (0. ..=v.width).contains(&x));
                    assert!(y.is_finite() && (0. ..=v.height).contains(&y));
                    if source == "4" || source == "-4" {
                        let radius = v.world_x(x).hypot(v.world_y(y));
                        assert!((radius - 4.).abs() <= 0.06, "{source}: {radius}");
                    }
                    if source == "1/cos(θ)" {
                        assert!((v.world_x(x) - 1.).abs() < 0.07);
                    }
                    if source == "sqrt(cos(θ))" {
                        assert!(v.world_x(x) >= -0.07);
                    }
                }
            }
            if source != "1e200" {
                assert!(p.segments.len() > 80, "{source}");
            } else {
                assert!(p.segments.is_empty());
            }
        }
    }
    #[test]
    fn polar_chirp_covers_a_dense_reference_at_every_zoom() {
        let e = Expression::parse_polar("4*cos(3*θ^2)", None).unwrap();
        for zoom in MIN_ZOOM..=MAX_ZOOM {
            let scale = zoom_scale(zoom);
            let v = View {
                left: WORLD_LIMIT * scale - 500.,
                top: WORLD_LIMIT * scale - 400.,
                width: 1000.,
                height: 800.,
                scale,
            };
            let p = plot_mode(&e, v, true);
            assert!(!p.limited || p.enveloped, "zoom {zoom}");
            assert!(p.work <= WORK_LIMIT && p.segments.len() * 2 <= MAX_POINTS);
            // Evaluate the formula independently, including both ends of the
            // angular domain. Missing petal tips and radial strokes must fail.
            for i in 0..3000 {
                let theta = std::f64::consts::TAU * i as f64 / 2999.;
                let r = 4. * (3. * theta * theta).cos();
                let x = 500. + scale * r * theta.cos();
                let y = 400. - scale * r * theta.sin();
                if !(0. ..=v.width).contains(&x) || !(0. ..=v.height).contains(&y) {
                    continue;
                }
                let distance = p
                    .segments
                    .iter()
                    .map(|&[a, b]| {
                        let dx = b.0 - a.0;
                        let dy = b.1 - a.1;
                        let length_squared = dx * dx + dy * dy;
                        let t = if length_squared == 0. {
                            0.
                        } else {
                            (((x - a.0) * dx + (y - a.1) * dy) / length_squared).clamp(0., 1.)
                        };
                        (x - a.0 - t * dx).hypot(y - a.1 - t * dy)
                    })
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    distance <= 0.75,
                    "zoom {zoom}, θ={theta}, distance={distance}"
                );
            }
        }
    }
    #[test]
    fn polar_refinement_keeps_hard_caps_when_detail_is_unresolvable() {
        for source in ["4*cos(2000*θ^2)", "4*cos(256*θ)", "tan(θ)", "1/(θ-1)"] {
            let e = Expression::parse_polar(source, None).unwrap();
            for scale in [4., 64., 256.] {
                let v = view().reframe(1000., 800., scale);
                let p = plot_mode(&e, v, true);
                assert!(p.work <= WORK_LIMIT && p.segments.len() * 2 <= MAX_POINTS);
                for point in p.segments.iter().flatten() {
                    assert!(point.0.is_finite() && (0. ..=v.width).contains(&point.0));
                    assert!(point.1.is_finite() && (0. ..=v.height).contains(&point.1));
                }
            }
        }
    }
    #[test]
    fn discontinuities_never_get_connecting_segments() {
        for (source, pole) in [
            ("1/x", 0.),
            ("1/(x-.12345)", 0.12345),
            ("tan(x)", std::f64::consts::FRAC_PI_2),
            ("sec(x)", std::f64::consts::FRAC_PI_2),
            ("csc(x)", 0.),
        ] {
            let p = plot(&Expression::parse(source, true, None).unwrap(), view());
            assert!(!p.segments.is_empty(), "{source}");
            for s in p.segments {
                let a = view().world_x(s[0].0);
                let b = view().world_x(s[1].0);
                assert!(!(a < pole && b > pole), "{source}: {a}..{b}");
            }
        }
    }
    #[test]
    fn finite_bounded_and_clipped() {
        for source in [
            "sin(x)",
            "x^2",
            "sqrt(x)",
            "ln(x)",
            "1e200*x",
            "sin(100000*x)",
            "(-2)^x",
        ] {
            let p = plot(&Expression::parse(source, true, None).unwrap(), view());
            assert!(p.work <= WORK_LIMIT);
            assert!(p.segments.len() * 2 <= MAX_POINTS);
            for s in p.segments {
                for (x, y) in s {
                    assert!(x.is_finite() && y.is_finite());
                    assert!(y >= -1e-8 && y <= 192. + 1e-8);
                }
            }
        }
    }
    #[test]
    fn ordinary_curves_have_detail() {
        let p = plot(&Expression::parse("sin(x)", true, None).unwrap(), view());
        assert!(p.segments.len() > 90);
        assert!(!p.limited);
        let p = plot(&Expression::parse("1000*x", true, None).unwrap(), view());
        assert!(!p.segments.is_empty());
    }
    #[test]
    fn larger_view_and_partial_domains_remain_useful() {
        let wide = View {
            left: 3580.,
            top: 3800.,
            width: 960.,
            height: 520.,
            scale: 16.,
        };
        for source in ["1/x", "sqrt(x)", "ln(x)", "tan(x)"] {
            let p = plot(&Expression::parse(source, true, None).unwrap(), wide);
            assert!(p.work <= WORK_LIMIT, "{source}");
            assert!(p.segments.len() > 100, "{source}");
            if matches!(source, "sqrt(x)" | "ln(x)") {
                assert!(p.segments.iter().all(|s| wide.world_x(s[0].0) >= 0.));
            }
        }
    }
    #[test]
    fn resize_and_zoom_keep_world_center() {
        let old = View {
            left: 4077.,
            top: 3980.,
            ..view()
        };
        let cx = old.world_x(old.width / 2.);
        let cy = old.world_y(old.height / 2.);
        for level in MIN_ZOOM..=MAX_ZOOM {
            let new = old.reframe(640., 400., zoom_scale(level));
            assert_eq!(new.world_x(320.), cx);
            assert_eq!(new.world_y(200.), cy);
            let restored = new.reframe(old.width, old.height, old.scale);
            assert_eq!(restored.left, old.left);
            assert_eq!(restored.top, old.top);
        }
        let edge = View {
            left: 0.,
            top: 0.,
            ..old
        }
        .reframe(1200., 900., zoom_scale(MIN_ZOOM));
        assert_eq!((edge.left, edge.top), (0., 0.));
        assert_eq!(edge.world_x(0.), -WORLD_LIMIT);
        assert_eq!(edge.world_y(0.), WORLD_LIMIT);
        let edge = View {
            left: 8100.,
            top: 8100.,
            ..old
        }
        .reframe(1200., 900., zoom_scale(MIN_ZOOM));
        assert_eq!(edge.left + edge.width, edge.extent());
        assert_eq!(edge.top + edge.height, edge.extent());
    }
    #[test]
    fn zoomed_curves_keep_discontinuities_and_work_limits() {
        for level in MIN_ZOOM..=MAX_ZOOM {
            let v = view().reframe(960., 640., zoom_scale(level));
            assert!(tick_step(v.scale) * v.scale >= 54.);
            for source in ["sin(x)", "1/x", "sqrt(x)"] {
                let p = plot(&Expression::parse(source, true, None).unwrap(), v);
                assert!(!p.segments.is_empty(), "{source} at zoom {level}");
                assert!(p.work <= WORK_LIMIT);
                assert!(p.segments.len() * 2 <= MAX_POINTS);
                if source == "1/x" {
                    assert!(p
                        .segments
                        .iter()
                        .all(|s| !(v.world_x(s[0].0) < 0. && v.world_x(s[1].0) > 0.)));
                }
            }
        }
    }
}
