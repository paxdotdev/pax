//! Cached, surface-local alpha masks. Paint is rasterized once per changed
//! source, feathered in two passes, then multiplied with the enclosing mask.
use super::data::GpuGlobals;
use bytemuck::{Pod, Zeroable};
use std::collections::{HashMap, HashSet};
use wgpu::util::DeviceExt;

pub(crate) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;

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
    pub paint: Paint,
}

pub(crate) struct MaskResource {
    signature: u64,
    size: [u32; 2],
    source: wgpu::TextureView,
    temporary: wgpu::TextureView,
    result: wgpu::TextureView,
    _result_texture: wgpu::Texture,
    pub bind_group: wgpu::BindGroup,
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
                uniform_entry(1, true, std::mem::size_of::<Paint>() as u64),
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
        let paint_pipeline = pipeline(device, &paint_layout, include_str!("alpha_mask.wgsl"), true);
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
            .map_or(0, |m| m.signature)
    }

    pub fn retain(&mut self, active: &HashSet<u32>) {
        self.masks.retain(|id, _| active.contains(id));
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        id: u32,
        signature: u64,
        draws: &[Draw],
        size: [u32; 2],
        globals: GpuGlobals,
        feather: f32,
        parent: Option<u32>,
    ) {
        if self
            .masks
            .get(&id)
            .is_some_and(|m| m.signature == signature && m.size == size)
        {
            return;
        }
        if !self.masks.get(&id).is_some_and(|m| m.size == size) {
            let source = texture(device, size).create_view(&Default::default());
            let temporary = texture(device, size).create_view(&Default::default());
            let result_texture = texture(device, size);
            let result = result_texture.create_view(&Default::default());
            let bind_group = sample_group(device, &self.layout, &result, &self.sampler);
            self.masks.insert(
                id,
                MaskResource {
                    signature: 0,
                    size,
                    source,
                    temporary,
                    result,
                    _result_texture: result_texture,
                    bind_group,
                },
            );
        }
        let target = &self.masks[&id];
        let mut vertices = Vec::<[f32; 2]>::new();
        let mut indices = Vec::<u32>::new();
        let mut ranges = Vec::new();
        let stride = (std::mem::size_of::<Paint>() as u64)
            .div_ceil(device.limits().min_uniform_buffer_offset_alignment as u64)
            * device.limits().min_uniform_buffer_offset_alignment as u64;
        let mut paints = vec![0u8; (stride as usize * draws.len()).max(stride as usize)];
        for (i, draw) in draws.iter().enumerate() {
            let base = vertices.len() as u32;
            vertices.extend(&draw.vertices);
            let start = indices.len() as u32;
            indices.extend(draw.indices.iter().map(|index| index + base));
            ranges.push(start..indices.len() as u32);
            let offset = i * stride as usize;
            let bytes = bytemuck::bytes_of(&draw.paint);
            paints[offset..offset + bytes.len()].copy_from_slice(bytes);
        }
        // Buffer allocations are bounded by current source geometry. Texture
        // allocations are reused across animation frames and released with owners.
        let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Alpha mask vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Alpha mask indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let ub = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Alpha mask paints"),
            contents: &paints,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let gb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Alpha mask globals"),
            contents: bytemuck::bytes_of(&globals),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Alpha mask paints"),
            layout: &self.paint_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: gb.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &ub,
                        offset: 0,
                        size: wgpu::BufferSize::new(std::mem::size_of::<Paint>() as u64),
                    }),
                },
            ],
        });
        {
            let mut pass = render_pass(encoder, &target.source);
            if !indices.is_empty() {
                pass.set_pipeline(&self.paint_pipeline);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                for (i, range) in ranges.iter().enumerate() {
                    pass.set_bind_group(0, &group, &[(i as u64 * stride) as u32]);
                    pass.draw_indexed(range.clone(), 0, 0..1);
                }
            }
        }
        let parent = parent
            .and_then(|id| self.masks.get(&id))
            .map_or(&self.white, |m| &m.result);
        for (source, destination, params) in [
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
        ] {
            let filter = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Alpha mask feather"),
                contents: bytemuck::cast_slice(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });
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
        self.masks.get_mut(&id).unwrap().signature = signature;
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
            paint: Paint {
                params: [alpha, 1.0, 0.0, 0.0],
                ..Default::default()
            },
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
        let globals = GpuGlobals {
            resolution: [64.0, 64.0],
            dpr: [1.0, 1.0],
        };
        let mut encoder = device.create_command_encoder(&Default::default());
        masks.render(
            &device,
            &mut encoder,
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
            2,
            2,
            &[square(0.0, 32.0, 1.0)],
            [64, 64],
            globals,
            3.0,
            None,
        );
        let mut gradient = square(0.0, 64.0, 1.0);
        gradient.paint.params = [0.0, 1.0, 2.0, 0.0];
        gradient.paint.axis = [0.0, 0.0, 64.0, 0.0];
        gradient.paint.stops[1] = [64.0, 1.0, 0.0, 0.0];
        masks.render(
            &device,
            &mut encoder,
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
            4,
            4,
            &[],
            [64, 64],
            globals,
            0.0,
            None,
        );
        queue.submit([encoder.finish()]);
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
