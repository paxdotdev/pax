//! A positive polar contour keeps every wave one closed loop, without folds or
//! self-intersections. The same contour drives drawing and tile contact.
use pax_kit::*;
use std::f64::consts::TAU;

pub const WAVE_MS: f64 = 2350.0;
const SEGMENTS: usize = 64;

#[pax]
#[custom(Defaults)]
pub struct Ripple {
    pub id: usize,
    pub x: f64,
    pub y: f64,
    pub radius: f64,
    pub progress: f64,
    pub started_at_ms: u64,
    pub hits: u32,
    pub elements: Vec<PathElement>,
    pub band_width: f64,
    pub opacity: f64,
}

impl Ripple {
    pub fn new(id: usize, x: f64, y: f64, width: f64, height: f64, now: u64) -> Self {
        Self {
            id,
            x: x / width,
            y: y / height,
            radius: 0.0,
            progress: 0.0,
            started_at_ms: now,
            hits: 0,
            elements: Vec::new(),
            band_width: 0.0,
            opacity: 0.0,
        }
    }

    pub fn update(&mut self, now: u64, width: f64, height: f64) {
        self.progress = ((now.saturating_sub(self.started_at_ms)) as f64 / WAVE_MS).clamp(0.0, 1.0);
        // Size travel to the farthest corner from this click, so central
        // waves have time to drift instead of leaving the viewport immediately.
        self.radius = (self.x.max(1.0 - self.x) * width).hypot(self.y.max(1.0 - self.y) * height)
            * 1.32
            + 96.0;
        let center_radius = self.radius * self.progress;
        // Keep an open hole even at birth; the band thickens as the bubble grows.
        self.band_width = (width.min(height) * 0.085)
            .clamp(38.0, 78.0)
            .min(center_radius * 0.8);
        self.opacity = (self.progress / 0.055).clamp(0.0, 1.0)
            * ((1.0 - self.progress) / 0.22).clamp(0.0, 1.0);
        let points: Vec<_> = (0..SEGMENTS)
            .map(|i| {
                let angle = i as f64 / SEGMENTS as f64 * TAU;
                let radius = self.contour_radius(angle);
                (
                    self.x * width + radius * angle.cos(),
                    self.y * height + radius * angle.sin(),
                )
            })
            .collect();
        self.elements = closed_spline(&points);
    }

    pub fn contour_radius(&self, angle: f64) -> f64 {
        let phase = self.id as f64 * 1.618 + self.progress * 2.6;
        let distortion = 1.0
            + 0.105 * (3.0 * angle + phase).sin()
            + 0.065 * (2.0 * angle - phase * 0.8).cos()
            + 0.027 * (5.0 * angle + phase * 1.4).sin();
        self.radius * self.progress * distortion
    }

    pub fn touches_tile(&self, index: usize, columns: usize, width: f64, height: f64) -> bool {
        let rows = 15usize.div_ceil(columns);
        let w = width / columns as f64;
        let h = height / rows as f64;
        let x0 = (index % columns) as f64 * w;
        let y0 = (index / columns) as f64 * h;
        let origin = (self.x * width, self.y * height);
        if origin.0 >= x0 && origin.0 <= x0 + w && origin.1 >= y0 && origin.1 <= y0 + h {
            return true;
        }
        // Include the nearest point and sample every edge. Testing filled wave
        // reach also handles a slow frame that skipped over a narrow tile.
        let reached = |x: f64, y: f64| {
            let dx = x - origin.0;
            let dy = y - origin.1;
            dx.hypot(dy) <= self.contour_radius(dy.atan2(dx)) + self.band_width * 0.5
        };
        if reached(origin.0.clamp(x0, x0 + w), origin.1.clamp(y0, y0 + h)) {
            return true;
        }
        (0..=12).any(|i| {
            let t = i as f64 / 12.0;
            reached(x0 + w * t, y0)
                || reached(x0 + w * t, y0 + h)
                || reached(x0, y0 + h * t)
                || reached(x0 + w, y0 + h * t)
        })
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
    fn contour_stays_positive_and_closed_through_its_lifetime() {
        for id in 1..12 {
            let mut wave = Ripple::new(id, 200.0, 300.0, 1000.0, 800.0, 0);
            for time in (50..WAVE_MS as u64).step_by(50) {
                wave.update(time, 1000.0, 800.0);
                for i in 0..360 {
                    assert!(wave.contour_radius(i as f64 * TAU / 360.0) > wave.band_width * 0.5);
                }
                assert!(matches!(wave.elements.last(), Some(PathElement::Close)));
            }
        }
    }
    #[test]
    fn contact_starts_at_origin_and_eventually_reaches_every_tile() {
        let mut wave = Ripple::new(1, 10.0, 10.0, 1000.0, 600.0, 0);
        wave.update(0, 1000.0, 600.0);
        assert!(wave.touches_tile(0, 5, 1000.0, 600.0));
        assert!(!wave.touches_tile(14, 5, 1000.0, 600.0));
        wave.update(WAVE_MS as u64, 1000.0, 600.0);
        assert!((0..15).all(|i| wave.touches_tile(i, 5, 1000.0, 600.0)));
    }
}
