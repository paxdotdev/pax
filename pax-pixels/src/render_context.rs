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

mod mesh_renderer;
mod stencil_renderer;
mod texture_renderer;
use crate::render_backend::data::GpuVertex;
use crate::render_backend::RenderBackend;

use mesh_renderer::MeshRenderer;
use stencil_renderer::StencilRenderer;
use texture_renderer::TextureRenderer;

pub struct WgpuRenderer<'w> {
    backend: RenderBackend<'w>,
    pub texture_renderer: TextureRenderer,
    pub stencil_renderer: StencilRenderer,
    pub mesh_renderer: MeshRenderer,
    encoder: wgpu::CommandEncoder,
    tolerance: f32,
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
        }
    }

    // fn current_transform(&self) -> Transform2D {
    //     Transform2D::from_arrays(
    //         self.buffers
    //             .transforms
    //             .get(self.transform_index_stack.last().cloned().unwrap_or(0))
    //             .expect("at least one identity transform should exist on the transform stack")
    //             .transform,
    //     )
    // }

    pub fn stroke_path(&mut self, path: Path, stroke_fill: Fill, stroke_width: f32) {
        // let prim_id = self.push_primitive_def(stroke_fill);
        // let options = StrokeOptions::tolerance(self.tolerance).with_line_width(stroke_width);
        // let mut geometry_builder =
        //     BuffersBuilder::new(&mut self.buffers.geometry, |vertex: StrokeVertex| {
        //         GpuVertex {
        //             position: vertex.position().to_array(),
        //             normal: [0.0; 2],
        //             prim_id,
        //         }
        //     });
        // match StrokeTessellator::new().tessellate_path(&path, &options, &mut geometry_builder) {
        //     Ok(_) => {}
        //     Err(e) => log::warn!("{:?}", e),
        // };
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

        // TODO don't recreate encoder/render pass each frame
        let mesh = self
            .mesh_renderer
            .make_mesh(&self.backend.device, &geometry, fill);
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
        // let transform = self.current_transform();
        // self.render_backend.render_image(image, transform, rect);
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
        // TODO finish and recreate encoder here
        // if self.buffers.primitives.len() > 0 {
        //     self.render_backend.render_primitives(&mut self.buffers);
        // }
        // self.buffers.reset();
        // self.transform_index_stack.clear();
    }

    pub fn save(&mut self) {
        // let transform_len = self.transform_index_stack.len();
        // self.saves
        //     .push((transform_len, self.render_backend.get_clip_depth() as usize));
    }

    pub fn restore(&mut self) {
        // if let Some((t_pen, clip_depth)) = self.saves.pop() {
        //     self.transform_index_stack.truncate(t_pen);
        //     self.render_backend
        //         .reset_stencil_depth_to(clip_depth as u32);
        // }
    }

    pub fn transform(&mut self, transform: Transform2D) {
        self.backend.set_transform(transform);
    }

    pub fn clip(&mut self, path: Path) {
        // fine to transform on CPU - shouldn't be large meshes
        // let path = path.transformed(&self.current_transform());
        // if self.buffers.primitives.len() > 0 {
        //     self.render_backend.render_primitives(&mut self.buffers);
        //     let CpuBuffers {
        //         geometry,
        //         primitives,
        //         transforms: _,
        //         colors,
        //         gradients,
        //     } = &mut self.buffers;
        //     geometry.vertices.clear();
        //     geometry.indices.clear();
        //     primitives.clear();
        //     colors.clear();
        //     gradients.clear();
        // }
        // let options = FillOptions::tolerance(self.tolerance);
        // let mut geometry = VertexBuffers::new();
        // let mut geometry_builder = BuffersBuilder::new(&mut geometry, |vertex: FillVertex| {
        //     stencil_renderer::Vertex {
        //         position: vertex.position().to_array(),
        //     }
        // });
        // match FillTessellator::new().tessellate_path(&path, &options, &mut geometry_builder) {
        //     Ok(_) => {}
        //     Err(e) => log::warn!("{:?}", e),
        // };
        // self.render_backend.push_stencil(geometry);
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

impl Color {
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { rgba: [r, g, b, a] }
    }

    //credit: ChatGPT
    pub fn hsva(h: f32, s: f32, v: f32, a: f32) -> Self {
        let i = (h * 6.0).floor() as i32;
        let f = h * 6.0 - i as f32;
        let p = v * (1.0 - s);
        let q = v * (1.0 - f * s);
        let t = v * (1.0 - (1.0 - f) * s);

        let (r, g, b) = match i % 6 {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            _ => (v, p, q),
        };
        Self { rgba: [r, g, b, a] }
    }

    //Credit piet library: https://docs.rs/piet/latest/src/piet/color.rs.html#130-173
    pub fn hlca(h: f32, l: f32, c: f32, a: f32) -> Self {
        // The reverse transformation from Lab to XYZ, see
        // https://en.wikipedia.org/wiki/CIELAB_color_space
        fn f_inv(t: f32) -> f32 {
            let d = 6. / 29.;
            if t > d {
                t.powi(3)
            } else {
                3. * d * d * (t - 4. / 29.)
            }
        }
        let th = h * (std::f32::consts::PI / 180.);
        let a_2 = c * th.cos();
        let b = c * th.sin();
        let ll = (l + 16.) * (1. / 116.);
        // Produce raw XYZ values
        let x = f_inv(ll + a_2 * (1. / 500.));
        let y = f_inv(ll);
        let z = f_inv(ll - b * (1. / 200.));
        // This matrix is the concatenation of three sources.
        // First, the white point is taken to be ICC standard D50, so
        // the diagonal matrix of [0.9642, 1, 0.8249]. Note that there
        // is some controversy around this value. However, it matches
        // the other matrices, thus minimizing chroma error.
        //
        // Second, an adaption matrix from D50 to D65. This is the
        // inverse of the recommended D50 to D65 adaptation matrix
        // from the W3C sRGB spec:
        // https://www.w3.org/Graphics/Color/srgb
        //
        // Finally, the conversion from XYZ to linear sRGB values,
        // also taken from the W3C sRGB spec.
        let r_lin = 3.02172918 * x - 1.61692294 * y - 0.40480625 * z;
        let g_lin = -0.94339358 * x + 1.91584267 * y + 0.02755094 * z;
        let b_lin = 0.06945666 * x - 0.22903204 * y + 1.15957526 * z;
        fn gamma(u: f32) -> f32 {
            if u <= 0.0031308 {
                12.92 * u
            } else {
                1.055 * u.powf(1. / 2.4) - 0.055
            }
        }
        Self {
            rgba: [
                gamma(r_lin).clamp(0.0, 1.0),
                gamma(g_lin).clamp(0.0, 1.0),
                gamma(b_lin).clamp(0.0, 1.0),
                a,
            ],
        }
    }
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
