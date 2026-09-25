use anyhow::anyhow;
use bytemuck::Pod;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
#[cfg(any(target_os = "ios", target_os = "macos"))]
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use lyon::lyon_tessellation::VertexBuffers;
use wgpu::{
    util::{DeviceExt, StagingBelt},
    BindGroup, BindGroupLayout, BufferUsages, CommandBuffer, CompositeAlphaMode, Device,
    IndexFormat, PresentMode, RenderPipeline, SurfaceConfiguration, SurfaceTexture, Texture,
    TextureFormat, TextureFormatFeatureFlags, TextureUsages, TextureView,
};

pub(crate) mod alpha_mask;
pub mod data;
mod gpu_resources;
pub mod stencil;
mod texture;

#[cfg(all(test, target_os = "macos"))]
mod retained_clip_tests;

pub(crate) use texture::CachedTextureResource;

use data::{GpuGlobals, GpuPrimitive, GpuSceneLighting, GpuVertex};

use crate::{
    render_backend::texture::{
        corners_to_texture_vertices, RetainedImageResource, TexturePipelineResources,
        TextureRenderer,
    },
    Box2D, Transform2D,
};

use self::{
    data::{GpuColor, GpuGradient, GpuMaterial, GpuTransform},
    gpu_resources::create_multisampled_framebuffer,
    stencil::{StencilPipelineResources, StencilRenderer},
};

const TRANSPARENT_CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};

const OPAQUE_FALLBACK_CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};
const STAGING_BELT_CHUNK_SIZE: wgpu::BufferAddress = 256 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct VectorPipelineKey {
    format: TextureFormat,
    sample_count: u32,
}

/// Shared GPU device context used by sibling render surfaces.
pub struct GpuContext {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    max_surface_dimension: u32,
    primitive_bind_group_layout: BindGroupLayout,
    alpha_mask_layout: BindGroupLayout,
    vector_pipelines: RefCell<HashMap<VectorPipelineKey, Rc<RenderPipeline>>>,
    texture_pipelines: RefCell<HashMap<VectorPipelineKey, TexturePipelineResources>>,
    stencil_pipelines: RefCell<HashMap<VectorPipelineKey, StencilPipelineResources>>,
}

pub type SharedGpuContext = Rc<GpuContext>;

impl GpuContext {
    async fn new(
        instance: wgpu::Instance,
        compatible_surface: &wgpu::Surface<'_>,
        _config: &RenderConfig,
    ) -> Result<SharedGpuContext, anyhow::Error> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: default_power_preference(),
                compatible_surface: Some(compatible_surface),
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
        // Keep wgpu's conservative downlevel browser preset as the common WebGPU baseline. The
        // preset name reflects its upstream compatibility tier; it does not enable a GL backend.
        #[cfg(target_arch = "wasm32")]
        let required_limits = wgpu::Limits {
            // Interrupted paint mixtures have a variable number of endpoints.
            max_storage_buffers_per_shader_stage: 1,
            max_storage_buffer_binding_size: 128 * 1024 * 1024,
            ..wgpu::Limits::downlevel_webgl2_defaults()
        };
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
        let max_surface_dimension = device.limits().max_texture_dimension_2d.max(1);
        log::info!(
            "render backend: max surface dimension {}",
            max_surface_dimension
        );
        let primitive_bind_group_layout = create_primitive_bind_group_layout(&device);
        let alpha_mask_layout = alpha_mask::sampling_layout(&device);

        Ok(Rc::new(Self {
            instance,
            adapter,
            device,
            queue,
            max_surface_dimension,
            primitive_bind_group_layout,
            alpha_mask_layout,
            vector_pipelines: RefCell::new(HashMap::new()),
            texture_pipelines: RefCell::new(HashMap::new()),
            stencil_pipelines: RefCell::new(HashMap::new()),
        }))
    }

    fn vector_pipeline(&self, format: TextureFormat, sample_count: u32) -> Rc<RenderPipeline> {
        let key = VectorPipelineKey {
            format,
            sample_count,
        };
        if let Some(pipeline) = self.vector_pipelines.borrow().get(&key) {
            return Rc::clone(pipeline);
        }

        let pipeline = Rc::new(RenderBackend::create_pipeline(
            &self.device,
            format,
            sample_count,
            &self.primitive_bind_group_layout,
            &self.alpha_mask_layout,
        ));
        self.vector_pipelines
            .borrow_mut()
            .insert(key, Rc::clone(&pipeline));
        pipeline
    }

    fn texture_pipeline_resources(
        &self,
        format: TextureFormat,
        sample_count: u32,
    ) -> TexturePipelineResources {
        let key = VectorPipelineKey {
            format,
            sample_count,
        };
        if let Some(resources) = self.texture_pipelines.borrow().get(&key) {
            return resources.clone();
        }

        let resources = TextureRenderer::create_pipeline_resources(
            &self.device,
            format,
            sample_count,
            &self.alpha_mask_layout,
        );
        self.texture_pipelines
            .borrow_mut()
            .insert(key, resources.clone());
        resources
    }

    fn stencil_pipeline_resources(
        &self,
        format: TextureFormat,
        sample_count: u32,
    ) -> StencilPipelineResources {
        let key = VectorPipelineKey {
            format,
            sample_count,
        };
        if let Some(resources) = self.stencil_pipelines.borrow().get(&key) {
            return resources.clone();
        }

        let resources =
            StencilRenderer::create_pipeline_resources(&self.device, sample_count, format);
        self.stencil_pipelines
            .borrow_mut()
            .insert(key, resources.clone());
        resources
    }

    pub(crate) fn submit_command_buffers<I>(&self, command_buffers: I)
    where
        I: IntoIterator<Item = CommandBuffer>,
    {
        self.queue.submit(command_buffers);
    }
}

fn surface_clear_color(alpha_mode: CompositeAlphaMode) -> wgpu::Color {
    if alpha_mode == CompositeAlphaMode::Opaque {
        // Opaque browser fallbacks expose the clear RGB directly; keep the old white default there.
        OPAQUE_FALLBACK_CLEAR_COLOR
    } else {
        // Alpha-capable surfaces must clear to transparent black so translucent geometry does not
        // accumulate white RGB in otherwise-transparent tile regions.
        TRANSPARENT_CLEAR_COLOR
    }
}

fn default_power_preference() -> wgpu::PowerPreference {
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        wgpu::PowerPreference::HighPerformance
    }

    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        wgpu::PowerPreference::LowPower
    }
}

/// GPU resource sizing and initial surface configuration.
pub struct RenderConfig {
    pub debug: bool,
    index_buffer_size: u64,
    vertex_buffer_size: u64,
    primitive_buffer_size: u64,
    colors_buffer_size: u64,
    gradients_buffer_size: u64,
    materials_buffer_size: u64,
    transforms_buffer_size: u64,
    clip_transforms_buffer_size: u64,
    pub initial_width: u32,
    pub initial_height: u32,
    pub initial_dpr: [f32; 2],
    prefer_browser_premultiplied_alpha: bool,
}

pub(crate) const MAX_BATCH_PRIMITIVES: usize = 512;
pub(crate) const MAX_BATCH_COLORS: usize = 512;
pub(crate) const MAX_BATCH_GRADIENTS: usize = 64;
pub(crate) const MAX_BATCH_MATERIALS: usize = 512;
pub(crate) const MAX_BATCH_TRANSFORMS: usize = 480;
// Downlevel browser adapters can expose a 16 KiB max uniform binding size. Keep the scene
// transform uniform arena under that ceiling.
pub(crate) const MAX_SCENE_TRANSFORMS: usize = 480;
pub(crate) const MAX_SCENE_CLIPS: usize = 480;

impl RenderConfig {
    /// Construct default buffer capacities for an initial surface size.
    pub fn new(_debug: bool, width: u32, height: u32, dpr: [f32; 2]) -> Self {
        Self {
            debug: false,
            index_buffer_size: 2 << 12,
            vertex_buffer_size: 2 << 12,
            primitive_buffer_size: MAX_BATCH_PRIMITIVES as u64,
            colors_buffer_size: MAX_BATCH_COLORS as u64,
            gradients_buffer_size: MAX_BATCH_GRADIENTS as u64,
            materials_buffer_size: MAX_BATCH_MATERIALS as u64,
            transforms_buffer_size: MAX_SCENE_TRANSFORMS as u64,
            clip_transforms_buffer_size: MAX_SCENE_CLIPS as u64,
            initial_width: width,
            initial_height: height,
            initial_dpr: dpr,
            prefer_browser_premultiplied_alpha: false,
        }
    }

