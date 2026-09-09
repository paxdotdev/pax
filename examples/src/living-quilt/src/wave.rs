//! A positive polar contour keeps every wave one closed loop, without folds or
//! self-intersections. The same contour drives drawing and tile contact.
use pax_kit::*;
use std::f64::consts::TAU;

pub const WAVE_MS: f64 = 1175.0;
const SEGMENTS: usize = 24;

#[pax]
#[custom(Defaults)]
#[has_helpers]
pub struct Ripple {
    pub id: usize,
    pub x: f64,
    pub y: f64,
    pub started_at_ms: u64,
}

impl Ripple {
    pub fn new(id: usize, x: f64, y: f64, width: f64, height: f64, now: u64) -> Self {
        Self {
            id,
            x: x / width,
            y: y / height,
            started_at_ms: now,
        }
    }

    pub fn is_finished(&self, now: u64) -> bool {
        now.saturating_sub(self.started_at_ms) as f64 >= WAVE_MS
    }

    pub fn sample(&self, now: u64, width: f64, height: f64) -> WaveSample {
        let progress = ((now.saturating_sub(self.started_at_ms)) as f64 / WAVE_MS).clamp(0.0, 1.0);
        // Size travel to the farthest corner from this click, so central
        // waves have time to drift instead of leaving the viewport immediately.
        let radius = (self.x.max(1.0 - self.x) * width).hypot(self.y.max(1.0 - self.y) * height)
            * 1.32
            + 96.0;
        let center_radius = radius * progress;
        // Keep an open hole even at birth; the band thickens as the bubble grows.
        let band_width = (width.min(height) * 0.0425)
            .clamp(19.0, 39.0)
            .min(center_radius * 0.4)
            / 3.0;
        let opacity =
            (progress / 0.055).clamp(0.0, 1.0) * ((1.0 - progress) / 0.22).clamp(0.0, 1.0);
        WaveSample {
            id: self.id,
            x: self.x * width,
            y: self.y * height,
            progress,
            radius,
            band_width,
            opacity,
        }
    }
}

// Direct Path bindings keep geometry local to each repeated mask leaf. These
// pure helpers also work off-tree, where component templates are not mounted.
#[helpers]
impl Ripple {
    pub fn path(ripple: Ripple, now: u64, width: f64, height: f64) -> Vec<PathElement> {
        ripple.sample(now, width, height).elements()
    }

    pub fn width(ripple: Ripple, now: u64, width: f64, height: f64) -> f64 {
        ripple.sample(now, width, height).band_width
    }

    pub fn alpha(ripple: Ripple, now: u64) -> f64 {
        ripple.sample(now, 0.0, 0.0).opacity
    }
}

// A cheap analytic sample is shared by contact checks and per-ring drawing.
// Only drawing constructs a path; repeat descriptors never contain geometry.
#[derive(Clone, Copy, Default)]
pub struct WaveSample {
    id: usize,
    x: f64,
    y: f64,
    pub progress: f64,
    pub radius: f64,
    pub band_width: f64,
    pub opacity: f64,
}

impl WaveSample {
    pub fn elements(&self) -> Vec<PathElement> {
        let points: Vec<_> = (0..SEGMENTS)
            .map(|i| {
                let angle = i as f64 / SEGMENTS as f64 * TAU;
                let radius = self.contour_radius(angle);
                (self.x + radius * angle.cos(), self.y + radius * angle.sin())
            })
            .collect();
        closed_spline(&points)
    }

    pub fn contour_radius(&self, angle: f64) -> f64 {
        self.radius * self.progress * self.contour_profile(angle).0
    }

    fn contour_profile(&self, angle: f64) -> (f64, f64) {
        let phase = self.id as f64 * 1.618 + self.progress * 2.6;
        let distortion = 1.0
            + 0.105 * (3.0 * angle + phase).sin()
            + 0.065 * (2.0 * angle - phase * 0.8).cos()
            + 0.027 * (5.0 * angle + phase * 1.4).sin();
        let derivative = 0.315 * (3.0 * angle + phase).cos()
            - 0.13 * (2.0 * angle - phase * 0.8).sin()
            + 0.135 * (5.0 * angle + phase * 1.4).cos();
        (distortion, derivative)
    }

