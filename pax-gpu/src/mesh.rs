//! Automatic row-connected bicubic patches, independent of consumer path tessellation.

/// Position followed by four premultiplied color channels. Extra lanes preserve WGSL alignment.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Control(pub [f32; 8]);

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Patch {
    /// Tensor-product Bernstein controls, horizontal coordinate varying fastest.
    pub controls: [Control; 16],
}

#[derive(Debug, PartialEq)]
pub(crate) enum MeshError {
    Dimensions,
    NonFinite,
    QualityLimit,
}

/// Convert uniform Hermite rows into Bernstein patches. Computing horizontal curves first and
/// applying the same operator vertically makes neighboring patches share boundary controls.
/// Explicit Bézier handles can later supply these controls without changing the rasterizer.
pub(crate) fn patches(rows: &[Vec<Control>]) -> Result<Vec<Patch>, MeshError> {
    let height = rows.len();
    let width = rows.first().map_or(0, Vec::len);
    if !(2..=8).contains(&width)
        || !(2..=8).contains(&height)
        || rows.iter().any(|row| row.len() != width)
    {
        return Err(MeshError::Dimensions);
    }
    if rows
        .iter()
        .flatten()
        .any(|p| p.0.iter().any(|v| !v.is_finite()))
    {
        return Err(MeshError::NonFinite);
    }
    let horizontal: Vec<Vec<[Control; 4]>> = rows
        .iter()
        .map(|row| (0..width - 1).map(|x| curve(row, x)).collect())
        .collect();
    let mut result = Vec::with_capacity((width - 1) * (height - 1));
    for y in 0..height - 1 {
        for x in 0..width - 1 {
            let mut controls = [Control([0.0; 8]); 16];
            for u in 0..4 {
                let column: Vec<_> = horizontal.iter().map(|row| row[x][u]).collect();
                for (v, control) in curve(&column, y).into_iter().enumerate() {
                    controls[v * 4 + u] = control;
                }
            }
            if controls.iter().any(|p| p.0.iter().any(|v| !v.is_finite())) {
                return Err(MeshError::NonFinite);
            }
            result.push(Patch { controls });
        }
    }
    Ok(result)
}

fn curve(points: &[Control], i: usize) -> [Control; 4] {
    let tangent = |index: usize, channel: usize| {
        let before = index.saturating_sub(1);
        let after = (index + 1).min(points.len() - 1);
        // f64 intermediates keep valid large f32 endpoints from overflowing on subtraction.
        (points[after].0[channel] as f64 - points[before].0[channel] as f64)
            / (after - before) as f64
    };
    let a = points[i];
    let b = points[i + 1];
    [
        a,
        Control(std::array::from_fn(|c| {
            (a.0[c] as f64 + tangent(i, c) / 3.0) as f32
        })),
        Control(std::array::from_fn(|c| {
            (b.0[c] as f64 - tangent(i + 1, c) / 3.0) as f32
        })),
        b,
    ]
}

impl Patch {
    #[cfg(test)]
    pub(crate) fn evaluate(&self, u: f64, v: f64) -> [f64; 8] {
        let bernstein = |t: f64| {
            let s = 1.0 - t;
            [s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t]
        };
        let bu = bernstein(u);
        let bv = bernstein(v);
        std::array::from_fn(|channel| {
            (0..4)
                .flat_map(|y| (0..4).map(move |x| (x, y)))
                .map(|(x, y)| bu[x] * bv[y] * self.controls[y * 4 + x].0[channel] as f64)
                .sum()
        })
    }

    /// Conservative per-channel second-derivative bound. Bernstein weights form a convex
    /// combination, so extrema of derivative controls bound the entire curved patch.
    fn curvature(&self, channel: usize) -> f64 {
        let p = |x: usize, y: usize| self.controls[y * 4 + x].0[channel] as f64;
        let mut uu = 0.0f64;
        let mut vv = 0.0f64;
        let mut uv = 0.0f64;
        for y in 0..4 {
            for x in 0..2 {
                uu = uu.max(6.0 * (p(x + 2, y) - 2.0 * p(x + 1, y) + p(x, y)).abs());
                vv = vv.max(6.0 * (p(y, x + 2) - 2.0 * p(y, x + 1) + p(y, x)).abs());
            }
        }
        for y in 0..3 {
            for x in 0..3 {
                uv = uv.max(9.0 * (p(x + 1, y + 1) - p(x + 1, y) - p(x, y + 1) + p(x, y)).abs());
            }
        }
        // Barycentric vertex offsets have per-axis variance <= h²/4; Cauchy-Schwarz
        // bounds their mixed absolute moment by h²/4 too. Taylor's 1/2 remainder
        // therefore gives (Muu + 2*Muv + Mvv) * h²/8 on either parameter triangle.
        (uu + 2.0 * uv + vv) / 8.0
    }
}

