/// Sealed PropertyId needed for slotmap (strictly internal)
mod private {
    slotmap::new_key_type!(
        pub struct PropertyId;
    );
}

mod erased_property;
use std::{any::Any, marker::PhantomData, rc::Rc};

pub use erased_property::ErasedProperty;
use serde::{Deserialize, Serialize};

mod properties_table;
use properties_table::PROPERTY_TABLE;

use self::properties_table::{PropertyData, PropertyType};

/// PropertyVal represents a restriction on valid generic types that a property can
/// contain. All T need to be Clone (to enable .get()) + 'static (no references/
/// lifetimes)
pub trait PropertyValue: Default + Clone + 'static {}
impl<T: Default + Clone + 'static> PropertyValue for T {}

#[derive(Debug, Clone)]
pub struct Property<T> {
    erased: ErasedProperty,
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
            erased: ErasedProperty::new(Box::new(val), PropertyType::Literal, name),
            _phantom: PhantomData {},
        }
    }

    pub fn computed(
        evaluator: impl Fn() -> T + 'static,
        dependents: &Vec<&ErasedProperty>,
    ) -> Self {
        Self::computed_optional_name(evaluator, dependents, None)
    }
    /// Used by engine to create dependency chains, the evaluator fires and
    /// re-computes a property each time it's dependencies change.
    pub fn computed_with_name(
        evaluator: impl Fn() -> T + 'static,
        dependents: &Vec<&ErasedProperty>,
        name: &str,
    ) -> Self {
        Self::computed_optional_name(evaluator, dependents, Some(name))
    }

    fn computed_optional_name(
        evaluator: impl Fn() -> T + 'static,
        dependents: &Vec<&ErasedProperty>,
        name: Option<&str>,
    ) -> Self {
        let dependent_property_ids: Vec<_> = dependents.iter().map(|v| v.get_id()).collect();
        let start_val = T::default();
        let evaluator = Rc::new(move || Box::new(evaluator()) as Box<dyn Any>);
        Self {
            erased: ErasedProperty::new(
                Box::new(start_val),
                PropertyType::Expr {
                    evaluator,
                    dirty: true,
                    subscriptions: dependent_property_ids,
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
        let mut to_drop_later: Option<Box<dyn Any>> = None;
        let mut value_to_drop: Box<dyn Any> = Box::new({});
        PROPERTY_TABLE.with(|t| {
            t.with_prop_data_mut(self.erased.id, |original_prop_data: &mut PropertyData| {
                t.with_prop_data(target.erased.id, |target_prop_data| {
                    // remove self from all subscriber lists (we are soon overwriting)
                    if let PropertyType::Expr { subscriptions, .. } = &original_prop_data.prop_type {
                        for id in subscriptions {
                            t.with_prop_data_mut(*id, |dep_prop| {
                                dep_prop.subscribers.retain(|v| *v != self.erased.id);
                            })
                            .map_err(|e| {
                                format!(
                                    "coudn't find subscription of \"{}\" to remove: {}",
                                    t.debug_name(self.erased.id),
                                    e,
                                )
                            })
                            .unwrap();
                        }
                    }
                    // push source (self) as a subscriber of the values target subscribes to
                    if let PropertyType::Expr { subscriptions, .. } = &target_prop_data.prop_type {
                        for id in subscriptions {
                            t.with_prop_data_mut(*id, |dep_prop| {
                                dep_prop.subscribers.push(self.erased.id);
                            })
                            .map_err(|e| {
                                format!(
                                    "coudn't find subscription of \"{}\" to add: {}",
                                    t.debug_name(target.erased.id),
                                    e,
                                )
                            })
                            .unwrap();
                        }
                    }

                    // NOTE: original_prop_data.prop_type is holding onto a closure that can capture
                    // properties that could be dropped here
                    to_drop_later = Some(Box::new(std::mem::replace(
                        &mut original_prop_data.prop_type,
                        target_prop_data.prop_type.clone(),
                    )));

                    let target_value = target_prop_data
                        .value
                        .downcast_ref::<T>()
                        .expect("value should contain correct type");
                    value_to_drop = std::mem::replace(
                        &mut original_prop_data.value,
                        Box::new(target_value.clone()),
                    );
                })
                .map_err(|e| format!("replace_with target prop err: {}", e))
                .unwrap();
            })
            .map_err(|e| format!("replace_with source prop err: {}", e))
            .unwrap();
            // make sure dependencies of self
            // know that something has changed
            t.get_value_mut(self.erased.id, |_: &mut T| {});

            let mut names = t.debug_names.borrow_mut();
            if let Some(target_name) = names.get(target.erased.id) {
                let curr_name = names
                    .get(self.erased.id)
                    .map(String::as_str)
                    .unwrap_or("un-named");
                let new_name = format!("{} <repl_with> {}", curr_name, target_name.to_owned());
                names.insert(self.erased.id, new_name);
            };
        });
    }

    /// Gets the currently stored value. Might be computationally
    /// expensive in a large reactivity network since this triggers
    /// re-evaluation of dirty property chains
    pub fn get(&self) -> T {
        PROPERTY_TABLE.with(|t| t.get_value_ref(self.erased.id, |v: &T| v.clone()))
    }

    /// Sets this properties value, and sets the drity bit of all of
    /// it's dependencies if not already set
    pub fn set(&self, val: T) {
        PROPERTY_TABLE.with(
            // WARNING: do not remove std::mem::replace
            |t| t.get_value_mut(self.erased.id, |v| std::mem::replace(v, val)),
        );
    }

    pub fn erase(&self) -> ErasedProperty {
        self.erased.clone()
    }
}
