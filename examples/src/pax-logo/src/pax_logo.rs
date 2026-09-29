use pax_kit::*;

#[pax]
#[custom(Default)]
#[file("pax_logo.pax")]
pub struct PaxLogo {
    pub fill: Property<Paint>,
}

impl Default for PaxLogo {
    fn default() -> Self {
        Self {
            fill: Property::new(Paint::Solid(Color::BLACK)),
        }
    }
}
