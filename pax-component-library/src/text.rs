use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
use std::cmp::Ordering;
use std::iter;

use crate::common;

#[pax]
#[inlined(
    <Text
        style=style
        text=text
        width=100%
        height=100%
    />
)]
#[custom(Default)]
pub struct PaxText {
    pub align: Property<TextAlignHorizontal>,
    pub size: Property<f64>,
    pub color: Property<Color>,
    pub text: Property<String>,

    // private
    pub style: Property<TextStyle>,
}

impl Default for PaxText {
    fn default() -> Self {
        let align = Property::new(TextAlignHorizontal::Center);
        let size = Property::new(12.0);
        let color = Property::new(Color::WHITE);

        let cp_align = align.clone();
        let cp_size = size.clone();
        let cp_color = color.clone();
        let deps = [color.untyped(), cp_align.untyped(), cp_size.untyped()];
        let style = Property::computed(
            move || common::text_style(cp_size.get(), cp_align.get(), cp_color.get()),
            &deps,
        );
        Self {
            color,
            align,
            size,
            style,
            text: Property::default(),
        }
    }
}
