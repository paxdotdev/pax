#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[custom(Default)]
#[file("animated_pax_logo.pax")]
pub struct AnimatedPaxLogo {
    pub fill: Property<Fill>,
    pub letter_fill: Property<Fill>,
    pub p_billow_lower: Property<f64>,
    pub p_billow_upper: Property<f64>,
    pub a_billow_lower: Property<f64>,
    pub a_billow_upper: Property<f64>,
    pub x_billow_lower: Property<f64>,
    pub x_billow_upper: Property<f64>,
}

impl Default for AnimatedPaxLogo {
    fn default() -> Self {
        Self {
            fill: Property::new(Fill::Solid(Color::BLACK)),
            letter_fill: Property::new(Fill::Solid(Color::WHITE)),
            p_billow_lower: Property::new(0.0),
            p_billow_upper: Property::new(0.0),
            a_billow_lower: Property::new(0.0),
            a_billow_upper: Property::new(0.0),
            x_billow_lower: Property::new(0.0),
            x_billow_upper: Property::new(0.0),
        }
    }
}
