use pax_kit::*;

use crate::kaleidoscope::KaleidoscopeVariant;

#[pax]
#[custom(Default)]
#[file("tendril_limb.pax")]
pub struct TendrilLimb {
    pub variant: Property<KaleidoscopeVariant>,
    pub shape_index: Property<usize>,
    pub segment: Property<f64>,
    pub thickness: Property<f64>,
}

impl Default for TendrilLimb {
    fn default() -> Self {
        Self {
            variant: Property::new(KaleidoscopeVariant::Crimson),
            shape_index: Property::new(0),
            segment: Property::new(120.0),
            thickness: Property::new(36.0),
        }
    }
}
