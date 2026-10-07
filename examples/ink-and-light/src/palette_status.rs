use pax_kit::*;

#[pax]
#[file("palette_status.pax")]
pub struct PaletteStatus {
    pub is_sealed: Property<bool>,
}
