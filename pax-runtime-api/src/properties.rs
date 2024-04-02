use std::{any::Any, cell::RefCell, marker::PhantomData, rc::Rc};

use serde::{Deserialize, Serialize};
use slotmap::{SlotMap, SparseSecondaryMap};

use self::private::PropId;

/// Reactive property type. Shallow clones can be cheaply made.
#[derive(Debug)]
pub struct ErasedProperty {
    id: PropId,
}

impl Clone for ErasedProperty {
    fn clone(&self) -> Self {
        glob_prop_table(|t| {
            t.with_prop_data_mut(self.id, |prop_data| {
                prop_data.ref_count += 1;
            })
            .map_err(|e| format!("couldn't increase ref count: {}", e))
            .unwrap();
        });
        ErasedProperty { id: self.id }
    }
}

impl Drop for ErasedProperty {
    fn drop(&mut self) {
        glob_prop_table(|t| {
            let ref_count = t
                .with_prop_data_mut(self.id, |prop_data| {
                    prop_data.ref_count -= 1;
                    prop_data.ref_count
                })
                .map_err(|e| format!("coun't decrease ref count: {}", e))
                .unwrap();
            if ref_count == 0 {
                t.remove_entry(self.id);
            }
        });
    }
}

impl<T: Default + PropVal> Default for Property<T> {
    fn default() -> Self {
        Property::new_with_name(
            T::default(),
            &format!("from default ({})", std::any::type_name::<T>()),
        )
    }
}

/// PropVal represents a restriction on valid generic types that a property can
/// contain. All T need to be Clone (to enable .get()) + 'static (no references/
/// lifetimes)
pub trait PropVal: Default + Clone + 'static {}
impl<T: Default + Clone + 'static> PropVal for T {}

mod private {
    slotmap::new_key_type!(
        /// Sealed PropId (should be internal impl detail)
        pub struct PropId;
    );
}

/// Note that serialization and deserialization fully disconnects properties,
/// and only loads them back in as literals.
impl<'de, T: PropVal + Deserialize<'de>> Deserialize<'de> for Property<T> {
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
impl<T: PropVal + Serialize> Serialize for Property<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // TODO check if literal or computed, error on computed?
        self.get().serialize(serializer)
    }
}

impl ErasedProperty {
    fn new(val: Box<dyn Any>, data: PropType, debug_name: Option<&str>) -> Self {
        ErasedProperty {
            id: glob_prop_table(|t| t.add_entry(val, data, debug_name)),
        }
    }
}

