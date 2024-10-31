use bytemuck::{Pod, Zeroable};
use lyon::tessellation::VertexBuffers;
use wgpu::util::DeviceExt;
use wgpu::{BufferUsages, Device, Queue, RenderPipeline};

pub struct StencilRenderer {
    stencil_pipeline: RenderPipeline,
    decrement_pipeline: RenderPipeline,
    vertices_buffer: wgpu::Buffer,
    indices_buffer: wgpu::Buffer,
    stencil_texture: wgpu::Texture,
    stencil_view: wgpu::TextureView,
    stencil_layer: u32,
    stencil_geometry_stack: Vec<VertexBuffers<Vertex, u16>>,
    width: u32,
    height: u32,
    stencil_bind_group: wgpu::BindGroup,
    _stencil_bind_group_layout: wgpu::BindGroupLayout,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
}

impl StencilRenderer {
    pub fn new(device: &Device, width: u32, height: u32, globals: &wgpu::Buffer) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Stencil Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("stencil.wgsl").into()),
        });

        let stencil_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("stencil_bind_group_layout"),
            });

        let stencil_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &stencil_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.as_entire_binding(),
            }],
            label: Some("stencil_bind_group"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Stencil Pipeline Layout"),
            bind_group_layouts: &[&stencil_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Main stencil pipeline for incrementing
        let stencil_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Stencil Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[None],
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
                        compare: wgpu::CompareFunction::Always,
                        fail_op: wgpu::StencilOperation::Keep,
                        depth_fail_op: wgpu::StencilOperation::Keep,
                        pass_op: wgpu::StencilOperation::IncrementClamp,
                    },
                    back: wgpu::StencilFaceState::IGNORE,
                    read_mask: !0,
                    write_mask: !0,
                },
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Decrement pipeline for popping
        let decrement_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Decrement Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[None],
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
                        compare: wgpu::CompareFunction::Always,
                        fail_op: wgpu::StencilOperation::Keep,
                        depth_fail_op: wgpu::StencilOperation::Keep,
                        pass_op: wgpu::StencilOperation::DecrementClamp,
                    },
                    back: wgpu::StencilFaceState::IGNORE,
                    read_mask: !0,
                    write_mask: !0,
                },
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let vertices = [Vertex {
            position: [0.0, 0.0],
        }; 1024];
        let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Stencil Vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        let indices: [u16; 1024] = [0; 1024];
        let indices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Stencil Indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
        });

        let (stencil_texture, stencil_view) = Self::create_stencil_texture(device, width, height);

        Self {
            stencil_pipeline,
            decrement_pipeline,
            vertices_buffer,
            indices_buffer,
            stencil_texture,
            stencil_view,
            width,
            height,
            stencil_layer: 0,
            stencil_geometry_stack: vec![],
            stencil_bind_group,
            _stencil_bind_group_layout: stencil_bind_group_layout,
        }
    }

    fn create_stencil_texture(
        device: &Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Stencil Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Stencil8,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    pub fn resize(&mut self, device: &Device, width: u32, height: u32) {
        if width == self.width && height == self.height {
            return;
        }

        (self.stencil_texture, self.stencil_view) =
            Self::create_stencil_texture(device, width, height);
        self.width = width;
        self.height = height;
    }

    pub fn push_stencil(
        &mut self,
        device: &Device,
        queue: &Queue,
        geometry: VertexBuffers<Vertex, u16>,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Stencil Encoder"),
        });

        queue.write_buffer(
            &self.vertices_buffer,
            0,
            bytemuck::cast_slice(&geometry.vertices),
        );
        queue.write_buffer(
            &self.indices_buffer,
            0,
            bytemuck::cast_slice(&geometry.indices),
        );

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Stencil Pass"),
                color_attachments: &[None],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.stencil_view,
                    depth_ops: None,
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.stencil_pipeline);
            render_pass.set_vertex_buffer(0, self.vertices_buffer.slice(..));
            render_pass.set_bind_group(0, &self.stencil_bind_group, &[]);
            render_pass.set_index_buffer(self.indices_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..(geometry.indices.len() as u32), 0, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));
        self.stencil_geometry_stack.push(geometry);
        self.stencil_layer += 1;
    }

    pub fn reset_stencil_depth_to(&mut self, device: &Device, queue: &Queue, depth: u32) {
        while self.stencil_layer > depth {
            let Some(geometry) = self.stencil_geometry_stack.pop() else {
                log::error!("geometry stack shouldn't be embty when stencil layer > 0");
                break;
            };
            queue.write_buffer(
                &self.vertices_buffer,
                0,
                bytemuck::cast_slice(&geometry.vertices),
            );
            queue.write_buffer(
                &self.indices_buffer,
                0,
                bytemuck::cast_slice(&geometry.indices),
            );

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Stencil Pop Encoder"),
            });

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Stencil Pop Pass"),
                    color_attachments: &[None],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.stencil_view,
                        depth_ops: None,
                        stencil_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                render_pass.set_pipeline(&self.decrement_pipeline);
                render_pass.set_bind_group(0, &self.stencil_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertices_buffer.slice(..));
                render_pass.set_bind_group(0, &self.stencil_bind_group, &[]);
                render_pass
                    .set_index_buffer(self.indices_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..(geometry.indices.len() as u32), 0, 0..1);
            }

            queue.submit(std::iter::once(encoder.finish()));
            self.stencil_layer = self.stencil_layer.saturating_sub(1);
        }
    }

    pub fn get_stencil(&self) -> (&wgpu::TextureView, u32) {
        (&self.stencil_view, self.stencil_layer)
    }

    pub fn clear(&mut self, device: &Device, queue: &Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Stencil Clear Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Stencil Clear Pass"),
                color_attachments: &[None],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.stencil_view,
                    depth_ops: None,
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.stencil_pipeline);
        }

        queue.submit(std::iter::once(encoder.finish()));
        self.stencil_layer = 0;
        self.stencil_geometry_stack.clear();
    }
}
