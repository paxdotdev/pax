use pax_engine::pax;
use pax_runtime::api::{Color, Property, Size, Stroke};
use pax_runtime::api::{Fill, Numeric};

use crate::types::text::TextStyle;

use crate::types::{ImageFit, PathElement};

#[pax]
#[primitive("pax_std::frame::FrameInstance")]
pub struct Frame {}

#[pax]
#[primitive("pax_std::group::GroupInstance")]
pub struct Group {}

#[pax]
#[primitive("pax_std::scrollbar::ScrollbarInstance")]
pub struct Scrollbar {
    pub size_inner_pane_x: Property<Size>,
    pub size_inner_pane_y: Property<Size>,
    pub scroll_x: Property<f64>,
    pub scroll_y: Property<f64>,
}

#[pax]
#[primitive("pax_std::rectangle::RectangleInstance")]
pub struct Rectangle {
    pub stroke: Property<Stroke>,
    pub fill: Property<Fill>,
    pub corner_radii: Property<crate::types::RectangleCornerRadii>,
}

#[pax]
#[primitive("pax_std::primitives::ellipse::EllipseInstance")]
pub struct Ellipse {
    pub stroke: Property<Stroke>,
    pub fill: Property<Fill>,
}

#[pax]
#[primitive("pax_std::primitives::path::PathInstance")]
pub struct Path {
    pub elements: Property<Vec<PathElement>>,
    pub stroke: Property<Stroke>,
    pub fill: Property<Color>,
}

#[pax]
#[primitive("pax_std::primitives::text::TextInstance")]
pub struct Text {
    pub editable: Property<bool>,
    pub text: Property<String>,
    pub style: Property<TextStyle>,
    pub style_link: Property<TextStyle>,
}

#[pax]
#[primitive("pax_std::primitives::checkbox::CheckboxInstance")]
pub struct Checkbox {
    pub checked: Property<bool>,
}

#[pax]
#[primitive("pax_std::primitives::textbox::TextboxInstance")]
pub struct Textbox {
    pub text: Property<String>,
    pub background: Property<Color>,
    pub stroke: Property<Stroke>,
    pub border_radius: Property<Numeric>,
    pub style: Property<TextStyle>,
    pub focus_on_mount: Property<bool>,
}

#[pax]
#[primitive("pax_std::primitives::dropdown::DropdownInstance")]
pub struct Dropdown {
    pub options: Property<Vec<String>>,
    pub selected_id: Property<u32>,
    pub style: Property<TextStyle>,
    pub background: Property<Color>,
    pub stroke: Property<Stroke>,
}

#[pax]
#[primitive("pax_std::primitives::radio_set::RadioSetInstance")]
pub struct RadioSet {
    pub options: Property<Vec<String>>,
    pub selected_id: Property<u32>,
    pub style: Property<TextStyle>,
    pub background: Property<Color>,
}

#[pax]
#[primitive("pax_std::primitives::slider::SliderInstance")]
pub struct Slider {
    pub value: Property<f64>,
    pub step: Property<f64>,
    pub min: Property<f64>,
    pub max: Property<f64>,
    pub accent: Property<Color>,
}

#[pax]
#[primitive("pax_std::primitives::button::ButtonInstance")]
pub struct Button {
    pub label: Property<String>,
    pub color: Property<Color>,
    pub style: Property<TextStyle>,
}

#[pax]
#[primitive("pax_std::primitives::image::ImageInstance")]
pub struct Image {
    pub path: Property<String>,
    pub fit: Property<ImageFit>,
}

#[pax]
#[inlined(< Group / >)]
pub struct BlankComponent {}


pub mod button;
pub mod checkbox;
pub mod dropdown;
pub mod ellipse;
pub mod frame;
pub mod group;
pub mod image;
pub mod path;
pub mod radio_set;
pub mod rectangle;
pub mod scrollbar;
pub mod slider;
pub mod text;
pub mod textbox;


fn patch_if_needed<T: PartialEq + Clone>(
    old_state: &mut Option<T>,
    patch: &mut Option<T>,
    new_value: T,
) -> bool {
    if !old_state.as_ref().is_some_and(|v| v == &new_value) {
        *patch = Some(new_value.clone());
        *old_state = Some(new_value);
        true
    } else {
        false
    }
}
