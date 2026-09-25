//! Cached, surface-local alpha masks. Paint is rasterized once per changed
//! source, feathered in two passes, then multiplied with the enclosing mask.
use super::{data::GpuGlobals, write_staged_buffer};
use bytemuck::{Pod, Zeroable};
use std::collections::{HashMap, HashSet};
use wgpu::util::StagingBelt;

pub(crate) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;

// Retain ordinary animation capacity, but release a burst's high-water mark
// once demand falls below a quarter of this threshold. This does not cap draws.
const RETAINED_BUFFER_BYTES: u64 = 1024 * 1024;

fn buffer_capacity(required: u64, current: u64, limit: u64) -> Option<u64> {
    let required = required.max(wgpu::COPY_BUFFER_ALIGNMENT);
    let limit = limit - limit % wgpu::COPY_BUFFER_ALIGNMENT;
    if required > limit || required % wgpu::COPY_BUFFER_ALIGNMENT != 0 {
        return None;
    }
    if current >= required
        && current <= limit
        && !(current > RETAINED_BUFFER_BYTES && required <= RETAINED_BUFFER_BYTES / 4)
    {
        return Some(current);
    }
    Some(
        required
            .checked_next_power_of_two()
            .unwrap_or(required)
            .min(limit),
    )
}

struct UploadBuffer {
    buffer: wgpu::Buffer,
    capacity: u64,
}

impl UploadBuffer {
    fn new(device: &wgpu::Device, label: &str, usage: wgpu::BufferUsages, required: u64) -> Self {
        let capacity = buffer_capacity(required, 0, device.limits().max_buffer_size)
            .expect("alpha mask upload sizes must be validated before allocation");
        Self {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: capacity,
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            capacity,
        }
    }

    fn resize(
        &mut self,
        device: &wgpu::Device,
        label: &str,
        usage: wgpu::BufferUsages,
        required: u64,
    ) -> bool {
        if buffer_capacity(required, self.capacity, device.limits().max_buffer_size)
            == Some(self.capacity)
        {
            return false;
        }
        // Drop our handle, never destroy: already encoded commands still own
        // their old buffer/bind group until WGPU retires that work.
        *self = Self::new(device, label, usage, required);
        true
    }
}

struct UploadLayout {
    vertex_bytes: u64,
    index_bytes: u64,
    paint_bytes: u64,
    term_bytes: u64,
    stride: u64,
}

impl UploadLayout {
    fn checked(draws: &[Draw], limits: &wgpu::Limits) -> Option<Self> {
        let mut vertices = 0u64;
        let mut indices = 0u64;
        for draw in draws {
            if draw
                .indices
                .iter()
                .any(|&index| index as usize >= draw.vertices.len())
            {
                return None;
            }
            vertices = vertices.checked_add(draw.vertices.len().try_into().ok()?)?;
            indices = indices.checked_add(draw.indices.len().try_into().ok()?)?;
        }
        if vertices > u32::MAX as u64 || indices > u32::MAX as u64 {
            return None;
        }
        let stride = 16u64.div_ceil(limits.min_uniform_buffer_offset_alignment as u64)
            * limits.min_uniform_buffer_offset_alignment as u64;
        let layout = Self {
            vertex_bytes: vertices.checked_mul(8)?,
            index_bytes: indices.checked_mul(4)?,
            paint_bytes: stride.checked_mul(draws.len().max(1).try_into().ok()?)?,
            term_bytes: (draws
                .iter()
                .try_fold(0usize, |sum, draw| sum.checked_add(draw.paints.len()))?
                .max(1) as u64)
                .checked_mul(std::mem::size_of::<Paint>() as u64)?,
            stride,
        };
        // Dynamic offsets are u32 even on platforms with larger buffer limits.
        if layout.paint_bytes > u32::MAX as u64
            || layout.term_bytes > limits.max_storage_buffer_binding_size as u64
        {
            return None;
        }
        for bytes in [
            layout.vertex_bytes,
            layout.index_bytes,
            layout.paint_bytes,
            layout.term_bytes,
        ] {
            usize::try_from(bytes).ok()?;
            buffer_capacity(bytes, 0, limits.max_buffer_size)?;
        }
        Some(layout)
    }
}

