use anyhow::anyhow;
use bytemuck::Pod;
use std::collections::HashSet;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use std::ffi::c_void;

use lyon::lyon_tessellation::VertexBuffers;
use wgpu::{
    util::DeviceExt, BindGroup, BindGroupLayout, BufferUsages, CompositeAlphaMode, Device,
    IndexFormat, PresentMode, RenderPipeline, SurfaceConfiguration, SurfaceTexture, Texture,
    TextureFormat, TextureFormatFeatureFlags, TextureUsages, TextureView,
};

pub mod data;
mod gpu_resources;
pub mod stencil;
mod texture;

pub(crate) use texture::CachedTextureResource;

use data::{GpuGlobals, GpuPrimitive, GpuVertex};

use crate::{
    render_backend::texture::{
        corners_to_texture_vertices, RetainedImageResource, TextureRenderer,
    },
    Box2D, Transform2D,
};

use self::{
    data::{GpuColor, GpuGradient, GpuTransform},
    gpu_resources::create_multisampled_framebuffer,
    stencil::StencilRenderer,
};

pub struct RenderConfig {
    pub debug: bool,
    index_buffer_size: u64,
    vertex_buffer_size: u64,
    primitive_buffer_size: u64,
    colors_buffer_size: u64,
    gradients_buffer_size: u64,
    transforms_buffer_size: u64,
    pub initial_width: u32,
    pub initial_height: u32,
    pub initial_dpr: f32,
}

pub(crate) const MAX_BATCH_PRIMITIVES: usize = 512;
pub(crate) const MAX_BATCH_COLORS: usize = 512;
pub(crate) const MAX_BATCH_GRADIENTS: usize = 64;
pub(crate) const MAX_BATCH_TRANSFORMS: usize = 512;
pub(crate) const MAX_SCENE_TRANSFORMS: usize = 1024;

impl RenderConfig {
    pub fn new(_debug: bool, width: u32, height: u32, dpr: f32) -> Self {
        Self {
            debug: false,
            index_buffer_size: 2 << 12,
            vertex_buffer_size: 2 << 12,
            primitive_buffer_size: MAX_BATCH_PRIMITIVES as u64,
            colors_buffer_size: MAX_BATCH_COLORS as u64,
            gradients_buffer_size: MAX_BATCH_GRADIENTS as u64,
            transforms_buffer_size: MAX_SCENE_TRANSFORMS as u64,
            initial_width: width,
            initial_height: height,
            initial_dpr: dpr,
        }
    }
}

fn next_capacity(required: usize) -> u64 {
    required.max(1).next_power_of_two() as u64
}

fn write_u16_buffer_padded(queue: &wgpu::Queue, buffer: &wgpu::Buffer, data: &[u16]) {
    if data.len() % 2 == 0 {
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(data));
        return;
    }

    let mut padded = Vec::with_capacity(data.len() + 1);
    padded.extend_from_slice(data);
    padded.push(0);
    queue.write_buffer(buffer, 0, bytemuck::cast_slice(&padded));
}

pub struct RenderBackend<'w> {
    //configuration
    config: RenderConfig,
    pub(crate) globals: GpuGlobals,

    //gpu pipeline references
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'w>,
    surface_config: SurfaceConfiguration,
    max_surface_dimension: u32,
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    primitive_bind_group_layout: BindGroupLayout,

    //buffers
    globals_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    primitive_buffer: wgpu::Buffer,
    transforms_buffer: wgpu::Buffer,
    colors_buffer: wgpu::Buffer,
    gradients_buffer: wgpu::Buffer,

    index_count: u64,

    // plugins / extensions
    texture_renderer: TextureRenderer,
    stencil_renderer: StencilRenderer,
    multisampled_target: Option<MultisampledTarget>,
    sample_count: u32,
    active_frame: Option<ActiveFrame>,
    pending_clear: bool,
}

struct ActiveFrame {
    view: TextureView,
    surface: SurfaceTexture,
}

struct MultisampledTarget {
    _texture: Texture,
    view: TextureView,
}

