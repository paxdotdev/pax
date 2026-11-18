use pax_kit::*;

// Canonical Pax artwork ejected as filled contours, so the PCB remains visible
// through the lettering without a background-colored patch or an alpha mask.
#[pax]
#[file("circuit_logo.pax")]
pub struct CircuitLogo {
    pub fill: Property<Vec<Fill>>,
}
