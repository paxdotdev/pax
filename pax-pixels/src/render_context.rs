use crate::Box2D;
use crate::Image;
use crate::Point2D;
use crate::Transform2D;
use crate::Vector2D;
use lyon::lyon_tessellation::BuffersBuilder;
use lyon::lyon_tessellation::FillOptions;
use lyon::lyon_tessellation::FillTessellator;
use lyon::lyon_tessellation::FillVertex;
use lyon::lyon_tessellation::VertexBuffers;
use lyon::path::Path;
use lyon::tessellation::StrokeOptions;
use lyon::tessellation::StrokeTessellator;
use lyon::tessellation::StrokeVertex;

mod mesh_renderer;
mod stencil_renderer;
mod texture_renderer;
use crate::render_backend::data::GpuVertex;
use crate::render_backend::RenderBackend;

use mesh_renderer::MeshRenderer;
use stencil_renderer::StencilRenderer;
use texture_renderer::TextureRenderer;

pub struct WgpuRenderer<'w> {
    // rendering
    backend: RenderBackend<'w>,
    pub texture_renderer: TextureRenderer,
    pub stencil_renderer: StencilRenderer,
    pub mesh_renderer: MeshRenderer,
    encoder: wgpu::CommandEncoder,

    // config
    tolerance: f32,

    //state
    transform_stack: Vec<Transform2D>,
    // points to restore to, transform_stack/clipping_stack respectively
    save_points: Vec<(usize, usize)>,
}

impl<'w> WgpuRenderer<'w> {
    pub fn new(backend: RenderBackend<'w>) -> Self {
        let texture_renderer = TextureRenderer::new(&backend);
        let stencil_renderer = StencilRenderer::new(&backend);
        let mesh_renderer = MeshRenderer::new(&backend);
        let encoder = backend
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        Self {
            backend,
            texture_renderer,
            stencil_renderer,
            mesh_renderer,
            encoder,
            //TODO expose as option
            tolerance: 0.5,
            transform_stack: vec![],
            save_points: vec![],
        }
    }

    pub fn stroke_path(&mut self, path: Path, stroke_fill: Fill, stroke_width: f32) {
        let options = StrokeOptions::tolerance(self.tolerance).with_line_width(stroke_width);
        let mut geometry = VertexBuffers::new();
        let mut geometry_builder =
            BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| GpuVertex {
                position: vertex.position().to_array(),
                normal: [0.0; 2],
                prim_id: 0,
            });
        match StrokeTessellator::new().tessellate_path(&path, &options, &mut geometry_builder) {
            Ok(_) => {}
            Err(e) => log::warn!("{:?}", e),
        };

        // TODO ------- everything bellow looks the same for fill --------

        // TODO don't recreate this mesh each frame, instead return this mesh from a create_fill_path_resource method
        let mesh = self
            .mesh_renderer
            .make_mesh(&self.backend.device, &geometry, stroke_fill);

