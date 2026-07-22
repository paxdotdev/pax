#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[custom(Default)]
#[file("animated_pax_logo.pax")]
pub struct AnimatedPaxLogo {
    pub fill: Property<Fill>,
    pub playhead: Property<f64>,
}

impl Default for AnimatedPaxLogo {
    fn default() -> Self {
        Self {
            fill: Property::new(Fill::Solid(Color::BLACK)),
            playhead: Property::new(0.0),
        }
    }
}
