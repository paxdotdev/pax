use anyhow::anyhow;
use bytemuck::Pod;

use lyon::lyon_tessellation::VertexBuffers;
use wgpu::{
    util::DeviceExt, BindGroup, BindGroupLayout, BufferUsages, CompositeAlphaMode, Device,
    IndexFormat, InstanceFlags, PresentMode, RenderPipeline, SurfaceConfiguration, SurfaceTexture,
    TextureFormat, TextureUsages, TextureView,
};
use winit::window::Window;

pub mod data;
mod gpu_resources;
mod texture;
use data::{GpuGlobals, GpuPrimitive, GpuVertex};

use crate::{render_backend::texture::TextureRenderer, Box2D, Point2D};

use self::data::{GpuColor, GpuGradient, GpuTransform};

pub struct RenderConfig {
    pub debug: bool,
    index_buffer_size: u64,
    vertex_buffer_size: u64,
    primitive_buffer_size: u64,
    colors_buffer_size: u64,
    gradients_buffer_size: u64,
    clipping_buffer_size: u64,
    transform_buffer_size: u64,
    pub initial_width: u32,
    pub initial_height: u32,
    pub initial_dpr: u32,
}

impl RenderConfig {
    pub fn new(debug: bool, width: u32, height: u32, dpr: u32) -> Self {
        Self {
            debug: false,
            index_buffer_size: 2 << 12,
            vertex_buffer_size: 2 << 12,
            primitive_buffer_size: 512,
            colors_buffer_size: 512,
            gradients_buffer_size: 64,
            clipping_buffer_size: 64,
            transform_buffer_size: 64,
            initial_width: width,
            initial_height: height,
            initial_dpr: dpr,
        }
    }
}

pub struct RenderBackend {
    //configuration
    config: RenderConfig,
    pub(crate) globals: GpuGlobals,

    //gpu pipeline references
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    surface_config: SurfaceConfiguration,
    pipeline: RenderPipeline,
    bind_group: BindGroup,

    //buffers
    globals_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    primitive_buffer: wgpu::Buffer,
    clipping_buffer: wgpu::Buffer,
    colors_buffer: wgpu::Buffer,
    gradients_buffer: wgpu::Buffer,

    index_count: u64,

    texture_renderer: TextureRenderer,
}