struct MaskBuffers {
    vertices: UploadBuffer,
    indices: UploadBuffer,
    paints: UploadBuffer,
    terms: UploadBuffer,
    globals: UploadBuffer,
    filters: [UploadBuffer; 2],
    paint_group: wgpu::BindGroup,
}

fn paint_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    globals: &wgpu::Buffer,
    paints: &wgpu::Buffer,
    terms: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Alpha mask paints"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: paints,
                    offset: 0,
                    size: wgpu::BufferSize::new(16),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: terms.as_entire_binding(),
            },
        ],
    })
}

impl MaskBuffers {
    fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, sizes: &UploadLayout) -> Self {
        use wgpu::BufferUsages as Usage;
        let paints = UploadBuffer::new(
            device,
            "Alpha mask paints",
            Usage::UNIFORM,
            sizes.paint_bytes,
        );
        let globals = UploadBuffer::new(device, "Alpha mask globals", Usage::UNIFORM, 16);
        let terms = UploadBuffer::new(
            device,
            "Alpha mask paint terms",
            Usage::STORAGE,
            sizes.term_bytes,
        );
        let paint_group = paint_group(
            device,
            layout,
            &globals.buffer,
            &paints.buffer,
            &terms.buffer,
        );
        Self {
            vertices: UploadBuffer::new(
                device,
                "Alpha mask vertices",
                Usage::VERTEX,
                sizes.vertex_bytes,
            ),
            indices: UploadBuffer::new(
                device,
                "Alpha mask indices",
                Usage::INDEX,
                sizes.index_bytes,
            ),
            paints,
            terms,
            globals,
            filters: std::array::from_fn(|_| {
                UploadBuffer::new(device, "Alpha mask feather", Usage::UNIFORM, 16)
            }),
            paint_group,
        }
    }

    fn resize(
        &mut self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sizes: &UploadLayout,
    ) {
        use wgpu::BufferUsages as Usage;
        self.vertices.resize(
            device,
            "Alpha mask vertices",
            Usage::VERTEX,
            sizes.vertex_bytes,
        );
        self.indices.resize(
            device,
            "Alpha mask indices",
            Usage::INDEX,
            sizes.index_bytes,
        );
        let terms_changed = self.terms.resize(
            device,
            "Alpha mask paint terms",
            Usage::STORAGE,
            sizes.term_bytes,
        );
        if self.paints.resize(
            device,
            "Alpha mask paints",
            Usage::UNIFORM,
            sizes.paint_bytes,
        ) || terms_changed
        {
            self.paint_group = paint_group(
                device,
                layout,
                &self.globals.buffer,
                &self.paints.buffer,
                &self.terms.buffer,
            );
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
pub(crate) struct Paint {
    pub stops: [[f32; 4]; 8],
    pub axis: [f32; 4],
    pub off_axis: [f32; 4],
    pub params: [f32; 4],
}

pub(crate) struct Draw {
    pub vertices: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub paints: Vec<Paint>,
}

pub(crate) struct MaskResource {
    signature: Option<u64>,
    size: [u32; 2],
    source: wgpu::TextureView,
    temporary: wgpu::TextureView,
    result: wgpu::TextureView,
    _result_texture: wgpu::Texture,
    pub bind_group: wgpu::BindGroup,
    buffers: MaskBuffers,
}

pub(crate) struct AlphaMasks {
    pub layout: wgpu::BindGroupLayout,
    pub white_bind_group: wgpu::BindGroup,
    white: wgpu::TextureView,
    sampler: wgpu::Sampler,
    paint_layout: wgpu::BindGroupLayout,
    filter_layout: wgpu::BindGroupLayout,
    paint_pipeline: wgpu::RenderPipeline,
    filter_pipeline: wgpu::RenderPipeline,
    masks: HashMap<u32, MaskResource>,
}

pub(crate) fn sampling_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Alpha mask sampling"),
        entries: &[texture_entry(0), sampler_entry(1)],
    })
}

fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            multisampled: false,
            view_dimension: wgpu::TextureViewDimension::D2,
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
        },
        count: None,
    }
}

fn sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn uniform_entry(binding: u32, dynamic: bool, size: u64) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: dynamic,
            min_binding_size: wgpu::BufferSize::new(size),
        },
        count: None,
    }
}

fn texture(device: &wgpu::Device, size: [u32; 2]) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Alpha mask surface"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

