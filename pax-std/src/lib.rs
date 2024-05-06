pub mod types;

#[allow(unused_imports)]
pub mod scroller;
#[allow(unused_imports)]
pub mod stacker;

pub mod components {
    pub use super::scroller::*;
    pub use super::stacker::*;
}

pub mod primitives {
    use pax_engine::pax;
    use pax_runtime::api::{Color, Property, Size, StringBox, Stroke};
    use pax_runtime::api::{Fill, Numeric};

    use crate::types::text::TextStyle;

    use crate::types::PathElement;
    #[pax]
    #[primitive("pax_std_primitives::frame::FrameInstance")]
    pub struct Frame {}

    #[pax]
    #[primitive("pax_std_primitives::group::GroupInstance")]
    pub struct Group {}

    #[pax]
    #[primitive("pax_std_primitives::scrollbar::ScrollbarInstance")]
    pub struct Scrollbar {
        pub size_inner_pane_x: Property<Size>,
        pub size_inner_pane_y: Property<Size>,
        pub scroll_x: Property<f64>,
        pub scroll_y: Property<f64>,
    }

    #[pax]
    #[primitive("pax_std_primitives::rectangle::RectangleInstance")]
    pub struct Rectangle {
        pub stroke: Property<Stroke>,
        pub fill: Property<Fill>,
        pub corner_radii: Property<crate::types::RectangleCornerRadii>,
    }

    #[pax]
    #[primitive("pax_std_primitives::ellipse::EllipseInstance")]
    pub struct Ellipse {
        pub stroke: Property<Stroke>,
        pub fill: Property<Fill>,
    }

    #[pax]
    #[primitive("pax_std_primitives::path::PathInstance")]
    pub struct Path {
        pub elements: Property<Vec<PathElement>>,
        pub stroke: Property<Stroke>,
        pub fill: Property<Color>,
    }

    #[pax]
    #[primitive("pax_std_primitives::text::TextInstance")]
    pub struct Text {
        pub editable: Property<bool>,
        pub text: Property<StringBox>,
        pub style: Property<TextStyle>,
        pub style_link: Property<TextStyle>,
    }

    #[pax]
    #[primitive("pax_std_primitives::checkbox::CheckboxInstance")]
    pub struct Checkbox {
        pub checked: Property<bool>,
    }

    #[pax]
    #[primitive("pax_std_primitives::textbox::TextboxInstance")]
    pub struct Textbox {
        pub text: Property<StringBox>,
        pub background: Property<Color>,
        pub stroke: Property<Stroke>,
        pub border_radius: Property<Numeric>,
        pub style: Property<TextStyle>,
        pub focus_on_mount: Property<bool>,
    }

    #[pax]
    #[primitive("pax_std_primitives::button::ButtonInstance")]
    pub struct Button {
        pub label: Property<StringBox>,
        pub style: Property<TextStyle>,
    }

    #[pax]
    #[primitive("pax_std_primitives::image::ImageInstance")]
    pub struct Image {
        pub path: Property<StringBox>,
    }

    #[pax]
    #[inlined(<Group/>)]
    pub struct BlankComponent {}
}
