use pax_kit::*;

#[pax]
#[custom(Default)]
#[file("pax_logo_board.pax")]
pub struct PaxLogoBoard {
    pub fill: Property<Paint>,
}

impl Default for PaxLogoBoard {
    fn default() -> Self {
        Self {
            fill: Property::new(Paint::Solid(Color::BLACK)),
        }
    }
}
