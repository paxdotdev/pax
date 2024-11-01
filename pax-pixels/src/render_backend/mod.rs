use anyhow::anyhow;

use lyon::lyon_tessellation::VertexBuffers;
use wgpu::{
    util::DeviceExt, BufferUsages, CompositeAlphaMode, PresentMode, SurfaceConfiguration,
    SurfaceTexture, TextureFormat, TextureUsages, TextureView,
};

pub mod data;
mod gpu_resources;

use data::{GpuGlobals, GpuVertex};

use crate::{Fill, Transform2D};

use self::data::GpuTransform;

pub struct RenderConfig {
    pub debug: bool,
    pub width: u32,
    pub height: u32,
    pub initial_dpr: u32,
}

impl RenderConfig {
    pub fn new(_debug: bool, width: u32, height: u32, dpr: u32) -> Self {
        Self {
            debug: false,
            width,
            height,
            initial_dpr: dpr,
        }
    }
}

pub struct RenderBackend<'w> {
    //configuration
    pub config: RenderConfig,

    //gpu pipeline references
    _adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    surface: wgpu::Surface<'w>,
    pub surface_config: SurfaceConfiguration,

    // general buffers
    pub(crate) globals: GpuGlobals,
    pub globals_buffer: wgpu::Buffer,
    pub transform_buffer: wgpu::Buffer,
}

impl<'w> RenderBackend<'w> {
    #[cfg(target_arch = "wasm32")]
    pub async fn to_canvas(
        canvas: web_sys::HtmlCanvasElement,
        config: RenderConfig,
    ) -> Result<Self, anyhow::Error> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: if config.debug {
                wgpu::InstanceFlags::DEBUG
            } else {
                wgpu::InstanceFlags::default()
            },
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });
        let surface_target = wgpu::SurfaceTarget::Canvas(canvas);
        let surface = instance.create_surface(surface_target)?;
        Self::new(surface, instance, config).await
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
            .ok_or(anyhow!("couldn't find  adapter"))?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::default(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .expect("couldn't find device");

        const TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: TEXTURE_FORMAT,
            width: config.width,
            height: config.height,
            present_mode: PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::PreMultiplied,
            view_formats: vec![TEXTURE_FORMAT],
            desired_maximum_frame_latency: 2, //TODO 1 for lower latency?
        };
        surface.configure(&device, &surface_config);

        let globals = GpuGlobals {
            resolution: [config.width as f32, config.height as f32],
            dpr: config.initial_dpr,
            _pad2: 0,
        };
        let globals_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gpu Globals"),
            contents: bytemuck::cast_slice(&[globals]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        // global transform - can be set between render calls
        let transform = GpuTransform::default();
        let transform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gpu transform"),
            contents: bytemuck::cast_slice(&[transform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let initial_width = config.width;
        let initial_height = config.height;
        let initial_dpr = config.initial_dpr;

        let mut backend = Self {
            // globally used GPU state
            _adapter: adapter,
            surface,
            device,
            queue,
            config,
            surface_config,
            globals_buffer,
            globals,
            transform_buffer,
        };
        backend.globals.dpr = initial_dpr;
        backend.resize(initial_width, initial_height);
        Ok(backend)
    }

    pub fn set_transform(&mut self, transform: Transform2D) {
        self.queue.write_buffer(
            &self.transform_buffer,
            0,
            bytemuck::cast_slice(&[GpuTransform {
                transform: transform.to_arrays(),
                _pad: 0,
                _pad2: 0,
            }]),
        );
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.globals.resolution = [width as f32, height as f32];
        self.queue.write_buffer(
            &self.globals_buffer,
            0,
            bytemuck::cast_slice(&[self.globals]),
        );
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn get_screen_texture(&self) -> (SurfaceTexture, TextureView) {
        let screen_surface = self.surface.get_current_texture().unwrap();
        let screen_texture = screen_surface
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        (screen_surface, screen_texture)
    }

    // pub(crate) fn render_image(&mut self, image: &Image, transform: Transform2D, rect: Box2D) {
    //     let (screen_surface, screen_texture) = self.get_screen_texture();
    //     self.texture_renderer.render_image(
    //         &self.device,
    //         &self.queue,
    //         &screen_texture,
    //         &self.globals_buffer,
    //         &self.stencil_renderer,
    //         &image.rgba,
    //         image.pixel_width,
    //         transform,
    //         rect,
    //     );
    //     screen_surface.present();
    // }

    pub(crate) fn clear(&mut self) {
        self.set_transform(Transform2D::identity());
        let (screen_surface, screen_texture) = self.get_screen_texture();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let _r = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &screen_texture,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        screen_surface.present();
    }
}

#[derive(Clone)]
pub struct Image {
    pub rgba: Vec<u8>,
    pub pixel_width: u32,
    pub pixel_height: u32,
}
