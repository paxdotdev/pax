//! LocalProperty adapters that expose runtime values to expression scopes.

use super::*;
use std::{cell::LazyCell, rc::Rc};

type PaxValueAdapter =
    LazyCell<LocalProperty<PaxValue>, Box<dyn FnOnce() -> LocalProperty<PaxValue>>>;

// Keep fast preparation explicit if CommonProperties gains a new value type.
// Nested reactive fields or custom conversions require the normal path.
pub(crate) trait CommonPropertyValue: PropertyValue + ToPaxValue {}
impl CommonPropertyValue for Option<String> {}
impl CommonPropertyValue for Option<Size> {}
impl CommonPropertyValue for Option<Rotation> {}
impl CommonPropertyValue for Option<Transform2D> {}
impl CommonPropertyValue for Option<Opacity> {}
impl CommonPropertyValue for Option<LayoutRole> {}
impl CommonPropertyValue for Option<bool> {}

#[cfg(test)]
mod tests;

/// Runtime property adapter used by expression scopes.
///
/// Scope construction and typed forwarding keep the original property handle.
/// The cached `PaxValue` conversion graph is created on its first dynamic read;
/// cloned variables share that cache, in the original graph and node scope.
/// Calling `prepare()` initializes the value before lifecycle work. Common-property
/// scope variables can do this without creating their conversion graph.
#[derive(Clone)]
pub struct Variable {
    untyped_property: UntypedProperty,
    converted_to_pax_value: Rc<PaxValueAdapter>,
    rust_type: std::any::TypeId,
    prepare_value: Option<fn(&UntypedProperty)>,
}

impl Variable {
    /// Wraps an untyped property and exposes it as a `PaxValue`.
    pub fn new<T: PropertyValue + ToPaxValue>(untyped_property: UntypedProperty) -> Self {
        Self::new_from_typed_property(LocalProperty::<T>::new_from_untyped(
            untyped_property.clone(),
        ))
    }

    /// Wraps a typed property and exposes it as a `PaxValue`.
    pub fn new_from_typed_property<T: PropertyValue + ToPaxValue>(
        property: impl PropertyBinding<T>,
    ) -> Self {
        let property = property.local();
        let untyped_property = property.untyped();
        let initialize = crate::properties::owner_initializer(move || {
            let deps = [property.untyped()];
            let nested_watch = std::cell::RefCell::<Option<LocalProperty<()>>>::new(None);
            LocalProperty::computed(
                move || {
                    let value = property.get();
                    let dependencies = value.nested_properties();
                    let mut watch = nested_watch.borrow_mut();
                    if dependencies.is_empty() {
                        // Primitive snapshots need no nested subscription. Release
                        // a previous one when a compound value loses its fields.
                        watch.take();
                    } else {
                        let source = property.clone();
                        let initial = std::cell::Cell::new(true);
                        // Graph edges store IDs, not owning handles. Retain the
                        // nested views so unmount/remount can restore their identity.
                        let observed = dependencies.clone();
                        let next = LocalProperty::computed(
                            move || {
                                let _ = &observed;
                                if !initial.replace(false) {
                                    source.notify_nested_change();
                                }
                            },
                            &dependencies,
                        );
                        if let Some(watch) = watch.as_ref() {
                            watch.replace_with(next);
                        } else {
                            *watch = Some(next);
                        }
                        let watch = watch.as_ref().unwrap();
                        crate::properties::register_effect_property(watch);
                        watch.get();
                    }
                    value.to_pax_value_in_graph()
                },
                &deps,
            )
        });
        let converted_to_pax_value: Rc<PaxValueAdapter> =
            Rc::new(LazyCell::new(Box::new(initialize)));
        Variable {
            untyped_property,
            converted_to_pax_value,
            rust_type: std::any::TypeId::of::<T>(),
            prepare_value: None,
        }
    }

    // Only CommonProperties opts in: its known plain value types need neither
    // custom conversion nor nested-property observers to prepare for mount.
    pub(crate) fn new_from_common_property<T: CommonPropertyValue>(
        property: LocalProperty<T>,
    ) -> Self {
        fn prepare_value<T: PropertyValue>(property: &UntypedProperty) {
            LocalProperty::<T>::new_from_untyped(property.clone()).read(|_| ());
        }

        let mut variable = Self::new_from_typed_property(property);
        variable.prepare_value = Some(prepare_value::<T>);
        variable
    }

    /// Initializes the current value before lifecycle work.
    ///
    /// Common-property scope variables evaluate their typed value without
    /// allocating a cached `PaxValue` adapter. Other variables use the normal
    /// conversion path, preserving custom conversions and nested observers.
    /// Repeated calls reuse clean values; subsequent writes remain reactive.
    pub fn prepare(&self) {
        if let Some(prepare_value) = self.prepare_value {
            prepare_value(&self.untyped_property);
        } else {
            self.get_as_pax_value();
        }
    }

    /// Creates an independent one-way binding when exact typed forwarding is
    /// safe. Unlike a double binding, writes/easing on the result never mutate
    /// the source. Mismatched types and custom conversions return `None`.
    pub fn try_typed_binding<T: PropertyValue + CoercionRules>(
        &self,
        name: &str,
    ) -> Option<LocalProperty<T>> {
        if self.rust_type != std::any::TypeId::of::<T>()
            || !self.untyped_property.has_value_type::<T>()
            || !pax_value::is_typed_binding_safe::<T>()
        {
            return None;
        }
        // Both adapter and storage types were checked. Replacement preserves T
        // and the generational property handle; no borrowed value escapes.
        let source = LocalProperty::<T>::new_from_untyped(self.untyped_property.clone());
        Some(LocalProperty::computed_with_name(
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
        LazyCell::force(&self.converted_to_pax_value).get()
    }

    /// Reads the current `PaxValue` by reference.
    pub fn read_pax_value_ref<V>(&self, f: impl FnOnce(&PaxValue) -> V) -> V {
        LazyCell::force(&self.converted_to_pax_value).read(f)
    }
}
