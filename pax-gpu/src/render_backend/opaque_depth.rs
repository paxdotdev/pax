//! Retained planar depth parameters. Paint and topology remain shared with the
//! ordered compositor; each composition owns its own depth mapping.
use super::*;

pub(crate) struct Resource {
    paint_buffer: wgpu::Buffer,
    uniform: wgpu::Buffer,
    last: [[f32; 4]; 2],
    pub(super) pipeline: Rc<RenderPipeline>,
    pub(super) bind_group: BindGroup,
}

impl RenderBackend<'_> {
    pub(crate) fn prepare_opaque_depth(
        &mut self,
        vector: &RetainedVectorResource,
        retained: &mut Option<Resource>,
        plane: [[f32; 4]; 2],
    ) -> u64 {
        let shared = &vector.shared;
        if let Some(resource) = retained
            .as_mut()
            .filter(|r| r.paint_buffer == shared._primitive_buffer)
        {
            if resource.last == plane {
                return 0;
            }
            // Queue writes could overwrite parameters used by an earlier encoded
            // draw. Copies in the retained encoder preserve publication order.
            let mut encoder = self.take_retained_encoder();
            self.staging_belt
                .write(&mut encoder, &resource.uniform, bytemuck::bytes_of(&plane));
            self.retained_encoder = Some(encoder);
            self.staging_belt_pending_recall = true;
            self.needs_device_poll = true;
            resource.last = plane;
        } else {
            let uniform = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("opaque planar depth"),
                    contents: bytemuck::bytes_of(&plane),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });
            let bind_group = self.create_retained_bind_group_with_depth(
                &shared._primitive_buffer,
                &shared._colors_buffer,
                &shared._gradients_buffer,
                &shared._materials_buffer,
                shared.uniform_capacity,
                Some(&uniform),
            );
            *retained = Some(Resource {
                // Even a Weak<Rc> blocks Rc::get_mut used by paint updates.
                // GPU buffer handles identify the resource without owning that Rc.
                paint_buffer: shared._primitive_buffer.clone(),
                uniform,
                last: plane,
                pipeline: self.context.vector_pipeline_with_depth(
                    self.surface_config.format,
                    self.sample_count,
                    shared.uniform_capacity,
                    true,
                ),
                bind_group,
            });
        }
        std::mem::size_of_val(&plane) as u64
    }
}

impl GpuContext {
    pub(super) fn opaque_sample_locations(&self, samples: u32) -> wgpu::Buffer {
        if let Some(buffer) = self.sample_locations.borrow().get(&samples) {
            return buffer.clone();
        }
        // WebGPU fixes the sample pattern across pixels, but does not expose its
        // locations. Capture it once using a one-pixel primitive, entirely on GPU.
        // Sharing these values avoids both pixel-center MSAA depth and independently
        // interpolated coordinates that can differ between coplanar decorations.
        assert!(matches!(samples, 1 | 4));
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("opaque sample locations"),
            size: 64,
            usage: BufferUsages::STORAGE | BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("opaque sample location capture"),
                source: wgpu::ShaderSource::Wgsl(include_str!("sample_locations.wgsl").into()),
            });
        let layout = self
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sample location capture"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sample location capture"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("sample location capture"),
                bind_group_layouts: &[&layout],
                immediate_size: 0,
            });
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("sample location capture"),
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
                        format: TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: samples,
                    ..Default::default()
                },
                multiview_mask: None,
                cache: None,
            });
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sample location capture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: samples,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sample location capture"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        // This initialization submits immediately while its temporary target is
        // alive; it cannot escape into a deferred frame's attachment leases.
        self.queue.submit([encoder.finish()]);
        self.unfenced_submission.set(true);
        self.sample_locations
            .borrow_mut()
            .insert(samples, buffer.clone());
        buffer
    }
}
