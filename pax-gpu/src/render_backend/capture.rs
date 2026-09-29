//! Shared retained subtree surfaces for opacity composition and detached alpha sources. Scratch
//! attachments use the active capture domain; retained textures store only intersecting bounds.
use super::*;
use bytemuck::Zeroable;
use wgpu::util::DeviceExt;
use wgpu::Buffer;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) enum CaptureKey {
    Opacity(u32, u32),
    Alpha(u32),
}

struct CachedGroup {
    signature: u64,
    domain_size: (u32, u32),
    rect: (u32, u32, u32, u32),
    texture: Texture,
    bind_group: BindGroup,
    vertices: Buffer,
}

pub(super) struct RetainedSurfaces {
    pipeline: RenderPipeline,
    layout: BindGroupLayout,
    sampler: wgpu::Sampler,
    groups: HashMap<CaptureKey, CachedGroup>,
    scratch: Vec<CaptureTarget>,
    pub(super) stack: Vec<CaptureTarget>,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
    opacity: f32,
}

impl RetainedSurfaces {
    pub(super) fn texture_view(&self, key: CaptureKey, size: [u32; 2]) -> (TextureView, [f32; 4]) {
        let group = &self.groups[&key];
        let (x, y, w, h) = group.rect;
        (
            group.texture.create_view(&Default::default()),
            [
                x as f32 / size[0] as f32,
                y as f32 / size[1] as f32,
                w as f32 / size[0] as f32,
                h as f32 / size[1] as f32,
            ],
        )
    }

    fn new(backend: &RenderBackend<'_>) -> Self {
        let device = &backend.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Retained surface sampling"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
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
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Premultiplied subtree composition"),
            source: wgpu::ShaderSource::Wgsl(include_str!("opacity.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Retained surface pipeline layout"),
            bind_group_layouts: &[&layout],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Retained surface pipeline"), layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader, entry_point: Some("vs_main"), compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader, entry_point: Some("fs_main"), compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: backend.surface_config.format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(), depth_stencil: None,
            multisample: wgpu::MultisampleState { count: backend.sample_count, ..Default::default() },
            multiview_mask: None, cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Retained surface sampler"),
            ..Default::default()
        });
        Self {
            pipeline,
            layout,
            sampler,
            groups: HashMap::new(),
            scratch: Vec::new(),
            stack: Vec::new(),
        }
    }
}

