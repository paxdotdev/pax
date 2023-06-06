#[macro_use]
extern crate lazy_static;

pub mod types;
pub mod stacker;

pub mod components {
    pub use super::stacker::*;
}

pub mod primitives {
    use pax::Pax;

    use crate::types::PathSegment;

    #[derive(Pax)]
    #[primitive("pax_std_primitives::frame::FrameInstance")]
    pub struct Frame {}

    #[derive(Pax)]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::group::GroupInstance")]
    pub struct Group {}

    #[derive(Pax)]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::rectangle::RectangleInstance")]
    pub struct Rectangle {
        pub stroke: pax::Property<crate::types::Stroke>,
        pub fill: pax::Property<crate::types::Color>,
    }

    #[derive(Pax)]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::ellipse::EllipseInstance")]
    pub struct Ellipse {
        pub stroke: pax::Property<crate::types::Stroke>,
        pub fill: pax::Property<crate::types::Color>,
    }

    #[derive(Pax)]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::path::PathInstance")]
    pub struct Path {
        pub segments: pax::Property<Vec<PathSegment>>,
        pub stroke: pax::Property<crate::types::Stroke>,
        pub fill: pax::Property<crate::types::Color>,
    }

    #[derive(Pax)]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::text::TextInstance")]
    pub struct Text {
        pub text: pax::Property<String>,
        pub font: pax::Property<crate::types::Font>,
        pub fill: pax::Property<crate::types::Color>,
    }
}
