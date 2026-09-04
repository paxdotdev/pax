use pax_kit::*;

#[pax]
#[custom(Default)]
#[file("logo_card.pax")]
pub struct LogoCard {
    pub is_compact: Property<bool>,
    pub logo_progress: Property<f64>,
}

impl Default for LogoCard {
    fn default() -> Self {
        Self {
            is_compact: Property::new(false),
            logo_progress: Property::new(1.0),
        }
    }
}
