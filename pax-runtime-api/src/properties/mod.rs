use serde::{Deserialize, Serialize};
use std::{marker::PhantomData, rc::Rc};

mod graph_operations;
mod properties_table;
#[cfg(test)]
mod tests;
mod untyped_property;

use crate::{EasingCurve, Interpolatable, TransitionQueueEntry};

use self::properties_table::{PropertyType, PROPERTY_TIME};
use properties_table::PROPERTY_TABLE;
pub use untyped_property::UntypedProperty;

/// Sealed PropertyId needed for slotmap (strictly internal)
mod private {
    slotmap::new_key_type!(
        pub struct PropertyId;
    );
}

/// Bound for values that can live inside Pax `Property<T>`.
///
/// Values must be cloneable for `.get()`, interpolatable for transitions, and
/// `'static` because properties are stored in the runtime graph.
pub trait PropertyValue: Default + Clone + Interpolatable + 'static {}
impl<T: Default + Clone + Interpolatable + 'static> PropertyValue for T {}

impl<T: PropertyValue> Interpolatable for Property<T> {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        let cp_self = self.clone();
        let cp_other = other.clone();
        Property::computed(
            move || cp_self.get().interpolate(&cp_other.get(), t),
            &[self.untyped(), other.untyped()],
        )
    }
}
/// A reactive value node in Pax's property graph.
///
/// `Property<T>` is the primary state and binding primitive used by generated
/// components, PAXEL expressions, and Rust component logic.
#[derive(Clone)]
pub struct Property<T> {
    untyped: UntypedProperty,
    _phantom: PhantomData<T>,
}

impl<T: PropertyValue + std::fmt::Debug> std::fmt::Debug for Property<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Property ({:?})", self.get())
    }
}

impl<T: PropertyValue> Property<T> {
    /// Creates a literal property with an initial value.
    pub fn new(val: T) -> Self {
        Self::new_optional_name(val, None)
    }

    // Rewraps an untyped property with a typed view.
    pub fn new_from_untyped(untyped: UntypedProperty) -> Self {
        Self {
            untyped,
            _phantom: PhantomData {},
        }
    }

    /// Creates a computed property from an evaluator and dependency list.
    pub fn computed(evaluator: impl Fn() -> T + 'static, dependents: &[UntypedProperty]) -> Self {
        Self::computed_with_config(evaluator, dependents, None)
    }

    /// Creates a named literal property, useful for diagnostics.
    pub fn new_with_name(val: T, name: &str) -> Self {
        Self::new_optional_name(val, Some(name))
    }

    /// Creates a named computed property, useful for diagnostics.
    pub fn computed_with_name(
        evaluator: impl Fn() -> T + 'static,
        dependents: &[UntypedProperty],
        name: &str,
    ) -> Self {
        Self::computed_with_config(evaluator, dependents, Some(name))
    }

    fn new_optional_name(val: T, name: Option<&str>) -> Self {
        Self {
            untyped: UntypedProperty::new(val, Vec::with_capacity(0), PropertyType::Literal, name),
            _phantom: PhantomData {},
        }
    }

    fn computed_with_config(
        evaluator: impl Fn() -> T + 'static,
        dependents: &[UntypedProperty],
        name: Option<&str>,
    ) -> Self {
        let inbound: Vec<_> = dependents.iter().map(|v| v.get_id()).collect();
        let start_val = T::default();
        let evaluator = Rc::new(evaluator);
        Self {
            untyped: UntypedProperty::new(
                start_val,
                inbound,
                PropertyType::Computed { evaluator },
                name,
            ),
            _phantom: PhantomData {},
        }
    }

    /// Immediately starts an ease transition from the current value to end_val, over time frames, following curve.
    pub fn ease_to(&self, end_val: T, time: u64, curve: EasingCurve) {
        self.ease_to_value(end_val, time, curve, true);
    }

    /// Enqueues an ease transition from the current value to end_val, over time frames, following
    /// curve, which will start after all currently enqueued transitions finish.
    pub fn ease_to_later(&self, end_val: T, time: u64, curve: EasingCurve) {
        self.ease_to_value(end_val, time, curve, false);
    }

    /// Shared logic for easing operations
    fn ease_to_value(&self, end_val: T, time: u64, curve: EasingCurve, overwrite: bool) {
        PROPERTY_TABLE.with(|t| {
            t.transition(
                self.untyped.id,
                TransitionQueueEntry {
                    duration_frames: time,
                    curve,
                    ending_value: end_val,
                },
                overwrite,
            )
        })
    }

    /// Gets the currently stored value. Might be computationally
    /// expensive in a large reactivity network since this triggers
    /// re-evaluation of dirty property chains
    pub fn get(&self) -> T {
        PROPERTY_TABLE.with(|t| t.get_value(self.untyped.id))
    }

    /// Sets this properties value and sets the dirty bit recursively of all of
    /// its dependencies if not already set
    pub fn set(&self, val: T) {
        PROPERTY_TABLE.with(|t| t.set_value(self.untyped.id, val));
    }

    /// Get access to a mutable reference to the inner value T.
    /// Will trigger updates for dependents of this property, regardless
    /// of if the value actually changed
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        // This is a temporary impl of the update method.
        // (very bad perf comparatively, but very safe).
        let mut val = self.get();
        f(&mut val);
        self.set(val);
    }

    /// Reads the inner value by reference.
    ///
    /// Panics if this property is already borrowed, which can happen if `read`
    /// is called inside a read of the same property.
    pub fn read<V>(&self, f: impl FnOnce(&T) -> V) -> V {
        PROPERTY_TABLE.with(|t| t.read_value(self.untyped.id, f))
    }

    /// Replaces this property's evaluator, dependencies, and value with `target`, while keeping dependents.
    ///
    /// This can introduce circular dependencies if used carelessly. It is
    /// intended for changing a property from literal to computed (or vice
    /// versa) without severing existing outbound links.
    pub fn replace_with(&self, target: Property<T>) {
        PROPERTY_TABLE.with(|t| {
            // we know self contains T, and that target contains T, so this should never panic
            t.replace_property_keep_outbound_connections::<T>(self.untyped.id, target.untyped.id)
        })
    }

    /// Casts this property to its untyped version.
    pub fn untyped(&self) -> UntypedProperty {
        self.untyped.clone()
    }
}

impl<T: PropertyValue> Default for Property<T> {
    fn default() -> Self {
        Property::new(T::default())
    }
}

// Serialization and deserialization fully disconnects properties,
// and only loads them back in as literal values.
impl<'de, T: PropertyValue + Deserialize<'de>> Deserialize<'de> for Property<T> {
    fn deserialize<D>(deserializer: D) -> Result<Property<T>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = T::deserialize(deserializer)?;
        Ok(Property::new(value))
    }
}

impl<T: PropertyValue + Serialize> Serialize for Property<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // TODO check if literal or computed, error on computed?
        self.get().serialize(serializer)
    }
}

// Utility method to inspect total entry count in property table.
pub fn property_table_total_properties_count() -> usize {
    PROPERTY_TABLE.with(|t| t.total_properties_count())
}

#[doc(hidden)]
pub fn register_effect_property(prop: &Property<()>) {
    PROPERTY_TABLE.with(|t| t.register_effect(prop.untyped.id));
}

#[doc(hidden)]
pub fn drain_effects(max_iterations: usize) -> usize {
    PROPERTY_TABLE.with(|t| t.drain_effects(max_iterations))
}

// Registers the runtime clock property used by transition/easing machinery.
pub fn register_time(prop: &Property<u64>) {
    PROPERTY_TIME.with_borrow_mut(|time| *time = prop.clone());
}
