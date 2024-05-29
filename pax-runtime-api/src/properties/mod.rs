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

/// PropertyValue represents a restriction on valid generic types that a property
/// can contain. All T need to be Clone (to enable .get()) + 'static (no
/// references/ lifetimes)
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
/// A typed wrapper over a UntypedProperty that casts to/from an untyped
/// property on get/set
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
    pub fn new(val: T) -> Self {
        Self::new_optional_name(val, None)
    }

    pub fn computed(evaluator: impl Fn() -> T + 'static, dependents: &[UntypedProperty]) -> Self {
        Self::computed_with_config(evaluator, dependents, None)
    }

    pub fn new_with_name(val: T, name: &str) -> Self {
        Self::new_optional_name(val, Some(name))
    }

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

    pub fn ease_to(&self, end_val: T, time: u64, curve: EasingCurve) {
        self.ease_to_value(end_val, time, curve, true);
    }

    pub fn ease_to_later(&self, end_val: T, time: u64, curve: EasingCurve) {
        self.ease_to_value(end_val, time, curve, false);
    }

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

    /// Sets this properties value and sets the drity bit recursively of all of
    /// it's dependencies if not already set
    pub fn set(&self, val: T) {
        PROPERTY_TABLE.with(|t| t.set_value(self.untyped.id, val));
    }

    // Get access to a mutable reference to the inner value T.
    // Always updates dependents of this property, no matter
    // if the value changed or not
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        // This is a temporary impl of the update method.
        // (very bad perf comparatively, but very safe).
        let mut val = self.get();
        f(&mut val);
        self.set(val);
    }

    // Get access to a reference to the inner value T.
    pub fn read<V>(&self, f: impl FnOnce(&T) -> V) -> V {
        let val = self.get();
        f(&val)
    }

    /// replaces a properties evaluation/inbounds/value to be the same as
    /// target, while keeping it's dependents.
    /// WARNING: this method can introduce circular dependencies if one is not careful.
    /// Using it wrongly can introduce memory leaks and inconsistent property behaviour.
    /// This method can be used to replace an inner value from for example a literal to
    /// a computed computed, while keeping the link to it's dependents
    pub fn replace_with(&self, target: Property<T>) {
        PROPERTY_TABLE.with(|t| {
            // we know self contains T, and that target contains T, so this should never panic
            t.replace_property_keep_outbound_connections::<T>(self.untyped.id, target.untyped.id)
        })
    }

    /// Casts this property to it's untyped version
    pub fn untyped(&self) -> UntypedProperty {
        self.untyped.clone()
    }
}

impl<T: PropertyValue> Default for Property<T> {
    fn default() -> Self {
        Property::new(T::default())
    }
}

/// Serialization and deserialization fully disconnects properties,
/// and only loads them back in as literals.
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

/// Utility method to inspect total entry count in property table
pub fn property_table_total_properties_count() -> usize {
    PROPERTY_TABLE.with(|t| t.total_properties_count())
}

pub fn register_time(prop: &Property<u64>) {
    PROPERTY_TIME.with_borrow_mut(|time| *time = prop.clone());
}
