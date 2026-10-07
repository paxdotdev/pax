//! Canonical logical-to-physical backing contract for both planning and allocation.

use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug)]
struct DensityState {
    owner: Option<u32>,
    density: [f64; 2],
    target: [f64; 2],
    since: f64,
}

/// Grow immediately at display-quality density, but shrink only after 500 ms
/// at a stable scale. Logarithmic upward quantization bounds extra density to
/// about 4.5% and makes rotation/floating-point noise allocation-inert.
#[derive(Default)]
pub struct RasterDensityTracker {
    layers: BTreeMap<usize, DensityState>,
}

impl RasterDensityTracker {
    pub fn observe(
        &mut self,
        layer: usize,
        owner: Option<u32>,
        density: [f64; 2],
        now: f64,
    ) -> ([f64; 2], bool) {
        let target = density.map(|v| 2.0_f64.powf((v.log2() * 16.0 - 1e-9).ceil() / 16.0));
        let s = self.layers.entry(layer).or_insert(DensityState {
            owner,
            density: target,
            target,
            since: now,
        });
        if s.owner != owner || now < s.since {
            *s = DensityState {
                owner,
                density: target,
                target,
                since: now,
            };
        }
        if s.target != target {
            s.target = target;
            s.since = now;
        }
        for axis in 0..2 {
            s.density[axis] = s.density[axis].max(target[axis]);
        }
        if now - s.since >= 500.0 {
            s.density = target;
        }
        (s.density, s.density != target)
    }

    pub fn retain(&mut self, layers: &HashSet<usize>) {
        self.layers.retain(|id, _| layers.contains(id));
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SurfaceMetrics {
    pub logical_width: f32,
    pub logical_height: f32,
    pub surface_width: u32,
    pub surface_height: u32,
    pub dpr: [f32; 2],
    pub density_clamped: bool,
}

/// Size a backing at the projected display density, including densities below
/// one for minified content. Only a backend hard limit may reduce that density.
pub fn projected_surface_metrics(
    width: f64,
    height: f64,
    density: [f64; 2],
    limit: u32,
) -> SurfaceMetrics {
    let requested = [
        (width * density[0]).ceil().max(1.0),
        (height * density[1]).ceil().max(1.0),
    ];
    let backing = requested.map(|v| v.min(limit.max(1) as f64) as u32);
    installed_surface_metrics(
        width,
        height,
        backing,
        requested[0] > backing[0] as f64 || requested[1] > backing[1] as f64,
    )
}

/// Allocation consumes the already planned physical size; it must not apply
/// display scale a second time or reconstruct it from a different snapshot.
pub fn installed_surface_metrics(
    width: f64,
    height: f64,
    backing: [u32; 2],
    density_clamped: bool,
) -> SurfaceMetrics {
    SurfaceMetrics {
        logical_width: width as f32,
        logical_height: height as f32,
        surface_width: backing[0],
        surface_height: backing[1],
        dpr: [
            (backing[0] as f64 / width) as f32,
            (backing[1] as f64 / height) as f32,
        ],
        density_clamped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minification_is_not_a_quality_fallback() {
        let s = projected_surface_metrics(1000.0, 500.0, [0.5, 0.25], 4096);
        assert_eq!([s.surface_width, s.surface_height], [500, 125]);
        assert_eq!(s.dpr, [0.5, 0.25]);
        assert!(!s.density_clamped);
    }

    #[test]
    fn fractional_sizes_round_up_and_allocation_is_identical() {
        let s = projected_surface_metrics(100.1, 60.3, [1.25, 1.5], 4096);
        assert_eq!([s.surface_width, s.surface_height], [126, 91]);
        let allocated = installed_surface_metrics(
            100.1,
            60.3,
            [s.surface_width, s.surface_height],
            s.density_clamped,
        );
        assert_eq!(s.dpr, allocated.dpr);
    }

    #[test]
    fn hardware_limits_are_explicit() {
        let s = projected_surface_metrics(3000.0, 100.0, [2.0, 2.0], 4096);
        assert!(s.density_clamped);
        assert_eq!([s.surface_width, s.surface_height], [4096, 200]);
    }

    #[test]
    fn density_grows_without_delay_and_shrinks_only_after_stability() {
        let mut t = RasterDensityTracker::default();
        assert_eq!(t.observe(1, Some(2), [2.0; 2], 0.0), ([2.0; 2], false));
        assert_eq!(t.observe(1, Some(2), [1.0; 2], 10.0), ([2.0; 2], true));
        assert_eq!(t.observe(1, Some(2), [1.0; 2], 509.0), ([2.0; 2], true));
        assert_eq!(t.observe(1, Some(2), [1.0; 2], 510.0), ([1.0; 2], false));
        assert_eq!(t.observe(1, Some(2), [4.0; 2], 520.0), ([4.0; 2], false));
        assert_eq!(t.observe(1, Some(3), [0.5; 2], 530.0), ([0.5; 2], false));
    }

    #[test]
    fn floating_point_noise_does_not_increase_density() {
        let mut t = RasterDensityTracker::default();
        assert_eq!(
            t.observe(1, None, [2.0 + 1e-14, 2.0 - 1e-14], 0.0),
            ([2.0; 2], false)
        );
        let (density, _) = t.observe(1, None, [2.2; 2], 1.0);
        assert!(density[0] >= 2.2 && density[0] < 2.2 * 1.045);
        t.retain(&HashSet::new());
        assert_eq!(t.observe(1, None, [0.5; 2], 2.0), ([0.5; 2], false));
    }
}