impl RenderBackend {
    #[cfg(target_arch = "wasm32")]
    pub async fn to_canvas(
        canvas: web_sys::HtmlCanvasElement,
        config: RenderConfig,
    ) -> Result<Self, anyhow::Error> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: if config.debug {
                InstanceFlags::DEBUG
            } else {
                InstanceFlags::default()
            },
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });
        let surface = instance.create_surface_from_canvas(canvas)?;
        Self::new(surface, instance, config).await
    }

    /// SAFETY: window needs to live longer than the renderer
    pub async unsafe fn from_winit(
        window: &Window,
        config: RenderConfig,
    ) -> Result<Self, anyhow::Error> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: if config.debug {
                InstanceFlags::DEBUG
            } else {
                InstanceFlags::default()
            },
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });
        //SAFETY: window needs to live longer than the backend renderer
        let surface = unsafe { instance.create_surface(window)? };
        Self::new(surface, instance, config).await
    }

    pub async fn new(
        surface: wgpu::Surface,
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
                    features: wgpu::Features::default(),
                    limits: wgpu::Limits::downlevel_webgl2_defaults(),
                },
                None,
            )
            .await
            .expect("couldn't find device");

        const TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: TEXTURE_FORMAT,
            width: config.initial_width,
            height: config.initial_height,
            present_mode: PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::PreMultiplied,
            view_formats: vec![TEXTURE_FORMAT],
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
            resolution: [config.initial_width as f32, config.initial_height as f32],
            dpr: config.initial_dpr,
            _pad2: 0,
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
        let (_, clipping_buffer) = create_buffer::<GpuTransform>(
            &device,
            "Clipping Buffer",
            config.clipping_buffer_size,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let (_, colors_buffer) = create_buffer::<GpuTransform>(
            &device,
            "Clipping Buffer",
            config.colors_buffer_size,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let (_, gradients_buffer) = create_buffer::<GpuGradient>(
            &device,
            "Clipping Buffer",
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
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
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
                    resource: clipping_buffer.as_entire_binding(),
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
        let pipeline =
            Self::create_pipeline(&device, surface_config.format, primitive_bind_group_layout);

        let texture_renderer = TextureRenderer::new(&device);

        Ok(Self {
            texture_renderer,
            _adapter: adapter,
            surface,
            device,
            queue,
            config,
            surface_config,
            bind_group,
            pipeline,
            vertex_buffer,
            index_buffer,
            primitive_buffer,
            clipping_buffer,
            globals_buffer,
            colors_buffer,
            gradients_buffer,
            globals,
            index_count: 0,
        })
    }

    fn create_pipeline(
        device: &Device,
        format: TextureFormat,
        primitive_bind_group_layout: BindGroupLayout,
    ) -> RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("geometry.wgsl").into()),
        });
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&primitive_bind_group_layout],
                push_constant_ranges: &[],
            });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[GpuVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32, dpr: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.globals.resolution = [width as f32, height as f32];
        self.globals.dpr = dpr;
        self.queue.write_buffer(
            &self.globals_buffer,
            0,
            bytemuck::cast_slice(&[self.globals]),
        );
        self.surface.configure(&self.device, &self.surface_config);
    }

    fn write_buffers(&mut self, buffers: &mut CpuBuffers) {
        let CpuBuffers {
            geometry: ref mut geom,
            ref mut primitives,
            ref mut stencils,
            ref mut colors,
            ref mut gradients,
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
        while stencils.len() * std::mem::size_of::<GpuTransform>() % ALIGNMENT != 0 {
            stencils.push(GpuTransform::default());
        }
        while colors.len() * std::mem::size_of::<GpuColor>() % ALIGNMENT != 0 {
            colors.push(GpuColor::default());
        }
        while gradients.len() * std::mem::size_of::<GpuGradient>() % ALIGNMENT != 0 {
            gradients.push(GpuGradient::default());
        }

        if geom.indices.len() >= self.config.index_buffer_size as usize
            || geom.vertices.len() >= self.config.vertex_buffer_size as usize
            || primitives.len() >= self.config.primitive_buffer_size as usize
            || stencils.len() >= self.config.clipping_buffer_size as usize
            || colors.len() >= self.config.colors_buffer_size as usize
            || gradients.len() >= self.config.gradients_buffer_size as usize
        {
            //TODO do resize here instead
            pax_runtime_api::log("render backend: buffer to large, skipping render");
        }

        self.queue
            .write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&geom.indices));
        self.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&geom.vertices));
        self.queue
            .write_buffer(&self.primitive_buffer, 0, bytemuck::cast_slice(primitives));
        self.queue
            .write_buffer(&self.colors_buffer, 0, bytemuck::cast_slice(colors));
        self.queue
            .write_buffer(&self.gradients_buffer, 0, bytemuck::cast_slice(gradients));
        self.queue
            .write_buffer(&self.clipping_buffer, 0, bytemuck::cast_slice(stencils));

        self.index_count = geom.indices.len() as u64;
    }

    pub(crate) fn render_primitives(&mut self, buffers: &mut CpuBuffers) {
        let (screen_surface, screen_texture) = self.get_screen_texture();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &screen_texture,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        // load: wgpu::LoadOp::Clear(wgpu::Color {
                        //     r: 0.0,
                        //     g: 0.0,
                        //     b: 0.0,
                        //     a: 0.0,
                        // }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.write_buffers(buffers);

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.index_count as u32, 0, 0..1);
        }

        //render primitives
        self.queue.submit(std::iter::once(encoder.finish()));
        screen_surface.present();
    }

    fn get_screen_texture(&self) -> (SurfaceTexture, TextureView) {
        let screen_surface = self.surface.get_current_texture().unwrap();
        let screen_texture = screen_surface
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        (screen_surface, screen_texture)
    }

    pub(crate) fn render_image(&mut self, image: &Image) {
        let (screen_surface, screen_texture) = self.get_screen_texture();
        self.texture_renderer.render_image(
            &self.device,
            &self.queue,
            &screen_texture,
            &self.globals_buffer,
            &image.rgba,
            image.pixel_width,
            image.rect,
        );
        screen_surface.present();
    }

    pub(crate) fn clear(&mut self) {
        let (screen_surface, screen_texture) = self.get_screen_texture();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

pub(crate) struct CpuBuffers {
    pub geometry: VertexBuffers<GpuVertex, u16>,
    pub primitives: Vec<GpuPrimitive>,
    pub stencils: Vec<GpuTransform>,
    pub colors: Vec<GpuColor>,
    pub gradients: Vec<GpuGradient>,
}
impl CpuBuffers {
    pub(crate) fn reset(&mut self) {
        self.geometry.vertices.clear();
        self.geometry.indices.clear();
        self.primitives.clear();
        self.stencils.clear();
        self.stencils.push(GpuTransform::default());
        self.colors.clear();
        self.gradients.clear();
    }
}

#[derive(Clone)]
pub struct Image {
    pub rect: Box2D,
    pub rgba: Vec<u8>,
    pub pixel_width: u32,
    pub pixel_height: u32,
}
