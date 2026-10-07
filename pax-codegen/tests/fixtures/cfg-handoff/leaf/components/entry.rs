mod private {
    use pax_kit::*;
    #[pax_kit::pax]
    #[inlined(@settings {})]
    pub struct Leaf {
        #[cfg(feature = "selected")]
        pub selected: Property<bool>,
        #[cfg(not(feature = "selected"))]
        pub impossible: Property<MissingType>,
        #[cfg(test)]
        pub dependency_test_only: Property<bool>,
    }
}
pub use private::Leaf;