    fn normal_direction(&self, angle: f64) -> usize {
        let (radius, derivative) = self.contour_profile(angle);
        // For q = r(theta) e_r, the outward normal is r e_r - r' e_theta.
        // Unlike a ray from the click, it follows the local bend of the wave.
        let nx = radius * angle.cos() + derivative * angle.sin();
        let ny = radius * angle.sin() - derivative * angle.cos();
        if nx.abs() > ny.abs() {
            if nx > 0.0 {
                1
            } else {
                3
            }
        } else if ny > 0.0 {
            2
        } else {
            0
        }
    }

    pub fn contact_direction(
        &self,
        index: usize,
        columns: usize,
        width: f64,
        height: f64,
    ) -> Option<usize> {
        let rows = 15usize.div_ceil(columns);
        let w = width / columns as f64;
        let h = height / rows as f64;
        let x0 = (index % columns) as f64 * w;
        let y0 = (index / columns) as f64 * h;
        let origin = (self.x, self.y);
        if origin.0 >= x0 && origin.0 <= x0 + w && origin.1 >= y0 && origin.1 <= y0 + h {
            // The tile containing the click is reached at birth, before there
            // is a unique contact point. Use the contour toward its center.
            let angle = (y0 + h * 0.5 - origin.1).atan2(x0 + w * 0.5 - origin.0);
            return Some(self.normal_direction(angle));
        }
        // Find first contact rather than picking the first reached edge in
        // iteration order. Sampling includes the nearest point and each edge;
        // filled-wave reach also handles a frame that skipped over the band.
        let mut contact = (f64::INFINITY, 0.0);
        let mut consider = |x: f64, y: f64| {
            let dx = x - origin.0;
            let dy = y - origin.1;
            let angle = dy.atan2(dx);
            let travel =
                (dx.hypot(dy) - self.band_width * 0.5).max(0.0) / self.contour_profile(angle).0;
            if travel < contact.0 {
                contact = (travel, angle);
            }
        };
        consider(origin.0.clamp(x0, x0 + w), origin.1.clamp(y0, y0 + h));
        for i in 0..=12 {
            let t = i as f64 / 12.0;
            consider(x0 + w * t, y0);
            consider(x0 + w * t, y0 + h);
            consider(x0, y0 + h * t);
            consider(x0 + w, y0 + h * t);
        }
        (contact.0 <= self.radius * self.progress).then(|| self.normal_direction(contact.1))
    }
}

