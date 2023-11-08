use bytemuck::Pod;
use bytemuck::Zeroable;
use wgpu::BufferUsages;
use wgpu::IndexFormat;
use wgpu::{util::RenderEncoder, BindGroupLayout, RenderPass};

use crate::Box2D;
use wgpu::util::DeviceExt;
use wgpu::TextureFormat;

pub struct TextureRenderer {
    vertices_buffer: wgpu::Buffer,
    indices_buffer: wgpu::Buffer,

    texture_sampler: wgpu::Sampler,
    texture_pipeline: wgpu::RenderPipeline,
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl TextureRenderer {
    pub fn new(device: &wgpu::Device) -> Self {
        const TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
        let texture_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Texture Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("textures.wgsl").into()),
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
                label: Some("texture_bind_group_layout"),
            });
        let texture_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Texture Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let texture_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Texture Pipeline"),
            layout: Some(&texture_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &texture_shader,
                entry_point: "vs_main",
                buffers: &[TextureVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &texture_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: TEXTURE_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let vertices = [TextureVertex::default(); 6];
        let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        let indices: &[u16] = &[0, 1, 2, 1, 2, 3];
        let indices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: BufferUsages::INDEX,
        });
        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        Self {
            vertices_buffer,
            indices_buffer,
            texture_sampler,
            texture_pipeline,
            texture_bind_group_layout,
        }
    }

    pub fn render_image(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        rgba: &[u8],
        rgba_width: u32,
        location: Box2D,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Texture Encoder"),
        });
        let height = rgba.len() as u32 / (rgba_width * 4);
        let verts = [
            TextureVertex {
                position: [location.min.x, location.min.y],
                texture_coord: [0.0, 0.0],
            },
            TextureVertex {
                position: [location.max.x, location.min.y],
                texture_coord: [1.0, 0.0],
            },
            TextureVertex {
                position: [location.min.x, location.max.y],
                texture_coord: [0.0, 1.0],
            },
            TextureVertex {
                position: [location.max.x, location.max.y],
                texture_coord: [1.0, 1.0],
            },
        ];
        queue.write_buffer(&self.vertices_buffer, 0, bytemuck::cast_slice(&verts));

        let size = wgpu::Extent3d {
            width: rgba_width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Texture Creation"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * rgba_width),
                rows_per_image: Some(height),
            },
            size,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.texture_sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        {
            //write image in render pass
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Texture Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&self.texture_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertices_buffer.slice(..));
            render_pass.set_index_buffer(self.indices_buffer.slice(..), IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..1);
        }
        queue.submit(std::iter::once(encoder.finish()));
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub(crate) struct TextureVertex {
    pub position: [f32; 2],
    pub texture_coord: [f32; 2],
}

impl TextureVertex {
    pub(crate) fn desc() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBS: [wgpu::VertexAttribute; 2] =
            wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TextureVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBS,
        }
    }
}
