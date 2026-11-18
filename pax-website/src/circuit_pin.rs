use pax_kit::*;

#[pax]
#[file("circuit_pin.pax")]
pub struct CircuitPin {
    pub side: Property<f64>,
}
