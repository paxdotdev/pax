//! Shared retained subtree surfaces for opacity composition and detached alpha sources. Scratch
//! attachments use the active capture domain; retained textures store only intersecting bounds.
use super::*;
use bytemuck::Zeroable;
use wgpu::util::DeviceExt;
use wgpu::Buffer;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) enum CaptureKey {
    Opacity(u32, u32),
    #[cfg(feature = "3d")]
    Presentation(u64, u32),
    #[cfg(feature = "3d")]
    SpatialGroup(u64, u32),
    #[cfg(feature = "3d")]
    SpatialVisibility(u64, u64, u64),
    #[cfg(feature = "3d")]
    SpatialOpacity(u64, u32, u32, usize),
    Alpha(u32),
    #[cfg(feature = "shaders")]
    EffectSource(u32),
}

impl CaptureKey {
    fn uses_source_domain(&self) -> bool {
        match self {
            Self::Alpha(_) => true,
            #[cfg(feature = "shaders")]
            Self::EffectSource(_) => true,
            Self::Opacity(..) => false,
            #[cfg(feature = "3d")]
            Self::Presentation(..) | Self::SpatialGroup(..) | Self::SpatialOpacity(..) | Self::SpatialVisibility(..) => false,
        }
    }
}

enum CaptureBlend {
    SourceOver,
    #[cfg(feature = "3d")]
    Replace,
    #[cfg(feature = "3d")]
    Add,
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
    #[cfg(feature = "3d")]
    replacement_pipeline: RenderPipeline,
    #[cfg(feature = "3d")]
    additive_pipeline: RenderPipeline,
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
    #[cfg(feature = "shaders")]
    pub(super) fn effect_source(
        &self,
        key: CaptureKey,
        globals: &data::GpuGlobals,
    ) -> (Texture, [f32; 4]) {
        let group = &self.groups[&key];
        let (x, y, w, h) = group.rect;
        (
            group.texture.clone(),
            [
                globals.origin[0] + x as f32 / globals.dpr[0],
                globals.origin[1] + y as f32 / globals.dpr[1],
                w as f32 / globals.dpr[0],
                h as f32 / globals.dpr[1],
            ],
        )
    }

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
        let make_pipeline = |blend| device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(), depth_stencil: None,
            multisample: wgpu::MultisampleState { count: backend.sample_count, ..Default::default() },
            multiview_mask: None, cache: None,
        });
        let pipeline = make_pipeline(Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING));
        #[cfg(feature = "3d")]
        let replacement_pipeline = make_pipeline(None);
        #[cfg(feature = "3d")]
        let additive_pipeline = make_pipeline(Some(wgpu::BlendState {
            color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
            alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
        }));
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Retained surface sampler"),
            ..Default::default()
        });
        Self {
            pipeline,
            #[cfg(feature = "3d")]
            replacement_pipeline,
            #[cfg(feature = "3d")]
            additive_pipeline,
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
        #[cfg(feature = "shaders")]
        if let Some(effects) = &mut self.effects {
            effects.retain(
                &active
                    .iter()
                    .filter_map(|k| match k {
                        CaptureKey::EffectSource(id) => Some(*id),
                        _ => None,
                    })
                    .collect(),
            );
        }

        if !active.iter().any(CaptureKey::uses_source_domain) {
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
        #[cfg(feature = "shaders")]
        {
            self.effects = None;
        }
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
                    Attachment::new(texture, view, &self.attachment_uses)
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
        // Keep clears, retained paint, copies, and composition in submission order
        // without consuming one unsubmitted Metal command buffer per capture.
        let mut encoder = self.take_retained_encoder();
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
        self.retained_encoder = Some(encoder);
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
        self.end_capture_rect(key, signature, rect);
    }

    /// Use an exact physical rectangle when an effect has already aligned its coarse pixel grid.
    pub(crate) fn end_capture_rect(
        &mut self,
        key: CaptureKey,
        signature: u64,
        rect: (u32, u32, u32, u32),
    ) {
        let mut encoder = self.take_retained_encoder();
        let surfaces = self.retained_surfaces.as_mut().unwrap();
        let mut scratch = surfaces.stack.pop().unwrap();
        // Opacity capacity follows the physical surface; shader source capacity
        // follows the bounded hysteresis policy below. Alpha sources stay exact.
        let opacity = match key {
            CaptureKey::Opacity(..) => true,
            #[cfg(feature = "3d")]
            CaptureKey::SpatialGroup(..) | CaptureKey::SpatialOpacity(..) | CaptureKey::SpatialVisibility(..) => true,
            _ => false,
        };
        let size = if opacity {
            let previous = surfaces.groups.get(&key);
            (
                rect.2.next_multiple_of(64)
                    .max(previous.map_or(0, |g| g.texture.width()))
                    .min(self.surface_config.width),
                rect.3.next_multiple_of(64)
                    .max(previous.map_or(0, |g| g.texture.height()))
                    .min(self.surface_config.height),
            )
        } else {
            (rect.2, rect.3)
        };
        #[cfg(feature = "shaders")]
        let size = if matches!(key, CaptureKey::EffectSource(_)) {
            // Small geometry changes must not replace the source and every dependent
            // pass texture. Keep bounded spare capacity; logical bounds remain exact.
            // 64 is divisible by every supported pass downsample factor.
            let capacity = |required: u32, previous: u32| {
                if previous >= required && required > previous / 4 {
                    previous
                } else {
                    required
                        .div_ceil(64)
                        .saturating_mul(64)
                        .min(self.device.limits().max_texture_dimension_2d)
                }
            };
            let previous = surfaces.groups.get(&key);
            (
                capacity(rect.2, previous.map_or(0, |g| g.texture.width())),
                capacity(rect.3, previous.map_or(0, |g| g.texture.height())),
            )
        } else {
            size
        };
        // Moving a capture changes its copy/composite origin, not its storage size.
        if surfaces
            .groups
            .get(&key)
            .is_none_or(|g| g.texture.width() != size.0 || g.texture.height() != size.1)
        {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Retained subtree"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
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
        group.rect = rect;
        group.signature = signature;
        group.domain_size = (self.surface_config.width, self.surface_config.height);
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
        self.retained_encoder = Some(encoder);
    }

    pub(crate) fn draw_capture(&mut self, key: CaptureKey, opacity: f32) {
        self.draw_capture_blend(key, opacity, CaptureBlend::SourceOver);
    }

    // Presence states are mutually exclusive complete scene views. Their
    // premultiplied colors/alphas sum; source-over would attenuate the backdrop twice.
    #[cfg(feature = "3d")]
    pub(crate) fn draw_capture_weighted(&mut self, key: CaptureKey, weight: f32, first: bool) {
        self.draw_capture_blend(key, weight, if first { CaptureBlend::Replace } else { CaptureBlend::Add });
    }

    fn draw_capture_blend(&mut self, key: CaptureKey, opacity: f32, mode: CaptureBlend) {
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
        let mut encoder = self.take_retained_encoder();
        let surfaces = self.retained_surfaces.as_ref().unwrap();
        let group = &surfaces.groups[&key];
        let (x, y, w, h) = group.rect;
        let left = x as f32 / self.surface_config.width as f32 * 2.0 - 1.0;
        let right = (x + w) as f32 / self.surface_config.width as f32 * 2.0 - 1.0;
        let top = 1.0 - y as f32 / self.surface_config.height as f32 * 2.0;
        let bottom = 1.0 - (y + h) as f32 / self.surface_config.height as f32 * 2.0;
        let u = w as f32 / group.texture.width() as f32;
        let v = h as f32 / group.texture.height() as f32;
        let vertices = [
            (left, top, 0., 0.),
            (left, bottom, 0., v),
            (right, top, u, 0.),
            (right, top, u, 0.),
            (left, bottom, 0., v),
            (right, bottom, u, v),
        ]
        .map(|(x, y, u, v)| Vertex {
            position: [x, y],
            uv: [u, v],
            opacity,
        });
        self.queue
            .write_buffer(&group.vertices, 0, bytemuck::cast_slice(&vertices));
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
            let pipeline = match mode {
                CaptureBlend::SourceOver => &surfaces.pipeline,
                #[cfg(feature = "3d")]
                CaptureBlend::Replace => &surfaces.replacement_pipeline,
                #[cfg(feature = "3d")]
                CaptureBlend::Add => &surfaces.additive_pipeline,
            };
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &group.bind_group, &[]);
            pass.set_vertex_buffer(0, group.vertices.slice(..));
            pass.draw(0..6, 0..1);
        }
        self.retained_encoder = Some(encoder);
    }
}

#[cfg(test)]
impl RenderBackend<'_> {
    pub(crate) fn alpha_source_has_working_storage(&self) -> bool {
        self.source_stencil.is_some()
            || self
                .retained_surfaces
                .as_ref()
                .is_some_and(|s| !s.scratch.is_empty())
    }

    #[cfg(feature = "3d")]
    pub(crate) fn spatial_fade_capture_count(&self) -> usize {
        self.retained_surfaces.as_ref().map_or(0, |s| s.groups.keys()
            .filter(|key| matches!(key, CaptureKey::SpatialVisibility(..))).count())
    }

    pub(crate) fn capture_texture_for_test(&self, key: CaptureKey) -> Texture {
        self.retained_surfaces.as_ref().unwrap().groups[&key]
            .texture
            .clone()
    }

    pub(crate) fn alpha_capture_count(&self) -> usize {
        self.retained_surfaces.as_ref().map_or(0, |s| {
            s.groups
                .keys()
                .filter(|key| key.uses_source_domain())
                .count()
        })
    }
}
