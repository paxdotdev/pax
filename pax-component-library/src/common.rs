use pax_engine::{api::Color, Property};
use pax_std::types::{
    text::{Font, TextAlignHorizontal, TextAlignVertical, TextStyle},
    Size,
};

pub(crate) fn text_style(
    size: f64,
    align_horizontal: TextAlignHorizontal,
) -> pax_std::types::text::TextStyle {
    TextStyle {
        font: Property::new(Font::default()),
        font_size: Property::new(Size::Pixels(size.into())),
        fill: Property::new(Color::WHITE),
        underline: Default::default(),
        align_multiline: Property::new(align_horizontal.clone()),
        align_vertical: Property::new(TextAlignVertical::Center),
        align_horizontal: Property::new(align_horizontal),
    }
}
