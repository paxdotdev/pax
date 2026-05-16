use bytemuck::{Pod, Zeroable};
use lyon::tessellation::VertexBuffers;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use wgpu::util::DeviceExt;
use wgpu::{BufferUsages, Device, Queue, RenderPipeline};

fn padded_u16_buffer(data: &[u16]) -> Vec<u16> {
    if data.len() % 2 == 0 {
        return data.to_vec();
    }

    let mut padded = Vec::with_capacity(data.len() + 1);
    padded.extend_from_slice(data);
    padded.push(0);
    padded
}

struct CachedStencilGeometry {
    vertices_buffer: wgpu::Buffer,
    indices_buffer: wgpu::Buffer,
    index_count: u32,
}

#[derive(Clone, Copy, Debug)]
struct StencilStackEntry {
    clip_id: u32,
    geometry_signature: u64,
}

#[derive(Default)]
pub(crate) struct PreparedStencilSync {
    popped: Vec<StencilStackEntry>,
    pushed: Vec<StencilStackEntry>,
}

impl PreparedStencilSync {
    pub(crate) fn is_empty(&self) -> bool {
        self.popped.is_empty() && self.pushed.is_empty()
    }
}

/// One clip geometry instance to draw into the stencil buffer.
pub struct ClipDraw<'a> {
    pub clip_id: u32,
    pub geometry_signature: u64,
    pub geometry: &'a VertexBuffers<Vertex, u16>,
}

/// Maintains the stencil stack used to render nested vector clips.
pub struct StencilRenderer {
    stencil_pipeline: Rc<RenderPipeline>,
    decrement_pipeline: Rc<RenderPipeline>,
    color_stencil_pipeline: Rc<RenderPipeline>,
    color_decrement_pipeline: Rc<RenderPipeline>,
    stencil_texture: wgpu::Texture,
    stencil_view: wgpu::TextureView,
    stencil_layer: u32,
    stencil_geometry_stack: Vec<StencilStackEntry>,
    cached_geometry: HashMap<u64, CachedStencilGeometry>,
    cached_clip_instances: HashMap<u32, wgpu::Buffer>,
    width: u32,
    height: u32,
    sample_count: u32,
    stencil_bind_group: wgpu::BindGroup,
}

#[derive(Clone)]
pub(crate) struct StencilPipelineResources {
    stencil_pipeline: Rc<RenderPipeline>,
    decrement_pipeline: Rc<RenderPipeline>,
    color_stencil_pipeline: Rc<RenderPipeline>,
    color_decrement_pipeline: Rc<RenderPipeline>,
    stencil_bind_group_layout: Rc<wgpu::BindGroupLayout>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
/// Tessellated stencil vertex.
pub struct Vertex {
    pub position: [f32; 2],
}

impl StencilRenderer {
    /// Create the stencil pipelines and backing texture.
    #[allow(dead_code)]
    pub fn new(
        device: &Device,
        width: u32,
        height: u32,
        sample_count: u32,
        color_format: wgpu::TextureFormat,
        globals: &wgpu::Buffer,
        clip_transforms: &wgpu::Buffer,
    ) -> Self {
        let resources = Self::create_pipeline_resources(device, sample_count, color_format);
        Self::with_pipeline_resources(
            device,
            width,
            height,
            sample_count,
            globals,
            clip_transforms,
            resources,
        )
    }

