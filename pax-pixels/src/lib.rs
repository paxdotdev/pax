pub mod render_backend;
pub mod render_context;
pub type Transform2D = lyon::geom::euclid::default::Transform2D<f32>;
pub type Point2D = lyon::geom::euclid::default::Point2D<f32>;
pub type Vector2D = lyon::geom::euclid::default::Vector2D<f32>;
pub type Box2D = lyon::geom::Box2D<f32>;
pub use lyon::geom::{point, Angle};
pub use lyon::path::builder::BorderRadii;
pub use lyon::path::Path;
pub use lyon::path::Winding;
pub use render_backend::Image;
pub use render_context::Color;
pub use render_context::Fill;
pub use render_context::GradientStop;
pub use render_context::GradientType;
pub use render_context::Stroke;
pub use render_context::WgpuRenderer;

pub struct StrokeStyle {} //TODOrefactor

pub trait RenderContext {
    fn fill_path(&mut self, path: Path, fill: Fill);
    fn stroke_path(&mut self, path: Path, stroke: Stroke);
    fn draw_image(&mut self, image: Image);
    fn push_transform(&mut self, transform: Transform2D);
    fn pop_transform(&mut self);
    fn push_clipping_bounds(&mut self, bounds: Box2D);
    fn pop_clipping_bounds(&mut self);
    fn resize(&mut self, width: f32, height: f32, dpr: f32);
    fn size(&self) -> (f32, f32);
    fn flush(&mut self);
}
