//! Bounded, retained bicubic paint textures shared by the vector and alpha-mask samplers.

use crate::mesh::{self, Control, MeshError, Patch};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};
use wgpu::util::DeviceExt;

const MAX_PATCHES: usize = 49;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct Domain {
    origin: [f32; 2],
    size: [f32; 2],
    subdivision: u32,
    padding: [u32; 3],
}

#[derive(Default, Debug, PartialEq)]
pub(crate) struct RasterStats {
    pub texture_allocations: u64,
    pub coefficient_upload_bytes: u64,
    pub raster_passes: u64,
    pub cache_hits: u64,
    pub topology_allocations: u64,
    pub texture_bytes: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct MeshInput {
    pub rows: Vec<Vec<Control>>,
    pub projected_axes: [[f32; 2]; 2],
    pub domain: [f32; 4],
}

impl MeshInput {
    pub(crate) fn new(rows: Vec<Vec<([f32; 2], crate::Color)>>, axes: [[f32; 2]; 2]) -> Self {
        let rows: Vec<Vec<_>> = rows
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|(p, color)| {
                        let [r, g, b, a] = color.rgba;
                        Control([p[0], p[1], r * a, g * a, b * a, a, 0.0, 0.0])
                    })
                    .collect()
            })
            .collect();
        let mut domain = [0.0, 0.0, 1.0, 1.0];
        if let Ok(field) = mesh::patches(&rows) {
            let mut min = [f32::INFINITY; 2];
            let mut max = [f32::NEG_INFINITY; 2];
            for p in field.iter().flat_map(|p| p.controls.iter()) {
                for axis in 0..2 {
                    min[axis] = min[axis].min(p.0[axis]);
                    max[axis] = max[axis].max(p.0[axis]);
                }
            }
            // The convex hull also covers curved overshoot outside the anchors' bounds, including
            // paint on the outer half of a stroke. Degenerate fields remain clear when rasterized.
            domain = [
                min[0],
                min[1],
                (max[0] - min[0]).max(f32::EPSILON),
                (max[1] - min[1]).max(f32::EPSILON),
            ];
        }
        Self {
            rows,
            projected_axes: axes,
            domain,
        }
    }
}

pub(crate) struct PaintPage {
    texture: wgpu::Texture,
    pub bind_group: wgpu::BindGroup,
    slots: Vec<Option<MeshTexture>>,
    key: PageKey,
}

#[derive(Clone, Default, PartialEq, Eq, Hash)]
struct PageKey {
    size: [u32; 3],
    // Exact float bits avoid approximate sharing of visibly different fields.
    fields: Vec<u32>,
}

impl PageKey {
    fn new(inputs: &[MeshInput], size: [u32; 3]) -> Self {
        let mut fields = Vec::new();
        for input in inputs {
            fields.push(input.rows.len() as u32);
            fields.push(input.rows.first().map_or(0, Vec::len) as u32);
            fields.extend(input.domain.map(f32::to_bits));
            for row in &input.rows {
                fields.push(row.len() as u32);
                fields.extend(row.iter().flat_map(|p| p.0.map(f32::to_bits)));
            }
        }
        Self { size, fields }
    }
}

const MAX_PAGE_BYTES: u64 = 64 * 1024 * 1024;

fn page_extent(
    inputs: &[MeshInput],
    dpr: [f32; 2],
    limits: &wgpu::Limits,
    previous: Option<[u32; 2]>,
) -> Result<[u32; 3], &'static str> {
    let layers = u32::try_from(inputs.len().max(1)).map_err(|_| "too many mesh paint terms")?;
    if layers > limits.max_texture_array_layers {
        return Err("mesh paint mixture exceeds the texture array layer limit");
    }
    let mut size = [1u32; 2];
    for input in inputs {
        for axis in 0..2 {
            let vector = input.projected_axes[axis];
            let projected = (vector[0] * dpr[0]).hypot(vector[1] * dpr[1]) * input.domain[axis + 2];
            if !projected.is_finite() || projected > limits.max_texture_dimension_2d as f32 {
                return Err("mesh paint resolution exceeds the device texture dimension");
            }
            let bucket = (projected.ceil() as u32).max(1).div_ceil(32) * 32;
            size[axis] = size[axis].max(bucket);
        }
    }
    if let Some(previous) = previous {
        for axis in 0..2 {
            if size[axis] <= previous[axis] && size[axis] * 2 > previous[axis] {
                size[axis] = previous[axis];
            }
        }
    }
    let bytes = u64::from(size[0]) * u64::from(size[1]) * u64::from(layers) * 4;
    if bytes > MAX_PAGE_BYTES {
        return Err("mesh paint page exceeds its 64 MiB retention budget");
    }
    Ok([size[0], size[1], layers])
}

pub(crate) fn sampling_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Mesh paint sampling"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