impl RenderBackend<'_> {
    pub(crate) fn capture_is_resident(&self, key: CaptureKey) -> bool {
        self.retained_surfaces
            .as_ref()
            .is_some_and(|s| s.groups.contains_key(&key))
    }

    pub(crate) fn retain_captures(&mut self, active: &HashSet<CaptureKey>) {
        if !active.iter().any(|key| matches!(key, CaptureKey::Alpha(_))) {
            self.source_stencil = None;
        }
        if let Some(surfaces) = &mut self.retained_surfaces {
            surfaces.groups.retain(|key, _| active.contains(key));
            surfaces.scratch.retain(|target| {
                let size = (target.texture.width(), target.texture.height());
                size == (self.surface_config.width, self.surface_config.height)
                    || surfaces.groups.values().any(|g| g.domain_size == size)
            });
            if active.is_empty() {
                surfaces.scratch.clear();
            }
        }
    }

    pub(crate) fn reset_captures(&mut self) {
        self.retained_surfaces = None;
        self.source_stencil = None;
    }

    pub(crate) fn capture_is_cached(&mut self, key: CaptureKey, signature: u64) -> bool {
        if self.retained_surfaces.as_ref().is_none() {
            self.retained_surfaces = Some(RetainedSurfaces::new(self));
        }
        self.retained_surfaces
            .as_ref()
            .unwrap()
            .groups
            .get(&key)
            .is_some_and(|g| g.signature == signature)
    }

    pub(crate) fn begin_capture(&mut self) {
        let scratch = self
            .retained_surfaces
            .as_mut()
            .unwrap()
            .scratch
            .iter()
            .position(|target| {
                target.texture.width() == self.surface_config.width
                    && target.texture.height() == self.surface_config.height
            })
            .map(|index| {
                self.retained_surfaces
                    .as_mut()
                    .unwrap()
                    .scratch
                    .swap_remove(index)
            })
            .unwrap_or_else(|| {
                let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Capture scratch tile"),
                    size: wgpu::Extent3d {
                        width: self.surface_config.width,
                        height: self.surface_config.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: self.surface_config.format,
                    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
                    view_formats: &[],
                });
                let view = texture.create_view(&Default::default());
                let multisampled_target = (self.sample_count > 1).then(|| {
                    let (texture, view) = create_multisampled_framebuffer(
                        &self.device,
                        &self.surface_config,
                        self.sample_count,
                    );
                    MultisampledTarget {
                        _texture: texture,
                        view,
                    }
                });
                CaptureTarget {
                    texture,
                    view,
                    multisampled_target,
                    initialized: false,
                }
            });
        self.retained_surfaces.as_mut().unwrap().stack.push(scratch);
        let load = self.take_color_load_op();
        let (view, resolve) = self.current_color_attachment_view_clones();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Clear capture scratch"),
            });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear capture scratch"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: resolve.as_ref(),
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        self.enqueue_command_buffer(encoder.finish());
    }

    pub(crate) fn end_capture(&mut self, key: CaptureKey, signature: u64, bounds: ScissorRect) {
        let rect = physical_scissor_rect(
            Some(bounds),
            self.globals.dpr,
            self.globals.origin,
            self.surface_config.width,
            self.surface_config.height,
        )
        .expect("visible group bounds");
        let surfaces = self.retained_surfaces.as_mut().unwrap();
        let mut scratch = surfaces.stack.pop().unwrap();
        if surfaces.groups.get(&key).is_none_or(|g| g.rect != rect) {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Retained subtree"),
                size: wgpu::Extent3d {
                    width: rect.2,
                    height: rect.3,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.surface_config.format,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Retained subtree sampling"),
                layout: &surfaces.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&surfaces.sampler),
                    },
                ],
            });
            let vertices = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Retained subtree quad"),
                    contents: bytemuck::cast_slice(&[Vertex::zeroed(); 6]),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                });
            surfaces.groups.insert(
                key,
                CachedGroup {
                    signature,
                    domain_size: (self.surface_config.width, self.surface_config.height),
                    rect,
                    texture,
                    bind_group,
                    vertices,
                },
            );
        }
        let group = surfaces.groups.get_mut(&key).unwrap();
        group.signature = signature;
        group.domain_size = (self.surface_config.width, self.surface_config.height);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Cache subtree"),
            });
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &scratch.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: rect.0,
                    y: rect.1,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            group.texture.as_image_copy(),
            wgpu::Extent3d {
                width: rect.2,
                height: rect.3,
                depth_or_array_layers: 1,
            },
        );
        scratch.initialized = false;
        surfaces.scratch.push(scratch);
        self.enqueue_command_buffer(encoder.finish());
    }

    pub(crate) fn draw_capture(&mut self, key: CaptureKey, opacity: f32) {
        let load = self.take_color_load_op();
        self.ensure_active_frame();
        let mut targets = vec![(self.current_color_attachment_view_clones(), load)];
        if self.should_render_capture_target() {
            self.ensure_capture_target();
            let load = self.take_capture_color_load_op(load);
            let target = self.capture_target.as_ref().unwrap();
            let (view, resolve) = target.color_attachment_views();
            targets.push(((view.clone(), resolve.cloned()), load));
        }
        let surfaces = self.retained_surfaces.as_ref().unwrap();
        let group = &surfaces.groups[&key];
        let (x, y, w, h) = group.rect;
        let left = x as f32 / self.surface_config.width as f32 * 2.0 - 1.0;
        let right = (x + w) as f32 / self.surface_config.width as f32 * 2.0 - 1.0;
        let top = 1.0 - y as f32 / self.surface_config.height as f32 * 2.0;
        let bottom = 1.0 - (y + h) as f32 / self.surface_config.height as f32 * 2.0;
        let vertices = [
            (left, top, 0., 0.),
            (left, bottom, 0., 1.),
            (right, top, 1., 0.),
            (right, top, 1., 0.),
            (left, bottom, 0., 1.),
            (right, bottom, 1., 1.),
        ]
        .map(|(x, y, u, v)| Vertex {
            position: [x, y],
            uv: [u, v],
            opacity,
        });
        self.queue
            .write_buffer(&group.vertices, 0, bytemuck::cast_slice(&vertices));
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Present opacity group"),
            });
        for ((view, resolve), load) in &targets {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Present opacity group"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: resolve.as_ref(),
                    ops: wgpu::Operations {
                        load: *load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&surfaces.pipeline);
            pass.set_bind_group(0, &group.bind_group, &[]);
            pass.set_vertex_buffer(0, group.vertices.slice(..));
            pass.draw(0..6, 0..1);
        }
        self.enqueue_command_buffer(encoder.finish());
    }
}

#[cfg(test)]
impl RenderBackend<'_> {
    pub(crate) fn alpha_capture_count(&self) -> usize {
        self.retained_surfaces.as_ref().map_or(0, |s| {
            s.groups
                .keys()
                .filter(|key| matches!(key, CaptureKey::Alpha(_)))
                .count()
        })
    }
}