impl<T: PropVal> Property<T> {
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
            erased: ErasedProperty::new(Box::new(val), PropType::Literal, name),
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
                PropType::Expr {
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
        // We want the target's value to be dropped after the mutable table access (glob_prop_table) to avoid borrowing issues
        let mut to_drop_later: Option<Box<dyn Any>> = None;
        let mut value_to_drop: Box<dyn Any> = Box::new({});
        glob_prop_table(|t| {
            t.with_prop_data_mut(self.erased.id, |original_prop_data: &mut PropertyData| {
                t.with_prop_data(target.erased.id, |target_prop_data| {
                    // remove self from all subscriber lists (we are soon overwriting)
                    if let PropType::Expr { subscriptions, .. } = &original_prop_data.prop_type {
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
                    if let PropType::Expr { subscriptions, .. } = &target_prop_data.prop_type {
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
        glob_prop_table(|t| t.get_value_ref(self.erased.id, |v: &T| v.clone()))
    }

    /// Sets this properties value, and sets the drity bit of all of
    /// it's dependencies if not already set
    pub fn set(&self, val: T) {
        glob_prop_table(
            // WARNING: do not remove std::mem::replace
            |t| t.get_value_mut(self.erased.id, |v| std::mem::replace(v, val)),
        );
    }
}

fn glob_prop_table<V>(f: impl FnOnce(&PropertyTable) -> V) -> V {
    thread_local! {
        static PROPERTY_TABLE: PropertyTable = PropertyTable::new();
    };
    PROPERTY_TABLE.with(|table| f(table))
}

pub struct PropertyTable {
    entries: RefCell<SlotMap<PropId, RefCell<PropertyData>>>,
    debug_names: RefCell<SparseSecondaryMap<PropId, String>>,
}

impl std::fmt::Debug for PropertyTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PropertyTable")
            .field(
                "prop_ids",
                &self
                    .entries
                    .borrow()
                    .iter()
                    .map(|(k, v)| (k, v.borrow().subscribers.to_owned()))
                    .collect::<Vec<_>>(),
            )
            .finish_non_exhaustive()
    }
}

impl PropertyTable {
    pub fn new() -> Self {
        Self {
            entries: Default::default(),
            debug_names: Default::default(),
        }
    }

    fn add_entry(
        &self,
        start_val: Box<dyn Any>,
        data: PropType,
        debug_name: Option<&str>,
    ) -> PropId {
        let id = {
            let Ok(mut sm) = self.entries.try_borrow_mut() else {
                panic!(
                    "couldn't create new property \"{}\"- table already borrowed",
                    debug_name.unwrap_or("<no name>")
                );
            };
            sm.insert(RefCell::new(PropertyData {
                ref_count: 1,
                value: start_val,
                subscribers: Vec::with_capacity(0),
                prop_type: data,
            }))
        };
        if let Some(name) = debug_name {
            self.debug_names.borrow_mut().insert(id, name.to_owned());
        }
        self.with_prop_data(id, |prop_data| {
            if let PropType::Expr { subscriptions, .. } = &prop_data.prop_type {
                for dep_id in subscriptions {
                    self.with_prop_data_mut(*dep_id, |dep_prop| {
                        dep_prop.subscribers.push(id);
                    })
                    .map_err(|e| {
                        format!(
                            "couldn't push depdendent of \"{}\": {}",
                            self.debug_name(id),
                            e
                        )
                    })
                    .unwrap();
                }
            }
        })
        .expect("recently added entry can be mutated");
        id
    }

    fn with_prop_data_mut<V>(
        &self,
        id: PropId,
        f: impl FnOnce(&mut PropertyData) -> V,
    ) -> Result<V, String> {
        let sm = self.entries.try_borrow().map_err(|_| {
            format!(
                "failed to borrow table for use by property \"{}\" - already mutably borrowed",
                self.debug_name(id),
            )
        })?;
        let prop_data = sm.get(id).ok_or(&format!(
            "tried to get prop_data for property \"{}\" that doesn't exist anymore",
            self.debug_name(id)
        ))?;
        let mut prop_data = prop_data.try_borrow_mut().map_err(|_| {
            format!(
                "failed to borrow prop_data mutably of property \"{}\" mutably - already borrowed",
                self.debug_name(id)
            )
        })?;
        Ok(f(&mut *prop_data))
    }

    fn with_prop_data<V>(
        &self,
        id: PropId,
        f: impl FnOnce(&PropertyData) -> V,
    ) -> Result<V, String> {
        let sm = self.entries.try_borrow().map_err(|_| {
            format!(
                "failed to borrow table for use by property \"{}\" - already mutably borrowed",
                self.debug_name(id),
            )
        })?;
        let prop_data = sm.get(id).ok_or(&format!(
            "tried to get prop_data for property \"{}\" that doesn't exist anymore",
            self.debug_name(id)
        ))?;
        let prop_data = prop_data.try_borrow().map_err(|_| {
            format!(
                "failed to borrow prop_data of property \"{}\" - already borrowed mutably",
                self.debug_name(id),
            )
        })?;
        Ok(f(&*prop_data))
    }

    // re-computes the value and all of it's dependencies if dirty
    fn update_value(&self, id: PropId) {
        let evaluator = self
            .with_prop_data_mut(id, |prop_data| {
                if let PropType::Expr {
                    dirty, evaluator, ..
                } = &mut prop_data.prop_type
                {
                    // This dirty checking should be done automatically by sub-components (dependents)
                    // of the computed during the "get" calls while computing it.
                    if *dirty == true {
                        *dirty = false;
                        return Some(evaluator.clone());
                    }
                }
                None
            })
            .map_err(|e| format!("can't update value: {}", e))
            .unwrap();
        if let Some(evaluator) = evaluator {
            let new_value = evaluator();
            self.with_prop_data_mut(id, |prop_data| {
                // WARNING: don't remove this. old value needs to be returned outside of this prop_data mut before being dropped
                std::mem::replace(&mut prop_data.value, new_value)
            })
            .map_err(|e| format!("update_value error: {}", e))
            .unwrap();
        }
    }

    /// Main function to get access to a reference inside of a property.
    /// Makes sure the value is up to date before calling the provided closure
    fn get_value_ref<T: PropVal, V>(&self, id: PropId, f: impl FnOnce(&T) -> V) -> V {
        self.update_value(id);
        self.with_prop_data(id, |prop_data| {
            let value = prop_data
                .value
                .downcast_ref::<T>()
                .expect("value should contain correct type");
            f(value)
        })
        .map_err(|e| format!("get_value_ref error: {}", e))
        .unwrap()
    }

    // WARNING: read the below before using this function
    // NOTE: This NEVER updates the value before access, do that before if needed
    // by calling self.update_value(..)
    // NOTE: This always assumes the underlying data was changed, and marks
    // it and it's dependents as dirty irrespective of actual modification
    fn get_value_mut<T: PropVal, V>(&self, id: PropId, f: impl FnOnce(&mut T) -> V) -> V {
        let mut to_dirty = vec![];
        let ret_value = self
            .with_prop_data_mut(id, |prop_data: &mut PropertyData| {
                let value = prop_data
                    .value
                    .downcast_mut()
                    .expect("value should contain correct type");
                let ret = f(value);
                to_dirty.extend_from_slice(&prop_data.subscribers);
                ret
            })
            .map_err(|e| format!("coudn't modify value of \"{}\": {}", self.debug_name(id), e))
            .unwrap();
        while let Some(dep_id) = to_dirty.pop() {
            self.with_prop_data_mut(dep_id, |dep_data| {
                if dep_id == id {
                    unreachable!(
                        "property cycles should never happen with literals/computeds being a DAG"
                    );
                }
                let PropType::Expr { ref mut dirty, .. } = dep_data.prop_type else {
                    unreachable!("non-computeds shouldn't depend on other properties")
                };
                if !*dirty {
                    *dirty = true;
                    to_dirty.extend_from_slice(&dep_data.subscribers);
                }
            })
            .map_err(|e| {
                format!(
                    "couldn't update dependency with name \"{}\": {}",
                    self.debug_name(dep_id),
                    e
                )
            })
            .unwrap();
        }
        ret_value
    }

    fn remove_entry(&self, id: PropId) {
        let prop_data = {
            let Ok(mut sm) = self.entries.try_borrow_mut() else {
                panic!(
                    "failed to remove property \"{}\" - propertytable already borrowed",
                    self.debug_name(id),
                );
            };
            let prop_data = sm.remove(id).expect("tried to remove non-existent prop");
            prop_data
        }
        .into_inner();
        for sub in prop_data.subscribers {
            self.with_prop_data_mut(sub, |s| {
                if let PropType::Expr { subscriptions, .. } = &mut s.prop_type {
                    subscriptions.retain(|s| s != &id);
                }
            })
            .map_err(|e| {
                format!(
                    "coudln't remove subscription to \"{}\": {}",
                    self.debug_name(id),
                    e
                )
            })
            .unwrap();
        }
        if let PropType::Expr { subscriptions, .. } = prop_data.prop_type {
            for subscription in subscriptions {
                self.with_prop_data_mut(subscription, |sub| {
                    sub.subscribers.retain(|v| v != &id);
                })
                .map_err(|e| {
                    format!(
                        "couldn't remove subscription from \"{}\": {}",
                        self.debug_name(id),
                        e
                    )
                })
                .unwrap();
            }
        }
    }

    fn debug_name(&self, id: PropId) -> String {
        self.debug_names
            .borrow()
            .get(id)
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("<NO DEBUG NAME>")
            .to_owned()
    }
}

struct PropertyData {
    value: Box<dyn Any>,
    subscribers: Vec<PropId>,
    ref_count: usize,
    prop_type: PropType,
}

#[derive(Clone)]
enum PropType {
    Literal,
    Expr {
        //
        evaluator: Rc<dyn Fn() -> Box<dyn Any>>,
        dirty: bool,
        subscriptions: Vec<PropId>,
    },
}

#[derive(Debug, Clone)]
pub struct Property<T> {
    erased: ErasedProperty,
    _phantom: PhantomData<T>,
}

impl ErasedProperty {
    pub fn get<T: PropVal>(&self) -> Property<T> {
        Property {
            erased: self.clone(),
            _phantom: PhantomData,
        }
    }

    pub fn get_id(&self) -> PropId {
        self.id
    }
}

pub trait Erasable {
    fn erase(&self) -> ErasedProperty;
}

impl<T> Erasable for Property<T> {
    fn erase(&self) -> ErasedProperty {
        self.erased.clone()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_literal_set_get() {
        let prop = Property::literal(5);
        assert_eq!(prop.get(), 5);
        prop.set(2);
        assert_eq!(prop.get(), 2);
    }

    #[test]
    fn test_computed_get() {
        let prop = Property::<i32>::computed(|| 42, &vec![]);
        assert_eq!(prop.get(), 42);
    }

    #[test]
    fn test_computed_dependent_on_literal() {
        let prop_1 = Property::literal(2);
        let p1 = prop_1.clone();
        let prop_2 = Property::<i32>::computed(move || p1.get() * 5, &vec![&prop_1.erase()]);

        assert_eq!(prop_2.get(), 10);
        prop_1.set(3);
        assert_eq!(prop_2.get(), 15);
    }

    #[test]
    fn test_property_replacement() {
        let prop_1 = Property::literal(2);
        let p1 = prop_1.clone();
        let prop_2 = Property::computed(move || p1.get(), &vec![&prop_1.erase()]);

        let prop_3 = Property::literal(6);
        let p3 = prop_3.clone();
        let prop_4 = Property::computed(move || p3.get(), &vec![&prop_3.erase()]);

        assert_eq!(prop_2.get(), 2);
        assert_eq!(prop_4.get(), 6);
        prop_3.replace_with(prop_2);
        assert_eq!(prop_4.get(), 2);
    }

    #[test]
    fn test_larger_network() {
        let prop_1 = Property::literal(2);
        let prop_2 = Property::literal(6);

        let p1 = prop_1.clone();
        let p2 = prop_2.clone();
        let prop_3 = Property::computed(
            move || p1.get() * p2.get(),
            &vec![&prop_1.erase(), &prop_2.erase()],
        );
        let p1 = prop_1.clone();
        let p3 = prop_3.clone();
        let prop_4 = Property::computed(
            move || p1.get() + p3.get(),
            &vec![&prop_1.erase(), &prop_3.erase()],
        );

        assert_eq!(prop_4.get(), 14);
        prop_1.set(1);
        assert_eq!(prop_4.get(), 7);
        prop_2.set(2);
        assert_eq!(prop_4.get(), 3);
    }

    #[test]
    fn test_cleanup() {
        assert!(glob_prop_table(|t| t.entries.borrow().is_empty()));
        let prop = Property::literal(5);
        assert_eq!(glob_prop_table(|t| t.entries.borrow().len()), 1);
        drop(prop);
        assert!(glob_prop_table(|t| t.entries.borrow().is_empty()));
    }

    #[test]
    fn test_recursive_props() {
        {
            let prop_of_prop = Property::literal(Property::literal(3));
            let prop_of_prop_clone = prop_of_prop.clone();
            prop_of_prop_clone.get().set(1);
            assert_eq!(prop_of_prop.get().get(), prop_of_prop_clone.get().get());
        }
        assert!(glob_prop_table(|t| t.entries.borrow().is_empty()));
    }
}
