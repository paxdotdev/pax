use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
use std::cmp::Ordering;
use std::iter;

#[pax]
#[inlined(
    <RadioSet
        selected_id=selected_id
        options=options
        style=text_style
    />
)]
#[custom(Default)]
pub struct PaxRadioSet {
    pub selected_id: Property<u32>,
    pub options: Property<Vec<String>>,
    pub text_style: Property<TextStyle>,
}

impl Default for PaxRadioSet {
    fn default() -> Self {
        Self {
            options: Default::default(),
            selected_id: Default::default(),
            text_style: Property::new(TextStyle {
                font: Property::new(Font::default()),
                font_size: Property::new(Size::Pixels(12.0.into())),
                fill: Property::new(Color::WHITE),
                underline: Default::default(),
                align_multiline: Property::new(TextAlignHorizontal::Left),
                align_vertical: Property::new(TextAlignVertical::Center),
                align_horizontal: Property::new(TextAlignHorizontal::Left),
            }),
        }
    }
}
