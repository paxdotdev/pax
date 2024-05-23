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
    <Slider accent=accent
        value=value
        step=step
        min=min
        max=max
    />
)]
#[custom(Default)]
pub struct PaxSlider {
    pub accent: Property<Color>,
    pub value: Property<f64>,
    pub step: Property<f64>,
    pub min: Property<f64>,
    pub max: Property<f64>,
}

impl Default for PaxSlider {
    fn default() -> Self {
        Self {
            accent: Property::new(Color::GRAY),
            value: Property::new(0.5),
            step: Property::new(0.01),
            min: Property::new(0.0),
            max: Property::new(1.0),
        }
    }
}
