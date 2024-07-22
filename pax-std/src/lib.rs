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
    use pax_runtime::api::{Color, Property, Size, Stroke};
    use pax_runtime::api::{Fill, Numeric};
    use piet::util::LayoutDefaults;

    use crate::types::text::{Font, TextAlignHorizontal, TextAlignVertical, TextStyle};

    use crate::types::{ImageFit, PathElement};
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
        pub text: Property<String>,
        pub style: Property<TextStyle>,
        pub style_link: Property<TextStyle>,
    }

    #[pax]
    #[primitive("pax_std_primitives::checkbox::CheckboxInstance")]
    #[custom(Default)]
    pub struct Checkbox {
        pub background: Property<Color>,
        pub background_checked: Property<Color>,
        pub outline: Property<Stroke>,
        pub border_radius: Property<f64>,

        pub checked: Property<bool>,
    }

    impl Default for Checkbox {
        fn default() -> Self {
            Self {
                background: Property::new(Color::rgb(243.into(), 244.into(), 246.into())),
                background_checked: Property::new(Color::rgb(27.into(), 100.into(), 242.into())),
                outline: Property::new(Stroke {
                    color: Property::new(Color::rgb(209.into(), 213.into(), 219.into())),
                    width: Property::new(Size::Pixels(1.into())),
                }),
                border_radius: Property::new(5.0),
                checked: Property::new(false),
            }
        }
    }

    #[pax]
    #[primitive("pax_std_primitives::textbox::TextboxInstance")]
    #[custom(Default)]
    pub struct Textbox {
        pub text: Property<String>,
        pub background: Property<Color>,
        pub stroke: Property<Stroke>,
        pub border_radius: Property<Numeric>,
        pub style: Property<TextStyle>,
        pub focus_on_mount: Property<bool>,
    }

    impl Default for Textbox {
        fn default() -> Self {
            Self {
                text: Default::default(),
                background: Property::new(Color::rgb(249.into(), 250.into(), 251.into())),
                stroke: Property::new(Stroke {
                    color: Property::new(Color::rgb(209.into(), 213.into(), 219.into())),
                    width: Property::new(Size::Pixels(1.into())),
                }),
                border_radius: Property::new(8.0.into()),
                style: Property::new(TextStyle {
                    font: Property::new(Font::default()),
                    font_size: Property::new(Size::Pixels(Numeric::F64(14.0))),
                    fill: Property::new(Color::BLACK),
                    underline: Property::new(false),
                    align_horizontal: Property::new(TextAlignHorizontal::Left),
                    align_multiline: Property::new(TextAlignHorizontal::Left),
                    align_vertical: Property::new(TextAlignVertical::Center),
                }),
                focus_on_mount: Property::new(false),
            }
        }
    }

    #[pax]
    #[primitive("pax_std_primitives::dropdown::DropdownInstance")]
    #[custom(Default)]
    pub struct Dropdown {
        pub stroke: Property<Stroke>,
        pub options: Property<Vec<String>>,
        pub selected_id: Property<u32>,
        pub style: Property<TextStyle>,
        pub background: Property<Color>,
        pub border_radius: Property<f64>,
    }

    impl Default for Dropdown {
        fn default() -> Self {
            Self {
                options: Property::new(vec!["option 1".to_owned(), "option 2".to_owned()]),
                selected_id: Property::new(0),
                background: Property::new(Color::rgb(249.into(), 250.into(), 251.into())),
                stroke: Property::new(Stroke {
                    color: Property::new(Color::rgb(209.into(), 213.into(), 219.into())),
                    width: Property::new(Size::Pixels(1.into())),
                }),
                border_radius: Property::new(8.0.into()),
                style: Property::new(TextStyle {
                    font: Property::new(Font::default()),
                    font_size: Property::new(Size::Pixels(Numeric::F64(14.0))),
                    fill: Property::new(Color::BLACK),
                    underline: Property::new(false),
                    align_horizontal: Property::new(TextAlignHorizontal::Left),
                    align_multiline: Property::new(TextAlignHorizontal::Left),
                    align_vertical: Property::new(TextAlignVertical::Center),
                }),
            }
        }
    }

    #[pax]
    #[primitive("pax_std_primitives::radio_set::RadioSetInstance")]
    #[custom(Default)]
    pub struct RadioSet {
        pub background: Property<Color>,
        pub background_checked: Property<Color>,
        pub outline: Property<Stroke>,
        pub options: Property<Vec<String>>,
        pub selected_id: Property<u32>,
        pub style: Property<TextStyle>,
    }

    impl Default for RadioSet {
        fn default() -> Self {
            Self {
                background: Property::new(Color::rgb(243.into(), 244.into(), 246.into())),
                background_checked: Property::new(Color::rgb(27.into(), 100.into(), 242.into())),
                outline: Property::new(Stroke {
                    color: Property::new(Color::rgb(209.into(), 213.into(), 219.into())),
                    width: Property::new(Size::Pixels(1.into())),
                }),
                options: Property::new(vec!["option 1".to_string(), "option 2".to_string()]),
                selected_id: Property::new(0),
                style: Property::new(TextStyle {
                    font: Property::new(Font::default()),
                    font_size: Property::new(Size::Pixels(Numeric::F64(14.0))),
                    fill: Property::new(Color::BLACK),
                    underline: Property::new(false),
                    align_horizontal: Property::new(TextAlignHorizontal::Left),
                    align_multiline: Property::new(TextAlignHorizontal::Left),
                    align_vertical: Property::new(TextAlignVertical::Center),
                }),
            }
        }
    }

    #[pax]
    #[primitive("pax_std_primitives::slider::SliderInstance")]
    #[custom(Default)]
    pub struct Slider {
        pub background: Property<Color>,
        pub accent: Property<Color>,
        pub border_radius: Property<f64>,
        pub value: Property<f64>,
        pub step: Property<f64>,
        pub min: Property<f64>,
        pub max: Property<f64>,
    }

    impl Default for Slider {
        fn default() -> Self {
            Self {
                value: Property::new(0.0),
                step: Property::new(1.0),
                min: Property::new(0.0),
                max: Property::new(100.0),
                accent: Property::new(Color::rgb(27.into(), 100.into(), 242.into())),
                border_radius: Property::new(5.0),
                background: Property::new(Color::rgb(229.into(), 231.into(), 235.into())),
            }
        }
    }

    #[pax]
    #[primitive("pax_std_primitives::button::ButtonInstance")]
    #[custom(Default)]
    pub struct Button {
        pub label: Property<String>,
        pub color: Property<Color>,
        pub hover_color: Property<Color>,
        pub border_radius: Property<f64>,
        pub outline: Property<Stroke>,
        pub style: Property<TextStyle>,
    }

    impl Default for Button {
        fn default() -> Self {
            Self {
                color: Property::new(Color::rgb(27.into(), 100.into(), 242.into())),
                hover_color: Property::new(Color::rgb(26.into(), 86.into(), 219.into())),
                border_radius: Property::new(8.0),
                label: Property::new(String::from("button")),
                style: Property::new(TextStyle {
                    font: Property::new(Font::default()),
                    font_size: Property::new(Size::Pixels(Numeric::F64(20.0))),
                    fill: Property::new(Color::WHITE),
                    underline: Property::new(false),
                    align_multiline: Property::new(TextAlignHorizontal::Center),
                    align_vertical: Property::new(TextAlignVertical::Center),
                    align_horizontal: Property::new(TextAlignHorizontal::Center),
                }),

                outline: Property::new(Stroke::default()),
            }
        }
    }

    #[pax]
    #[primitive("pax_std_primitives::image::ImageInstance")]
    pub struct Image {
        pub path: Property<String>,
        pub fit: Property<ImageFit>,
    }

    #[pax]
    #[inlined(<Group/>)]
    pub struct BlankComponent {}
}
