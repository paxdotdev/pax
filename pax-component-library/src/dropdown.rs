use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
use std::cmp::Ordering;
use std::iter;

use crate::common;

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
            text_style: Property::new(common::text_style(
                12.0,
                TextAlignHorizontal::Center,
                Color::WHITE,
            )),
            background: Property::new(Color::rgb(30.into(), 30.into(), 30.into())),
            stroke: Property::new(Stroke {
                color: Property::new(Color::BLACK),
                width: Property::new(Size::Pixels(1.0.into())),
            }),
        }
    }
}
