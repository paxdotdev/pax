#[macro_use]
extern crate lazy_static;

pub mod types;
pub mod stacker;

pub mod components {
    pub use super::stacker::*;
}

pub mod primitives {
    use pax_lang::Pax;
    use pax_lang::api::numeric::Numeric;
    use pax_lang::api::SizePixels;

    use crate::types::PathSegment;
    use crate::types::text::{LinkStyle, SizeWrapper};

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
        pub stroke: pax_lang::Property<crate::types::Stroke>,
        pub fill: pax_lang::Property<crate::types::Color>,
    }

    #[derive(Pax)]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::ellipse::EllipseInstance")]
    pub struct Ellipse {
        pub stroke: pax_lang::Property<crate::types::Stroke>,
        pub fill: pax_lang::Property<crate::types::Color>,
    }

    #[derive(Pax)]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::path::PathInstance")]
    pub struct Path {
        pub segments: pax_lang::Property<Vec<PathSegment>>,
        pub stroke: pax_lang::Property<crate::types::Stroke>,
        pub fill: pax_lang::Property<crate::types::Color>,
    }

    #[derive(Pax)]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::text::TextInstance")]
    pub struct Text {
        pub text: pax_lang::Property<String>,
        pub font: pax_lang::Property<crate::types::text::Font>,
        pub fill: pax_lang::Property<crate::types::Color>,
        // stop-gap fix SizePixel since built-in
        pub size_font: pax_lang::Property<SizeWrapper>,
        pub style_link: pax_lang::Property<Option<LinkStyle>>,
        pub align_multiline: pax_lang::Property<Option<crate::types::text::TextAlignHorizontal>>,
        pub align_vertical: pax_lang::Property<crate::types::text::TextAlignVertical>,
        pub align_horizontal: pax_lang::Property<crate::types::text::TextAlignHorizontal>,
        // stop-gap fix add required types as properties
        pub font_weight : pax_lang::Property<crate::types::text::FontWeight>,
        pub font_style : pax_lang::Property<crate::types::text::FontStyle>,
    }

    #[derive(Pax)]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::image::ImageInstance")]
    pub struct Image {
        pub path: pax_lang::Property<String>,
    }
}
