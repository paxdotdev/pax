#[macro_use]
extern crate lazy_static;

pub mod types;
pub mod stacker;

pub mod components {
    pub use super::stacker::*;
}

pub mod primitives {
    use pax::{pax_primitive};

    #[cfg(feature = "parser")]
    use pax_compiler;
    #[cfg(feature = "parser")]
    use pax_message::reflection::PathQualifiable;
    use crate::types::PathSegment;

    #[pax_primitive("./pax-std-primitives",  pax_std_primitives::frame::FrameInstance)]
    pub struct Frame {}

    #[pax_primitive("./pax-std-primitives",  pax_std_primitives::group::GroupInstance)]
    pub struct Group {}

    #[pax_primitive("./pax-std-primitives",  pax_std_primitives::rectangle::RectangleInstance)]
    pub struct Rectangle {
        pub stroke: pax::Property<crate::types::Stroke>,
        pub fill: pax::Property<crate::types::Color>,
    }

    #[pax_primitive("./pax-std-primitives",  pax_std_primitives::ellipse::EllipseInstance)]
    pub struct Ellipse {
        pub stroke: pax::Property<crate::types::Stroke>,
        pub fill: pax::Property<crate::types::Color>,
    }

    #[pax_primitive("./pax-std-primitives",  pax_std_primitives::path::PathInstance)]
    pub struct Path {
        pub segments: pax::Property<Vec<PathSegment>>,
        pub stroke: pax::Property<crate::types::Stroke>,
        pub fill: pax::Property<crate::types::Color>,
    }

    #[pax_primitive("./pax-std-primitives",  pax_std_primitives::text::TextInstance)]
    pub struct Text {
        pub text: pax::Property<String>,
        pub font: pax::Property<crate::types::Font>,
        pub fill: pax::Property<crate::types::Color>,
        pub align_multiline: pax::Property<Option<crate::types::TextAlignHorizontal>>,
        pub align_vertical: pax::Property<crate::types::TextAlignVertical>,
        pub align_horizontal: pax::Property<crate::types::TextAlignHorizontal>,
    }
}