impl PaintPage {
    pub(crate) fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        size: [u32; 3],
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Retained mesh paint page"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: size[2],
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Mesh paint linear sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Retained mesh paint page sampling"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        Self {
            texture,
            bind_group,
            slots: Vec::new(),
            key: PageKey::default(),
        }
    }
}

pub(crate) struct MeshRasterizer {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    indices: RefCell<HashMap<u32, wgpu::Buffer>>,
    // Weak entries share only live fields; animation never retains historical textures.
    pages: RefCell<HashMap<PageKey, Weak<PaintPage>>>,
}

/// Owned by a retained paint slot, with content used only for invalidation, never cache identity.
pub(crate) struct MeshTexture {
    pub texture: wgpu::Texture,
    coefficients: wgpu::Buffer,
    domain_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    patches: Vec<Patch>,
    domain: Domain,
}

impl MeshRasterizer {
    pub(crate) fn prepare_page(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        layout: &wgpu::BindGroupLayout,
        page: &mut Option<Rc<PaintPage>>,
        inputs: &[MeshInput],
        dpr: [f32; 2],
        stats: &mut RasterStats,
    ) -> Result<(), String> {
        if inputs.is_empty() {
            *page = None;
            return Ok(());
        }
        let previous = page
            .as_ref()
            .map(|p| [p.texture.width(), p.texture.height()]);
        let size = page_extent(inputs, dpr, &device.limits(), previous)?;
        let key = PageKey::new(inputs, size);
        if page.as_ref().is_some_and(|p| p.key == key) {
            stats.cache_hits += inputs.len() as u64;
            return Ok(());
        }
        let initial_field = page.is_none();
        {
            let mut pages = self.pages.borrow_mut();
            pages.retain(|_, value| value.strong_count() > 0);
            // Share initial static fields. Once a slot changes, keep it private: otherwise
            // synchronized animated consumers would re-share and copy-on-write every frame.
            if initial_field {
                if let Some(shared) = pages.get(&key).and_then(Weak::upgrade) {
                    *page = Some(shared);
                    stats.cache_hits += inputs.len() as u64;
                    return Ok(());
                }
            }
        }
        // Validate the whole page before touching textures so a failed term never leaves a
        // partially updated crossfade. The caller discards the page on any error.
        for input in inputs {
            let field =
                mesh::patches(&input.rows).map_err(|e| format!("invalid mesh paint: {e:?}"))?;
            mesh::subdivision(
                &field,
                [
                    size[0] as f64 / input.domain[2] as f64,
                    size[1] as f64 / input.domain[3] as f64,
                ],
                0,
            )
            .map_err(|e| format!("mesh paint quality limit: {e:?}"))?;
        }
        // A shared field becomes private when one consumer starts animating. An unshared
        // slot keeps its coefficient buffers and texture through successive updates.
        let destination = page;
        let mut owned = destination.take().and_then(|p| Rc::try_unwrap(p).ok());
        let page = &mut owned;
        let resized = page.as_ref().is_none_or(|p| {
            p.texture.width() != size[0]
                || p.texture.height() != size[1]
                || p.texture.depth_or_array_layers() != size[2]
        });
        if resized {
            let slots = page.take().map_or_else(Vec::new, |p| p.slots);
            let mut next = PaintPage::new(device, layout, size);
            next.slots = slots;
            *page = Some(next);
            stats.texture_allocations += 1;
            stats.texture_bytes += u64::from(size[0]) * u64::from(size[1]) * u64::from(size[2]) * 4;
        }
        let page = page.as_mut().unwrap();
        page.slots.resize_with(inputs.len(), || None);
        for (i, (slot, input)) in page.slots.iter_mut().zip(inputs).enumerate() {
            self.rasterize_target(
                device,
                queue,
                encoder,
                slot,
                &input.rows,
                input.domain,
                [size[0], size[1]],
                stats,
                Some((&page.texture, i as u32, resized)),
            )
            .map_err(|e| format!("mesh paint raster: {e:?}"))?;
        }
        page.key = key.clone();
        let shared = Rc::new(owned.unwrap());
        if initial_field {
            self.pages.borrow_mut().insert(key, Rc::downgrade(&shared));
        }
        *destination = Some(shared);
        Ok(())
    }

    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Mesh patch coefficients"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mesh raster layout"),
            bind_group_layouts: &[&layout],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Bicubic mesh paint"),
            source: wgpu::ShaderSource::Wgsl(include_str!("mesh_paint.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Mesh paint replacement raster"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    // Self-overlap replaces all four premultiplied channels, including alpha.
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        Self {
            layout,
            pipeline,
            indices: RefCell::new(HashMap::new()),
            pages: RefCell::new(HashMap::new()),
        }
    }

    /// Encode only changed content, reusing coefficient buffers and texture for a stable slot.
    /// An invalid update returns an error; consumers must not draw a previous valid texture.
    #[cfg(test)]
    pub(crate) fn rasterize(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        slot: &mut Option<MeshTexture>,
        rows: &[Vec<Control>],
        bounds: [f32; 4],
        resolution: [u32; 2],
        stats: &mut RasterStats,
    ) -> Result<(), MeshError> {
        self.rasterize_target(
            device, queue, encoder, slot, rows, bounds, resolution, stats, None,
        )
    }

    fn rasterize_target(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        slot: &mut Option<MeshTexture>,
        rows: &[Vec<Control>],
        bounds: [f32; 4],
        resolution: [u32; 2],
        stats: &mut RasterStats,
        target: Option<(&wgpu::Texture, u32, bool)>,
    ) -> Result<(), MeshError> {
        if bounds.iter().any(|v| !v.is_finite())
            || bounds[2] <= 0.0
            || bounds[3] <= 0.0
            || resolution
                .iter()
                .any(|v| *v == 0 || *v > device.limits().max_texture_dimension_2d)
        {
            return Err(MeshError::Dimensions);
        }
        let patches = mesh::patches(rows)?;
        let domain = Domain {
            origin: [bounds[0], bounds[1]],
            size: [bounds[2], bounds[3]],
            subdivision: mesh::subdivision(
                &patches,
                [
                    resolution[0] as f64 / bounds[2] as f64,
                    resolution[1] as f64 / bounds[3] as f64,
                ],
                slot.as_ref().map_or(0, |s| s.domain.subdivision),
            )?,
            padding: [0; 3],
        };
        let resized = target.is_some_and(|(_, _, force)| force)
            || slot.as_ref().is_none_or(|s| {
                s.texture.width() != resolution[0] || s.texture.height() != resolution[1]
            });
        if !resized
            && slot
                .as_ref()
                .is_some_and(|s| s.patches == patches && s.domain == domain)
        {
            stats.cache_hits += 1;
            return Ok(());
        }
        if slot.is_none() {
            let coefficients = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Retained mesh coefficients"),
                size: (MAX_PATCHES * std::mem::size_of::<Patch>()) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let domain_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Retained mesh domain"),
                contents: bytemuck::bytes_of(&domain),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Retained mesh raster bindings"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: coefficients.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: domain_buffer.as_entire_binding(),
                    },
                ],
            });
            *slot = Some(MeshTexture {
                texture: target.map_or_else(
                    || texture(device, resolution),
                    |(texture, _, _)| texture.clone(),
                ),
                coefficients,
                domain_buffer,
                bind_group,
                patches: Vec::new(),
                domain,
            });
            if target.is_none() {
                stats.texture_allocations += 1;
            }
        } else if resized {
            slot.as_mut().unwrap().texture = target.map_or_else(
                || texture(device, resolution),
                |(texture, _, _)| texture.clone(),
            );
            if target.is_none() {
                stats.texture_allocations += 1;
            }
        }
        let slot = slot.as_mut().unwrap();
        if slot.patches != patches {
            let bytes = bytemuck::cast_slice(&patches);
            queue.write_buffer(&slot.coefficients, 0, bytes);
            stats.coefficient_upload_bytes += bytes.len() as u64;
        }
        if slot.domain != domain {
            queue.write_buffer(&slot.domain_buffer, 0, bytemuck::bytes_of(&domain));
        }
        let view = slot.texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2),
            base_array_layer: target.map_or(0, |(_, layer, _)| layer),
            array_layer_count: Some(1),
            ..Default::default()
        });
        let mut indices = self.indices.borrow_mut();
        let index_buffer = indices.entry(domain.subdivision).or_insert_with(|| {
            let n = domain.subdivision;
            let mut values = Vec::with_capacity((6 * n * n) as usize);
            for y in 0..n {
                for x in 0..n {
                    let a = (y * (n + 1) + x) as u16;
                    let b = a + 1;
                    let d = a + n as u16 + 1;
                    let c = d + 1;
                    values.extend_from_slice(&[a, b, c, a, c, d]);
                }
            }
            stats.topology_allocations += 1;
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Shared mesh parameter topology"),
                contents: bytemuck::cast_slice(&values),
                usage: wgpu::BufferUsages::INDEX,
            })
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Retained mesh paint raster"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &slot.bind_group, &[]);
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(
                0..6 * domain.subdivision * domain.subdivision,
                0,
                0..patches.len() as u32,
            );
        }
        slot.patches = patches;
        slot.domain = domain;
        stats.raster_passes += 1;
        Ok(())
    }
}

fn texture(device: &wgpu::Device, resolution: [u32; 2]) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Retained mesh paint"),
        size: wgpu::Extent3d {
            width: resolution[0],
            height: resolution[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

#[cfg(test)]
mod tests;