fn sample_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Alpha mask sampler"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    shader: &str,
    vertex: bool,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Alpha mask shader"),
        source: wgpu::ShaderSource::Wgsl(shader.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Alpha mask pipeline layout"),
        bind_group_layouts: &[layout],
        immediate_size: 0,
    });
    let attrs = wgpu::vertex_attr_array![0 => Float32x2];
    let vertex_layout = [wgpu::VertexBufferLayout {
        array_stride: 8,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &attrs,
    }];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Alpha mask pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: if vertex { &vertex_layout } else { &[] },
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: FORMAT,
                blend: vertex.then_some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}

impl AlphaMasks {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, layout: wgpu::BindGroupLayout) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Alpha mask linear sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let white_texture = texture(device, [1, 1]);
        queue.write_texture(
            white_texture.as_image_copy(),
            &[255],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(1),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let white = white_texture.create_view(&Default::default());
        let white_bind_group = sample_group(device, &layout, &white, &sampler);
        let paint_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Alpha mask paint layout"),
            entries: &[
                uniform_entry(0, false, 16),
                uniform_entry(1, true, 16),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let filter_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Alpha mask filter layout"),
            entries: &[
                texture_entry(0),
                sampler_entry(1),
                texture_entry(2),
                uniform_entry(3, false, 16),
            ],
        });
        let paint_pipeline = pipeline(
            device,
            &paint_layout,
            concat!(
                include_str!("radial_gradient.wgsl"),
                "\n",
                include_str!("alpha_mask.wgsl")
            ),
            true,
        );
        let filter_pipeline = pipeline(
            device,
            &filter_layout,
            include_str!("alpha_blur.wgsl"),
            false,
        );
        Self {
            layout,
            white_bind_group,
            white,
            sampler,
            paint_layout,
            filter_layout,
            paint_pipeline,
            filter_pipeline,
            masks: HashMap::new(),
        }
    }

    pub fn bind_group(&self, id: Option<u32>) -> &wgpu::BindGroup {
        id.and_then(|id| self.masks.get(&id))
            .map(|m| &m.bind_group)
            .unwrap_or(&self.white_bind_group)
    }

    pub fn signature(&self, id: Option<u32>) -> u64 {
        id.and_then(|id| self.masks.get(&id))
            .and_then(|m| m.signature)
            .unwrap_or(0)
    }

    pub fn invalidate(&mut self) {
        // Signatures describe encoded work. If its commands are abandoned,
        // even identical subsequent input must rasterize again.
        for mask in self.masks.values_mut() {
            mask.signature = None;
        }
    }

    pub fn retain(&mut self, active: &HashSet<u32>) {
        self.masks.retain(|id, _| active.contains(id));
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_belt: &mut StagingBelt,
        id: u32,
        signature: u64,
        draws: &[Draw],
        size: [u32; 2],
        globals: GpuGlobals,
        feather: f32,
        parent: Option<u32>,
    ) -> bool {
        if self
            .masks
            .get(&id)
            .is_some_and(|m| m.signature == Some(signature) && m.size == size)
        {
            return false;
        }
        // Invalid input must fail closed, including clearing old coverage.
        // Validate before allocating CPU/GPU storage or narrowing draw offsets.
        let limits = device.limits();
        let (draws, sizes) = match UploadLayout::checked(draws, &limits) {
            Some(sizes) => (draws, sizes),
            None => {
                log::error!("alpha mask upload exceeds device/index limits or has invalid indices");
                (
                    &[][..],
                    UploadLayout::checked(&[], &limits)
                        .expect("empty alpha mask fits device limits"),
                )
            }
        };
        if !self.masks.get(&id).is_some_and(|m| m.size == size) {
            let source = texture(device, size).create_view(&Default::default());
            let temporary = texture(device, size).create_view(&Default::default());
            let result_texture = texture(device, size);
            let result = result_texture.create_view(&Default::default());
            let bind_group = sample_group(device, &self.layout, &result, &self.sampler);
            self.masks.insert(
                id,
                MaskResource {
                    signature: None,
                    size,
                    source,
                    temporary,
                    result,
                    _result_texture: result_texture,
                    bind_group,
                    buffers: MaskBuffers::new(device, &self.paint_layout, &sizes),
                },
            );
        }
        self.masks
            .get_mut(&id)
            .unwrap()
            .buffers
            .resize(device, &self.paint_layout, &sizes);
        let target = &self.masks[&id];
        let mut vertices = Vec::<[f32; 2]>::with_capacity(sizes.vertex_bytes as usize / 8);
        let mut indices = Vec::<u32>::with_capacity(sizes.index_bytes as usize / 4);
        let mut ranges = Vec::with_capacity(draws.len());
        let stride = sizes.stride;
        let mut paints = vec![0u8; sizes.paint_bytes as usize];
        let mut terms = Vec::new();
        for (i, draw) in draws.iter().enumerate() {
            let base = vertices.len() as u32;
            vertices.extend(&draw.vertices);
            let start = indices.len() as u32;
            indices.extend(draw.indices.iter().map(|index| index + base));
            ranges.push(start..indices.len() as u32);
            let offset = i * stride as usize;
            let range = [terms.len() as u32, draw.paints.len() as u32, 0, 0];
            let bytes = bytemuck::bytes_of(&range);
            paints[offset..offset + bytes.len()].copy_from_slice(bytes);
            terms.extend_from_slice(&draw.paints);
        }
        if terms.is_empty() {
            terms.push(Paint::default());
        }
        let buffers = &target.buffers;
        // Copies belong to this command stream, not Queue::write_buffer's
        // pre-submit upload batch. A later version can safely reuse the same
        // destination even when both versions are submitted together.
        for (buffer, bytes) in [
            (&buffers.vertices.buffer, bytemuck::cast_slice(&vertices)),
            (&buffers.indices.buffer, bytemuck::cast_slice(&indices)),
            (&buffers.paints.buffer, paints.as_slice()),
            (&buffers.terms.buffer, bytemuck::cast_slice(&terms)),
            (&buffers.globals.buffer, bytemuck::bytes_of(&globals)),
        ] {
            write_staged_buffer(staging_belt, encoder, buffer, bytes);
        }
        {
            let mut pass = render_pass(encoder, &target.source);
            if !indices.is_empty() {
                pass.set_pipeline(&self.paint_pipeline);
                pass.set_vertex_buffer(0, buffers.vertices.buffer.slice(..sizes.vertex_bytes));
                pass.set_index_buffer(
                    buffers.indices.buffer.slice(..sizes.index_bytes),
                    wgpu::IndexFormat::Uint32,
                );
                for (i, range) in ranges.iter().enumerate() {
                    pass.set_bind_group(0, &buffers.paint_group, &[(i as u64 * stride) as u32]);
                    pass.draw_indexed(range.clone(), 0, 0..1);
                }
            }
        }
        let parent = parent
            .and_then(|id| self.masks.get(&id))
            .map_or(&self.white, |m| &m.result);
        for (index, (source, destination, params)) in [
            (
                &target.source,
                &target.temporary,
                [1.0 / size[0] as f32, 0.0, feather * globals.dpr[0], 0.0],
            ),
            (
                &target.temporary,
                &target.result,
                [0.0, 1.0 / size[1] as f32, feather * globals.dpr[1], 1.0],
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let filter = &buffers.filters[index].buffer;
            write_staged_buffer(staging_belt, encoder, filter, bytemuck::cast_slice(&params));
            // Do not retain texture-dependent filter groups: parent textures
            // can be replaced independently by resize/removal/recreation.
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Alpha mask filter"),
                layout: &self.filter_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(parent),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: filter.as_entire_binding(),
                    },
                ],
            });
            let mut pass = render_pass(encoder, destination);
            pass.set_pipeline(&self.filter_pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.masks.get_mut(&id).unwrap().signature = Some(signature);
        true
    }
}

