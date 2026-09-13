//! Property adapters that expose runtime values to expression scopes.

use super::*;

#[cfg(test)]
mod tests;

/// Runtime property adapter used by expression scopes.
#[derive(Clone)]
pub struct Variable {
    untyped_property: UntypedProperty,
    converted_to_pax_value: Property<PaxValue>,
    rust_type: std::any::TypeId,
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
            rust_type: std::any::TypeId::of::<T>(),
        }
    }

    /// Creates an independent one-way binding when exact typed forwarding is
    /// safe. Unlike a double binding, writes/easing on the result never mutate
    /// the source. Mismatched types and custom conversions return `None`.
    pub fn try_typed_binding<T: PropertyValue + CoercionRules>(
        &self,
        name: &str,
    ) -> Option<Property<T>> {
        if self.rust_type != std::any::TypeId::of::<T>()
            || !self.untyped_property.has_value_type::<T>()
            || !pax_value::is_typed_binding_safe::<T>()
        {
            return None;
        }
        // Both adapter and storage types were checked. Replacement preserves T
        // and the generational property handle; no borrowed value escapes.
        let source = Property::<T>::new_from_untyped(self.untyped_property.clone());
        Some(Property::computed_with_name(
            move || source.get(),
            &[self.untyped_property.clone()],
            name,
        ))
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