/// A shared power-of-two subdivision prevents cracks across every cell in a connected mesh.
/// Bounds apply to the unclamped field; clamping is nonexpansive. The caller supplies projected
/// pixels per local unit along each coordinate (including DPR and affine scale).
pub(crate) fn subdivision(
    patches: &[Patch],
    pixels_per_unit: [f64; 2],
    previous: u32,
) -> Result<u32, MeshError> {
    if pixels_per_unit.iter().any(|v| !v.is_finite() || *v < 0.0) {
        return Err(MeshError::NonFinite);
    }
    let mut ratio = 0.0f64;
    for patch in patches {
        // Triangle interpolation's Taylor remainder is bounded by curvature / n².
        let geometry =
            patch.curvature(0) * pixels_per_unit[0] + patch.curvature(1) * pixels_per_unit[1];
        ratio = ratio.max(geometry / 0.125);
        for channel in 2..6 {
            ratio = ratio.max(patch.curvature(channel) * 1024.0);
        }
    }
    let mut count = 1u32;
    while f64::from(count * count) < ratio && count < 128 {
        count *= 2;
    }
    if f64::from(count * count) < ratio {
        return Err(MeshError::QualityLimit);
    }
    // Keep the previous topology until demand falls below half its linear density.
    if previous.is_power_of_two() && previous <= 128 && count < previous && count * 2 >= previous {
        count = previous;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lattice(width: usize, height: usize) -> Vec<Vec<Control>> {
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| {
                        let x = x as f32;
                        let y = y as f32;
                        Control([x + 0.1 * y * y, y + 0.2 * x * x, x * y, x, y, 1.0, 0.0, 0.0])
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn anchors_and_shared_edges_and_derivatives() {
        for size in [2, 3, 8] {
            let rows = lattice(size, size);
            let patches = patches(&rows).unwrap();
            let width = size - 1;
            for y in 0..width {
                for x in 0..width {
                    let patch = &patches[y * width + x];
                    for (u, v) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        assert_eq!(
                            patch.evaluate(u as f64, v as f64),
                            rows[y + v][x + u].0.map(f64::from)
                        );
                    }
                    for i in 0..4 {
                        if x + 1 < width {
                            let next = &patches[y * width + x + 1];
                            assert_eq!(patch.controls[i * 4 + 3], next.controls[i * 4]);
                            for c in 0..6 {
                                let left =
                                    patch.controls[i * 4 + 3].0[c] - patch.controls[i * 4 + 2].0[c];
                                let right =
                                    next.controls[i * 4 + 1].0[c] - next.controls[i * 4].0[c];
                                assert!((left - right).abs() < 0.00001);
                            }
                        }
                        if y + 1 < width {
                            let next = &patches[(y + 1) * width + x];
                            assert_eq!(patch.controls[12 + i], next.controls[i]);
                            for c in 0..6 {
                                let above =
                                    patch.controls[12 + i].0[c] - patch.controls[8 + i].0[c];
                                let below = next.controls[4 + i].0[c] - next.controls[i].0[c];
                                assert!((above - below).abs() < 0.00001);
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn invalid_inputs_and_quality_cap_are_explicit() {
        for rows in [
            vec![],
            lattice(1, 2),
            lattice(2, 9),
            vec![vec![Control([0.0; 8]); 2], vec![]],
        ] {
            assert_eq!(patches(&rows), Err(MeshError::Dimensions));
        }
        let mut rows = lattice(3, 3);
        rows[1][1].0[0] = f32::NAN;
        assert_eq!(patches(&rows), Err(MeshError::NonFinite));
        let field = patches(&lattice(3, 3)).unwrap();
        assert_eq!(
            subdivision(&field, [1e9; 2], 0),
            Err(MeshError::QualityLimit)
        );
        let count = subdivision(&field, [1.0; 2], 0).unwrap();
        assert_eq!(subdivision(&field, [1.0; 2], count * 2).unwrap(), count * 2);
    }

    #[test]
    fn tessellation_error_is_bounded_between_vertices() {
        let field = patches(&lattice(3, 3)).unwrap();
        let n = subdivision(&field, [100.0; 2], 0).unwrap();
        for patch in field {
            for y in 0..n {
                for x in 0..n {
                    for (s, t) in [(0.2, 0.3), (0.8, 0.7), (0.3, 0.3)] {
                        let uv = |dx, dy| {
                            patch.evaluate((x as f64 + dx) / n as f64, (y as f64 + dy) / n as f64)
                        };
                        let exact = uv(s, t);
                        let (vertices, weights) = if s >= t {
                            (
                                [uv(0.0, 0.0), uv(1.0, 0.0), uv(1.0, 1.0)],
                                [1.0 - s, s - t, t],
                            )
                        } else {
                            (
                                [uv(0.0, 0.0), uv(1.0, 1.0), uv(0.0, 1.0)],
                                [1.0 - t, s, t - s],
                            )
                        };
                        for c in 0..6 {
                            let approx: f64 = (0..3).map(|i| weights[i] * vertices[i][c]).sum();
                            let limit = if c < 2 { 0.125 / 100.0 } else { 1.0 / 1024.0 };
                            assert!((exact[c] - approx).abs() <= limit);
                        }
                    }
                }
            }
        }
    }
}
