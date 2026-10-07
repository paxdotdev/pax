use pax_kit::*;
use crate::Leaf;
#[pax]
#[cfg_attr(widget_config, cfg_attr(feature = "fancy", file("fancy.pax")))]
pub struct Widget {
    #[cfg(test)]
    pub dependency_test_only: Property<bool>,
    #[cfg(feature = "fancy")]
    pub fancy: Property<Numeric>,
    #[cfg(not(feature = "fancy"))]
    pub impossible: Property<MissingType>,
    #[cfg(target_arch = "wasm32")]
    pub web: Property<bool>,
    #[cfg(not(target_arch = "wasm32"))]
    pub native: Property<bool>,
    #[cfg(debug_assertions)]
    pub debug: Property<bool>,
}
