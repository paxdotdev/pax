//! Opaque retained meshes share the canvas stencil but resolve mutual depth on GPU.
use super::data::GpuMaterial;
use bytemuck::{Pod, Zeroable};
use pax_runtime_api::mesh3::ConvexMesh;
use std::{
    collections::HashMap,
    rc::{Rc, Weak},
};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Uniform {
    pub world: [[f32; 4]; 4],
    pub surface: [[f32; 4]; 4],
    pub camera_clip: [[f32; 4]; 2],
    pub normal: [[f32; 4]; 3],
    pub color: [f32; 4],
    pub material: GpuMaterial,
    pub depth_range: [f32; 4],
    pub lights: [u32; 4],
}

struct Geometry {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
}
pub(crate) struct Resource {
    geometry: Rc<Geometry>,
    pub uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}
pub(crate) struct Renderer {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    geometry: HashMap<usize, Weak<Geometry>>,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, samples: u32) -> Self {
        let source = [
            include_str!("scene_lighting.wgsl"),
            include_str!("lighting.wgsl"),
            include_str!("solid.wgsl"),
        ]
        .join("\n");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("opaque solid"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("solid uniforms"),
            entries: &(0..3)
                .map(|binding| wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                })
                .collect::<Vec<_>>(),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("solid"),
            bind_group_layouts: &[&layout],
            immediate_size: 0,
        });
        let face = wgpu::StencilFaceState {
            compare: wgpu::CompareFunction::Equal,
            fail_op: wgpu::StencilOperation::Keep,
            depth_fail_op: wgpu::StencilOperation::Keep,
            pass_op: wgpu::StencilOperation::Keep,
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("opaque solid depth"), layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), compilation_options: Default::default(), buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
            }] },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_main"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: super::STENCIL_FORMAT, depth_write_enabled: true, depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState { front: face, back: face, read_mask: !0, write_mask: 0 }, bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState { count: samples, ..Default::default() }, multiview_mask: None, cache: None,
        });
        Self {
            pipeline,
            layout,
            geometry: HashMap::new(),
        }
    }

    pub fn resource(
        &mut self,
        device: &wgpu::Device,
        globals: &wgpu::Buffer,
        lighting: &wgpu::Buffer,
        mesh: &Rc<ConvexMesh>,
        uniform: &Uniform,
    ) -> (Resource, usize) {
        self.geometry.retain(|_, weak| weak.strong_count() != 0);
        let key = Rc::as_ptr(mesh) as usize;
        let mut uploaded = 0;
        let geometry = self
            .geometry
            .get(&key)
            .and_then(Weak::upgrade)
            .unwrap_or_else(|| {
                let vertices: Vec<_> = mesh
                    .vertices()
                    .iter()
                    .map(|v| Vertex {
                        position: v.position,
                        normal: v.normal,
                        uv: v.uv,
                    })
                    .collect();
                let vb = bytemuck::cast_slice(&vertices);
                let ib = bytemuck::cast_slice(mesh.indices());
                uploaded = vb.len() + ib.len();
                let geometry = Rc::new(Geometry {
                    vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("solid vertices"),
                        contents: vb,
                        usage: wgpu::BufferUsages::VERTEX,
                    }),
                    indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("solid indices"),
                        contents: ib,
                        usage: wgpu::BufferUsages::INDEX,
                    }),
                    count: mesh.indices().len() as u32,
                });
                self.geometry.insert(key, Rc::downgrade(&geometry));
                geometry
            });
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("solid pose/material"),
            contents: bytemuck::bytes_of(uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("solid"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globals.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: lighting.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffer.as_entire_binding(),
                },
            ],
        });
        (
            Resource {
                geometry,
                uniform: buffer,
                bind_group,
            },
            uploaded,
        )
    }

    pub fn draw<'a>(&'a self, resource: &'a Resource, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &resource.bind_group, &[]);
        pass.set_vertex_buffer(0, resource.geometry.vertices.slice(..));
        pass.set_index_buffer(
            resource.geometry.indices.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        pass.draw_indexed(0..resource.geometry.count, 0, 0..1);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn solid_shader_matches_uniform_layout() {
        let source = [
            include_str!("scene_lighting.wgsl"),
            include_str!("lighting.wgsl"),
            include_str!("solid.wgsl"),
        ]
        .join("\n");
        let module = naga::front::wgsl::parse_str(&source).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
        let ty = module
            .types
            .iter()
            .find(|(_, t)| t.name.as_deref() == Some("Solid"))
            .unwrap()
            .1;
        let naga::TypeInner::Struct { span, .. } = ty.inner else {
            panic!()
        };
        assert_eq!(span as usize, std::mem::size_of::<super::Uniform>());
    }
}

impl super::RenderBackend<'_> {
    pub(crate) fn create_solid(
        &mut self,
        mesh: &Rc<ConvexMesh>,
        uniform: &Uniform,
    ) -> (Resource, usize) {
        let renderer = self.solid_renderer.get_or_insert_with(|| {
            Renderer::new(&self.device, self.surface_config.format, self.sample_count)
        });
        renderer.resource(
            &self.device,
            &self.globals_buffer,
            &self.lighting_buffer,
            mesh,
            uniform,
        )
    }
    pub(crate) fn update_solid(&self, resource: &Resource, uniform: &Uniform) {
        self.queue
            .write_buffer(&resource.uniform, 0, bytemuck::bytes_of(uniform));
    }
    pub(crate) fn clear_solid_depth(&mut self) {
        let mut encoder = self.take_retained_encoder();
        let view = self.stencil_renderer.stencil_view_clone();
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("opaque solid depth clear"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        self.retained_encoder = Some(encoder);
    }
}
