use bytemuck::Pod;
use bytemuck::Zeroable;
use lyon::geom::Point;
use wgpu::BufferUsages;
use wgpu::IndexFormat;

use crate::render_backend::RenderBackend;
use crate::Box2D;
use crate::Transform2D;
use wgpu::util::DeviceExt;
use wgpu::TextureFormat;

pub struct ImageResource {
    vertices_buffer: wgpu::Buffer,
    indices_buffer: wgpu::Buffer,
    texture_bind_group: wgpu::BindGroup,
    texture: wgpu::Texture,
}

impl ImageResource {
    fn new(
        backend: &RenderBackend,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        texture_sampler: &wgpu::Sampler,
        transform: &Transform2D,
        location: &Box2D,
        rgba: &[u8],
        rgba_width: u32,
    ) -> Self {
        let points = get_transformed_corners(&location, &transform);
        let height = rgba.len() as u32 / (rgba_width * 4);
        let vertices = [
            TextureVertex {
                position: points[0].to_array(),
                texture_coord: [0.0, 0.0],
            },
            TextureVertex {
                position: points[1].to_array(),
                texture_coord: [1.0, 0.0],
            },
            TextureVertex {
                position: points[2].to_array(),
                texture_coord: [0.0, 1.0],
            },
            TextureVertex {
                position: points[3].to_array(),
                texture_coord: [1.0, 1.0],
            },
        ];
        let vertices_buffer =
            backend
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                });
        let indices: &[u16] = &[1, 0, 2, 1, 2, 3];
        let indices_buffer = backend
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: BufferUsages::INDEX,
            });

        let size = wgpu::Extent3d {
            width: rgba_width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = backend.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Texture Creation"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        backend.queue.write_texture(
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

        let texture_bind_group = backend
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: backend.globals_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&texture_sampler),
                    },
                ],
                label: Some("texture_bind_group"),
            });

        Self {
            vertices_buffer,
            indices_buffer,
            texture_bind_group,
            texture,
        }
    }
}

pub struct TextureRenderer {
    texture_sampler: wgpu::Sampler,
    texture_pipeline: wgpu::RenderPipeline,
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl TextureRenderer {
    pub fn new(backend: &RenderBackend) -> Self {
        const TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
        let texture_shader = backend
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Texture Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../shaders/textures.wgsl").into(),
                ),
            });

        let texture_bind_group_layout =
            backend
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                    label: Some("texture_bind_group_layout"),
                });
        let texture_pipeline_layout =
            backend
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Texture Pipeline Layout"),
                    bind_group_layouts: &[&texture_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let texture_pipeline =
            backend
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Texture Pipeline"),
                    layout: Some(&texture_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &texture_shader,
                        entry_point: "vs_main",
                        buffers: &[TextureVertex::desc()],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &texture_shader,
                        entry_point: "fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: TEXTURE_FORMAT,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
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
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Stencil8,
                        depth_write_enabled: false,
                        depth_compare: wgpu::CompareFunction::Always,
                        stencil: wgpu::StencilState {
                            front: wgpu::StencilFaceState {
                                compare: wgpu::CompareFunction::Equal,
                                fail_op: wgpu::StencilOperation::Keep,
                                depth_fail_op: wgpu::StencilOperation::Keep,
                                pass_op: wgpu::StencilOperation::Keep,
                            },
                            back: wgpu::StencilFaceState::IGNORE,
                            read_mask: !0,
                            write_mask: !0,
                        },
                        bias: Default::default(),
                    }),
                    multisample: wgpu::MultisampleState {
                        count: 1,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview: None,
                    cache: None,
                });

        let texture_sampler = backend.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        Self {
            texture_sampler,
            texture_pipeline,
            texture_bind_group_layout,
        }
    }

    pub fn make_image(
        &self,
        backend: &RenderBackend,
        transform: &Transform2D,
        location: &Box2D,
        rgba: &[u8],
        rgba_width: u32,
    ) -> ImageResource {
        ImageResource::new(
            backend,
            &self.texture_bind_group_layout,
            &self.texture_sampler,
            transform,
            location,
            rgba,
            rgba_width,
        )
    }

    pub fn render_image(&self, render_pass: &mut wgpu::RenderPass, image: &ImageResource) {
        render_pass.set_pipeline(&self.texture_pipeline);
        render_pass.set_bind_group(0, &image.texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, image.vertices_buffer.slice(..));
        render_pass.set_index_buffer(image.indices_buffer.slice(..), IndexFormat::Uint16);
        render_pass.draw_indexed(0..6, 0, 0..1);
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

fn get_transformed_corners(box2d: &Box2D, transform: &Transform2D) -> [Point<f32>; 4] {
    let min = box2d.min;
    let max = box2d.max;

    // Get all 4 corners
    let corners = [
        Point::new(min.x, min.y), // Top-left
        Point::new(max.x, min.y), // Top-right
        Point::new(min.x, max.y), // Bottom-left
        Point::new(max.x, max.y), // Bottom-right
    ];

    // Transform each corner
    [
        transform.transform_point(corners[0]),
        transform.transform_point(corners[1]),
        transform.transform_point(corners[2]),
        transform.transform_point(corners[3]),
    ]
}
