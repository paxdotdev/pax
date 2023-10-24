pub mod types;

#[allow(unused_imports)]
pub mod stacker;

pub mod components {
    pub use super::stacker::*;
}

pub mod primitives {
    use pax_lang::Pax;
    use pax_runtime_api::Size;

    use crate::types::text::TextStyle;
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
    #[primitive("pax_std_primitives::scroller::ScrollerInstance")]
    pub struct Scroller {
        pub size_inner_pane_x: pax_lang::Property<Size>,
        pub size_inner_pane_y: pax_lang::Property<Size>,
        pub scroll_enabled_x: pax_lang::Property<bool>,
        pub scroll_enabled_y: pax_lang::Property<bool>,
    }

    #[derive(Pax)]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::rectangle::RectangleInstance")]
    pub struct Rectangle {
        pub stroke: pax_lang::Property<crate::types::Stroke>,
        pub fill: pax_lang::Property<crate::types::Fill>,
        pub corner_radii: pax_lang::Property<crate::types::RectangleCornerRadii>,
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
        pub style: pax_lang::Property<TextStyle>,
        pub style_link: pax_lang::Property<TextStyle>,
    }

    #[derive(Pax)]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::checkbox::CheckboxInstance")]
    pub struct Checkbox {
        pub checked: pax_lang::Property<bool>
        //pub style: pax_lang::Property<TextStyle>,
    }

    #[derive(Pax)]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::image::ImageInstance")]
    pub struct Image {
        pub path: pax_lang::Property<String>,
    }
}
