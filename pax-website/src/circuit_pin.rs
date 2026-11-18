use pax_kit::*;

#[pax]
#[file("circuit_pin.pax")]
pub struct CircuitPin {
    /// -1 or 1 bends toward the package. Zero is a straight breakout socket contact.
    pub side: Property<f64>,
}
