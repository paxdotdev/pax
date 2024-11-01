use crate::{
    render_backend::{
        data::{GpuMeshMetadata, GpuVertex},
        RenderBackend,
    },
    Fill, GradientType,
};
use lyon::tessellation::VertexBuffers;
use wgpu::util::DeviceExt;

pub struct MeshResource {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,

    bind_group: wgpu::BindGroup,
    _metadata_buffer: wgpu::Buffer,
}

impl MeshResource {
    pub(crate) fn new(
        device: &wgpu::Device,
        mesh_bind_group_layout: &wgpu::BindGroupLayout,
        geometry: &VertexBuffers<GpuVertex, u16>,
        fill: Fill,
    ) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh vertices"),
            contents: bytemuck::cast_slice(&geometry.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh indicies"),
            contents: bytemuck::cast_slice(&geometry.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let mesh_metadata = match fill {
            Fill::Solid(color) => GpuMeshMetadata {
                type_id: 0,
                colors: {
                    let mut colors = [[0.0; 4]; 8];
                    colors[0] = color.rgba;
                    colors
                },
                ..Default::default()
            },
            Fill::Gradient {
                gradient_type,
                pos,
                main_axis,
                off_axis,
                stops,
            } => {
                if stops.len() > 8 {
                    log::warn!("can't draw graidents with more than 8 stops. truncating.");
                }
                let len = stops.len().min(8);
                let mut colors_buff = [[0.0; 4]; 8];
                let mut stops_buff = [0.0; 8];
                for i in 0..len {
                    colors_buff[i] = stops[i].color.rgba;
                    stops_buff[i] = stops[i].stop;
                }
                //this should be filled in with custom gradient stuff later:
                GpuMeshMetadata {
                    type_id: match gradient_type {
                        GradientType::Linear => 1,
                        GradientType::Radial => 2,
                    },
                    position: pos.to_array(),
                    main_axis: main_axis.to_array(),
                    off_axis: off_axis.to_array(),
                    stop_count: len as u32,
                    colors: colors_buff,
                    stops: stops_buff,
                    _padding: [0; 16],
                }
            }
        };

        let metadata_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh metadata"),
            contents: bytemuck::cast_slice(&[mesh_metadata]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let mesh_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &mesh_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: metadata_buffer.as_entire_binding(),
            }],
            label: Some("mesh_bind_group"),
        });

        Self {
            vertex_buffer,
            index_buffer,
            index_count: geometry.indices.len() as u32,
            _metadata_buffer: metadata_buffer,
            bind_group: mesh_bind_group,
        }
    }
}

pub struct MeshRenderer {
    pipeline: wgpu::RenderPipeline,
    mesh_bind_group_layout: wgpu::BindGroupLayout,
    globals_bind_group: wgpu::BindGroup,
}

impl MeshRenderer {
    pub fn new(backend: &RenderBackend) -> Self {
        let mesh_bind_group_layout =
            backend
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                    label: Some("mesh_bind_group_layout"),
                });

        let globals_bind_group_layout =
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
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                    label: Some("globals_bind_group_layout"),
                });

        let pipeline = Self::create_pipeline(
            &backend.device,
            backend.surface_config.format,
            &globals_bind_group_layout,
            &mesh_bind_group_layout,
        );

        // TODO move this to backend itself? (and use in other render plugins as well)
        let globals_bind_group = backend
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &globals_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: backend.globals_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: backend.transform_buffer.as_entire_binding(),
                    },
                ],
                label: Some("globals_bind_group"),
            });

        Self {
            pipeline,
            mesh_bind_group_layout,
            globals_bind_group,
        }
    }

    fn create_pipeline(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        globals_bind_group_layout: &wgpu::BindGroupLayout,
        mesh_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&globals_bind_group_layout, &mesh_bind_group_layout],
                push_constant_ranges: &[],
            });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/geometry.wgsl").into()),
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[GpuVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
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
        })
    }

    pub(crate) fn make_mesh(
        &self,
        device: &wgpu::Device,
        geometry: &VertexBuffers<GpuVertex, u16>,
        fill: Fill,
    ) -> MeshResource {
        MeshResource::new(device, &self.mesh_bind_group_layout, geometry, fill)
    }

    pub(crate) fn render_meshes(
        &self,
        render_pass: &mut wgpu::RenderPass,
        meshes: &[MeshResource],
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.globals_bind_group, &[]);
        {
            for mesh in meshes {
                render_pass.set_bind_group(1, &mesh.bind_group, &[]);
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..mesh.index_count as u32, 0, 0..1);
            }
        }
    }
}
