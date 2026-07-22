use pax_kit::*;

#[pax]
#[custom(Default)]
#[file("pax_logo_board.pax")]
pub struct PaxLogoBoard {
    pub fill: Property<Fill>,
}

impl Default for PaxLogoBoard {
    fn default() -> Self {
        Self {
            fill: Property::new(Fill::Solid(Color::BLACK)),
        }
    }
}
