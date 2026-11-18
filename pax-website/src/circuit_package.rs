use crate::circuit_pin::CircuitPin;
use pax_kit::*;

#[pax]
#[file("circuit_package.pax")]
pub struct CircuitPackage {
    pub label: Property<String>,
    pub reference: Property<String>,
    pub pins_per_side: Property<usize>,
    pub face_opacity: Property<f64>,
}