    /// Request premultiplied browser canvas alpha for WebGPU surfaces.
    ///
    /// wgpu's web backend currently reports only `Opaque` alpha in surface capabilities, but its
    /// configure path accepts `PreMultiplied` and maps it to `GPUCanvasAlphaMode::Premultiplied`.
    /// Pax needs that for transparent browser-owned scroller islands.
    pub fn with_browser_premultiplied_alpha(mut self, enabled: bool) -> Self {
        self.prefer_browser_premultiplied_alpha = enabled;
        self
    }
}

#[cfg(target_arch = "wasm32")]
fn sync_browser_canvas_backing_size(canvas: &web_sys::HtmlCanvasElement, width: u32, height: u32) {
    if canvas.width() != width {
        canvas.set_width(width);
    }
    if canvas.height() != height {
        canvas.set_height(height);
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

fn write_staged_buffer(
    staging_belt: &mut StagingBelt,
    encoder: &mut wgpu::CommandEncoder,
    buffer: &wgpu::Buffer,
    bytes: &[u8],
) {
    if bytes.is_empty() {
        return;
    }

    let size = wgpu::BufferSize::new(bytes.len() as wgpu::BufferAddress)
        .expect("staged buffer writes must be non-empty");
    let mut view = staging_belt.write_buffer(encoder, buffer, 0, size);
    view.copy_from_slice(bytes);
}

fn physical_scissor_rect(
    scissor: Option<ScissorRect>,
    dpr: [f32; 2],
    surface_width: u32,
    surface_height: u32,
) -> Option<(u32, u32, u32, u32)> {
    let Some(scissor) = scissor else {
        return Some((0, 0, surface_width.max(1), surface_height.max(1)));
    };

    let dpr_x = dpr[0].max(1.0);
    let dpr_y = dpr[1].max(1.0);
    let min_x = (scissor.min_x.min(scissor.max_x) * dpr_x)
        .floor()
        .clamp(0.0, surface_width as f32) as u32;
    let min_y = (scissor.min_y.min(scissor.max_y) * dpr_y)
        .floor()
        .clamp(0.0, surface_height as f32) as u32;
    let max_x = (scissor.min_x.max(scissor.max_x) * dpr_x)
        .ceil()
        .clamp(0.0, surface_width as f32) as u32;
    let max_y = (scissor.min_y.max(scissor.max_y) * dpr_y)
        .ceil()
        .clamp(0.0, surface_height as f32) as u32;
    let width = max_x.saturating_sub(min_x);
    let height = max_y.saturating_sub(min_y);
    if width == 0 || height == 0 {
        return None;
    }

    Some((min_x, min_y, width, height))
}

/// Low-level wgpu backend that owns surface, pipeline, and GPU buffers.
pub struct RenderBackend<'w> {
    context: SharedGpuContext,
    //configuration
    config: RenderConfig,
    pub(crate) globals: GpuGlobals,

    //gpu pipeline references
    device: wgpu::Device,
    queue: wgpu::Queue,
    staging_belt: StagingBelt,
    surface: wgpu::Surface<'w>,
    #[cfg(target_arch = "wasm32")]
    browser_canvas: Option<web_sys::HtmlCanvasElement>,
    surface_config: SurfaceConfiguration,
    max_surface_dimension: u32,
    pipeline: Rc<RenderPipeline>,
    bind_group: BindGroup,

    //buffers
    globals_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    primitive_buffer: wgpu::Buffer,
    transforms_buffer: wgpu::Buffer,
    clip_transforms_buffer: wgpu::Buffer,
    colors_buffer: wgpu::Buffer,
    gradients_buffer: wgpu::Buffer,
    materials_buffer: wgpu::Buffer,
    lighting_buffer: wgpu::Buffer,

    index_count: u64,

    // plugins / extensions
    texture_renderer: TextureRenderer,
    stencil_renderer: StencilRenderer,
    pub(crate) alpha_masks: alpha_mask::AlphaMasks,
    multisampled_target: Option<MultisampledTarget>,
    sample_count: u32,
    clear_color: wgpu::Color,
    active_frame: Option<ActiveFrame>,
    capture_target: Option<CaptureTarget>,
    pending_clear: bool,
    pending_command_buffers: Vec<CommandBuffer>,
    staging_belt_pending_recall: bool,
    needs_device_poll: bool,
    pending_capture_ids: Vec<u32>,
    completed_captures: Arc<Mutex<HashMap<u32, CapturedFrame>>>,
}

struct ActiveFrame {
    view: TextureView,
    surface: SurfaceTexture,
}

struct MultisampledTarget {
    _texture: Texture,
    view: TextureView,
}

struct CaptureTarget {
    texture: Texture,
    view: TextureView,
    multisampled_target: Option<MultisampledTarget>,
    initialized: bool,
}

impl CaptureTarget {
    fn color_attachment_views(&self) -> (&TextureView, Option<&TextureView>) {
        if let Some(multisampled_target) = &self.multisampled_target {
            (&multisampled_target.view, Some(&self.view))
        } else {
            (&self.view, None)
        }
    }
}

#[derive(Clone)]
/// CPU-readable screenshot payload captured from a rendered frame.
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub(crate) struct SharedRetainedVectorResource {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    vertex_capacity: usize,
    index_capacity: usize,
    primitive_capacity: usize,
    color_capacity: usize,
    gradient_capacity: usize,
    material_capacity: usize,
    _primitive_buffer: wgpu::Buffer,
    _colors_buffer: wgpu::Buffer,
    _gradients_buffer: wgpu::Buffer,
    _materials_buffer: wgpu::Buffer,
}

pub(crate) struct RetainedVectorResource {
    bind_group: BindGroup,
    shared: Rc<SharedRetainedVectorResource>,
}

impl RetainedVectorResource {
    pub(crate) fn shared(&self) -> &Rc<SharedRetainedVectorResource> {
        &self.shared
    }

    pub(crate) fn is_drawable(&self) -> bool {
        self.shared.index_count > 0
            && self.shared.vertex_capacity > 0
            && self.shared.index_capacity > 0
    }
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

pub(crate) struct RetainedBatchRun<'a> {
    pub alpha_mask: Option<u32>,
    pub stencil_index: u32,
    pub scissor: Option<ScissorRect>,
    pub draws: Vec<RetainedDraw<'a>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScissorRect {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

pub(crate) struct PrimitiveBatch<'a> {
    pub buffers: CpuBuffers,
    pub segments: Vec<PrimitiveBatchSegment<'a>>,
}

pub(crate) struct PrimitiveBatchSegment<'a> {
    pub alpha_mask: Option<u32>,
    pub stencil_depth: u32,
    pub clips: Vec<stencil::ClipDraw<'a>>,
    pub scissor: Option<ScissorRect>,
    pub index_start: u32,
    pub index_count: u32,
}

struct PrimitiveBatchRenderPlan {
    alpha_mask: Option<u32>,
    stencil_sync: stencil::PreparedStencilSync,
    stencil_reference: u32,
    scissor: Option<(u32, u32, u32, u32)>,
    index_start: u32,
    index_count: u32,
}

fn create_primitive_bind_group_layout(device: &Device) -> BindGroupLayout {
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
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 5,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 6,
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
    })
}

pub(crate) struct RetainedImageDraw {
    pub resource: RetainedImageResource,
}

impl<'w> RenderBackend<'w> {
    fn rebuild_main_bind_group(&mut self) {
        self.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.context.primitive_bind_group_layout,
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
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.materials_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: self.lighting_buffer.as_entire_binding(),
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
        required_materials: usize,
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
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            rebind_main_group = true;
        }

