pub mod render_backend;
pub mod render_context;
/// Affine transform type used by the retained GPU renderer.
pub type Transform2D = lyon::geom::euclid::default::Transform2D<f32>;
/// Point type used by tessellation and retained scene geometry.
pub type Point2D = lyon::geom::euclid::default::Point2D<f32>;
/// Vector type used by tessellation and retained scene geometry.
pub type Vector2D = lyon::geom::euclid::default::Vector2D<f32>;
/// Axis-aligned box type used for retained-node bounds.
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
pub use render_context::LightShape;
pub use render_context::Material;
pub use render_context::ResourceChurnStats;
pub use render_context::SceneLight;
pub use render_context::SceneLighting;
pub use render_context::Stroke;
pub use render_context::StrokeCap;
pub use render_context::WgpuRenderer;
pub use render_context::NATIVE_VECTOR_RESOURCE_CACHE_BYTES;
