//! Property adapters that expose runtime values to expression scopes.

use super::*;

/// Runtime property adapter used by expression scopes.
#[derive(Clone)]
pub struct Variable {
    untyped_property: UntypedProperty,
    converted_to_pax_value: Property<PaxValue>,
}

impl Variable {
    /// Wraps an untyped property and exposes it as a `PaxValue`.
    pub fn new<T: PropertyValue + ToPaxValue>(untyped_property: UntypedProperty) -> Self {
        Self::new_from_typed_property(Property::<T>::new_from_untyped(untyped_property.clone()))
    }

    /// Wraps a typed property and exposes it as a `PaxValue`.
    pub fn new_from_typed_property<T: PropertyValue + ToPaxValue>(property: Property<T>) -> Self {
        let untyped_property = property.untyped();
        let deps = [untyped_property.clone()];
        let pax_value_prop = Property::computed(move || property.get().to_pax_value(), &deps);
        Variable {
            untyped_property,
            converted_to_pax_value: pax_value_prop,
        }
    }

    // Returns the underlying untyped property for dependency graph wiring.
    pub fn get_untyped_property(&self) -> &UntypedProperty {
        &self.untyped_property
    }

    /// Reads the current value as a `PaxValue`.
    pub fn get_as_pax_value(&self) -> PaxValue {
        self.converted_to_pax_value.get()
    }

    /// Reads the current `PaxValue` by reference.
    pub fn read_pax_value_ref<V>(&self, f: impl FnOnce(&PaxValue) -> V) -> V {
        self.converted_to_pax_value.read(f)
    }
}
