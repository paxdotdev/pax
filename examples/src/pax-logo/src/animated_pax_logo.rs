#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[custom(Default)]
#[file("animated_pax_logo.pax")]
pub struct AnimatedPaxLogo {
    pub fill: Property<Fill>,
    pub letter_fill: Property<Fill>,
    /// Normalized animation position from the initial pose (0.0) to the
    /// finished logo (1.0). Consumers own playback by binding this property.
    pub progress: Property<f64>,
}

impl Default for AnimatedPaxLogo {
    fn default() -> Self {
        Self {
            fill: Property::new(Fill::Solid(Color::BLACK)),
            letter_fill: Property::new(Fill::Solid(Color::WHITE)),
            progress: Property::new(0.0),
        }
    }
}
