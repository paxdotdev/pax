pub mod types;

#[allow(unused_imports)]
pub mod stacker;

pub mod components {
    pub use super::stacker::*;
}

pub mod primitives {
    use pax_lang::pax;
    use pax_runtime_api::Property;
    use pax_runtime_api::Size;
    use pax_runtime_api::StringBox;

    use crate::types::text::TextStyle;
    use crate::types::PathSegment;

    #[pax]
    #[primitive("pax_std_primitives::frame::FrameInstance")]
    pub struct Frame {}

    #[pax]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::group::GroupInstance")]
    pub struct Group {}

    #[pax]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::scroller::ScrollerInstance")]
    pub struct Scroller {
        pub size_inner_pane_x: Property<Size>,
        pub size_inner_pane_y: Property<Size>,
        pub scroll_enabled_x: Property<bool>,
        pub scroll_enabled_y: Property<bool>,
    }

    #[pax]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::rectangle::RectangleInstance")]
    #[cfg_attr(debug_assertions, derive(Debug))]
    pub struct Rectangle {
        pub stroke: Property<crate::types::Stroke>,
        pub fill: Property<crate::types::Fill>,
        pub corner_radii: Property<crate::types::RectangleCornerRadii>,
    }

    #[pax]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::ellipse::EllipseInstance")]
    #[cfg_attr(debug_assertions, derive(Debug))]
    pub struct Ellipse {
        pub stroke: Property<crate::types::Stroke>,
        pub fill: Property<crate::types::Fill>,
    }

    #[pax]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::path::PathInstance")]
    #[cfg_attr(debug_assertions, derive(Debug))]
    pub struct Path {
        pub segments: Property<Vec<PathSegment>>,
        pub stroke: Property<crate::types::Stroke>,
        pub fill: Property<crate::types::Color>,
    }

    #[pax]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::text::TextInstance")]
    #[cfg_attr(debug_assertions, derive(Debug))]
    pub struct Text {
        pub text: Property<StringBox>,
        pub style: Property<TextStyle>,
        pub style_link: Property<TextStyle>,
    }

    #[pax]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::checkbox::CheckboxInstance")]
    pub struct Checkbox {
        pub checked: Property<bool>,
    }

    #[pax]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::textbox::TextboxInstance")]
    pub struct Textbox {
        pub text: Property<StringBox>,
    }

    #[pax]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::button::ButtonInstance")]
    pub struct Button {
        pub label: Property<StringBox>,
        pub style: Property<TextStyle>,
    }

    #[pax]
    #[custom(Imports)]
    #[primitive("pax_std_primitives::image::ImageInstance")]
    pub struct Image {
        pub path: Property<StringBox>,
    }
}
