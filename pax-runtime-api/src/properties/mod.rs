use serde::{Deserialize, Serialize};
use std::{any::Any, marker::PhantomData, rc::Rc};

mod graph_operations;
mod properties_table;
#[cfg(test)]
mod tests;
mod untyped_property;

use self::properties_table::{DirtificationFilter, PropertyType};
use properties_table::PROPERTY_TABLE;
pub use untyped_property::UntypedProperty;

/// Sealed PropertyId needed for slotmap (strictly internal)
mod private {
    slotmap::new_key_type!(
        pub struct PropertyId;
    );
}

/// PropertyValue represents a restriction on valid generic types that a property
/// can contain. All T need to be Clone (to enable .get()) + 'static (no
/// references/ lifetimes)
pub trait PropertyValue: Default + Clone + 'static {}
impl<T: Default + Clone + 'static> PropertyValue for T {}

/// A typed wrapper over a UntypedProperty that casts to/from an untyped
/// property on get/set
#[derive(Debug, Clone)]
pub struct Property<T> {
    untyped: UntypedProperty,
    _phantom: PhantomData<T>,
}

impl<T: PropertyValue> Property<T> {
    pub fn new(val: T) -> Self {
        Self::new_optional_name(val, None)
    }

    pub fn computed(
        evaluator: impl Fn() -> T + 'static,
        dependents: &Vec<&UntypedProperty>,
    ) -> Self {
        Self::computed_with_config(evaluator, dependents, |_, _| false, None)
    }

    pub fn new_with_name(val: T, name: &str) -> Self {
        Self::new_optional_name(val, Some(name))
    }

    pub fn computed_with_name(
        evaluator: impl Fn() -> T + 'static,
        dependents: &Vec<&UntypedProperty>,
        name: &str,
    ) -> Self {
        Self::computed_with_config(evaluator, dependents, |_, _| true, Some(name))
    }

    fn new_optional_name(val: T, name: Option<&str>) -> Self {
        Self {
            untyped: UntypedProperty::new(
                Box::new(val),
                PropertyType::Literal,
                |_: &_, _: &_| true,
                name,
            ),
            _phantom: PhantomData {},
        }
    }

    fn computed_with_config(
        evaluator: impl Fn() -> T + 'static,
        dependents: &Vec<&UntypedProperty>,
        filter: impl Fn(&T, &T) -> bool + 'static,
        name: Option<&str>,
    ) -> Self {
        let inbound: Vec<_> = dependents.iter().map(|v| v.get_id()).collect();
        let start_val = T::default();
        let evaluator = Rc::new(move || Box::new(evaluator()) as Box<dyn Any>);
        Self {
            untyped: UntypedProperty::new(
                Box::new(start_val),
                PropertyType::Computed {
                    evaluator,
                    dirty: true,
                    inbound,
                },
                move |a: &dyn Any, b: &dyn Any| {
                    filter(a.downcast_ref().unwrap(), b.downcast_ref().unwrap())
                },
                name,
            ),
            _phantom: PhantomData {},
        }
    }

    /// Gets the currently stored value. Might be computationally
    /// expensive in a large reactivity network since this triggers
    /// re-evaluation of dirty property chains
    pub fn get(&self) -> T {
        PROPERTY_TABLE.with(|t| {
            t.get_value(self.untyped.id)
        })
    }

    /// Sets this properties value and sets the drity bit recursively of all of
    /// it's dependencies if not already set
    pub fn set(&self, val: T) {
        PROPERTY_TABLE.with(|t| t.set_value(self.untyped.id, val));
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

impl<T: PropertyValue + PartialEq> Property<T> {
    pub fn silent_if_equal(&self) -> Self {
        let cp_self = self.clone();
        Self::computed_with_config(
            move || cp_self.get(),
            &vec![&self.untyped()],
            |a, b| a == b,
            None,
        )
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
