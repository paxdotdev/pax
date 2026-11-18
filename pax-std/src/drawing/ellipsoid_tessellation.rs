use std::f64::consts::PI;

pub(super) const SEGMENTS: [usize; 6] = [8, 16, 24, 32, 48, 64];
pub(super) const FINEST: usize = SEGMENTS.len() - 1;
const MAX_ERROR: f64 = 0.25;

fn error(radius: f64, level: usize) -> f64 {
    // Both UV axes have angular step h=2π/n. The sphere parameterization's
    // second derivative has norm at most 2; linear interpolation over each
    // right UV triangle therefore has error at most h²/2. The affine map's
    // Frobenius norm bounds its amplification, including shear and local Z.
    2.0 * (PI / SEGMENTS[level] as f64).powi(2) * radius
}

pub(super) fn level(matrix: [f64; 16], previous: Option<usize>) -> usize {
    if !matrix.iter().all(|v| v.is_finite())
        || matrix[3] != 0.0
        || matrix[7] != 0.0
        || matrix[11] != 0.0
        || matrix[15] != 1.0
    {
        return FINEST;
    }
    let radius = (0..3)
        .flat_map(|c| (0..3).map(move |r| matrix[c * 4 + r].powi(2)))
        .sum::<f64>()
        .sqrt();
    let choose = |budget| {
        (0..FINEST)
            .find(|&level| error(radius, level) <= budget)
            .unwrap_or(FINEST)
    };
    if let Some(previous) = previous {
        if error(radius, previous) <= MAX_ERROR || previous == FINEST {
            // Refine immediately when required, but coarsen only with headroom.
            // A stable-sized rotating ellipsoid keeps its retained GPU topology.
            return previous.min(choose(MAX_ERROR * 0.5));
        }
    }
    choose(MAX_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scale(radius: f64) -> [f64; 16] {
        [
            radius, 0., 0., 0., 0., radius, 0., 0., 0., 0., radius, 0., 0., 0., 0., 1.,
        ]
    }

    #[test]
    fn small_solids_reduce_work_with_a_conservative_error_bound() {
        for radius in [0.1, 1., 3., 8., 20., 80., 400.] {
            let selected = level(scale(radius), None);
            assert!(selected == FINEST || error(3_f64.sqrt() * radius, selected) <= MAX_ERROR);
        }
        assert!(SEGMENTS[level(scale(8.), None)] <= 48);
        assert_eq!(level(scale(400.), None), FINEST);
    }

    #[test]
    fn translation_and_rigid_rotation_keep_the_level() {
        let mut matrix = scale(8.);
        let selected = level(matrix, None);
        for angle in 0..360 {
            let (sin, cos) = (angle as f64).to_radians().sin_cos();
            matrix[0] = 8. * cos;
            matrix[2] = 8. * sin;
            matrix[8] = -8. * sin;
            matrix[10] = 8. * cos;
            matrix[12] = angle as f64 * 10.;
            assert_eq!(level(matrix, Some(selected)), selected);
        }
    }

    #[test]
    fn hysteresis_refines_before_exceeding_budget_without_threshold_chatter() {
        let threshold = MAX_ERROR / (2. * (PI / 16.).powi(2)) / 3_f64.sqrt();
        let coarse = level(scale(threshold * 0.99), None);
        let fine = level(scale(threshold * 1.01), Some(coarse));
        assert!(fine > coarse);
        assert_eq!(level(scale(threshold * 0.99), Some(fine)), fine);
        assert!(level(scale(threshold * 0.4), Some(fine)) <= coarse);
    }

    #[test]
    fn invalid_and_projective_maps_keep_existing_full_detail() {
        let mut matrix = scale(1.);
        matrix[11] = 0.1;
        assert_eq!(level(matrix, None), FINEST);
        matrix[11] = f64::NAN;
        assert_eq!(level(matrix, None), FINEST);
    }

    #[test]
    fn tessellated_surface_stays_within_the_error_bound_under_shear() {
        let matrix = [
            8., 1., 0., 0., 3., 5., 0., 0., 2., 0., 4., 0., 100., 200., 0., 1.,
        ];
        let n = SEGMENTS[level(matrix, None)];
        let point =
            |theta: f64, phi: f64| [phi.sin() * theta.cos(), -phi.cos(), phi.sin() * theta.sin()];
        let step = 2. * PI / n as f64;
        for y in 0..n / 2 {
            for x in 0..n {
                let a = [x as f64 * step, y as f64 * step];
                for corners in [
                    [a, [a[0] + step, a[1]], [a[0], a[1] + step]],
                    [
                        [a[0] + step, a[1]],
                        [a[0] + step, a[1] + step],
                        [a[0], a[1] + step],
                    ],
                ] {
                    for weights in [
                        [0.5, 0.5, 0.],
                        [0., 0.5, 0.5],
                        [0.5, 0., 0.5],
                        [0.2, 0.3, 0.5],
                    ] {
                        let uv: [f64; 2] = std::array::from_fn(|i| {
                            (0..3).map(|j| corners[j][i] * weights[j]).sum()
                        });
                        let exact = point(uv[0], uv[1]);
                        let vertices = corners.map(|uv| point(uv[0], uv[1]));
                        let delta: [f64; 3] = std::array::from_fn(|i| {
                            exact[i] - (0..3).map(|j| vertices[j][i] * weights[j]).sum::<f64>()
                        });
                        let distance = (0..3)
                            .map(|r| {
                                (0..3)
                                    .map(|c| matrix[c * 4 + r] * delta[c])
                                    .sum::<f64>()
                                    .powi(2)
                            })
                            .sum::<f64>()
                            .sqrt();
                        assert!(distance <= MAX_ERROR, "{distance}");
                    }
                }
            }
        }
    }
}
