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
    pub work: usize,
}

pub fn plot(expression: &Expression, view: View) -> Plot {
    let mut sampler = Sampler {
        expression,
        view,
        budget: WORK_LIMIT,
        output: Plot::default(),
    };
    let columns = (view.width / 3.0).ceil() as usize;
    for i in 0..columns {
        if sampler.exhausted() {
            break;
        }
        let a = view.world_x(i as f64 * 3.0);
        let b = view.world_x(((i + 1) as f64 * 3.0).min(view.width));
        sampler.interval(a, b, 0);
    }
    sampler.output.work = WORK_LIMIT - sampler.budget;
    sampler.output
}

struct Sampler<'a> {
    expression: &'a Expression,
    view: View,
    budget: usize,
    output: Plot,
}

impl Sampler<'_> {
    fn exhausted(&mut self) -> bool {
        if self.budget == 0 || self.output.segments.len() * 2 >= MAX_POINTS {
            self.output.limited = true;
            true
        } else {
            false
        }
    }

    fn interval(&mut self, a: f64, b: f64, depth: usize) {
        if self.exhausted() {
            return;
        }
        let range = self
            .expression
            .range(Range { lo: a, hi: b }, &mut self.budget);
        if let Some(range) = range {
            if range.lo > self.view.world_y(0.0) || range.hi < self.view.world_y(self.view.height) {
                return;
            }
            let ya = self.expression.sample(a, &mut self.budget);
            let yb = self.expression.sample(b, &mut self.budget);
            let mid = (a + b) * 0.5;
            let ym = self.expression.sample(mid, &mut self.budget);
            if let (Some(ya), Some(yb), Some(ym)) = (ya, yb, ym) {
                let tolerance = 0.5 / self.view.scale;
                // Range overshoot catches oscillation that midpoint-only sampling can alias.
                let straight = (ym - (ya * 0.5 + yb * 0.5)).abs() <= tolerance
                    && range.lo >= ya.min(yb) - tolerance
                    && range.hi <= ya.max(yb) + tolerance;
                if straight {
                    if let Some(segment) = clip_segment(a, ya, b, yb, self.view) {
                        self.output.segments.push(segment);
                    }
                    return;
                }
            }
        }
        // Resolve steep finite curves more finely, without spending the budget on
        // whole undefined regions such as the negative half of sqrt(x).
        let minimum_width = if range.is_some() { 0.015625 } else { 0.25 };
        if depth >= 12 || (b - a) * self.view.scale < minimum_width {
            // Undefined intervals are ordinary gaps. Finite but unresolved detail is disclosed.
            if range.is_some() {
                self.output.limited = true;
            }
            return;
        }
        let middle = a + (b - a) * 0.5;
        self.interval(a, middle, depth + 1);
        self.interval(middle, b, depth + 1);
    }
}

fn clip_segment(
    mut x0: f64,
    mut y0: f64,
    mut x1: f64,
    mut y1: f64,
    v: View,
) -> Option<[(f64, f64); 2]> {
    let bottom = v.world_y(v.height);
    let top = v.world_y(0.0);
    if y0.min(y1) > top || y0.max(y1) < bottom {
        return None;
    }
    let original = (x0, y0, x1, y1);
    // Scale before subtraction: opposite huge finite values may otherwise overflow.
    let intersect = |boundary: f64| {
        let magnitude = original.1.abs().max(original.3.abs()).max(1.0);
        let t = (boundary / magnitude - original.1 / magnitude)
            / (original.3 / magnitude - original.1 / magnitude);
        original.0 + (original.2 - original.0) * t
    };
    if y0 < bottom {
        x0 = intersect(bottom);
        y0 = bottom;
    }
    if y0 > top {
        x0 = intersect(top);
        y0 = top;
    }
    if y1 < bottom {
        x1 = intersect(bottom);
        y1 = bottom;
    }
    if y1 > top {
        x1 = intersect(top);
        y1 = top;
    }
    let points = [
        (
            (x0 + WORLD_LIMIT) * v.scale - v.left,
            (WORLD_LIMIT - y0) * v.scale - v.top,
        ),
        (
            (x1 + WORLD_LIMIT) * v.scale - v.left,
            (WORLD_LIMIT - y1) * v.scale - v.top,
        ),
    ];
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
            assert!(!p.limited, "{source}: work {}", p.work);
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
