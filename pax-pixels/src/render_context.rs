use crate::render_backend::CpuBuffers;
use crate::Box2D;
use crate::Image;
use crate::Point2D;
use crate::RenderContext;
use crate::Transform2D;
use crate::Vector2D;
use lyon::lyon_tessellation::BuffersBuilder;
use lyon::lyon_tessellation::FillOptions;
use lyon::lyon_tessellation::FillTessellator;
use lyon::lyon_tessellation::FillVertex;
use lyon::lyon_tessellation::VertexBuffers;
use lyon::path::Path;

use crate::render_backend::data::GpuColor;
use crate::render_backend::data::GpuGradient;
use crate::render_backend::data::GpuPrimitive;
use crate::render_backend::data::GpuTransform;
use crate::render_backend::data::GpuVertex;
use crate::render_backend::RenderBackend;

pub struct WgpuRenderer {
    buffers: CpuBuffers,
    images: Vec<Image>,
    render_backend: RenderBackend,
    transform_stack: Vec<Transform2D>,
    clipping_stack: Vec<u32>,
    fill_tessellator: FillTessellator,
    tolerance: f32,
}

const IDENTITY: Transform2D = Transform2D::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);

impl WgpuRenderer {
    pub fn new(render_backend: RenderBackend) -> Self {
        let geometry: VertexBuffers<GpuVertex, u16> = VertexBuffers::new();
        let fill_tessellator = FillTessellator::new();
        let default_clipp = GpuTransform {
            transform: [[0.0; 2]; 3],
            _pad: 0,
            _pad2: 0,
        };
        Self {
            images: vec![],
            render_backend,
            transform_stack: Vec::new(),
            buffers: CpuBuffers {
                geometry,
                primitives: Vec::new(),
                colors: Vec::new(),
                gradients: Vec::new(),
                stencils: vec![default_clipp],
            },
            clipping_stack: vec![0],
            fill_tessellator,
            tolerance: 0.5, //TODO expose as option
        }
    }

    pub fn push_transform(&mut self, transform: Transform2D) {
        let last = self.current_transform();
        self.transform_stack.push(transform.then(last));
    }

    pub fn pop_transform(&mut self) {
        self.transform_stack.pop();
    }

    fn current_transform(&self) -> &Transform2D {
        self.transform_stack.last().unwrap_or(&IDENTITY)
    }

    fn current_clipping_id(&self) -> u32 {
        *self.clipping_stack.last().expect("clipper stack empty???")
    }

    pub fn push_clipping_bounds(&mut self, bounds: Box2D) {
        let point_to_unit_rect = Transform2D::translation(-bounds.min.x, -bounds.min.y)
            .then_scale(1.0 / bounds.width(), 1.0 / bounds.height());
        let clipping_bounds = self
            .current_transform()
            .inverse()
            .expect("non-invertible transform was pushed to the stack") //TODO how to handle this better?
            .then(&point_to_unit_rect);
        self.clipping_stack.push(self.buffers.stencils.len() as u32);
        self.buffers.stencils.push(GpuTransform {
            transform: clipping_bounds.to_arrays(),
            _pad: 0,
            _pad2: 0,
        });
    }

    pub fn pop_clipping_bounds(&mut self) {
        self.clipping_stack.pop();
    }
}

impl RenderContext for WgpuRenderer {
    fn stroke_path(&mut self, _path: Path, _stroke: Stroke) {
        //TODOrefactor
        //unimplemented!()
    }

    fn fill_path(&mut self, path: Path, fill: Fill) {
        let path = path.transformed(self.current_transform());
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
                    pax_runtime_api::log(
                        "can't draw graidents with more than 8 stops. truncating.",
                    );
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
            clipping_id: self.current_clipping_id(),
            transform_id: 0,
            z_index: 0,
        };
        let prim_id = self.buffers.primitives.len() as u32;
        self.buffers.primitives.push(primitive);
        let options = FillOptions::tolerance(self.tolerance);
        let mut geometry_builder =
            BuffersBuilder::new(&mut self.buffers.geometry, |vertex: FillVertex| GpuVertex {
                position: vertex.position().to_array(),
                normal: [0.0; 2],
                prim_id,
            });
        match self
            .fill_tessellator
            .tessellate_path(&path, &options, &mut geometry_builder)
        {
            Ok(_) => {}
            Err(e) => pax_runtime_api::log(&format!("{:?}", e)),
        };
    }

    fn draw_image(&mut self, image: Image) {
        self.images.push(image);
    }

    fn flush(&mut self) {
        self.render_backend.render(&mut self.buffers, &self.images);
        self.images.clear();
        self.buffers.reset();
    }

    fn push_transform(&mut self, transform: Transform2D) {
        let last = self.current_transform();
        self.transform_stack.push(transform.then(last));
    }

    fn pop_transform(&mut self) {
        self.transform_stack.pop();
    }

    fn push_clipping_bounds(&mut self, bounds: Box2D) {
        let point_to_unit_rect = Transform2D::translation(-bounds.min.x, -bounds.min.y)
            .then_scale(1.0 / bounds.width(), 1.0 / bounds.height());
        let clipping_bounds = self
            .current_transform()
            .inverse()
            .expect("non-invertible transform was pushed to the stack") //TODO how to handle this better?
            .then(&point_to_unit_rect);
        self.clipping_stack.push(self.buffers.stencils.len() as u32);
        self.buffers.stencils.push(GpuTransform {
            transform: clipping_bounds.to_arrays(),
            _pad: 0,
            _pad2: 0,
        });
    }

    fn pop_clipping_bounds(&mut self) {
        self.clipping_stack.pop();
    }

    fn resize(&mut self, width: f32, height: f32, dpr: f32) {
        self.render_backend
            .resize(width as u32, height as u32, dpr as u32);
    }

    fn size(&self) -> (f32, f32) {
        self.render_backend.globals.resolution.into()
    }
}

pub struct GradientStop {
    pub color: Color,
    pub stop: f32,
}

pub enum GradientType {
    Linear,
    Radial,
}

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
