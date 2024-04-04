/// Sealed PropertyId needed for slotmap (strictly internal)
mod private {
    slotmap::new_key_type!(
        pub struct PropertyId;
    );
}

mod erased_property;
mod graph_operations;
pub use erased_property::UntypedProperty;

use std::{any::Any, marker::PhantomData, rc::Rc};

use serde::{Deserialize, Serialize};

mod properties_table;
#[cfg(test)]
mod tests;
use properties_table::PROPERTY_TABLE;

use self::properties_table::{PropertyData, PropertyType};

/// PropertyVal represents a restriction on valid generic types that a property can
/// contain. All T need to be Clone (to enable .get()) + 'static (no references/
/// lifetimes)
pub trait PropertyValue: Default + Clone + 'static {}
impl<T: Default + Clone + 'static> PropertyValue for T {}

#[derive(Debug, Clone)]
pub struct Property<T> {
    untyped: UntypedProperty,
    _phantom: PhantomData<T>,
}

impl<T: Default + PropertyValue> Default for Property<T> {
    fn default() -> Self {
        Property::new_with_name(
            T::default(),
            &format!("from default ({})", std::any::type_name::<T>()),
        )
    }
}

/// Note that serialization and deserialization fully disconnects properties,
/// and only loads them back in as literals.
impl<'de, T: PropertyValue + Deserialize<'de>> Deserialize<'de> for Property<T> {
    fn deserialize<D>(deserializer: D) -> Result<Property<T>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = T::deserialize(deserializer)?;
        Ok(Property::new_with_name(
            value,
            &format!("from deserialized ({})", std::any::type_name::<T>()),
        ))
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

impl<T: PropertyValue> Property<T> {
    pub fn new(val: T) -> Self {
        Self::literal(val)
    }

    pub fn new_with_name(val: T, name: &str) -> Self {
        Self::literal_optional_name(val, Some(name))
    }

    pub(crate) fn literal(val: T) -> Self {
        Self::literal_optional_name(val, None)
    }

    fn literal_optional_name(val: T, name: Option<&str>) -> Self {
        Self {
            untyped: UntypedProperty::new(Box::new(val), PropertyType::Literal, name),
            _phantom: PhantomData {},
        }
    }

    pub fn computed(
        evaluator: impl Fn() -> T + 'static,
        dependents: &Vec<&UntypedProperty>,
    ) -> Self {
        Self::computed_optional_name(evaluator, dependents, None)
    }
    /// Used by engine to create dependency chains, the evaluator fires and
    /// re-computes a property each time it's dependencies change.
    pub fn computed_with_name(
        evaluator: impl Fn() -> T + 'static,
        dependents: &Vec<&UntypedProperty>,
        name: &str,
    ) -> Self {
        Self::computed_optional_name(evaluator, dependents, Some(name))
    }

    fn computed_optional_name(
        evaluator: impl Fn() -> T + 'static,
        dependents: &Vec<&UntypedProperty>,
        name: Option<&str>,
    ) -> Self {
        let dependent_property_ids: Vec<_> = dependents.iter().map(|v| v.get_id()).collect();
        let start_val = T::default();
        let evaluator = Rc::new(move || Box::new(evaluator()) as Box<dyn Any>);
        Self {
            untyped: UntypedProperty::new(
                Box::new(start_val),
                PropertyType::Expr {
                    evaluator,
                    dirty: true,
                    inbound: dependent_property_ids,
                },
                name,
            ),
            _phantom: PhantomData {},
        }
    }

    /// WARNING: this method can introduce circular dependencies if one is not careful.
    /// Using it wrongly can introduce memory leaks and inconsistent property behaviour.
    /// This method can be used to replace an inner value from for example a literal to
    /// a computed computed, while keeping the link to it's dependents
    pub fn replace_with(&self, target: Property<T>) {
        // We want the target's value to be dropped after the mutable table access (PROPERTY_TABLE.with) to avoid borrowing issues
        PROPERTY_TABLE.with(|t| {
            t.disconnect_dependents(self.untyped.id);
            t.with_property_data_mut(self.untyped.id, |original_prop_data: &mut PropertyData| {
                t.with_property_data(target.untyped.id, |target_prop_data| {
                    // drops old value
                    original_prop_data.prop_type = target_prop_data.prop_type.clone();

                    let target_value = target_prop_data
                        .value
                        .downcast_ref::<T>()
                        .expect("value should contain correct type");
                    // drops old value
                    original_prop_data.value = Box::new(target_value.clone());
                });
            });
            t.connect_dependents(self.untyped.id);
            // make sure dependencies of self
            // know that something has changed
            t.dirtify_dependencies(self.untyped.id);

            let mut names = t.debug_names.borrow_mut();
            if let Some(target_name) = names.get(target.untyped.id) {
                let curr_name = names
                    .get(self.untyped.id)
                    .map(String::as_str)
                    .unwrap_or("un-named");
                let new_name = format!("{} <repl_with> {}", curr_name, target_name.to_owned());
                names.insert(self.untyped.id, new_name);
            };
        });
    }

    /// Gets the currently stored value. Might be computationally
    /// expensive in a large reactivity network since this triggers
    /// re-evaluation of dirty property chains
    pub fn get(&self) -> T {
        PROPERTY_TABLE.with(|t| t.get_value(self.untyped.id))
    }

    /// Sets this properties value, and sets the drity bit of all of
    /// it's dependencies if not already set
    pub fn set(&self, val: T) {
        PROPERTY_TABLE.with(|t| t.set_value(self.untyped.id, val));
    }

    pub fn as_untyped(&self) -> UntypedProperty {
        self.untyped.clone()
    }
}
