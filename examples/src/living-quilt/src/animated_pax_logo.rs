#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[custom(Default)]
#[file("animated_pax_logo.pax")]
pub struct AnimatedPaxLogo {
    pub fill: Property<Paint>,
    pub letter_fill: Property<Paint>,
    /// Normalized animation position from the initial pose (0.0) to the
    /// finished logo (1.0). Consumers own playback by binding this property.
    pub progress: Property<f64>,
}

impl Default for AnimatedPaxLogo {
    fn default() -> Self {
        Self {
            fill: Property::new(Paint::Solid(Color::BLACK)),
            letter_fill: Property::new(Paint::Solid(Color::WHITE)),
            progress: Property::new(0.0),
        }
    }
}
