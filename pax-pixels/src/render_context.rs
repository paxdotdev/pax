use crate::render_backend::stencil;
use crate::render_backend::CpuBuffers;
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

use crate::render_backend::data::GpuColor;
use crate::render_backend::data::GpuGradient;
use crate::render_backend::data::GpuPrimitive;
use crate::render_backend::data::GpuTransform;
use crate::render_backend::data::GpuVertex;
use crate::render_backend::RenderBackend;

pub struct WgpuRenderer<'w> {
    buffers: CpuBuffers,
    render_backend: RenderBackend<'w>,
    // these reference indicies in the CpuBuffer "transforms"
    transform_index_stack: Vec<usize>,
    // tuble of transform stack index to go to, and clip depth to go to
    saves: Vec<(usize, usize)>,
    tolerance: f32,
}

impl<'w> WgpuRenderer<'w> {
    pub fn new(render_backend: RenderBackend<'w>) -> Self {
        let geometry: VertexBuffers<GpuVertex, u16> = VertexBuffers::new();
        Self {
            render_backend,
            buffers: CpuBuffers {
                geometry,
                primitives: Vec::new(),
                colors: Vec::new(),
                gradients: Vec::new(),
                transforms: vec![GpuTransform::default()],
            },
            tolerance: 0.5, //TODO expose as option
            transform_index_stack: vec![],
            saves: vec![],
        }
    }

    fn current_transform(&self) -> Transform2D {
        Transform2D::from_arrays(
            self.buffers
                .transforms
                .get(self.transform_index_stack.last().cloned().unwrap_or(0))
                .expect("at least one identity transform should exist on the transform stack")
                .transform,
        )
    }

    pub fn stroke_path(&mut self, path: Path, stroke_fill: Fill, stroke_width: f32) {
        let prim_id = self.push_primitive_def(stroke_fill);
        let options = StrokeOptions::tolerance(self.tolerance).with_line_width(stroke_width);
        let mut geometry_builder =
            BuffersBuilder::new(&mut self.buffers.geometry, |vertex: StrokeVertex| {
                GpuVertex {
                    position: vertex.position().to_array(),
                    normal: [0.0; 2],
                    prim_id,
                }
            });
        match StrokeTessellator::new().tessellate_path(&path, &options, &mut geometry_builder) {
            Ok(_) => {}
            Err(e) => log::warn!("{:?}", e),
        };
    }

    pub fn fill_path(&mut self, path: Path, fill: Fill) {
        let prim_id = self.push_primitive_def(fill);
        let options = FillOptions::tolerance(self.tolerance);
        let mut geometry_builder =
            BuffersBuilder::new(&mut self.buffers.geometry, |vertex: FillVertex| GpuVertex {
                position: vertex.position().to_array(),
                normal: [0.0; 2],
                prim_id,
            });
        match FillTessellator::new().tessellate_path(&path, &options, &mut geometry_builder) {
            Ok(_) => {}
            Err(e) => log::warn!("{:?}", e),
        };
    }

    fn push_primitive_def(&mut self, fill: Fill) -> u32 {
        let fill_id;
        let fill_type_flag;
        match fill {
            Fill::Solid(color) => {
                fill_id = self.buffers.colors.len() as u16;
                fill_type_flag = 0;
                self.buffers.colors.push(GpuColor { color: color.rgba });
            }
            Fill::Gradient {
                gradient_type,
                pos,
                main_axis,
                off_axis,
                stops,
            } => {
                fill_id = self.buffers.gradients.len() as u16;
                fill_type_flag = 1;
                if stops.len() > 8 {
                    log::warn!("can't draw graidents with more than 8 stops. truncating.");
                }
                let len = stops.len().min(8);
                let mut colors_buff = [[0.0; 4]; 8];
                let mut stops_buff = [0.0; 8];
                for i in 0..len {
                    colors_buff[i] = stops[i].color.rgba;
                    stops_buff[i] = stops[i].stop;
                }
                //this should be filled in with custom gradient stuff later:
                self.buffers.gradients.push(GpuGradient {
                    type_id: match gradient_type {
                        GradientType::Linear => 0,
                        GradientType::Radial => 1,
                    },
                    position: pos.to_array(),
                    main_axis: main_axis.to_array(),
                    off_axis: off_axis.to_array(),
                    stop_count: len as u32,
                    colors: colors_buff,
                    stops: stops_buff,
                    _padding: [0; 16],
                });
            }
        }
        let primitive = GpuPrimitive {
            fill_id,
            fill_type_flag,
            clipping_id: 0,
            transform_id: self.transform_index_stack.last().cloned().unwrap_or(0) as u32,
            z_index: 0,
        };
        let prim_id = self.buffers.primitives.len() as u32;
        self.buffers.primitives.push(primitive);
        prim_id
    }

    pub fn clear(&mut self) {
        self.render_backend.clear();
    }

    pub fn draw_image(&mut self, image: &Image, rect: Box2D) {
        let transform = self.current_transform();
        if self.buffers.primitives.len() > 0 {
            self.render_backend.render_primitives(&mut self.buffers);
            let CpuBuffers {
                geometry,
                primitives,
                transforms: _,
                colors,
                gradients,
            } = &mut self.buffers;
            geometry.vertices.clear();
            geometry.indices.clear();
            primitives.clear();
            colors.clear();
            gradients.clear();
        }
        self.render_backend.render_image(image, transform, rect);
    }

    pub fn flush(&mut self) {
        if self.buffers.primitives.len() > 0 {
            self.render_backend.render_primitives(&mut self.buffers);
        }
        self.buffers.reset();
        self.transform_index_stack.clear();
    }

    pub fn save(&mut self) {
        let transform_len = self.transform_index_stack.len();
        self.saves
            .push((transform_len, self.render_backend.get_clip_depth() as usize));
    }

    pub fn restore(&mut self) {
        if let Some((t_pen, clip_depth)) = self.saves.pop() {
            self.transform_index_stack.truncate(t_pen);
            self.render_backend
                .reset_stencil_depth_to(clip_depth as u32);
        }
    }

    pub fn transform(&mut self, transform: Transform2D) {
        let new_ind = self.buffers.transforms.len();
        self.buffers.transforms.push(GpuTransform {
            transform: transform.then(&self.current_transform()).to_arrays(),
            _pad: 0,
            _pad2: 0,
        });
        self.transform_index_stack.push(new_ind);
    }

    pub fn clip(&mut self, path: Path) {
        // fine to transform on CPU - shouldn't be large meshes
        // let path = path.transformed(&self.current_transform());
        if self.buffers.primitives.len() > 0 {
            self.render_backend.render_primitives(&mut self.buffers);
            let CpuBuffers {
                geometry,
                primitives,
                transforms: _,
                colors,
                gradients,
            } = &mut self.buffers;
            geometry.vertices.clear();
            geometry.indices.clear();
            primitives.clear();
            colors.clear();
            gradients.clear();
        }
        let options = FillOptions::tolerance(self.tolerance);
        let mut geometry = VertexBuffers::new();
        let mut geometry_builder =
            BuffersBuilder::new(&mut geometry, |vertex: FillVertex| stencil::Vertex {
                position: vertex.position().to_array(),
            });
        match FillTessellator::new().tessellate_path(&path, &options, &mut geometry_builder) {
            Ok(_) => {}
            Err(e) => log::warn!("{:?}", e),
        };
        self.render_backend.push_stencil(geometry);
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.render_backend.resize(width as u32, height as u32);
    }

    pub fn size(&self) -> (f32, f32) {
        let res = &self.render_backend.globals.resolution;
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
    rgba: [f32; 4],
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