fn render_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    view: &'a wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Alpha mask pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
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
    })
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::{
        future::Future,
        sync::Arc,
        task::{Context, Poll, Wake, Waker},
    };

    #[test]
    fn capacity_reuses_grows_and_releases_exceptional_high_water_marks() {
        let limit = 16 * RETAINED_BUFFER_BYTES;
        assert_eq!(buffer_capacity(0, 0, limit), Some(4));
        assert_eq!(buffer_capacity(80, 128, limit), Some(128));
        assert_eq!(buffer_capacity(132, 128, limit), Some(256));
        assert_eq!(buffer_capacity(0, 1024, limit), Some(1024));
        assert_eq!(
            buffer_capacity(1024, 4 * RETAINED_BUFFER_BYTES, limit),
            Some(1024)
        );
        assert_eq!(
            buffer_capacity(2 * RETAINED_BUFFER_BYTES, 4 * RETAINED_BUFFER_BYTES, limit),
            Some(4 * RETAINED_BUFFER_BYTES)
        );
        assert_eq!(buffer_capacity(260, 0, 300), Some(300));
        assert_eq!(buffer_capacity(304, 0, 300), None);
        assert_eq!(buffer_capacity(7, 0, limit), None);
        assert_eq!(
            buffer_capacity(u64::MAX - 3, 0, u64::MAX),
            Some(u64::MAX - 3)
        );
    }

    #[test]
    fn upload_layout_checks_ranges_and_device_limits_before_allocation() {
        let limits = wgpu::Limits::default();
        let sizes = UploadLayout::checked(&[square(0.0, 32.0, 1.0)], &limits).unwrap();
        assert_eq!(sizes.vertex_bytes, 32);
        assert_eq!(sizes.index_bytes, 24);
        assert_eq!(sizes.paint_bytes, sizes.stride);
        assert_eq!(
            sizes.stride % limits.min_uniform_buffer_offset_alignment as u64,
            0
        );
        let mut invalid = square(0.0, 32.0, 1.0);
        invalid.indices.push(4);
        assert!(UploadLayout::checked(&[invalid], &limits).is_none());
        let tiny = wgpu::Limits {
            max_buffer_size: 16,
            ..limits
        };
        assert!(UploadLayout::checked(&[square(0.0, 32.0, 1.0)], &tiny).is_none());
    }

    struct GpuRig {
        device: wgpu::Device,
        queue: wgpu::Queue,
        masks: AlphaMasks,
        belt: StagingBelt,
        commands: Vec<wgpu::CommandBuffer>,
    }

    impl GpuRig {
        fn new() -> Self {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
            let adapter =
                block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                    .expect("GPU adapter");
            let (device, queue) =
                block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                    .expect("GPU device");
            let masks = AlphaMasks::new(&device, &queue, sampling_layout(&device));
            let belt = StagingBelt::new(device.clone(), 4096);
            Self {
                device,
                queue,
                masks,
                belt,
                commands: Vec::new(),
            }
        }

        fn draw(
            &mut self,
            id: u32,
            signature: u64,
            draws: &[Draw],
            size: [u32; 2],
            feather: f32,
            parent: Option<u32>,
        ) -> bool {
            self.draw_view(
                id,
                signature,
                draws,
                size,
                feather,
                parent,
                GpuGlobals {
                    resolution: [size[0] as f32, size[1] as f32],
                    dpr: [1.0, 1.0],
                },
            )
        }

        fn draw_view(
            &mut self,
            id: u32,
            signature: u64,
            draws: &[Draw],
            size: [u32; 2],
            feather: f32,
            parent: Option<u32>,
            globals: GpuGlobals,
        ) -> bool {
            let mut encoder = self.device.create_command_encoder(&Default::default());
            let changed = self.masks.render(
                &self.device,
                &mut encoder,
                &mut self.belt,
                id,
                signature,
                draws,
                size,
                globals,
                feather,
                parent,
            );
            if changed {
                self.belt.finish();
                self.commands.push(encoder.finish());
            }
            changed
        }

        // Capture immediately in the command stream, before a subsequent
        // version overwrites the same mask texture or reallocates its buffers.
        fn capture(&mut self, id: u32) -> wgpu::Buffer {
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Alpha mask version snapshot"),
                size: 256 * 64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder = self.device.create_command_encoder(&Default::default());
            encoder.copy_texture_to_buffer(
                self.masks.masks[&id]._result_texture.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(64),
                    },
                },
                wgpu::Extent3d {
                    width: 64,
                    height: 64,
                    depth_or_array_layers: 1,
                },
            );
            self.commands.push(encoder.finish());
            buffer
        }

        fn submit(&mut self) {
            self.queue.submit(std::mem::take(&mut self.commands));
            self.belt.recall();
        }

        fn read(&self, buffer: &wgpu::Buffer) -> Vec<u8> {
            let (send, recv) = std::sync::mpsc::channel();
            buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    send.send(result).unwrap();
                });
            self.device
                .poll(wgpu::PollType::wait_indefinitely())
                .unwrap();
            recv.recv().unwrap().unwrap();
            let bytes = buffer.slice(..).get_mapped_range().to_vec();
            buffer.unmap();
            bytes
        }
    }

    #[test]
    #[ignore = "requires a native GPU adapter"]
    fn alpha_mask_gpu_blended_paint_is_one_source_shape() {
        let mut rig = GpuRig::new();
        let mut draw = square(0.0, 64.0, 0.0);
        // Weighted alpha is .5, whereas repeated source-over would be .4375.
        draw.paints = vec![
            Paint {
                params: [0.5, 0.5, 0.0, 0.0],
                ..Default::default()
            };
            2
        ];
        rig.draw(1, 1, &[draw], [64, 64], 0.0, None);
        let capture = rig.capture(1);
        rig.submit();
        let bytes = rig.read(&capture);
        assert!(bytes[32 * 256 + 32].abs_diff(128) <= 1);
    }

    #[test]
    #[ignore = "requires a native GPU adapter"]
    fn alpha_mask_gpu_viewport_updates_burst_shrink_and_abandoned_uploads() {
        let mut rig = GpuRig::new();
        let error_scope = rig.device.push_error_scope(wgpu::ErrorFilter::Validation);
        rig.draw(1, 0, &[square(0.0, 32.0, 1.0)], [64, 64], 0.0, None);
        let original = rig.capture(1);
        let globals = rig.masks.masks[&1].buffers.globals.buffer.clone();
        rig.draw_view(
            1,
            1,
            &[square(0.0, 32.0, 1.0)],
            [64, 64],
            0.0,
            None,
            GpuGlobals {
                resolution: [128.0, 64.0],
                dpr: [0.5, 1.0],
            },
        );
        let scaled = rig.capture(1);
        assert_eq!(globals, rig.masks.masks[&1].buffers.globals.buffer);

        let mut burst = square(0.0, 32.0, 1.0);
        burst
            .vertices
            .resize(RETAINED_BUFFER_BYTES as usize / 8 + 1, [0.0, 0.0]);
        rig.draw(2, 1, &[burst], [64, 64], 0.0, None);
        let large = rig.capture(2);
        assert!(rig.masks.masks[&2].buffers.vertices.capacity > RETAINED_BUFFER_BYTES);
        rig.draw(2, 2, &[], [64, 64], 0.0, None);
        let empty = rig.capture(2);
        assert_eq!(rig.masks.masks[&2].buffers.vertices.capacity, 4);
        rig.submit();
        let row = 32 * 256;
        assert_eq!(rig.read(&original)[row + 24], 255);
        assert_eq!(rig.read(&scaled)[row + 24], 0);
        assert_eq!(rig.read(&large)[row + 24], 255);
        assert!(rig.read(&empty).iter().all(|&alpha| alpha == 0));

        // Mirror surface resize abandoning unsubmitted encoders and their belt.
        rig.draw(1, 2, &[square(32.0, 64.0, 1.0)], [64, 64], 0.0, None);
        rig.commands.clear();
        rig.masks.invalidate();
        rig.belt = StagingBelt::new(rig.device.clone(), 4096);
        assert!(rig.draw(1, 2, &[square(32.0, 64.0, 1.0)], [64, 64], 0.0, None));
        let recovered = rig.capture(1);
        rig.submit();
        let recovered = rig.read(&recovered);
        assert_eq!((recovered[row + 8], recovered[row + 48]), (0, 255));
        assert!(block_on(error_scope.pop()).is_none());
    }

    #[test]
    #[ignore = "requires a native GPU adapter"]
    fn alpha_mask_gpu_reuse_preserves_versions_ranges_and_resource_lifetimes() {
        let mut rig = GpuRig::new();
        let error_scope = rig.device.push_error_scope(wgpu::ErrorFilter::Validation);
        rig.draw(1, 1, &[square(0.0, 32.0, 1.0)], [64, 64], 0.0, None);
        let first = rig.capture(1);
        let vb = rig.masks.masks[&1].buffers.vertices.buffer.clone();
        let ib = rig.masks.masks[&1].buffers.indices.buffer.clone();
        let ub = rig.masks.masks[&1].buffers.paints.buffer.clone();
        let group = rig.masks.masks[&1].buffers.paint_group.clone();
        rig.draw(1, 2, &[square(32.0, 64.0, 0.5)], [64, 64], 0.0, None);
        let second = rig.capture(1);
        assert_eq!(vb, rig.masks.masks[&1].buffers.vertices.buffer);
        assert_eq!(ib, rig.masks.masks[&1].buffers.indices.buffer);
        assert_eq!(ub, rig.masks.masks[&1].buffers.paints.buffer);
        assert_eq!(group, rig.masks.masks[&1].buffers.paint_group);
        assert!(
            !rig.draw(1, 2, &[], [64, 64], 0.0, None),
            "unchanged signatures stay cached"
        );

        let many: Vec<_> = (0..16).map(|_| square(0.0, 64.0, 1.0)).collect();
        rig.draw(1, 3, &many, [64, 64], 0.0, None);
        let grown = rig.capture(1);
        assert_ne!(vb, rig.masks.masks[&1].buffers.vertices.buffer);
        assert_ne!(ub, rig.masks.masks[&1].buffers.paints.buffer);
        assert_ne!(group, rig.masks.masks[&1].buffers.paint_group);
        let grown_vb = rig.masks.masks[&1].buffers.vertices.buffer.clone();
        rig.draw(1, 4, &[square(0.0, 16.0, 1.0)], [64, 64], 0.0, None);
        let shrunk = rig.capture(1);
        assert_eq!(grown_vb, rig.masks.masks[&1].buffers.vertices.buffer);
        rig.draw(1, 5, &[], [64, 64], 0.0, None);
        let empty = rig.capture(1);

        // Drop/recreate an owner while all earlier versions are still queued.
        rig.masks.retain(&HashSet::new());
        assert!(rig.masks.masks.is_empty());
        rig.draw(1, 5, &[square(48.0, 64.0, 1.0)], [64, 64], 0.0, None);
        let recreated = rig.capture(1);
        rig.draw(1, 6, &[square(0.0, 32.0, 1.0)], [128, 128], 0.0, None);
        let resized = rig.capture(1);
        rig.submit();
        let row = 32 * 256;
        let first = rig.read(&first);
        let second = rig.read(&second);
        assert_eq!((first[row + 8], first[row + 48]), (255, 0));
        assert_eq!(second[row + 8], 0);
        assert!((second[row + 48] as i32 - 128).abs() <= 2);
        assert_eq!(rig.read(&grown)[row + 48], 255);
        let shrunk = rig.read(&shrunk);
        assert_eq!((shrunk[row + 8], shrunk[row + 48]), (255, 0));
        assert!(rig.read(&empty).iter().all(|&alpha| alpha == 0));
        let recreated = rig.read(&recreated);
        assert_eq!((recreated[row + 8], recreated[row + 56]), (0, 255));
        let resized = rig.read(&resized);
        assert_eq!((resized[row + 8], resized[row + 48]), (255, 0));

        // Another submission exercises staging recall/reuse, plus changed
        // filter uniforms and current parent bindings after parent replacement.
        rig.draw(2, 1, &[square(0.0, 32.0, 0.5)], [64, 64], 0.0, None);
        rig.draw(3, 1, &[square(0.0, 64.0, 1.0)], [64, 64], 0.0, Some(2));
        let nested_left = rig.capture(3);
        rig.masks.retain(&HashSet::from([1, 3]));
        rig.draw(2, 2, &[square(32.0, 64.0, 0.75)], [64, 64], 0.0, None);
        rig.draw(3, 2, &[square(0.0, 64.0, 1.0)], [64, 64], 0.0, Some(2));
        let nested_right = rig.capture(3);
        rig.draw(4, 1, &[square(0.0, 32.0, 1.0)], [64, 64], 3.0, None);
        let soft = rig.capture(4);
        rig.draw(4, 2, &[square(0.0, 32.0, 1.0)], [64, 64], 0.0, None);
        let hard = rig.capture(4);
        let mut invalid = square(0.0, 64.0, 1.0);
        invalid.indices.push(99);
        rig.draw(4, 3, &[invalid], [64, 64], 0.0, None);
        let invalid = rig.capture(4);
        rig.submit();
        let left = rig.read(&nested_left);
        let right = rig.read(&nested_right);
        assert!((left[row + 8] as i32 - 128).abs() <= 2);
        assert_eq!(left[row + 48], 0);
        assert_eq!(right[row + 8], 0);
        assert!((right[row + 48] as i32 - 191).abs() <= 2);
        assert!(rig.read(&soft)[row + 34] > 10);
        assert_eq!(rig.read(&hard)[row + 34], 0);
        assert!(rig.read(&invalid).iter().all(|&alpha| alpha == 0));
        assert!(block_on(error_scope.pop()).is_none());
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        struct ThreadWake(std::thread::Thread);
        impl Wake for ThreadWake {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
        }
        let waker = Waker::from(Arc::new(ThreadWake(std::thread::current())));
        let mut context = Context::from_waker(&waker);
        let mut future = std::pin::pin!(future);
        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(value) => return value,
                Poll::Pending => std::thread::park(),
            }
        }
    }

    fn square(x0: f32, x1: f32, alpha: f32) -> Draw {
        Draw {
            vertices: vec![[x0, 0.0], [x1, 0.0], [x1, 64.0], [x0, 64.0]],
            indices: vec![0, 1, 2, 0, 2, 3],
            paints: vec![Paint {
                params: [alpha, 1.0, 0.0, 0.0],
                ..Default::default()
            }],
        }
    }

    fn read_mask(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        masks: &AlphaMasks,
        id: u32,
    ) -> Vec<u8> {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Alpha regression readback"),
            size: 256 * 64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            masks.masks[&id]._result_texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(64),
                },
            },
            wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        let (send, recv) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                send.send(result).unwrap()
            });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        recv.recv().unwrap().unwrap();
        let bytes = buffer.slice(..).get_mapped_range().to_vec();
        buffer.unmap();
        bytes
    }

    #[test]
    #[ignore = "requires a native GPU adapter"]
    fn alpha_mask_gpu_pixels_cover_gradients_overlap_feather_and_nesting() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("GPU adapter");
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("GPU device");
        let mut masks = AlphaMasks::new(&device, &queue, sampling_layout(&device));
        let mut belt = StagingBelt::new(device.clone(), 4096);
        let globals = GpuGlobals {
            resolution: [64.0, 64.0],
            dpr: [1.0, 1.0],
        };
        let mut encoder = device.create_command_encoder(&Default::default());
        masks.render(
            &device,
            &mut encoder,
            &mut belt,
            1,
            1,
            &[square(0.0, 48.0, 0.5), square(16.0, 64.0, 0.5)],
            [64, 64],
            globals,
            0.0,
            None,
        );
        masks.render(
            &device,
            &mut encoder,
            &mut belt,
            2,
            2,
            &[square(0.0, 32.0, 1.0)],
            [64, 64],
            globals,
            3.0,
            None,
        );
        let mut gradient = square(0.0, 64.0, 1.0);
        gradient.paints[0].params = [0.0, 1.0, 2.0, 0.0];
        gradient.paints[0].axis = [0.0, 0.0, 64.0, 0.0];
        gradient.paints[0].stops[1] = [64.0, 1.0, 0.0, 0.0];
        masks.render(
            &device,
            &mut encoder,
            &mut belt,
            3,
            3,
            &[gradient],
            [64, 64],
            globals,
            0.0,
            Some(1),
        );
        masks.render(
            &device,
            &mut encoder,
            &mut belt,
            4,
            4,
            &[],
            [64, 64],
            globals,
            0.0,
            None,
        );
        belt.finish();
        queue.submit([encoder.finish()]);
        belt.recall();
        let row = 32 * 256;
        let overlap = read_mask(&device, &queue, &masks, 1);
        assert!((overlap[row + 8] as i32 - 128).abs() <= 2);
        assert!((overlap[row + 32] as i32 - 192).abs() <= 2);
        let soft = read_mask(&device, &queue, &masks, 2);
        assert!(soft[row + 24] > 245 && soft[row + 40] < 5);
        assert!((90..170).contains(&soft[row + 31]));
        assert!(soft[row + 34] > 10 && soft[row + 34] < 100);
        let nested = read_mask(&device, &queue, &masks, 3);
        assert!((nested[row + 32] as i32 - 97).abs() <= 3);
        assert!(nested[row + 4] < nested[row + 24]);
        assert!(read_mask(&device, &queue, &masks, 4)
            .iter()
            .all(|a| *a == 0));
    }
}
