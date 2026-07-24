#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[custom(Default)]
#[file("animated_pax_logo.pax")]
pub struct AnimatedPaxLogo {
    pub fill: Property<Fill>,
    pub letter_fill: Property<Fill>,
}

impl Default for AnimatedPaxLogo {
    fn default() -> Self {
        Self {
            fill: Property::new(Fill::Solid(Color::BLACK)),
            letter_fill: Property::new(Fill::Solid(Color::WHITE)),
        }
    }
}