        // TODO this code becomes the new "fill_path" method, everything above the "create"
        // TODO another big optimization: if transform/clip doesn't change, we can use the same render pass
        // for drawing all meshes and images.
        let mut render_pass =
            Self::main_draw_render_pass(&self.backend, &self.stencil_renderer, &mut self.encoder);
        self.mesh_renderer.render_meshes(&mut render_pass, &[mesh]);
    }

    pub fn fill_path(&mut self, path: Path, fill: Fill) {
        let options = FillOptions::tolerance(self.tolerance);
        let mut geometry = VertexBuffers::new();
        let mut geometry_builder =
            BuffersBuilder::new(&mut geometry, |vertex: FillVertex| GpuVertex {
                position: vertex.position().to_array(),
                normal: [0.0; 2],
                prim_id: 0,
            });
        match FillTessellator::new().tessellate_path(&path, &options, &mut geometry_builder) {
            Ok(_) => {}
            Err(e) => log::warn!("{:?}", e),
        };

        // TODO ------- everything bellow looks the same for stroke --------

        // TODO don't recreate this mesh each frame, instead return this mesh from a create_fill_path_resource method
        let mesh = self
            .mesh_renderer
            .make_mesh(&self.backend.device, &geometry, fill);

        // TODO this code becomes the new "fill_path" method, everything above the "create"
        let mut render_pass =
            Self::main_draw_render_pass(&self.backend, &self.stencil_renderer, &mut self.encoder);
        self.mesh_renderer.render_meshes(&mut render_pass, &[mesh]);
    }

    pub fn clear(&mut self) {
        self.backend.clear();
        // need to clear the stencil texture
        self.stencil_renderer
            .clear(&self.backend.device, &self.backend.queue);
    }

    pub fn draw_image(&mut self, image: &Image, rect: Box2D) {
        let transform = self.transform_stack.last().cloned().unwrap_or_default();

        // TODO don't create this every time an image is drawn, instead split into two parts: "load_image" that gives back
        // a resource, and "draw_image" that takes that resource and draws it using the below logic
        let image_resource = self.texture_renderer.make_image(
            &self.backend,
            &transform,
            &rect,
            &image.rgba,
            image.pixel_width,
        );
        let mut render_pass =
            Self::main_draw_render_pass(&self.backend, &self.stencil_renderer, &mut self.encoder);
        self.texture_renderer
            .render_image(&mut render_pass, &image_resource)
    }

    // used for image and mesh drawing
    pub fn main_draw_render_pass<'encoder>(
        backend: &RenderBackend,
        stencil_renderer: &StencilRenderer,
        encoder: &'encoder mut wgpu::CommandEncoder,
    ) -> wgpu::RenderPass<'encoder> {
        let (_screen_surface, screen_texture) = backend.get_screen_texture();
        let (stencil_texture, stencil_index) = stencil_renderer.get_stencil();
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Main draw render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &screen_texture,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
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
        });
        render_pass.set_stencil_reference(stencil_index);
        render_pass
    }

    pub fn flush(&mut self) {
        // draw everything scheduled with the previous encoder, and create a new one for the next frame
        let encoder = std::mem::replace(
            &mut self.encoder,
            self.backend
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                }),
        );
        self.backend.queue.submit(std::iter::once(encoder.finish()));
        let (screen_surface, _screen_texture) = self.backend.get_screen_texture();
        //render primitives
        screen_surface.present();
    }

    pub fn save(&mut self) {
        let transform_len = self.transform_stack.len();
        self.save_points
            .push((transform_len, self.stencil_renderer.stencil_layer as usize));
    }

    pub fn restore(&mut self) {
        if let Some((t_pen, clip_depth)) = self.save_points.pop() {
            self.transform_stack.truncate(t_pen);
            // make sure everything queued get's drawn with the current transform/stencil
            // before we modify it
            self.flush();
            self.backend
                .set_transform(self.transform_stack.last().cloned().unwrap_or_default());
            self.stencil_renderer.reset_stencil_depth_to(
                &self.backend.device,
                &self.backend.queue,
                clip_depth as u32,
            );
        }
    }

    pub fn transform(&mut self, transform: &Transform2D) {
        let prev_transform = self.transform_stack.last().cloned().unwrap_or_default();
        let new_transform = prev_transform.then(&transform);
        self.transform_stack.push(new_transform);
        // make sure everything queued get's drawn with the current transform
        // before we modify it
        self.flush();
        self.backend.set_transform(new_transform);
    }

    pub fn clip(&mut self, path: Path) {
        // make sure everything queued get's drawn with the current stencil
        // before we modify it
        self.flush();
        // fine to transform on CPU - shouldn't be large meshes
        let path = path.transformed(&self.transform_stack.last().cloned().unwrap_or_default());
        let options = FillOptions::tolerance(self.tolerance);
        let mut geometry = VertexBuffers::new();
        let mut geometry_builder = BuffersBuilder::new(&mut geometry, |vertex: FillVertex| {
            stencil_renderer::Vertex {
                position: vertex.position().to_array(),
            }
        });
        match FillTessellator::new().tessellate_path(&path, &options, &mut geometry_builder) {
            Ok(_) => {}
            Err(e) => log::warn!("{:?}", e),
        };
        self.stencil_renderer
            .push_stencil(&self.backend.device, &self.backend.queue, geometry);
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.backend.resize(width as u32, height as u32);
        // needs to resize stencil texture
        self.stencil_renderer
            .resize(&self.backend.device, width as u32, height as u32);
    }

    pub fn size(&self) -> (f32, f32) {
        let res = &self.backend.globals.resolution;
        (res[0], res[1])
    }
}

#[derive(Debug)]
pub struct GradientStop {
    pub color: Color,
    pub stop: f32,
}

#[derive(Debug)]
pub enum GradientType {
    Linear,
    Radial,
}

#[derive(Debug)]
pub struct Color {
    pub rgba: [f32; 4],
}

#[derive(Debug)]
pub enum Fill {
    Solid(Color),
    Gradient {
        gradient_type: GradientType,
        pos: Point2D,
        main_axis: Vector2D,
        off_axis: Vector2D,
        stops: Vec<GradientStop>,
    },
}

pub struct Stroke {
    pub color: Color,
    pub weight: f32,
}
