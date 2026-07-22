use pax_kit::*;

#[pax]
#[custom(Default)]
#[file("pax_logo.pax")]
pub struct PaxLogo {
    pub fill: Property<Fill>,
}

impl Default for PaxLogo {
    fn default() -> Self {
        Self {
            fill: Property::new(Fill::Solid(Color::BLACK)),
        }
    }
}