    pub(crate) fn with_pipeline_resources(
        device: &Device,
        width: u32,
        height: u32,
        sample_count: u32,
        globals: &wgpu::Buffer,
        clip_transforms: &wgpu::Buffer,
        resources: StencilPipelineResources,
    ) -> Self {
        let stencil_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: resources.stencil_bind_group_layout.as_ref(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globals.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: clip_transforms.as_entire_binding(),
                },
            ],
            label: Some("stencil_bind_group"),
        });
        let (stencil_texture, stencil_view) =
            Self::create_stencil_texture(device, width, height, sample_count);

        Self {
            stencil_pipeline: resources.stencil_pipeline,
            decrement_pipeline: resources.decrement_pipeline,
            color_stencil_pipeline: resources.color_stencil_pipeline,
            color_decrement_pipeline: resources.color_decrement_pipeline,
            stencil_texture,
            stencil_view,
            width,
            height,
            sample_count,
            stencil_layer: 0,
            stencil_geometry_stack: vec![],
            cached_geometry: HashMap::new(),
            cached_clip_instances: HashMap::new(),
            stencil_bind_group,
        }
    }

    pub(crate) fn create_pipeline_resources(
        device: &Device,
        sample_count: u32,
        color_format: wgpu::TextureFormat,
    ) -> StencilPipelineResources {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Stencil Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("stencil.wgsl").into()),
        });

        let stencil_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                label: Some("stencil_bind_group_layout"),
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Stencil Pipeline Layout"),
            bind_group_layouts: &[&stencil_bind_group_layout],
            immediate_size: 0,
        });

        fn create_pipeline(
            device: &Device,
            shader: &wgpu::ShaderModule,
            pipeline_layout: &wgpu::PipelineLayout,
            label: &'static str,
            sample_count: u32,
            color_target: Option<wgpu::TextureFormat>,
            stencil_op: wgpu::StencilOperation,
        ) -> RenderPipeline {
            let color_targets: Vec<Option<wgpu::ColorTargetState>> = match color_target {
                Some(format) => vec![Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::empty(),
                })],
                None => vec![None],
            };

            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(pipeline_layout),
                vertex: wgpu::VertexState {
                    module: shader,
                    entry_point: Some("vs_main"),
                    buffers: &[
                        wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                        },
                        wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<u32>() as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &wgpu::vertex_attr_array![1 => Uint32],
                        },
                    ],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: shader,
                    entry_point: Some("fs_main"),
                    targets: &color_targets,
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
                            pass_op: stencil_op,
                        },
                        back: wgpu::StencilFaceState::IGNORE,
                        read_mask: !0,
                        write_mask: !0,
                    },
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: sample_count,
                    ..Default::default()
                },
                multiview_mask: None,
                cache: None,
            })
        }

        // Main stencil pipeline for incrementing
        let stencil_pipeline = create_pipeline(
            device,
            &shader,
            &pipeline_layout,
            "Stencil Pipeline",
            sample_count,
            None,
            wgpu::StencilOperation::IncrementClamp,
        );
        let color_stencil_pipeline = create_pipeline(
            device,
            &shader,
            &pipeline_layout,
            "Color Stencil Pipeline",
            sample_count,
            Some(color_format),
            wgpu::StencilOperation::IncrementClamp,
        );
        let decrement_pipeline = create_pipeline(
            device,
            &shader,
            &pipeline_layout,
            "Decrement Pipeline",
            sample_count,
            None,
            wgpu::StencilOperation::DecrementClamp,
        );
        let color_decrement_pipeline = create_pipeline(
            device,
            &shader,
            &pipeline_layout,
            "Color Decrement Pipeline",
            sample_count,
            Some(color_format),
            wgpu::StencilOperation::DecrementClamp,
        );

        StencilPipelineResources {
            stencil_pipeline: Rc::new(stencil_pipeline),
            decrement_pipeline: Rc::new(decrement_pipeline),
            color_stencil_pipeline: Rc::new(color_stencil_pipeline),
            color_decrement_pipeline: Rc::new(color_decrement_pipeline),
            stencil_bind_group_layout: Rc::new(stencil_bind_group_layout),
        }
    }

    fn create_stencil_texture(
        device: &Device,
        width: u32,
        height: u32,
        sample_count: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Stencil Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Stencil8,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
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
            Self::create_stencil_texture(device, width, height, self.sample_count);
        self.width = width;
        self.height = height;
    }

    fn create_cached_geometry(
        device: &Device,
        geometry: &VertexBuffers<Vertex, u16>,
    ) -> CachedStencilGeometry {
        let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Stencil Vertices"),
            contents: bytemuck::cast_slice(&geometry.vertices),
            usage: BufferUsages::VERTEX,
        });
        let padded_indices = padded_u16_buffer(&geometry.indices);
        let indices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Stencil Indices"),
            contents: bytemuck::cast_slice(&padded_indices),
            usage: BufferUsages::INDEX,
        });

        CachedStencilGeometry {
            vertices_buffer,
            indices_buffer,
            index_count: geometry.indices.len() as u32,
        }
    }

    fn ensure_cached_geometry(
        &mut self,
        device: &Device,
        signature: u64,
        geometry: &VertexBuffers<Vertex, u16>,
    ) {
        self.cached_geometry
            .entry(signature)
            .or_insert_with(|| Self::create_cached_geometry(device, geometry));
    }

    fn ensure_cached_clip_instance(&mut self, device: &Device, clip_id: u32) {
        self.cached_clip_instances
            .entry(clip_id)
            .or_insert_with(|| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Stencil Clip Instance Buffer"),
                    contents: bytemuck::bytes_of(&clip_id),
                    usage: BufferUsages::VERTEX,
                })
            });
    }

    pub fn sync_stencil_stack(
        &mut self,
        device: &Device,
        queue: &Queue,
        depth: u32,
        clips: &[ClipDraw<'_>],
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Stencil Sync Encoder"),
        });
        self.encode_stencil_stack_sync(device, &mut encoder, depth, clips);
        queue.submit(std::iter::once(encoder.finish()));
    }

    pub fn encode_stencil_stack_sync(
        &mut self,
        device: &Device,
        encoder: &mut wgpu::CommandEncoder,
        depth: u32,
        clips: &[ClipDraw<'_>],
    ) {
        let prepared = self.prepare_stencil_stack_sync(device, depth, clips);
        if prepared.is_empty() {
            return;
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Stencil Sync Pass"),
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
                multiview_mask: None,
            });
            self.encode_prepared_stencil_sync(&mut render_pass, &prepared, false);
        }
    }

    pub(crate) fn prepare_stencil_stack_sync(
        &mut self,
        device: &Device,
        depth: u32,
        clips: &[ClipDraw<'_>],
    ) -> PreparedStencilSync {
        let mut prepared = PreparedStencilSync::default();
        while self.stencil_layer > depth {
            let Some(entry) = self.stencil_geometry_stack.pop() else {
                log::error!("geometry stack shouldn't be embty when stencil layer > 0");
                break;
            };
            prepared.popped.push(entry);
            self.stencil_layer = self.stencil_layer.saturating_sub(1);
        }

        for clip in clips {
            self.ensure_cached_geometry(device, clip.geometry_signature, clip.geometry);
            self.ensure_cached_clip_instance(device, clip.clip_id);
        }

        for clip in clips {
            let entry = StencilStackEntry {
                clip_id: clip.clip_id,
                geometry_signature: clip.geometry_signature,
            };
            self.stencil_geometry_stack.push(entry);
            self.stencil_layer += 1;
            prepared.pushed.push(entry);
        }

        prepared
    }

    pub(crate) fn encode_prepared_stencil_sync<'pass>(
        &'pass self,
        render_pass: &mut wgpu::RenderPass<'pass>,
        prepared: &'pass PreparedStencilSync,
        has_color_attachment: bool,
    ) {
        if prepared.is_empty() {
            return;
        }

        render_pass.set_bind_group(0, &self.stencil_bind_group, &[]);

        if !prepared.popped.is_empty() {
            let pipeline = if has_color_attachment {
                self.color_decrement_pipeline.as_ref()
            } else {
                self.decrement_pipeline.as_ref()
            };
            render_pass.set_pipeline(pipeline);
            for entry in &prepared.popped {
                self.draw_stencil_entry(render_pass, entry);
            }
        }

        if !prepared.pushed.is_empty() {
            let pipeline = if has_color_attachment {
                self.color_stencil_pipeline.as_ref()
            } else {
                self.stencil_pipeline.as_ref()
            };
            render_pass.set_pipeline(pipeline);
            for entry in &prepared.pushed {
                self.draw_stencil_entry(render_pass, entry);
            }
        }
    }

    fn draw_stencil_entry<'pass>(
        &'pass self,
        render_pass: &mut wgpu::RenderPass<'pass>,
        entry: &StencilStackEntry,
    ) {
        let Some(cached_geometry) = self.cached_geometry.get(&entry.geometry_signature) else {
            log::error!(
                "missing cached stencil geometry for signature {}",
                entry.geometry_signature
            );
            return;
        };
        let Some(instance_buffer) = self.cached_clip_instances.get(&entry.clip_id) else {
            log::error!(
                "missing cached stencil clip instance for clip {}",
                entry.clip_id
            );
            return;
        };
        render_pass.set_vertex_buffer(0, cached_geometry.vertices_buffer.slice(..));
        render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
        render_pass.set_index_buffer(
            cached_geometry.indices_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );
        render_pass.draw_indexed(0..cached_geometry.index_count, 0, 0..1);
    }

    pub fn needs_stack_sync(&self, depth: u32, clips: &[ClipDraw<'_>]) -> bool {
        self.stencil_layer > depth || !clips.is_empty()
    }

    pub fn retain_cached_resources(
        &mut self,
        active_signatures: &HashSet<u64>,
        active_clip_ids: &HashSet<u32>,
    ) {
        let stack_signatures: HashSet<u64> = self
            .stencil_geometry_stack
            .iter()
            .map(|entry| entry.geometry_signature)
            .collect();
        self.cached_geometry.retain(|signature, _| {
            active_signatures.contains(signature) || stack_signatures.contains(signature)
        });
        let stack_clip_ids: HashSet<u32> = self
            .stencil_geometry_stack
            .iter()
            .map(|entry| entry.clip_id)
            .collect();
        self.cached_clip_instances.retain(|clip_id, _| {
            active_clip_ids.contains(clip_id) || stack_clip_ids.contains(clip_id)
        });
    }

    pub fn get_stencil(&self) -> (&wgpu::TextureView, u32) {
        (&self.stencil_view, self.stencil_layer)
    }

    pub(crate) fn stencil_view_clone(&self) -> wgpu::TextureView {
        self.stencil_view.clone()
    }

    pub fn clear(&mut self, device: &Device, queue: &Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Stencil Clear Encoder"),
        });
        self.encode_clear(&mut encoder);
        queue.submit(std::iter::once(encoder.finish()));
    }

    pub fn encode_clear(&mut self, encoder: &mut wgpu::CommandEncoder) {
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
                multiview_mask: None,
            });

            render_pass.set_pipeline(self.stencil_pipeline.as_ref());
        }

        self.stencil_layer = 0;
        self.stencil_geometry_stack.clear();
    }
}