pub(crate) struct RetainedVectorResource {
    bind_group: BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    vertex_capacity: usize,
    index_capacity: usize,
    primitive_capacity: usize,
    color_capacity: usize,
    gradient_capacity: usize,
    _primitive_buffer: wgpu::Buffer,
    _colors_buffer: wgpu::Buffer,
    _gradients_buffer: wgpu::Buffer,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct VectorResourceDirty {
    pub geometry: bool,
    pub primitives: bool,
    pub transforms: bool,
    pub fill: bool,
}

impl VectorResourceDirty {
    pub fn any(self) -> bool {
        self.geometry || self.primitives || self.transforms || self.fill
    }
}

pub(crate) enum RetainedDraw<'a> {
    Vector(&'a RetainedVectorResource),
    Image {
        texture: &'a CachedTextureResource,
        draw: &'a RetainedImageDraw,
    },
}

pub(crate) struct RetainedImageDraw {
    pub resource: RetainedImageResource,
}

impl<'w> RenderBackend<'w> {
    fn rebuild_main_bind_group(&mut self) {
        self.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.primitive_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.globals_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.primitive_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.transforms_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.colors_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.gradients_buffer.as_entire_binding(),
                },
            ],
            label: Some("bind_group"),
        });
    }

    fn resize_shared_buffers_if_needed(
        &mut self,
        required_indices: usize,
        required_vertices: usize,
        required_primitives: usize,
        required_transforms: usize,
        required_colors: usize,
        required_gradients: usize,
    ) {
        let mut rebind_main_group = false;

        if required_indices > self.config.index_buffer_size as usize {
            self.config.index_buffer_size = next_capacity(required_indices);
            self.index_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Index Buffer"),
                size: self.config.index_buffer_size * std::mem::size_of::<u16>() as u64,
                usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        if required_vertices > self.config.vertex_buffer_size as usize {
            self.config.vertex_buffer_size = next_capacity(required_vertices);
            self.vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Vertex Buffer"),
                size: self.config.vertex_buffer_size * std::mem::size_of::<GpuVertex>() as u64,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        if required_primitives > self.config.primitive_buffer_size as usize {
            self.config.primitive_buffer_size = next_capacity(required_primitives);
            self.primitive_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Primitive Buffer"),
                size: self.config.primitive_buffer_size
                    * std::mem::size_of::<GpuPrimitive>() as u64,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            rebind_main_group = true;
        }

        if required_transforms > self.config.transforms_buffer_size as usize {
            log::error!(
                "render backend: transform arena capacity exceeded (required {}, capacity {})",
                required_transforms,
                self.config.transforms_buffer_size
            );
        }

        if required_colors > self.config.colors_buffer_size as usize {
            self.config.colors_buffer_size = next_capacity(required_colors);
            self.colors_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Colors Buffer"),
                size: self.config.colors_buffer_size * std::mem::size_of::<GpuColor>() as u64,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            rebind_main_group = true;
        }

        if required_gradients > self.config.gradients_buffer_size as usize {
            self.config.gradients_buffer_size = next_capacity(required_gradients);
            self.gradients_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Gradients Buffer"),
                size: self.config.gradients_buffer_size * std::mem::size_of::<GpuGradient>() as u64,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            rebind_main_group = true;
        }

        if rebind_main_group {
            self.rebuild_main_bind_group();
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn to_canvas(
        canvas: web_sys::HtmlCanvasElement,
        config: RenderConfig,
    ) -> Result<Self, anyhow::Error> {
        let instance = wgpu::util::new_instance_with_webgpu_detection(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            flags: if config.debug {
                wgpu::InstanceFlags::DEBUG
            } else {
                wgpu::InstanceFlags::default()
            },
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
        })
        .await;
        let surface_target = wgpu::SurfaceTarget::Canvas(canvas);
        let surface = instance.create_surface(surface_target)?;
        Self::new(surface, instance, config).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn to_canvas(
        _canvas: web_sys::HtmlCanvasElement,
        _config: RenderConfig,
    ) -> Result<Self, anyhow::Error> {
        Err(anyhow!(
            "canvas surfaces are only supported on wasm32 targets"
        ))
    }

    #[cfg(any(target_os = "ios", target_os = "macos"))]
    pub async unsafe fn to_core_animation_layer(
        layer: *mut c_void,
        config: RenderConfig,
    ) -> Result<RenderBackend<'static>, anyhow::Error> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: if config.debug {
                wgpu::InstanceFlags::DEBUG
            } else {
                wgpu::InstanceFlags::default()
            },
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
        });
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer))?
        };
        RenderBackend::<'static>::new(surface, instance, config).await
    }

    pub async fn new(
        surface: wgpu::Surface<'w>,
        instance: wgpu::Instance,
        config: RenderConfig,
    ) -> Result<Self, anyhow::Error> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|_| anyhow!("couldn't find adapter"))?;
        let adapter_info = adapter.get_info();
        log::info!(
            "render backend: using {:?} adapter \"{}\"",
            adapter_info.backend,
            adapter_info.name
        );
        #[cfg(target_arch = "wasm32")]
        let required_limits = wgpu::Limits::downlevel_webgl2_defaults();
        #[cfg(not(target_arch = "wasm32"))]
        let required_limits = adapter.limits();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::default(),
                required_limits,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("couldn't find device");
        let max_surface_dimension = device.limits().max_texture_dimension_2d;
        log::info!(
            "render backend: max surface dimension {}",
            max_surface_dimension
        );

        let surface_caps = surface.get_capabilities(&adapter);
        #[cfg(target_arch = "wasm32")]
        let surface_format = surface_caps
            .formats
            .first()
            .copied()
            .ok_or_else(|| anyhow!("surface reported no compatible texture formats"))?;
        #[cfg(not(target_arch = "wasm32"))]
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .or_else(|| {
                surface_caps.formats.iter().copied().find(|format| {
                    matches!(
                        format,
                        TextureFormat::Bgra8Unorm | TextureFormat::Rgba8Unorm
                    )
                })
            })
            .or_else(|| {
                surface_caps
                    .formats
                    .iter()
                    .copied()
                    .find(|format| *format == TextureFormat::Rgba16Float)
            })
            .or_else(|| surface_caps.formats.first().copied())
            .ok_or_else(|| anyhow!("surface reported no compatible texture formats"))?;
        log::info!(
            "render backend: surface format {:?} (srgb={})",
            surface_format,
            surface_format.is_srgb()
        );
        let alpha_mode = [
            CompositeAlphaMode::PreMultiplied,
            CompositeAlphaMode::PostMultiplied,
            CompositeAlphaMode::Opaque,
        ]
        .into_iter()
        .find(|mode| surface_caps.alpha_modes.contains(mode))
        .or_else(|| surface_caps.alpha_modes.first().copied())
        .ok_or_else(|| anyhow!("surface reported no compatible alpha modes"))?;
        #[cfg(target_arch = "wasm32")]
        if alpha_mode == CompositeAlphaMode::Opaque {
            log::warn!("render backend: browser surface only exposes opaque alpha");
        }
        let surface_format_features = adapter.get_texture_format_features(surface_format).flags;
        let stencil_format_features = adapter
            .get_texture_format_features(wgpu::TextureFormat::Stencil8)
            .flags;
        let sample_count = select_sample_count(surface_format_features, stencil_format_features);
        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: config.initial_width.max(1),
            height: config.initial_height.max(1),
            present_mode: PresentMode::Fifo,
            alpha_mode,
            view_formats: vec![surface_format],
            desired_maximum_frame_latency: 2, //TODO 1 for lower latency?
        };
        surface.configure(&device, &surface_config);

        fn create_buffer<T: Default + Clone + Pod>(
            device: &Device,
            name: &str,
            size: u64,
            usage_flags: BufferUsages,
        ) -> (Vec<T>, wgpu::Buffer) {
            let data: Vec<T> = vec![T::default(); size as usize];
            let buffer_ref = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(name),
                contents: bytemuck::cast_slice(&data),
                usage: usage_flags,
            });
            (data, buffer_ref)
        }

        let globals = GpuGlobals {
            resolution: [
                config.initial_width.max(1) as f32,
                config.initial_height.max(1) as f32,
            ],
            dpr: config.initial_dpr,
            _pad2: 0.0,
        };
        let (_, globals_buffer) = create_buffer::<GpuGlobals>(
            &device,
            "Primitive Buffer",
            1,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );

        let (_, vertex_buffer) = create_buffer::<GpuVertex>(
            &device,
            "Vertex Buffer",
            config.vertex_buffer_size,
            BufferUsages::VERTEX | BufferUsages::COPY_DST,
        );
        let (_, index_buffer) = create_buffer::<u16>(
            &device,
            "Index Buffer",
            config.index_buffer_size,
            BufferUsages::INDEX | BufferUsages::COPY_DST,
        );
        let (_, primitive_buffer) = create_buffer::<GpuPrimitive>(
            &device,
            "Primitive Buffer",
            config.primitive_buffer_size,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let (_, transforms_buffer) = create_buffer::<GpuTransform>(
            &device,
            "Transform Buffer",
            config.transforms_buffer_size,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let (_, colors_buffer) = create_buffer::<GpuColor>(
            &device,
            "Colors Buffer",
            config.colors_buffer_size,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let (_, gradients_buffer) = create_buffer::<GpuGradient>(
            &device,
            "Gradients Buffer",
            config.gradients_buffer_size,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );

        let primitive_bind_group_layout =
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
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("bind_group_layout"),
            });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &primitive_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globals_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: primitive_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: transforms_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: colors_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: gradients_buffer.as_entire_binding(),
                },
            ],
            label: Some("bind_group"),
        });

        let pipeline = Self::create_pipeline(
            &device,
            surface_config.format,
            sample_count,
            &primitive_bind_group_layout,
        );

        let texture_renderer = TextureRenderer::new(&device, surface_config.format, sample_count);
        let stencil_renderer = StencilRenderer::new(
            &device,
            config.initial_width,
            config.initial_height,
            sample_count,
            &globals_buffer,
        );

        let initial_width = config.initial_width;
        let initial_height = config.initial_height;
        let initial_dpr = config.initial_dpr;
        let mut backend = Self {
            texture_renderer,
            stencil_renderer,
            _adapter: adapter,
            surface,
            device,
            queue,
            config,
            surface_config,
            max_surface_dimension,
            bind_group,
            primitive_bind_group_layout,
            pipeline,
            vertex_buffer,
            index_buffer,
            primitive_buffer,
            transforms_buffer,
            globals_buffer,
            colors_buffer,
            gradients_buffer,
            globals,
            index_count: 0,
            multisampled_target: None,
            sample_count,
            active_frame: None,
            pending_clear: false,
        };
        backend.globals.dpr = initial_dpr;
        backend.resize(initial_width, initial_height);
        Ok(backend)
    }

    fn create_pipeline(
        device: &Device,
        format: TextureFormat,
        sample_count: u32,
        primitive_bind_group_layout: &BindGroupLayout,
    ) -> RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("geometry.wgsl").into()),
        });
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[primitive_bind_group_layout],
                immediate_size: 0,
            });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[GpuVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
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
                count: sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        })
    }

    pub fn push_stencil(
        &mut self,
        signature: u64,
        geometry: &VertexBuffers<stencil::Vertex, u16>,
        transform: Transform2D,
    ) {
        // self.stencil_renderer.clear(&self.device, &self.queue);
        self.stencil_renderer.push_stencil(
            &self.device,
            &self.queue,
            signature,
            geometry,
            transform,
        );
    }

    pub fn reset_stencil_depth_to(&mut self, depth: u32) {
        self.stencil_renderer
            .reset_stencil_depth_to(&self.device, &self.queue, depth);
    }

    pub fn get_clip_depth(&mut self) -> u32 {
        let (_, depth) = self.stencil_renderer.get_stencil();
        depth
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.resize_surface(width, height);
        self.set_viewport(width as f32, height as f32, self.globals.dpr);
    }

    pub fn resize_surface(&mut self, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        self.active_frame = None;
        self.pending_clear = false;
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.stencil_renderer.resize(&self.device, width, height);
        self.surface.configure(&self.device, &self.surface_config);
        self.multisampled_target = if self.sample_count > 1 {
            let (texture, view) = create_multisampled_framebuffer(
                &self.device,
                &self.surface_config,
                self.sample_count,
            );
            Some(MultisampledTarget {
                _texture: texture,
                view,
            })
        } else {
            None
        };
    }

    pub fn set_viewport(&mut self, width: f32, height: f32, dpr: f32) {
        self.globals.resolution = [width.max(1.0), height.max(1.0)];
        self.globals.dpr = dpr;
        self.queue.write_buffer(
            &self.globals_buffer,
            0,
            bytemuck::cast_slice(&[self.globals]),
        );
    }

    pub fn max_surface_dimension(&self) -> u32 {
        self.max_surface_dimension
    }

    fn create_retained_bind_group(
        &self,
        primitive_buffer: &wgpu::Buffer,
        colors_buffer: &wgpu::Buffer,
        gradients_buffer: &wgpu::Buffer,
    ) -> BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.primitive_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.globals_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: primitive_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.transforms_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: colors_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: gradients_buffer.as_entire_binding(),
                },
            ],
            label: Some("retained_bind_group"),
        })
    }

    pub(crate) fn create_vector_resource(
        &self,
        buffers: &mut CpuBuffers,
        retained_primitives: &[GpuPrimitive],
    ) -> RetainedVectorResource {
        let (vertices, indices, _, _, mut colors, mut gradients) = aligned_cpu_buffers(buffers);
        let vertex_capacity = vertices.len();
        let index_capacity = indices.len();
        let primitive_capacity = retained_primitives.len();
        let color_capacity = colors.len();
        let gradient_capacity = gradients.len();
        let mut primitives = retained_primitives.to_vec();
        primitives.resize(
            self.config.primitive_buffer_size as usize,
            GpuPrimitive::default(),
        );
        colors.resize(self.config.colors_buffer_size as usize, GpuColor::default());
        gradients.resize(
            self.config.gradients_buffer_size as usize,
            GpuGradient::default(),
        );
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Retained Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            });
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Retained Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            });
        let primitive_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Retained Primitive Buffer"),
                contents: bytemuck::cast_slice(&primitives),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });
        let colors_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Retained Color Buffer"),
                contents: bytemuck::cast_slice(&colors),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });
        let gradients_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Retained Gradient Buffer"),
                contents: bytemuck::cast_slice(&gradients),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });
        let bind_group =
            self.create_retained_bind_group(&primitive_buffer, &colors_buffer, &gradients_buffer);

        RetainedVectorResource {
            bind_group,
            vertex_buffer,
            index_buffer,
            index_count: buffers.geometry.indices.len() as u32,
            vertex_capacity,
            index_capacity,
            primitive_capacity,
            color_capacity,
            gradient_capacity,
            _primitive_buffer: primitive_buffer,
            _colors_buffer: colors_buffer,
            _gradients_buffer: gradients_buffer,
        }
    }

    pub(crate) fn update_vector_resource(
        &self,
        resource: &mut RetainedVectorResource,
        buffers: &mut CpuBuffers,
        retained_primitives: &[GpuPrimitive],
        dirty: VectorResourceDirty,
    ) -> bool {
        let (vertices, indices, _, _, colors, gradients) = aligned_cpu_buffers(buffers);
        if vertices.len() > resource.vertex_capacity
            || indices.len() > resource.index_capacity
            || retained_primitives.len() > resource.primitive_capacity
            || colors.len() > resource.color_capacity
            || gradients.len() > resource.gradient_capacity
        {
            return false;
        }

        if dirty.geometry {
            self.queue
                .write_buffer(&resource.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
            write_u16_buffer_padded(&self.queue, &resource.index_buffer, &indices);
        }
        if dirty.primitives {
            self.queue.write_buffer(
                &resource._primitive_buffer,
                0,
                bytemuck::cast_slice(retained_primitives),
            );
        }
        if dirty.fill {
            self.queue
                .write_buffer(&resource._colors_buffer, 0, bytemuck::cast_slice(&colors));
            self.queue.write_buffer(
                &resource._gradients_buffer,
                0,
                bytemuck::cast_slice(&gradients),
            );
        }
        resource.index_count = buffers.geometry.indices.len() as u32;
        true
    }

    pub(crate) fn update_scene_transforms(&self, transforms: &[GpuTransform]) {
        self.queue
            .write_buffer(&self.transforms_buffer, 0, bytemuck::cast_slice(transforms));
    }

    #[allow(dead_code)]
    pub(crate) fn draw_vector_resource(&mut self, resource: &RetainedVectorResource) {
        let load_op = self.take_color_load_op();
        self.ensure_active_frame();
        let (screen_texture, resolve_target) = self.current_color_attachment_views();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Retained Render Encoder"),
            });

        {
            let (stencil_texture, stencil_index) = self.stencil_renderer.get_stencil();
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Retained Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: screen_texture,
                    depth_slice: None,
                    resolve_target,
                    ops: wgpu::Operations {
                        load: load_op,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: stencil_texture,
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
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &resource.bind_group, &[]);
            render_pass.set_vertex_buffer(0, resource.vertex_buffer.slice(..));
            render_pass.set_stencil_reference(stencil_index);
            render_pass.set_index_buffer(resource.index_buffer.slice(..), IndexFormat::Uint16);
            render_pass.draw_indexed(0..resource.index_count, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    pub(crate) fn draw_retained_batch(&mut self, stencil_index: u32, draws: &[RetainedDraw<'_>]) {
        if draws.is_empty() {
            return;
        }

        let load_op = self.take_color_load_op();
        self.ensure_active_frame();
        let (screen_texture, resolve_target) = self.current_color_attachment_views();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Retained Batch Encoder"),
            });

        {
            let (stencil_texture, _) = self.stencil_renderer.get_stencil();
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Retained Batch Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: screen_texture,
                    depth_slice: None,
                    resolve_target,
                    ops: wgpu::Operations {
                        load: load_op,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: stencil_texture,
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
            render_pass.set_stencil_reference(stencil_index);
            for draw in draws {
                match draw {
                    RetainedDraw::Vector(resource) => {
                        render_pass.set_pipeline(&self.pipeline);
                        render_pass.set_bind_group(0, &resource.bind_group, &[]);
                        render_pass.set_vertex_buffer(0, resource.vertex_buffer.slice(..));
                        render_pass
                            .set_index_buffer(resource.index_buffer.slice(..), IndexFormat::Uint16);
                        render_pass.draw_indexed(0..resource.index_count, 0, 0..1);
                    }
                    RetainedDraw::Image { texture, draw } => {
                        self.texture_renderer.draw_retained_image_in_pass(
                            &mut render_pass,
                            texture,
                            &draw.resource,
                        );
                    }
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    pub(crate) fn push_stencil_geometry(
        &mut self,
        signature: u64,
        geometry: &VertexBuffers<stencil::Vertex, u16>,
        transform: Transform2D,
    ) {
        self.stencil_renderer.push_stencil(
            &self.device,
            &self.queue,
            signature,
            geometry,
            transform,
        );
    }

    pub(crate) fn retain_stencil_geometry(&mut self, active_signatures: &HashSet<u64>) {
        self.stencil_renderer
            .retain_cached_geometry(active_signatures);
    }

    pub(crate) fn create_cached_texture(
        &self,
        rgba: &[u8],
        rgba_width: u32,
        rgba_height: u32,
    ) -> CachedTextureResource {
        self.texture_renderer.create_cached_texture(
            &self.device,
            &self.queue,
            &self.globals_buffer,
            rgba,
            rgba_width,
            rgba_height,
        )
    }

    pub(crate) fn create_image_draw(
        &self,
        image_key: String,
        transform: Transform2D,
        rect: Box2D,
    ) -> RetainedImageDraw {
        let corners = transformed_corners(&rect, &transform);
        let verts = corners_to_texture_vertices(corners);
        let resource =
            self.texture_renderer
                .create_retained_image_resource(&self.device, image_key, verts);
        RetainedImageDraw { resource }
    }

    #[allow(dead_code)]
    pub(crate) fn update_image_draw(
        &self,
        draw: &RetainedImageDraw,
        transform: Transform2D,
        rect: Box2D,
    ) {
        let corners = transformed_corners(&rect, &transform);
        let verts = corners_to_texture_vertices(corners);
        self.texture_renderer
            .update_retained_image_resource(&self.queue, &draw.resource, verts);
    }

    #[allow(dead_code)]
    pub(crate) fn draw_image_resource(
        &mut self,
        texture: &CachedTextureResource,
        draw: &RetainedImageDraw,
    ) {
        let clear_target = std::mem::take(&mut self.pending_clear);
        self.ensure_active_frame();
        let (screen_texture, resolve_target) = self.current_color_attachment_views();
        self.texture_renderer.draw_retained_image(
            &self.device,
            &self.queue,
            screen_texture,
            resolve_target,
            &self.stencil_renderer,
            clear_target,
            texture,
            &draw.resource,
        );
    }

    fn write_buffers(&mut self, buffers: &mut CpuBuffers) {
        let CpuBuffers {
            geometry: ref mut geom,
            ref mut primitives,
            ref mut colors,
            ref mut gradients,
            ref mut transforms,
        } = buffers;
        //Add ghost triangles to follow COPY_BUFFER_ALIGNMENT requirement
        const ALIGNMENT: usize = 16;
        while geom.indices.len() * std::mem::size_of::<u16>() % ALIGNMENT != 0 {
            geom.indices.push(0);
            geom.indices.push(0);
            geom.indices.push(0);
        }
        while geom.vertices.len() * std::mem::size_of::<GpuVertex>() % ALIGNMENT != 0 {
            geom.vertices.push(GpuVertex::default());
        }
        while primitives.len() * std::mem::size_of::<GpuPrimitive>() % ALIGNMENT != 0 {
            primitives.push(GpuPrimitive::default());
        }
        while transforms.len() * std::mem::size_of::<GpuTransform>() % ALIGNMENT != 0 {
            transforms.push(GpuTransform::default());
        }
        while colors.len() * std::mem::size_of::<GpuColor>() % ALIGNMENT != 0 {
            colors.push(GpuColor::default());
        }
        while gradients.len() * std::mem::size_of::<GpuGradient>() % ALIGNMENT != 0 {
            gradients.push(GpuGradient::default());
        }

        self.resize_shared_buffers_if_needed(
            geom.indices.len(),
            geom.vertices.len(),
            primitives.len(),
            transforms.len(),
            colors.len(),
            gradients.len(),
        );

        write_u16_buffer_padded(&self.queue, &self.index_buffer, &geom.indices);
        self.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&geom.vertices));
        self.queue
            .write_buffer(&self.primitive_buffer, 0, bytemuck::cast_slice(primitives));
        self.queue
            .write_buffer(&self.colors_buffer, 0, bytemuck::cast_slice(colors));
        self.queue
            .write_buffer(&self.gradients_buffer, 0, bytemuck::cast_slice(gradients));
        self.queue
            .write_buffer(&self.transforms_buffer, 0, bytemuck::cast_slice(transforms));

        self.index_count = geom.indices.len() as u64;
    }

    pub(crate) fn render_primitives(&mut self, buffers: &mut CpuBuffers) {
        self.write_buffers(buffers);
        let load_op = self.take_color_load_op();
        self.ensure_active_frame();
        let (screen_texture, resolve_target) = self.current_color_attachment_views();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let (stencil_texture, stencil_index) = self.stencil_renderer.get_stencil();
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: screen_texture,
                    depth_slice: None,
                    resolve_target,
                    ops: wgpu::Operations {
                        load: load_op,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: stencil_texture,
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
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_stencil_reference(stencil_index); //this needs to be dynamic?
            render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.index_count as u32, 0, 0..1);
        }

        //render primitives
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    fn ensure_active_frame(&mut self) {
        if self.active_frame.is_none() {
            let surface = self.surface.get_current_texture().unwrap();
            let view = surface
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            self.active_frame = Some(ActiveFrame { view, surface });
        }
    }

    fn take_color_load_op(&mut self) -> wgpu::LoadOp<wgpu::Color> {
        if std::mem::take(&mut self.pending_clear) {
            wgpu::LoadOp::Clear(wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            })
        } else {
            wgpu::LoadOp::Load
        }
    }

    #[allow(dead_code)]
    pub(crate) fn render_image(&mut self, image: &Image, transform: Transform2D, rect: Box2D) {
        let clear_target = std::mem::take(&mut self.pending_clear);
        self.ensure_active_frame();
        let (screen_texture, resolve_target) = self.current_color_attachment_views();
        self.texture_renderer.render_image(
            &self.device,
            &self.queue,
            screen_texture,
            resolve_target,
            &self.globals_buffer,
            &self.stencil_renderer,
            clear_target,
            &image.rgba,
            image.pixel_width,
            transform,
            rect,
        );
    }

    pub(crate) fn clear(&mut self) {
        self.stencil_renderer.clear(&self.device, &self.queue);
        self.pending_clear = true;
    }

    pub(crate) fn present(&mut self) {
        if self.pending_clear {
            let load_op = self.take_color_load_op();
            self.ensure_active_frame();
            let (screen_texture, resolve_target) = self.current_color_attachment_views();
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

            {
                let _r = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: screen_texture,
                        depth_slice: None,
                        resolve_target,
                        ops: wgpu::Operations {
                            load: load_op,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
            }
            self.queue.submit(std::iter::once(encoder.finish()));
        }

        if let Some(screen_surface) = self.active_frame.take() {
            screen_surface.surface.present();
        }
    }

    fn current_color_attachment_views(&self) -> (&TextureView, Option<&TextureView>) {
        let surface_view = &self
            .active_frame
            .as_ref()
            .expect("active frame should exist after acquisition")
            .view;
        if let Some(multisampled_target) = &self.multisampled_target {
            (&multisampled_target.view, Some(surface_view))
        } else {
            (surface_view, None)
        }
    }
}

fn select_sample_count(
    surface_format_features: TextureFormatFeatureFlags,
    stencil_format_features: TextureFormatFeatureFlags,
) -> u32 {
    [4, 2]
        .into_iter()
        .find(|count| {
            surface_format_features.sample_count_supported(*count)
                && surface_format_features.contains(TextureFormatFeatureFlags::MULTISAMPLE_RESOLVE)
                && stencil_format_features.sample_count_supported(*count)
        })
        .unwrap_or(1)
}

fn aligned_cpu_buffers(
    buffers: &mut CpuBuffers,
) -> (
    Vec<GpuVertex>,
    Vec<u16>,
    Vec<GpuPrimitive>,
    Vec<GpuTransform>,
    Vec<GpuColor>,
    Vec<GpuGradient>,
) {
    let CpuBuffers {
        geometry,
        primitives,
        transforms,
        colors,
        gradients,
    } = buffers;
    let mut indices = geometry.indices.clone();
    let mut vertices = geometry.vertices.clone();
    let mut primitives = primitives.clone();
    let mut transforms = transforms.clone();
    let mut colors = colors.clone();
    let mut gradients = gradients.clone();

    const ALIGNMENT: usize = 16;
    if primitives.is_empty() {
        primitives.push(GpuPrimitive::default());
    }
    if transforms.is_empty() {
        transforms.push(GpuTransform::default());
    }
    if colors.is_empty() {
        colors.push(GpuColor::default());
    }
    if gradients.is_empty() {
        gradients.push(GpuGradient::default());
    }
    while indices.len() * std::mem::size_of::<u16>() % ALIGNMENT != 0 {
        indices.push(0);
        indices.push(0);
        indices.push(0);
    }
    while vertices.len() * std::mem::size_of::<GpuVertex>() % ALIGNMENT != 0 {
        vertices.push(GpuVertex::default());
    }
    while primitives.len() * std::mem::size_of::<GpuPrimitive>() % ALIGNMENT != 0 {
        primitives.push(GpuPrimitive::default());
    }
    while transforms.len() * std::mem::size_of::<GpuTransform>() % ALIGNMENT != 0 {
        transforms.push(GpuTransform::default());
    }
    while colors.len() * std::mem::size_of::<GpuColor>() % ALIGNMENT != 0 {
        colors.push(GpuColor::default());
    }
    while gradients.len() * std::mem::size_of::<GpuGradient>() % ALIGNMENT != 0 {
        gradients.push(GpuGradient::default());
    }

    (vertices, indices, primitives, transforms, colors, gradients)
}

fn transformed_corners(rect: &Box2D, transform: &Transform2D) -> [[f32; 2]; 4] {
    let min = rect.min;
    let max = rect.max;
    let corners = [
        crate::Point2D::new(min.x, min.y),
        crate::Point2D::new(max.x, min.y),
        crate::Point2D::new(min.x, max.y),
        crate::Point2D::new(max.x, max.y),
    ];
    [
        transform.transform_point(corners[0]).to_array(),
        transform.transform_point(corners[1]).to_array(),
        transform.transform_point(corners[2]).to_array(),
        transform.transform_point(corners[3]).to_array(),
    ]
}

#[derive(Debug)]
pub(crate) struct CpuBuffers {
    pub geometry: VertexBuffers<GpuVertex, u16>,
    pub primitives: Vec<GpuPrimitive>,
    pub transforms: Vec<GpuTransform>,
    pub colors: Vec<GpuColor>,
    pub gradients: Vec<GpuGradient>,
}

impl CpuBuffers {
    #[allow(dead_code)]
    pub(crate) fn reset(&mut self) {
        let CpuBuffers {
            geometry,
            primitives,
            transforms,
            colors,
            gradients,
        } = self;
        geometry.vertices.clear();
        geometry.indices.clear();
        primitives.clear();
        colors.clear();
        gradients.clear();
        // leave the identity transform at the start
        transforms.truncate(1);
    }
}

#[derive(Clone)]
pub struct Image {
    pub rgba: Vec<u8>,
    pub pixel_width: u32,
    pub pixel_height: u32,
}