        if required_materials > self.config.materials_buffer_size as usize {
            self.config.materials_buffer_size = next_capacity(required_materials);
            self.materials_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Materials Buffer"),
                size: self.config.materials_buffer_size * std::mem::size_of::<GpuMaterial>() as u64,
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
        let (backend, _) = Self::to_canvas_with_context(canvas, config, None).await?;
        Ok(backend)
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn to_canvas_with_context(
        canvas: web_sys::HtmlCanvasElement,
        config: RenderConfig,
        shared_context: Option<SharedGpuContext>,
    ) -> Result<(Self, SharedGpuContext), anyhow::Error> {
        sync_browser_canvas_backing_size(&canvas, config.initial_width, config.initial_height);
        if let Some(context) = shared_context {
            let surface_target = wgpu::SurfaceTarget::Canvas(canvas.clone());
            let surface = context.instance.create_surface(surface_target)?;
            let mut backend = Self::new_with_context(surface, Rc::clone(&context), config)?;
            backend.browser_canvas = Some(canvas);
            return Ok((backend, context));
        }

        let instance = Self::new_browser_instance(config.debug).await;
        let surface_target = wgpu::SurfaceTarget::Canvas(canvas.clone());
        let surface = instance.create_surface(surface_target)?;
        let context = GpuContext::new(instance, &surface, &config).await?;
        let mut backend = Self::new_with_context(surface, Rc::clone(&context), config)?;
        backend.browser_canvas = Some(canvas);
        Ok((backend, context))
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

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn to_canvas_with_context(
        _canvas: web_sys::HtmlCanvasElement,
        _config: RenderConfig,
        _shared_context: Option<SharedGpuContext>,
    ) -> Result<(Self, SharedGpuContext), anyhow::Error> {
        Err(anyhow!(
            "canvas surfaces are only supported on wasm32 targets"
        ))
    }

    #[cfg(any(target_os = "ios", target_os = "macos"))]
    pub async unsafe fn to_core_animation_layer(
        layer: *mut c_void,
        config: RenderConfig,
    ) -> Result<RenderBackend<'static>, anyhow::Error> {
        let (backend, _) =
            unsafe { Self::to_core_animation_layer_with_context(layer, config, None).await? };
        Ok(backend)
    }

    #[cfg(any(target_os = "ios", target_os = "macos"))]
    pub async unsafe fn to_core_animation_layer_with_context(
        layer: *mut c_void,
        config: RenderConfig,
        shared_context: Option<SharedGpuContext>,
    ) -> Result<(RenderBackend<'static>, SharedGpuContext), anyhow::Error> {
        if let Some(context) = shared_context {
            let surface = unsafe {
                context
                    .instance
                    .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer))?
            };
            let backend =
                RenderBackend::<'static>::new_with_context(surface, Rc::clone(&context), config)?;
            return Ok((backend, context));
        }

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
        let context = GpuContext::new(instance, &surface, &config).await?;
        let backend =
            RenderBackend::<'static>::new_with_context(surface, Rc::clone(&context), config)?;
        Ok((backend, context))
    }

    pub async fn new(
        surface: wgpu::Surface<'w>,
        instance: wgpu::Instance,
        config: RenderConfig,
    ) -> Result<Self, anyhow::Error> {
        let context = GpuContext::new(instance, &surface, &config).await?;
        Self::new_with_context(surface, context, config)
    }

    pub fn new_with_context(
        surface: wgpu::Surface<'w>,
        context: SharedGpuContext,
        config: RenderConfig,
    ) -> Result<Self, anyhow::Error> {
        let adapter = &context.adapter;
        let device = context.device.clone();
        let queue = context.queue.clone();
        let staging_belt = StagingBelt::new(device.clone(), STAGING_BELT_CHUNK_SIZE);
        let max_surface_dimension = context.max_surface_dimension;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|format| {
                matches!(
                    format,
                    TextureFormat::Bgra8Unorm | TextureFormat::Rgba8Unorm
                )
            })
            .or_else(|| {
                surface_caps
                    .formats
                    .iter()
                    .copied()
                    .find(|format| format.is_srgb())
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
        let alpha_mode =
            if cfg!(target_arch = "wasm32") && config.prefer_browser_premultiplied_alpha {
                CompositeAlphaMode::PreMultiplied
            } else {
                [
                    CompositeAlphaMode::PreMultiplied,
                    CompositeAlphaMode::PostMultiplied,
                    CompositeAlphaMode::Opaque,
                ]
                .into_iter()
                .find(|mode| surface_caps.alpha_modes.contains(mode))
                .or_else(|| surface_caps.alpha_modes.first().copied())
                .ok_or_else(|| anyhow!("surface reported no compatible alpha modes"))?
            };
        #[cfg(target_arch = "wasm32")]
        if alpha_mode == CompositeAlphaMode::Opaque {
            log::debug!("render backend: browser surface only exposes opaque alpha");
        }
        let surface_format_features = adapter.get_texture_format_features(surface_format).flags;
        #[cfg(target_arch = "wasm32")]
        let stencil_format_features = {
            let queried = adapter
                .get_texture_format_features(wgpu::TextureFormat::Stencil8)
                .flags;
            if queried.is_empty() {
                surface_format_features
            } else {
                queried
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        let stencil_format_features = adapter
            .get_texture_format_features(wgpu::TextureFormat::Stencil8)
            .flags;
        let sample_count = select_sample_count(surface_format_features, stencil_format_features);
        let mut surface_usage = TextureUsages::RENDER_ATTACHMENT;
        if surface_caps.usages.contains(TextureUsages::COPY_SRC) {
            surface_usage |= TextureUsages::COPY_SRC;
        } else {
            #[cfg(target_arch = "wasm32")]
            log::debug!(
                "render backend: surface does not support COPY_SRC; screenshots will use mirror target"
            );
        }
        let initial_width = config.initial_width.max(1).min(max_surface_dimension);
        let initial_height = config.initial_height.max(1).min(max_surface_dimension);
        if initial_width != config.initial_width || initial_height != config.initial_height {
            log::warn!(
                "render backend: clamped initial surface size from {}x{} to {}x{} (max {})",
                config.initial_width,
                config.initial_height,
                initial_width,
                initial_height,
                max_surface_dimension
            );
        }
        let surface_config = SurfaceConfiguration {
            usage: surface_usage,
            format: surface_format,
            width: initial_width,
            height: initial_height,
            present_mode: PresentMode::Fifo,
            alpha_mode,
            view_formats: vec![],
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
            resolution: [initial_width as f32, initial_height as f32],
            dpr: config.initial_dpr,
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
        let (_, clip_transforms_buffer) = create_buffer::<GpuTransform>(
            &device,
            "Clip Transform Buffer",
            config.clip_transforms_buffer_size,
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
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let (_, materials_buffer) = create_buffer::<GpuMaterial>(
            &device,
            "Materials Buffer",
            config.materials_buffer_size,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let (_, lighting_buffer) = create_buffer::<GpuSceneLighting>(
            &device,
            "Scene Lighting Buffer",
            1,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &context.primitive_bind_group_layout,
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
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: materials_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: lighting_buffer.as_entire_binding(),
                },
            ],
            label: Some("bind_group"),
        });

        let pipeline = context.vector_pipeline(surface_config.format, sample_count);

        let texture_renderer = TextureRenderer::with_pipeline_resources(
            &device,
            context.texture_pipeline_resources(surface_config.format, sample_count),
        );
        let stencil_renderer = StencilRenderer::with_pipeline_resources(
            &device,
            initial_width,
            initial_height,
            sample_count,
            &globals_buffer,
            &clip_transforms_buffer,
            context.stencil_pipeline_resources(surface_config.format, sample_count),
        );

        let initial_width = initial_width;
        let initial_height = initial_height;
        let initial_dpr = config.initial_dpr;
        let alpha_masks =
            alpha_mask::AlphaMasks::new(&device, &queue, context.alpha_mask_layout.clone());
        let mut backend = Self {
            alpha_masks,
            context: Rc::clone(&context),
            texture_renderer,
            stencil_renderer,
            surface,
            #[cfg(target_arch = "wasm32")]
            browser_canvas: None,
            device,
            queue,
            staging_belt,
            config,
            surface_config,
            max_surface_dimension,
            bind_group,
            pipeline,
            vertex_buffer,
            index_buffer,
            primitive_buffer,
            transforms_buffer,
            clip_transforms_buffer,
            globals_buffer,
            colors_buffer,
            gradients_buffer,
            materials_buffer,
            lighting_buffer,
            globals,
            index_count: 0,
            multisampled_target: None,
            sample_count,
            clear_color: surface_clear_color(alpha_mode),
            active_frame: None,
            capture_target: None,
            pending_clear: false,
            pending_command_buffers: Vec::new(),
            staging_belt_pending_recall: false,
            needs_device_poll: false,
            pending_capture_ids: Vec::new(),
            completed_captures: Arc::new(Mutex::new(HashMap::new())),
        };
        backend.globals.dpr = initial_dpr;
        backend.resize(initial_width, initial_height);
        Ok(backend)
    }

    #[cfg(target_arch = "wasm32")]
    async fn new_browser_instance(debug: bool) -> wgpu::Instance {
        let descriptor = wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            flags: if debug {
                wgpu::InstanceFlags::DEBUG
            } else {
                wgpu::InstanceFlags::default()
            },
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
        };

        wgpu::util::new_instance_with_webgpu_detection(&descriptor).await
    }

    fn create_pipeline(
        device: &Device,
        format: TextureFormat,
        sample_count: u32,
        primitive_bind_group_layout: &BindGroupLayout,
        alpha_mask_layout: &BindGroupLayout,
    ) -> RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("radial_gradient.wgsl"),
                    "\n",
                    include_str!("geometry.wgsl")
                )
                .into(),
            ),
        });
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[primitive_bind_group_layout, alpha_mask_layout],
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

    /// Current stencil clip depth.
    pub fn get_clip_depth(&mut self) -> u32 {
        let (_, depth) = self.stencil_renderer.get_stencil();
        depth
    }

    /// Resize both the physical surface and logical viewport.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.resize_surface(width, height);
        self.set_viewport(width as f32, height as f32, self.globals.dpr);
    }

    /// Resize the backing surface, clamping to device limits.
    pub fn resize_surface(&mut self, width: u32, height: u32) {
        let max_dim = self.max_surface_dimension.max(1);
        let requested_width = width.max(1);
        let requested_height = height.max(1);
        let width = requested_width.min(max_dim);
        let height = requested_height.min(max_dim);
        if width != requested_width || height != requested_height {
            log::warn!(
                "render backend: clamped surface resize from {}x{} to {}x{} (max {})",
                requested_width,
                requested_height,
                width,
                height,
                max_dim
            );
        }
        self.active_frame = None;
        self.capture_target = None;
        // New backing attachments have undefined contents. Force the next render to clear before
        // translucent retained geometry blends over the target.
        self.pending_clear = true;
        self.pending_command_buffers.clear();
        // Resize abandons encoded uploads. Forget their mask signatures and
        // staging allocations instead of recalling work that was never submitted.
        // Dropped WGPU handles remain alive for any older in-flight commands.
        self.alpha_masks.invalidate();
        self.staging_belt = StagingBelt::new(self.device.clone(), STAGING_BELT_CHUNK_SIZE);
        self.staging_belt_pending_recall = false;
        self.needs_device_poll = false;
        self.surface_config.width = width;
        self.surface_config.height = height;
        #[cfg(target_arch = "wasm32")]
        if let Some(canvas) = self.browser_canvas.as_ref() {
            sync_browser_canvas_backing_size(canvas, width, height);
        }
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

    /// Update logical viewport uniforms without reallocating the surface.
    pub fn set_viewport(&mut self, width: f32, height: f32, dpr: [f32; 2]) {
        self.globals.resolution = [width.max(1.0), height.max(1.0)];
        self.globals.dpr = dpr;
        self.queue.write_buffer(
            &self.globals_buffer,
            0,
            bytemuck::cast_slice(&[self.globals]),
        );
    }

    fn enqueue_command_buffer(&mut self, command_buffer: CommandBuffer) {
        self.pending_command_buffers.push(command_buffer);
    }

    fn enqueue_staged_command_buffer(&mut self, encoder: wgpu::CommandEncoder) {
        self.staging_belt.finish();
        self.enqueue_command_buffer(encoder.finish());
        self.staging_belt_pending_recall = true;
        self.needs_device_poll = true;
    }

    pub(crate) fn take_pending_command_buffers(&mut self) -> Vec<CommandBuffer> {
        std::mem::take(&mut self.pending_command_buffers)
    }

    pub(crate) fn has_pending_submission_cleanup(&self) -> bool {
        self.staging_belt_pending_recall || self.needs_device_poll
    }

    pub(crate) fn complete_submitted_work(&mut self) {
        if self.staging_belt_pending_recall {
            self.staging_belt.recall();
            self.staging_belt_pending_recall = false;
        }
        if self.needs_device_poll {
            let _ = self.device.poll(wgpu::PollType::Poll);
            self.needs_device_poll = false;
        }
    }

    pub(crate) fn submit_command_buffers(&self, command_buffers: Vec<CommandBuffer>) {
        self.context.submit_command_buffers(command_buffers);
    }

    pub(crate) fn submit_pending_commands(&mut self) {
        let command_buffers = self.take_pending_command_buffers();
        if !command_buffers.is_empty() {
            self.context.submit_command_buffers(command_buffers);
        }
        self.complete_submitted_work();
    }

    /// Maximum texture dimension supported by the active adapter.
    pub fn max_surface_dimension(&self) -> u32 {
        self.max_surface_dimension
    }

    pub fn shared_context_id(&self) -> usize {
        Rc::as_ptr(&self.context) as usize
    }

    pub(crate) fn request_screenshot_capture(&mut self, request_id: u32) {
        self.pending_capture_ids.push(request_id);
        if !self.surface_supports_copy_src() {
            self.ensure_capture_target();
        }
    }

    pub(crate) fn take_screenshot_capture(&mut self, request_id: u32) -> Option<CapturedFrame> {
        self.completed_captures.lock().ok()?.remove(&request_id)
    }

    fn create_retained_bind_group(
        &self,
        primitive_buffer: &wgpu::Buffer,
        colors_buffer: &wgpu::Buffer,
        gradients_buffer: &wgpu::Buffer,
        materials_buffer: &wgpu::Buffer,
    ) -> BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.context.primitive_bind_group_layout,
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
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: materials_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: self.lighting_buffer.as_entire_binding(),
                },
            ],
            label: Some("retained_bind_group"),
        })
    }

    pub(crate) fn create_vector_resource_from_shared(
        &self,
        shared: Rc<SharedRetainedVectorResource>,
    ) -> RetainedVectorResource {
        RetainedVectorResource {
            bind_group: self.create_retained_bind_group(
                &shared._primitive_buffer,
                &shared._colors_buffer,
                &shared._gradients_buffer,
                &shared._materials_buffer,
            ),
            shared,
        }
    }

    pub(crate) fn create_shared_vector_resource(
        &self,
        buffers: &mut CpuBuffers,
        retained_primitives: &[GpuPrimitive],
    ) -> (Rc<SharedRetainedVectorResource>, u64) {
        let (vertices, indices, _, _, mut colors, gradients, mut materials) =
            aligned_cpu_buffers(buffers);
        let vertex_capacity = vertices.len();
        let index_capacity = indices.len();
        let primitive_capacity = retained_primitives.len();
        let color_capacity = colors.len();
        let gradient_capacity = gradients.len();
        let material_capacity = materials.len();
        let mut primitives = retained_primitives.to_vec();
        primitives.resize(
            self.config.primitive_buffer_size as usize,
            GpuPrimitive::default(),
        );
        colors.resize(self.config.colors_buffer_size as usize, GpuColor::default());
        materials.resize(
            self.config.materials_buffer_size as usize,
            GpuMaterial::default(),
        );
        let upload_bytes = (std::mem::size_of_val(vertices.as_slice())
            + std::mem::size_of_val(indices.as_slice())
            + std::mem::size_of_val(primitives.as_slice())
            + std::mem::size_of_val(colors.as_slice())
            + std::mem::size_of_val(gradients.as_slice())
            + std::mem::size_of_val(materials.as_slice())) as u64;
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
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            });
        let materials_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Retained Material Buffer"),
                contents: bytemuck::cast_slice(&materials),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });
        (
            Rc::new(SharedRetainedVectorResource {
                vertex_buffer,
                index_buffer,
                index_count: buffers.geometry.indices.len() as u32,
                vertex_capacity,
                index_capacity,
                primitive_capacity,
                color_capacity,
                gradient_capacity,
                material_capacity,
                _primitive_buffer: primitive_buffer,
                _colors_buffer: colors_buffer,
                _gradients_buffer: gradients_buffer,
                _materials_buffer: materials_buffer,
            }),
            upload_bytes,
        )
    }

    pub(crate) fn update_vector_resource(
        &self,
        resource: &mut RetainedVectorResource,
        buffers: &mut CpuBuffers,
        retained_primitives: &[GpuPrimitive],
        dirty: VectorResourceDirty,
    ) -> Option<u64> {
        let shared = Rc::get_mut(&mut resource.shared)?;
        let mut geometry = None;
        if dirty.geometry {
            let aligned = aligned_geometry_buffers(buffers);
            if aligned.0.len() > shared.vertex_capacity || aligned.1.len() > shared.index_capacity {
                return None;
            }
            geometry = Some(aligned);
        }

        if dirty.primitives && retained_primitives.len() > shared.primitive_capacity {
            return None;
        }

        let mut fill = None;
        if dirty.fill {
            let colors = aligned_pod_slice(&buffers.colors);
            let gradients = aligned_pod_slice(&buffers.gradients);
            let materials = aligned_pod_slice(&buffers.materials);
            if colors.len() > shared.color_capacity
                || gradients.len() > shared.gradient_capacity
                || materials.len() > shared.material_capacity
            {
                return None;
            }
            fill = Some((colors, gradients, materials));
        }

        let mut upload_bytes = 0;
        if dirty.geometry {
            let (vertices, indices) =
                geometry.expect("aligned geometry should be available for dirty geometry");
            upload_bytes += std::mem::size_of_val(vertices.as_slice()) as u64;
            upload_bytes += std::mem::size_of_val(indices.as_slice()) as u64;
            self.queue
                .write_buffer(&shared.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
            write_u16_buffer_padded(&self.queue, &shared.index_buffer, &indices);
        }
        if dirty.primitives {
            upload_bytes += std::mem::size_of_val(retained_primitives) as u64;
            self.queue.write_buffer(
                &shared._primitive_buffer,
                0,
                bytemuck::cast_slice(retained_primitives),
            );
        }
        if dirty.fill {
            let (colors, gradients, materials) =
                fill.expect("aligned fill should be available for dirty fill");
            upload_bytes += std::mem::size_of_val(colors.as_slice()) as u64;
            upload_bytes += std::mem::size_of_val(gradients.as_slice()) as u64;
            upload_bytes += std::mem::size_of_val(materials.as_slice()) as u64;
            self.queue
                .write_buffer(&shared._colors_buffer, 0, bytemuck::cast_slice(&colors));
            self.queue.write_buffer(
                &shared._gradients_buffer,
                0,
                bytemuck::cast_slice(&gradients),
            );
            self.queue.write_buffer(
                &shared._materials_buffer,
                0,
                bytemuck::cast_slice(&materials),
            );
        }
        shared.index_count = buffers.geometry.indices.len() as u32;
        Some(upload_bytes)
    }

    pub(crate) fn update_scene_transforms(&self, transforms: &[GpuTransform]) {
        self.queue
            .write_buffer(&self.transforms_buffer, 0, bytemuck::cast_slice(transforms));
    }

    pub(crate) fn update_scene_clip_transforms(&self, transforms: &[GpuTransform]) {
        self.queue.write_buffer(
            &self.clip_transforms_buffer,
            0,
            bytemuck::cast_slice(transforms),
        );
    }

    #[allow(dead_code)]
    pub(crate) fn draw_vector_resource(&mut self, resource: &RetainedVectorResource) {
        if !resource.is_drawable() {
            return;
        }

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
            render_pass.set_pipeline(self.pipeline.as_ref());
            render_pass.set_bind_group(0, &resource.bind_group, &[]);
            render_pass.set_bind_group(1, self.alpha_masks.bind_group(None), &[]);
            render_pass.set_vertex_buffer(0, resource.shared.vertex_buffer.slice(..));
            render_pass.set_stencil_reference(stencil_index);
            render_pass
                .set_index_buffer(resource.shared.index_buffer.slice(..), IndexFormat::Uint16);
            render_pass.draw_indexed(0..resource.shared.index_count, 0, 0..1);
        }

        if self.should_render_capture_target() {
            self.ensure_capture_target();
            let capture_load_op = self.take_capture_color_load_op(load_op);
            let capture_target = self.capture_target.as_ref().unwrap();
            let (capture_texture, capture_resolve_target) = capture_target.color_attachment_views();
            let (stencil_texture, stencil_index) = self.stencil_renderer.get_stencil();
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Retained Screenshot Mirror Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: capture_texture,
                    depth_slice: None,
                    resolve_target: capture_resolve_target,
                    ops: wgpu::Operations {
                        load: capture_load_op,
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
            render_pass.set_pipeline(self.pipeline.as_ref());
            render_pass.set_bind_group(0, &resource.bind_group, &[]);
            render_pass.set_bind_group(1, self.alpha_masks.bind_group(None), &[]);
            render_pass.set_vertex_buffer(0, resource.shared.vertex_buffer.slice(..));
            render_pass.set_stencil_reference(stencil_index);
            render_pass
                .set_index_buffer(resource.shared.index_buffer.slice(..), IndexFormat::Uint16);
            render_pass.draw_indexed(0..resource.shared.index_count, 0, 0..1);
        }

        self.enqueue_command_buffer(encoder.finish());
    }

    pub(crate) fn render_alpha_mask(
        &mut self,
        id: u32,
        signature: u64,
        draws: &[alpha_mask::Draw],
        feather: f32,
        parent: Option<u32>,
    ) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Alpha mask encoder"),
            });
        let rendered = self.alpha_masks.render(
            &self.device,
            &mut encoder,
            &mut self.staging_belt,
            id,
            signature,
            draws,
            [
                self.surface_config.width.max(1),
                self.surface_config.height.max(1),
            ],
            self.globals,
            feather,
            parent,
        );
        if rendered {
            self.enqueue_staged_command_buffer(encoder);
        }
    }

    pub(crate) fn draw_retained_batch_runs(&mut self, runs: &[RetainedBatchRun<'_>]) {
        if runs.iter().all(|run| run.draws.is_empty()) {
            return;
        }

        let physical_scissors: Vec<_> = runs
            .iter()
            .map(|run| {
                physical_scissor_rect(
                    run.scissor,
                    self.globals.dpr,
                    self.surface_config.width,
                    self.surface_config.height,
                )
            })
            .collect();
        if runs
            .iter()
            .zip(physical_scissors.iter())
            .all(|(run, scissor)| run.draws.is_empty() || scissor.is_none())
        {
            return;
        }

        let load_op = self.take_color_load_op();
        self.ensure_active_frame();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Retained Batch Runs Encoder"),
            });

        {
            let (screen_texture, resolve_target) = self.current_color_attachment_view_clones();
            let stencil_texture = self.stencil_renderer.stencil_view_clone();
            let depth_stencil_attachment = Some(wgpu::RenderPassDepthStencilAttachment {
                view: &stencil_texture,
                depth_ops: None,
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
            });
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Retained Batch Runs Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &screen_texture,
                    depth_slice: None,
                    resolve_target: resolve_target.as_ref(),
                    ops: wgpu::Operations {
                        load: load_op,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            for (run, scissor) in runs.iter().zip(physical_scissors.iter()) {
                let Some(scissor) = scissor else {
                    continue;
                };
                if run.draws.is_empty() {
                    continue;
                }
                render_pass.set_stencil_reference(run.stencil_index);
                render_pass.set_scissor_rect(scissor.0, scissor.1, scissor.2, scissor.3);
                render_pass.set_bind_group(1, self.alpha_masks.bind_group(run.alpha_mask), &[]);
                for draw in &run.draws {
                    match draw {
                        RetainedDraw::Vector(resource) => {
                            if !resource.is_drawable() {
                                continue;
                            }
                            render_pass.set_pipeline(self.pipeline.as_ref());
                            render_pass.set_bind_group(0, &resource.bind_group, &[]);
                            render_pass
                                .set_vertex_buffer(0, resource.shared.vertex_buffer.slice(..));
                            render_pass.set_index_buffer(
                                resource.shared.index_buffer.slice(..),
                                IndexFormat::Uint16,
                            );
                            render_pass.draw_indexed(0..resource.shared.index_count, 0, 0..1);
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
        }

        if self.should_render_capture_target() {
            self.ensure_capture_target();
            let capture_load_op = self.take_capture_color_load_op(load_op);
            let capture_target = self.capture_target.as_ref().unwrap();
            let (capture_texture, capture_resolve_target) = capture_target.color_attachment_views();
            let stencil_texture = self.stencil_renderer.stencil_view_clone();
            let depth_stencil_attachment = Some(wgpu::RenderPassDepthStencilAttachment {
                view: &stencil_texture,
                depth_ops: None,
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
            });
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Retained Batch Runs Screenshot Mirror Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: capture_texture,
                    depth_slice: None,
                    resolve_target: capture_resolve_target,
                    ops: wgpu::Operations {
                        load: capture_load_op,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            for (run, scissor) in runs.iter().zip(physical_scissors.iter()) {
                let Some(scissor) = scissor else {
                    continue;
                };
                if run.draws.is_empty() {
                    continue;
                }
                render_pass.set_stencil_reference(run.stencil_index);
                render_pass.set_scissor_rect(scissor.0, scissor.1, scissor.2, scissor.3);
                render_pass.set_bind_group(1, self.alpha_masks.bind_group(run.alpha_mask), &[]);
                for draw in &run.draws {
                    match draw {
                        RetainedDraw::Vector(resource) => {
                            if !resource.is_drawable() {
                                continue;
                            }
                            render_pass.set_pipeline(self.pipeline.as_ref());
                            render_pass.set_bind_group(0, &resource.bind_group, &[]);
                            render_pass
                                .set_vertex_buffer(0, resource.shared.vertex_buffer.slice(..));
                            render_pass.set_index_buffer(
                                resource.shared.index_buffer.slice(..),
                                IndexFormat::Uint16,
                            );
                            render_pass.draw_indexed(0..resource.shared.index_count, 0, 0..1);
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
        }

        self.enqueue_command_buffer(encoder.finish());
    }

    pub(crate) fn sync_stencil_stack(&mut self, depth: u32, clips: &[stencil::ClipDraw<'_>]) {
        if !self.stencil_renderer.needs_stack_sync(depth, clips) {
            return;
        }
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Stencil Sync Encoder"),
            });
        self.stencil_renderer
            .encode_stencil_stack_sync(&self.device, &mut encoder, depth, clips);
        self.enqueue_command_buffer(encoder.finish());
    }

    pub(crate) fn retain_stencil_resources(
        &mut self,
        active_signatures: &HashSet<u64>,
        active_clip_ids: &HashSet<u32>,
    ) {
        self.stencil_renderer
            .retain_cached_resources(active_signatures, active_clip_ids);
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
        opacity: f32,
    ) -> RetainedImageDraw {
        let corners = transformed_corners(&rect, &transform);
        let verts = corners_to_texture_vertices(corners, opacity);
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
        opacity: f32,
    ) {
        let corners = transformed_corners(&rect, &transform);
        let verts = corners_to_texture_vertices(corners, opacity);
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
            self.clear_color,
            texture,
            &draw.resource,
        );
        if self.should_render_capture_target() {
            self.ensure_capture_target();
            let capture_clear_target = self.take_capture_clear_target(clear_target);
            let capture_target = self.capture_target.as_ref().unwrap();
            let (capture_texture, capture_resolve_target) = capture_target.color_attachment_views();
            self.texture_renderer.draw_retained_image(
                &self.device,
                &self.queue,
                capture_texture,
                capture_resolve_target,
                &self.stencil_renderer,
                capture_clear_target,
                self.clear_color,
                texture,
                &draw.resource,
            );
        }
    }

    fn write_buffers(&mut self, buffers: &mut CpuBuffers, encoder: &mut wgpu::CommandEncoder) {
        let CpuBuffers {
            geometry: ref mut geom,
            ref mut primitives,
            ref mut colors,
            ref mut gradients,
            ref mut materials,
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
        while materials.len() * std::mem::size_of::<GpuMaterial>() % ALIGNMENT != 0 {
            materials.push(GpuMaterial::default());
        }

        self.resize_shared_buffers_if_needed(
            geom.indices.len(),
            geom.vertices.len(),
            primitives.len(),
            transforms.len(),
            colors.len(),
            gradients.len(),
            materials.len(),
        );

        write_staged_buffer(
            &mut self.staging_belt,
            encoder,
            &self.index_buffer,
            bytemuck::cast_slice(&geom.indices),
        );
        write_staged_buffer(
            &mut self.staging_belt,
            encoder,
            &self.vertex_buffer,
            bytemuck::cast_slice(&geom.vertices),
        );
        write_staged_buffer(
            &mut self.staging_belt,
            encoder,
            &self.primitive_buffer,
            bytemuck::cast_slice(primitives),
        );
        write_staged_buffer(
            &mut self.staging_belt,
            encoder,
            &self.colors_buffer,
            bytemuck::cast_slice(colors),
        );
        write_staged_buffer(
            &mut self.staging_belt,
            encoder,
            &self.gradients_buffer,
            bytemuck::cast_slice(gradients),
        );
        write_staged_buffer(
            &mut self.staging_belt,
            encoder,
            &self.materials_buffer,
            bytemuck::cast_slice(materials),
        );
        write_staged_buffer(
            &mut self.staging_belt,
            encoder,
            &self.transforms_buffer,
            bytemuck::cast_slice(transforms),
        );

        self.index_count = geom.indices.len() as u64;
    }

    pub(crate) fn render_primitive_batches(&mut self, batches: &mut [PrimitiveBatch<'_>]) {
        if batches.is_empty() {
            return;
        }

        if self.should_render_capture_target() {
            self.render_primitive_batches_segmented(batches);
            return;
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Primitive Batch Encoder"),
            });

        if !self.pending_clear {
            self.stencil_renderer.encode_clear(&mut encoder);
            self.pending_clear = true;
        }

        for batch in batches {
            self.write_buffers(&mut batch.buffers, &mut encoder);

            let mut plans = Vec::new();
            for segment in &batch.segments {
                let stencil_sync = if self
                    .stencil_renderer
                    .needs_stack_sync(segment.stencil_depth, &segment.clips)
                {
                    self.stencil_renderer.prepare_stencil_stack_sync(
                        &self.device,
                        segment.stencil_depth,
                        &segment.clips,
                    )
                } else {
                    stencil::PreparedStencilSync::default()
                };
                let stencil_reference = self.stencil_renderer.get_stencil().1;
                let scissor = physical_scissor_rect(
                    segment.scissor,
                    self.globals.dpr,
                    self.surface_config.width,
                    self.surface_config.height,
                );
                plans.push(PrimitiveBatchRenderPlan {
                    alpha_mask: segment.alpha_mask,
                    stencil_sync,
                    stencil_reference,
                    scissor,
                    index_start: segment.index_start,
                    index_count: segment.index_count,
                });
            }

            let has_work = plans.iter().any(|plan| {
                !plan.stencil_sync.is_empty() || (plan.scissor.is_some() && plan.index_count > 0)
            });
            if !has_work {
                continue;
            }

            let load_op = self.take_color_load_op();
            self.ensure_active_frame();
            {
                let (screen_texture, resolve_target) = self.current_color_attachment_view_clones();
                let stencil_texture = self.stencil_renderer.stencil_view_clone();
                let depth_stencil_attachment = Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &stencil_texture,
                    depth_ops: None,
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                });
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Primitive Batch Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &screen_texture,
                        depth_slice: None,
                        resolve_target: resolve_target.as_ref(),
                        ops: wgpu::Operations {
                            load: load_op,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

                let full_surface_scissor = (
                    0,
                    0,
                    self.surface_config.width.max(1),
                    self.surface_config.height.max(1),
                );
                for plan in &plans {
                    if !plan.stencil_sync.is_empty() {
                        // Scissor state is sticky within a render pass. Stencil syncs must cover
                        // their full geometry, then drawable segments can restore their own scissor.
                        render_pass.set_scissor_rect(
                            full_surface_scissor.0,
                            full_surface_scissor.1,
                            full_surface_scissor.2,
                            full_surface_scissor.3,
                        );
                        self.stencil_renderer.encode_prepared_stencil_sync(
                            &mut render_pass,
                            &plan.stencil_sync,
                            true,
                        );
                    }

                    let Some(scissor) = plan.scissor else {
                        continue;
                    };
                    if plan.index_count == 0 {
                        continue;
                    }
                    render_pass.set_pipeline(self.pipeline.as_ref());
                    render_pass.set_bind_group(0, &self.bind_group, &[]);
                    render_pass.set_bind_group(
                        1,
                        self.alpha_masks.bind_group(plan.alpha_mask),
                        &[],
                    );
                    render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                    render_pass.set_stencil_reference(plan.stencil_reference);
                    render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
                    render_pass.set_scissor_rect(scissor.0, scissor.1, scissor.2, scissor.3);
                    render_pass.draw_indexed(
                        plan.index_start..plan.index_start + plan.index_count,
                        0,
                        0..1,
                    );
                }
            }
        }

        self.enqueue_staged_command_buffer(encoder);
    }

    fn render_primitive_batches_segmented(&mut self, batches: &mut [PrimitiveBatch<'_>]) {
        if batches.is_empty() {
            return;
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Primitive Batch Encoder"),
            });

        if !self.pending_clear {
            self.stencil_renderer.encode_clear(&mut encoder);
            self.pending_clear = true;
        }

        for batch in batches {
            self.write_buffers(&mut batch.buffers, &mut encoder);

            let mut segment_index = 0;
            while segment_index < batch.segments.len() {
                let segment = &batch.segments[segment_index];
                if self
                    .stencil_renderer
                    .needs_stack_sync(segment.stencil_depth, &segment.clips)
                {
                    self.stencil_renderer.encode_stencil_stack_sync(
                        &self.device,
                        &mut encoder,
                        segment.stencil_depth,
                        &segment.clips,
                    );
                }
                let stencil_reference = self.stencil_renderer.get_stencil().1;
                let run_start = segment_index;
                segment_index += 1;
                while segment_index < batch.segments.len() {
                    let next = &batch.segments[segment_index];
                    if self
                        .stencil_renderer
                        .needs_stack_sync(next.stencil_depth, &next.clips)
                    {
                        break;
                    }
                    segment_index += 1;
                }
                let drawable_segments: Vec<_> = batch.segments[run_start..segment_index]
                    .iter()
                    .filter_map(|segment| {
                        physical_scissor_rect(
                            segment.scissor,
                            self.globals.dpr,
                            self.surface_config.width,
                            self.surface_config.height,
                        )
                        .map(|scissor| (segment, scissor))
                    })
                    .collect();
                if drawable_segments.is_empty() {
                    continue;
                }
                let load_op = self.take_color_load_op();
                self.ensure_active_frame();
                {
                    let (screen_texture, resolve_target) = self.current_color_attachment_views();
                    let (stencil_texture, _) = self.stencil_renderer.get_stencil();
                    let depth_stencil_attachment = Some(wgpu::RenderPassDepthStencilAttachment {
                        view: stencil_texture,
                        depth_ops: None,
                        stencil_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                    });
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Primitive Batch Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: screen_texture,
                            depth_slice: None,
                            resolve_target,
                            ops: wgpu::Operations {
                                load: load_op,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    render_pass.set_pipeline(self.pipeline.as_ref());
                    render_pass.set_bind_group(0, &self.bind_group, &[]);
                    render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                    render_pass.set_stencil_reference(stencil_reference);
                    render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
                    for (segment, scissor) in &drawable_segments {
                        render_pass.set_bind_group(
                            1,
                            self.alpha_masks.bind_group(segment.alpha_mask),
                            &[],
                        );
                        render_pass.set_scissor_rect(scissor.0, scissor.1, scissor.2, scissor.3);
                        render_pass.draw_indexed(
                            segment.index_start..segment.index_start + segment.index_count,
                            0,
                            0..1,
                        );
                    }
                }

                if self.should_render_capture_target() {
                    self.ensure_capture_target();
                    let capture_load_op = self.take_capture_color_load_op(load_op);
                    let capture_target = self.capture_target.as_ref().unwrap();
                    let (capture_texture, capture_resolve_target) =
                        capture_target.color_attachment_views();
                    let (stencil_texture, _) = self.stencil_renderer.get_stencil();
                    let depth_stencil_attachment = Some(wgpu::RenderPassDepthStencilAttachment {
                        view: stencil_texture,
                        depth_ops: None,
                        stencil_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                    });
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Primitive Batch Screenshot Mirror Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: capture_texture,
                            depth_slice: None,
                            resolve_target: capture_resolve_target,
                            ops: wgpu::Operations {
                                load: capture_load_op,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    render_pass.set_pipeline(self.pipeline.as_ref());
                    render_pass.set_bind_group(0, &self.bind_group, &[]);
                    render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                    render_pass.set_stencil_reference(stencil_reference);
                    render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
                    for (segment, scissor) in &drawable_segments {
                        render_pass.set_bind_group(
                            1,
                            self.alpha_masks.bind_group(segment.alpha_mask),
                            &[],
                        );
                        render_pass.set_scissor_rect(scissor.0, scissor.1, scissor.2, scissor.3);
                        render_pass.draw_indexed(
                            segment.index_start..segment.index_start + segment.index_count,
                            0,
                            0..1,
                        );
                    }
                }
            }
        }

        self.enqueue_staged_command_buffer(encoder);
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
            wgpu::LoadOp::Clear(self.clear_color)
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
            self.clear_color,
            &image.rgba,
            image.pixel_width,
            transform,
            rect,
        );
        if self.should_render_capture_target() {
            self.ensure_capture_target();
            let capture_clear_target = self.take_capture_clear_target(clear_target);
            let capture_target = self.capture_target.as_ref().unwrap();
            let (capture_texture, capture_resolve_target) = capture_target.color_attachment_views();
            self.texture_renderer.render_image(
                &self.device,
                &self.queue,
                capture_texture,
                capture_resolve_target,
                &self.globals_buffer,
                &self.stencil_renderer,
                capture_clear_target,
                self.clear_color,
                &image.rgba,
                image.pixel_width,
                transform,
                rect,
            );
        }
    }

    pub(crate) fn clear(&mut self) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Stencil Clear Encoder"),
            });
        self.stencil_renderer.encode_clear(&mut encoder);
        self.enqueue_command_buffer(encoder.finish());
        self.pending_clear = true;
    }

    pub(crate) fn set_scene_lighting(&mut self, lighting: GpuSceneLighting) {
        self.queue
            .write_buffer(&self.lighting_buffer, 0, bytemuck::bytes_of(&lighting));
    }

    pub(crate) fn ensure_frame_cleared(&mut self) {
        if !self.pending_clear {
            self.clear();
        }
    }

    pub(crate) fn finish_frame(&mut self) {
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
            if self.should_render_capture_target() {
                self.ensure_capture_target();
                let capture_load_op = self.take_capture_color_load_op(load_op);
                let capture_target = self.capture_target.as_ref().unwrap();
                let (capture_texture, capture_resolve_target) =
                    capture_target.color_attachment_views();
                let _r = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Screenshot Mirror Clear Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: capture_texture,
                        depth_slice: None,
                        resolve_target: capture_resolve_target,
                        ops: wgpu::Operations {
                            load: capture_load_op,
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

        self.capture_surface_for_pending_requests();
    }

    pub(crate) fn present_queued_frame(&mut self) {
        if let Some(screen_surface) = self.active_frame.take() {
            screen_surface.surface.present();
        }
    }

    pub(crate) fn present(&mut self) {
        self.finish_frame();
        self.submit_pending_commands();
        self.present_queued_frame();
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

    fn current_color_attachment_view_clones(&self) -> (TextureView, Option<TextureView>) {
        let surface_view = self
            .active_frame
            .as_ref()
            .expect("active frame should exist after acquisition")
            .view
            .clone();
        if let Some(multisampled_target) = &self.multisampled_target {
            (multisampled_target.view.clone(), Some(surface_view))
        } else {
            (surface_view, None)
        }
    }

    fn surface_supports_copy_src(&self) -> bool {
        self.surface_config.usage.contains(TextureUsages::COPY_SRC)
    }

    fn should_render_capture_target(&self) -> bool {
        !self.surface_supports_copy_src() && !self.pending_capture_ids.is_empty()
    }

    fn ensure_capture_target(&mut self) {
        if self.surface_supports_copy_src() || self.capture_target.is_some() {
            return;
        }

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Surface Screenshot Mirror"),
            size: wgpu::Extent3d {
                width: self.surface_config.width.max(1),
                height: self.surface_config.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.surface_config.format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[self.surface_config.format],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let multisampled_target = if self.sample_count > 1 {
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
        self.capture_target = Some(CaptureTarget {
            texture,
            view,
            multisampled_target,
            initialized: false,
        });
    }

    fn take_capture_color_load_op(
        &mut self,
        screen_load_op: wgpu::LoadOp<wgpu::Color>,
    ) -> wgpu::LoadOp<wgpu::Color> {
        let Some(capture_target) = self.capture_target.as_mut() else {
            return screen_load_op;
        };
        if matches!(screen_load_op, wgpu::LoadOp::Clear(_)) {
            capture_target.initialized = true;
            return screen_load_op;
        }
        if !capture_target.initialized {
            capture_target.initialized = true;
            return wgpu::LoadOp::Clear(self.clear_color);
        }
        screen_load_op
    }

    fn take_capture_clear_target(&mut self, screen_clear_target: bool) -> bool {
        let Some(capture_target) = self.capture_target.as_mut() else {
            return screen_clear_target;
        };
        if screen_clear_target || !capture_target.initialized {
            capture_target.initialized = true;
            return true;
        }
        false
    }

    fn capture_surface_for_pending_requests(&mut self) {
        let capture_ids = std::mem::take(&mut self.pending_capture_ids);
        if capture_ids.is_empty() {
            return;
        }

        let width = self.surface_config.width.max(1);
        let height = self.surface_config.height.max(1);
        let surface_format = self.surface_config.format;
        let unpadded_bytes_per_row = width as usize * 4;
        let padded_bytes_per_row =
            unpadded_bytes_per_row.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize);
        let readback_size = padded_bytes_per_row * height as usize;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Surface Screenshot Readback"),
            size: readback_size as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Surface Screenshot Encoder"),
            });
        let source_texture = if self.surface_supports_copy_src() {
            let Some(active_frame) = self.active_frame.as_ref() else {
                return;
            };
            &active_frame.surface.texture
        } else {
            let Some(capture_target) = self.capture_target.as_ref() else {
                return;
            };
            &capture_target.texture
        };
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: source_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row as u32),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let completed_captures = Arc::clone(&self.completed_captures);
        let readback_for_callback = readback.clone();
        // The copy may be submitted later as part of a multi-surface frame. Mapping now would
        // make the buffer unavailable to that copy when its command buffer reaches the queue.
        encoder.map_buffer_on_submit(
            &readback,
            wgpu::MapMode::Read,
            ..,
            move |result| {
                if let Err(err) = result {
                    log::warn!("failed to map surface screenshot readback buffer: {err}");
                    return;
                }

                let mapped = readback_for_callback.slice(..).get_mapped_range();
                let mut rgba = vec![0; unpadded_bytes_per_row * height as usize];
                for row in 0..height as usize {
                    let src_start = row * padded_bytes_per_row;
                    let dst_start = row * unpadded_bytes_per_row;
                    rgba[dst_start..dst_start + unpadded_bytes_per_row]
                        .copy_from_slice(&mapped[src_start..src_start + unpadded_bytes_per_row]);
                }
                if matches!(
                    surface_format,
                    wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
                ) {
                    for pixel in rgba.chunks_exact_mut(4) {
                        pixel.swap(0, 2);
                    }
                }
                drop(mapped);
                readback_for_callback.unmap();

                let capture = CapturedFrame {
                    width,
                    height,
                    rgba,
                };
                match completed_captures.lock() {
                    Ok(mut completed) => {
                        for request_id in capture_ids {
                            completed.insert(request_id, capture.clone());
                        }
                    }
                    Err(err) => {
                        log::warn!(
                            "failed to store surface screenshot readback result: {:?}",
                            err
                        );
                    }
                }
            },
        );
        self.enqueue_command_buffer(encoder.finish());
        self.needs_device_poll = true;
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
    Vec<GpuMaterial>,
) {
    let CpuBuffers {
        geometry,
        primitives,
        transforms,
        colors,
        gradients,
        materials,
    } = buffers;
    let mut indices = geometry.indices.clone();
    let mut vertices = geometry.vertices.clone();
    let mut primitives = primitives.clone();
    let mut transforms = transforms.clone();
    let mut colors = colors.clone();
    let mut gradients = gradients.clone();
    let mut materials = materials.clone();

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
    if materials.is_empty() {
        materials.push(GpuMaterial::default());
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
    while materials.len() * std::mem::size_of::<GpuMaterial>() % ALIGNMENT != 0 {
        materials.push(GpuMaterial::default());
    }

    (
        vertices, indices, primitives, transforms, colors, gradients, materials,
    )
}

fn aligned_geometry_buffers(buffers: &CpuBuffers) -> (Vec<GpuVertex>, Vec<u16>) {
    let mut vertices = buffers.geometry.vertices.clone();
    let mut indices = buffers.geometry.indices.clone();

    const ALIGNMENT: usize = 16;
    while indices.len() * std::mem::size_of::<u16>() % ALIGNMENT != 0 {
        indices.push(0);
        indices.push(0);
        indices.push(0);
    }
    while vertices.len() * std::mem::size_of::<GpuVertex>() % ALIGNMENT != 0 {
        vertices.push(GpuVertex::default());
    }

    (vertices, indices)
}

fn aligned_pod_slice<T: Default + Clone + Pod>(slice: &[T]) -> Vec<T> {
    let mut aligned = slice.to_vec();

    const ALIGNMENT: usize = 16;
    if aligned.is_empty() {
        aligned.push(T::default());
    }
    while aligned.len() * std::mem::size_of::<T>() % ALIGNMENT != 0 {
        aligned.push(T::default());
    }

    aligned
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
    pub materials: Vec<GpuMaterial>,
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
            materials,
        } = self;
        geometry.vertices.clear();
        geometry.indices.clear();
        primitives.clear();
        colors.clear();
        gradients.clear();
        materials.clear();
        // leave the identity transform at the start
        transforms.truncate(1);
    }
}

#[cfg(test)]
mod shader_tests {
    #[test]
    fn alpha_mask_shaders_parse_and_validate() {
        for source in [
            concat!(
                include_str!("radial_gradient.wgsl"),
                "\n",
                include_str!("alpha_mask.wgsl")
            ),
            include_str!("alpha_blur.wgsl"),
            include_str!("textures.wgsl"),
        ] {
            let module = naga::front::wgsl::parse_str(source).expect("alpha shader parses");
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .expect("alpha shader validates");
        }
    }
    #[test]
    fn geometry_shader_parses_and_validates() {
        let module = naga::front::wgsl::parse_str(concat!(
            include_str!("radial_gradient.wgsl"),
            "\n",
            include_str!("geometry.wgsl")
        ))
        .expect("geometry shader should parse");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("geometry shader should validate");
    }
}

#[derive(Clone)]
/// Decoded RGBA image data ready for upload as a GPU texture.
pub struct Image {
    pub rgba: Vec<u8>,
    pub pixel_width: u32,
    pub pixel_height: u32,
}
