#[macro_use]
extern crate lazy_static;

pub mod types;
pub mod stacker;

pub mod components {
    pub use super::stacker::*;
}

pub mod primitives {
    use pax::{pax_primitive};
    use pax::api::numeric::Numeric;
    use pax::api::SizePixels;

    #[cfg(feature = "parser")]
    use pax_compiler;
    #[cfg(feature = "parser")]
    use pax_message::reflection::PathQualifiable;
    use crate::types::PathSegment;
    use crate::types::text::{LinkStyle, SizeWrapper};

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
        pub font: pax::Property<crate::types::text::Font>,
        pub fill: pax::Property<crate::types::Color>,
        // stop-gap fix SizePixel since built-in
        pub size_font: pax::Property<SizeWrapper>,
        pub style_link: pax::Property<Option<LinkStyle>>,
        pub align_multiline: pax::Property<Option<crate::types::text::TextAlignHorizontal>>,
        pub align_vertical: pax::Property<crate::types::text::TextAlignVertical>,
        pub align_horizontal: pax::Property<crate::types::text::TextAlignHorizontal>,
        // stop-gap fix add required types as properties
        pub font_weight : pax::Property<crate::types::text::FontWeight>,
        pub font_style : pax::Property<crate::types::text::FontStyle>,
    }

    #[pax_primitive("./pax-std-primitives",  pax_std_primitives::image::ImageInstance)]
    pub struct Image {
        pub path: pax::Property<String>,
    }
}
