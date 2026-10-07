use pax_kit::*;
use crate::Leaf;
#[pax]
#[cfg_attr(widget_config, file("plain.pax"))]
#[cfg_attr(not(widget_config), file("absent.pax"))]
pub struct Widget {
    #[cfg(test)]
    pub dependency_test_only: Property<bool>,
    #[cfg(not(feature = "fancy"))]
    pub plain: Property<Numeric>,
    #[cfg(feature = "fancy")]
    pub impossible: Property<MissingType>,
    #[cfg(target_arch = "wasm32")]
    pub web: Property<bool>,
    #[cfg(not(target_arch = "wasm32"))]
    pub native: Property<bool>,
    #[cfg(debug_assertions)]
    pub debug: Property<bool>,
}
