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
            text_style: Property::new(common::text_style(12.0, TextAlignHorizontal::Left)),
        }
    }
}
