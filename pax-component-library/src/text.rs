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
    pub text: Property<String>,

    // private
    pub style: Property<TextStyle>,
}

impl Default for PaxText {
    fn default() -> Self {
        let align = Property::new(TextAlignHorizontal::Center);
        let size = Property::new(12.0);

        let cp_align = align.clone();
        let cp_size = size.clone();
        let deps = [cp_align.untyped(), cp_size.untyped()];
        let style = Property::computed(
            move || common::text_style(cp_size.get(), cp_align.get()),
            &deps,
        );
        Self {
            align,
            size,
            style,
            text: Property::default(),
        }
    }
}
