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
    <Button
        style=style
        label=label
    />
)]
#[custom(Default)]
pub struct PaxButton {
    pub label: Property<String>,
    pub color: Property<Color>,

    // private
    pub style: Property<TextStyle>,
}

impl Default for PaxButton {
    fn default() -> Self {
        Self {
            color: Property::new(Color::rgb(20.into(), 20.into(), 20.into())),
            label: Property::new("Button".to_owned()),
            style: Property::new(common::text_style(
                12.0,
                TextAlignHorizontal::Center,
                Color::WHITE,
            )),
        }
    }
}