fn closed_spline(points: &[(f64, f64)]) -> Vec<PathElement> {
    let px = |x: f64| Size::Pixels(x.into());
    let mut path = vec![PathElement::Point(px(points[0].0), px(points[0].1))];
    for i in 0..points.len() {
        let p0 = points[(i + points.len() - 1) % points.len()];
        let p1 = points[i];
        let p2 = points[(i + 1) % points.len()];
        let p3 = points[(i + 2) % points.len()];
        path.push(PathElement::Cubic(
            px(p1.0 + (p2.0 - p0.0) / 6.0),
            px(p1.1 + (p2.1 - p0.1) / 6.0),
            px(p2.0 - (p3.0 - p1.0) / 6.0),
            px(p2.1 - (p3.1 - p1.1) / 6.0),
        ));
        path.push(PathElement::Point(px(p2.0), px(p2.1)));
    }
    path.push(PathElement::Close);
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paxel_helpers_derive_each_ring_from_immutable_data() {
        use pax_kit::pax_engine::api::pax_value::functions::call_function;

        Ripple::register_all_functions();
        let ripple = Ripple::new(3, 300.0, 200.0, 1000.0, 600.0, 0);
        let descriptor = ripple.clone().to_pax_value();
        let mut previous = None;
        for (now, width, height) in [
            (100u64, 1000.0, 600.0),
            (200, 1000.0, 600.0),
            (450, 390.0, 844.0),
            (800, 390.0, 844.0),
            (1175, 390.0, 844.0),
        ] {
            let args = vec![
                descriptor.clone(),
                now.to_pax_value(),
                width.to_pax_value(),
                height.to_pax_value(),
            ];
            let expected = ripple.sample(now, width, height);
            let path = call_function("Ripple".into(), "path".into(), args.clone()).unwrap();
            assert_eq!(path, expected.elements().to_pax_value());
            assert_eq!(
                call_function("Ripple".into(), "width".into(), args).unwrap(),
                expected.band_width.to_pax_value()
            );
            assert_eq!(
                call_function(
                    "Ripple".into(),
                    "alpha".into(),
                    vec![descriptor.clone(), now.to_pax_value()]
                )
                .unwrap(),
                expected.opacity.to_pax_value()
            );
            if let Some(previous) = previous {
                assert_ne!(path, previous);
            }
            previous = Some(path);
        }
        assert_eq!(descriptor, ripple.to_pax_value());
    }

    #[test]
    fn contour_stays_positive_and_closed_through_its_lifetime() {
        for id in 1..12 {
            let wave = Ripple::new(id, 200.0, 300.0, 1000.0, 800.0, 0);
            for time in (50..WAVE_MS as u64).step_by(50) {
                let wave = wave.sample(time, 1000.0, 800.0);
                for i in 0..360 {
                    assert!(wave.contour_radius(i as f64 * TAU / 360.0) > wave.band_width * 0.5);
                }
                let elements = wave.elements();
                assert!(matches!(elements.last(), Some(PathElement::Close)));
                assert_eq!(
                    elements
                        .iter()
                        .filter(|e| matches!(e, PathElement::Cubic(..)))
                        .count(),
                    24
                );
            }
        }
    }
    #[test]
    fn contact_starts_at_origin_and_eventually_reaches_every_tile() {
        let ripple = Ripple::new(1, 10.0, 10.0, 1000.0, 600.0, 0);
        let wave = ripple.sample(0, 1000.0, 600.0);
        assert!(wave.contact_direction(0, 5, 1000.0, 600.0).is_some());
        assert!(wave.contact_direction(14, 5, 1000.0, 600.0).is_none());
        let wave = ripple.sample(WAVE_MS as u64, 1000.0, 600.0);
        assert!((0..15).all(|i| wave.contact_direction(i, 5, 1000.0, 600.0).is_some()));
    }

    #[test]
    fn direction_matches_the_outward_curve_normal_not_just_the_click_ray() {
        let mut differs_from_radial = false;
        let wave = Ripple::new(1, 500.0, 300.0, 1000.0, 600.0, 0).sample(500, 1000.0, 600.0);
        for i in 0..360 {
            let angle = i as f64 * TAU / 360.0;
            let point = |theta: f64| {
                let r = wave.contour_radius(theta);
                (r * theta.cos(), r * theta.sin())
            };
            let a = point(angle - 0.00001);
            let b = point(angle + 0.00001);
            let normal = (b.1 - a.1, a.0 - b.0);
            let directions = [(0.0, -1.0), (1.0, 0.0), (0.0, 1.0), (-1.0, 0.0)];
            let chosen = wave.normal_direction(angle);
            let score = |d: (f64, f64)| normal.0 * d.0 + normal.1 * d.1;
            assert!(directions
                .iter()
                .all(|d| score(directions[chosen]) + 1e-5 >= score(*d)));
            let ray_best = directions
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| {
                    (a.0 * angle.cos() + a.1 * angle.sin())
                        .total_cmp(&(b.0 * angle.cos() + b.1 * angle.sin()))
                })
                .unwrap()
                .0;
            differs_from_radial |= chosen != ray_best;
        }
        assert!(differs_from_radial);
    }

    #[test]
    fn tiles_on_each_side_are_pushed_away_from_the_wave() {
        let wave = Ripple::new(1, 500.0, 300.0, 1000.0, 600.0, 0).sample(1200, 1000.0, 600.0);
        for (tile, direction) in [(2, 0), (9, 1), (12, 2), (5, 3)] {
            assert_eq!(
                wave.contact_direction(tile, 5, 1000.0, 600.0),
                Some(direction)
            );
        }
    }

    #[test]
    fn ring_stroke_is_one_third_the_previous_width() {
        for (width, height) in [(1000.0, 600.0), (390.0, 844.0)] {
            let ripple = Ripple::new(1, width * 0.5, height * 0.5, width, height, 0);
            for now in [0, 20, 100, 500, 1700] {
                let wave = ripple.sample(now, width, height);
                let previous = (width.min(height) * 0.0425)
                    .clamp(19.0, 39.0)
                    .min(wave.radius * wave.progress * 0.4);
                assert!((wave.band_width * 3.0 - previous).abs() < 1e-8);
            }
        }
    }

    #[test]
    fn ring_plays_twice_as_fast_and_fades_out_at_completion() {
        let ripple = Ripple::new(1, 500.0, 300.0, 1000.0, 600.0, 100);
        for elapsed in [0, 235, 470, 940, 1175, 2350] {
            let wave = ripple.sample(100 + elapsed, 1000.0, 600.0);
            let expected_progress = (elapsed as f64 * 2.0 / 2350.0).clamp(0.0, 1.0);
            assert!((wave.progress - expected_progress).abs() < 1e-8);
            if elapsed >= 1175 {
                assert_eq!(wave.opacity, 0.0);
            }
        }
    }
}
