//! A draw-local table of values shared by pixels that visit the same logical cell.
use super::effects::EffectDomain;
use super::*;
use pax_runtime_api::{ShaderParameterType, ShaderPass, ShaderValue};

pub(super) fn declaration(pass: &ShaderPass) -> Option<(usize, u32)> {
    pass.parameters
        .iter()
        .enumerate()
        .find_map(|(i, p)| match p.ty {
            ShaderParameterType::CellSpacing { padding } => Some((i, padding)),
            _ => None,
        })
}

pub(super) struct CellGrid {
    pub origin: [i32; 2],
    pub size: [u32; 2],
}

impl CellGrid {
    pub fn for_pass(
        pass: &ShaderPass,
        values: &[ShaderValue],
        domain: &EffectDomain,
        limit: u32,
    ) -> Option<Self> {
        let (slot, padding) = declaration(pass)?;
        if limit == 0 {
            return None;
        }
        let ShaderValue::Float(spacing) = values[slot] else {
            return None;
        };
        let [x, y, width, height] = domain[0];
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        // A homography with positive w over the rectangle reaches its extrema
        // at the corners. Across a horizon there is no bounded local rectangle.
        for (x, y) in [
            (x, y),
            (x + width, y),
            (x, y + height),
            (x + width, y + height),
        ] {
            let dot = |row: usize| {
                f64::from(domain[row][0]) * f64::from(x)
                    + f64::from(domain[row][1]) * f64::from(y)
                    + f64::from(domain[row][2])
            };
            let w = dot(3);
            if w <= 0. || !w.is_finite() {
                return None;
            }
            for axis in 0..2 {
                let cell = (dot(axis + 1) / w / f64::from(spacing)).floor();
                // Outside f32's exact-integer range, evaluate the shader directly.
                if !cell.is_finite() || cell.abs() > 8_388_608.0 {
                    return None;
                }
                lo[axis] = lo[axis].min(cell);
                hi[axis] = hi[axis].max(cell);
            }
        }
        let guard = f64::from(padding) + 1.;
        let extent = [0, 1].map(|i| hi[i] - lo[i] + 2. * guard + 1.);
        if extent.iter().any(|v| *v > f64::from(limit)) {
            return None;
        }
        let size = extent.map(|n| n as u32);
        let pixels = domain[9][..2]
            .iter()
            .map(|&v| u64::from((v as u32).div_ceil(pass.downsample)))
            .product::<u64>();
        // Under minification the cell table can cost more than the output itself.
        if u64::from(size[0]) * u64::from(size[1]) * 4 > pixels {
            return None;
        }
        Some(Self {
            origin: lo.map(|v| (v - guard) as i32),
            size,
        })
    }
}

pub(super) const FORMAT: TextureFormat = TextureFormat::Rgba32Float;
pub(super) const USAGE: TextureUsages =
    TextureUsages::RENDER_ATTACHMENT.union(TextureUsages::TEXTURE_BINDING);

pub(super) struct CellTexture {
    pub view: TextureView,
    pub binding: BindGroup,
    size: [u32; 2],
}

impl CellTexture {
    pub fn bytes(&self) -> u64 {
        Self::size_bytes(self.size)
    }
    fn size_bytes(size: [u32; 2]) -> u64 {
        u64::from(size[0]) * u64::from(size[1]) * 16
    }

    pub fn ensure(
        slot: &mut Option<Self>,
        device: &wgpu::Device,
        layout: &BindGroupLayout,
        size: [u32; 2],
        available: u64,
        stats: &mut crate::ResourceChurnStats,
    ) -> bool {
        if slot
            .as_ref()
            .is_some_and(|s| s.size[0] >= size[0] && s.size[1] >= size[1] && s.bytes() <= available)
        {
            return true;
        }
        if Self::size_bytes(size) > available {
            return false;
        }
        let rounded = size.map(|n| {
            n.next_multiple_of(16)
                .min(device.limits().max_texture_dimension_2d)
        });
        let size = if Self::size_bytes(rounded) <= available {
            rounded
        } else {
            size
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Effect cell field"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: USAGE,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Effect cell lookup"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
        });
        stats.effect_texture_allocations += 1;
        stats.effect_texture_bytes += Self::size_bytes(size);
        *slot = Some(Self {
            view,
            binding,
            size,
        });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pax_runtime_api::{ShaderInput, ShaderParameter};

    #[test]
    fn projected_cell_bounds_include_negative_neighbors_and_reject_unbounded_work() {
        let pass = ShaderPass {
            label: "cells",
            source: "",
            inputs: [ShaderInput::Subtree; 2],
            downsample: 1,
            parameters: &[ShaderParameter {
                name: "spacing",
                ty: ShaderParameterType::CellSpacing { padding: 2 },
            }],
        };
        let mut domain = EffectDomain::default();
        domain[0] = [-30., -18., 100., 48.];
        domain[1][0] = 1.;
        domain[2][1] = 1.;
        domain[3][2] = 1.;
        domain[9] = [200., 96., 256., 128.];
        let grid = |d: &EffectDomain, limit| {
            CellGrid::for_pass(&pass, &[ShaderValue::Float(8.)], d, limit)
        };
        let g = grid(&domain, 1024).unwrap();
        assert_eq!(g.origin, [-7, -6]);
        assert_eq!(g.size, [19, 13]);
        assert!(grid(&domain, 16).is_none());
        domain[3][0] = 0.05;
        assert!(grid(&domain, 1024).is_none(), "projection crosses w = 0");
        domain[3][0] = 0.;
        domain[1][0] = 100.;
        assert!(
            grid(&domain, 4096).is_none(),
            "minification makes direct evaluation cheaper"
        );
    }
}
