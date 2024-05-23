use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
use std::cmp::Ordering;
use std::iter;

#[pax]
#[inlined(
    <Dropdown options=options
        selected_id=selected_id
        style=text_style
        background=background
        stroke=stroke
    />
)]
#[custom(Default)]
pub struct PaxDropdown {
    pub options: Property<Vec<String>>,
    pub selected_id: Property<u32>,
    pub text_style: Property<TextStyle>,
    pub background: Property<Color>,
    pub stroke: Property<Stroke>,
}

impl Default for PaxDropdown {
    fn default() -> Self {
        Self {
            options: Property::default(),
            selected_id: Property::default(),
            text_style: Property::new(TextStyle {
                font: Property::new(Font::default()),
                font_size: Property::new(Size::Pixels(12.0.into())),
                fill: Property::new(Color::WHITE),
                underline: Default::default(),
                align_multiline: Property::new(TextAlignHorizontal::Center),
                align_vertical: Property::new(TextAlignVertical::Center),
                align_horizontal: Property::new(TextAlignHorizontal::Center),
            }),
            background: Property::new(Color::rgb(30.into(), 30.into(), 30.into())),
            stroke: Property::new(Stroke {
                color: Property::new(Color::BLACK),
                width: Property::new(Size::Pixels(1.0.into())),
            }),
        }
    }
}
