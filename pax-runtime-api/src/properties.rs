use std::{any::Any, cell::RefCell, marker::PhantomData, rc::Rc};

use serde::{Deserialize, Serialize};
use slotmap::SlotMap;

use self::private::PropId;

/// Reactive property type. Shallow clones can be cheaply made.
#[derive(Debug)]
pub struct Property<T> {
    id: PropId,
    _phantom: PhantomData<T>,
}

impl<T> Clone for Property<T> {
    fn clone(&self) -> Self {
        glob_prop_table(|t| {
            t.with_prop_data_mut(self.id, |prop_data| {
                prop_data.ref_count += 1;
            });
        });
        Property {
            id: self.id,
            _phantom: PhantomData,
        }
    }
}

impl<T> Drop for Property<T> {
    fn drop(&mut self) {
        glob_prop_table(|t| {
            let ref_count = t.with_prop_data_mut(self.id, |prop_data| {
                prop_data.ref_count -= 1;
                prop_data.ref_count
            });
            if ref_count == 0 {
                t.remove_entry(self.id);
            }
        });
    }
}
impl<T: Default + PropVal> Default for Property<T> {
    fn default() -> Self {
        Property::new(T::default())
    }
}

/// PropVal represents a restriction on valid generic types that a property can
/// contain. All T need to be Clone (to enable .get()) + 'static (no references/
/// lifetimes)
pub trait PropVal: Clone + 'static {}
impl<T: Clone + 'static> PropVal for T {}

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
        Ok(Property::new(value))
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

impl<T: PropVal> Property<T> {
    pub fn new(val: T) -> Self {
        Self::literal(val)
    }

    pub(crate) fn literal(val: T) -> Self {
        let id = glob_prop_table(|t| t.add_literal_entry(val));
        Self {
            id,
            _phantom: PhantomData {},
        }
    }

    /// Used by engine to create dependency chains, the evaluator fires and
    /// re-computes a property each time it's dependencies change.
    pub fn computed(evaluator: impl Fn() -> T + 'static, dependents: &Vec<&ErasedProperty>) -> Self {
        let dependent_property_ids: Vec<_> = dependents.iter().map(|v| v.get_id()).collect();
        let id = glob_prop_table(|t| {
            t.add_expr_entry(
                Rc::new(move || Box::new(evaluator())),
                &dependent_property_ids,
            )
        });
        Self {
            id,
            _phantom: PhantomData {},
        }
    }

    /// WARNING: this method can introduce circular dependencies if one is not careful.
    /// Using it wrongly can introduce memory leaks and inconsistent property behaviour.
    /// This method can be used to replace an inner value from for example a literal to
    /// a computed computed, while keeping the link to it's dependents
    pub fn replace_with(&self, prop: Property<T>) {
        glob_prop_table(|t| {
            t.with_prop_data_mut(self.id, |prop_data| {
                t.with_prop_data(prop.id, |other| {
                    prop_data.prop_type = other.prop_type.clone();
                    prop_data.value = if let Some(v) = other.value.downcast_ref::<T>() {
                        Box::new(v.clone())
                    } else {
                        Box::new(())
                    };
                });
            });
            // make sure dependencies of self
            // know that something has changed
            t.get_value_mut(self.id, |_: &mut T| {});
        })
    }

    /// Gets the currently stored value. Might be computationally
    /// expensive in a large reactivity network since this triggers
    /// re-evaluation of dirty property chains
    pub fn get(&self) -> T {
        glob_prop_table(|t| t.get_value_ref(self.id, |v: &T| v.clone()))
    }

    /// Sets this properties value, and sets the drity bit of all of
    /// it's dependencies if not already set
    pub fn set(&self, val: T) {
        glob_prop_table(|t| t.get_value_mut(self.id, |v| *v = val));
    }

    // Get mutable access to the underlying property value
    // NOTE: this always assumes the value was modifed and triggers
    // dirtying of it's dependencies, no matter if it actually was
    // or not
    pub fn update<V>(&self, f: impl FnOnce(&mut T) -> V) -> V {
        glob_prop_table(|t| {
            // needs to first evaluate if dirty (people can access the value in
            // the closure), see update_with_stale_value for one that does not
            t.update_value(self.id);
            t.get_value_mut(self.id, f)
        })
    }

    /// WARNING: do NOT use this function to read the
    /// value of a property.
    /// Updates value without first making sure that the
    /// surfaced mutable reference is up to date.
    pub fn update_with_stale_value<V>(&self, f: impl FnOnce(&mut T) -> V) -> V {
        glob_prop_table(|t| t.get_value_mut(self.id, f))
    }

    pub fn read<V>(&self, f: impl FnOnce(&T) -> V) -> V {
        glob_prop_table(|t| t.get_value_ref(self.id, f))
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
    // creation_trace: RefCell<Option<Vec<PropId>>>,
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
            // creation_trace: RefCell::new(None),
        }
    }

    fn add_literal_entry<T: PropVal>(&self, val: T) -> PropId {
        let mut sm = self.entries.borrow_mut();
        let prop_id = sm.insert(RefCell::new(PropertyData {
            ref_count: 1,
            value: Box::new(val),
            subscribers: Vec::with_capacity(0),
            prop_type: PropType::Literal,
        }));
        prop_id
    }

    fn add_expr_entry(
        &self,
        evaluator: Rc<dyn Fn() -> Box<dyn Any>>,
        dependents: &[PropId],
    ) -> PropId {
        let expr_id = {
            let mut sm = self.entries.borrow_mut();
            sm.insert(RefCell::new(PropertyData {
                ref_count: 1,
                value: Box::new(()),
                subscribers: Vec::with_capacity(0),
                prop_type: PropType::Expr {
                    dirty: true,
                    evaluator,
                    subscriptions: dependents.to_vec(),
                },
            }))
        };
        for id in dependents {
            self.with_prop_data_mut(*id, |dep_prop| {
                dep_prop.subscribers.push(expr_id);
            });
        }
        expr_id
    }

    fn with_prop_data_mut<V>(&self, id: PropId, f: impl FnOnce(&mut PropertyData) -> V) -> V {
        let sm = self.entries.borrow();
        let mut prop_data = sm
            .get(id)
            .expect(
                "tried to access property entry that doesn't exist anymore,\
 is it's PropertyScope already cleaned up?",
            )
            .try_borrow_mut()
            .expect("tried to access same property internals recursively");
        let res = f(&mut *prop_data);
        res
    }

    fn with_prop_data<V>(&self, id: PropId, f: impl FnOnce(&PropertyData) -> V) -> V {
        let sm = self.entries.borrow();
        let prop_data = sm
            .get(id)
            .expect(
                "tried to access property entry that doesn't exist anymore,\
 is it's PropertyScope already cleaned up?",
            )
            .try_borrow()
            .expect("tried to access same property interals recursively while mutably borrowed");
        let res = f(&*prop_data);
        res
    }

    // re-computes the value and all of it's dependencies if dirty
    fn update_value(&self, id: PropId) {
        let evaluator = self.with_prop_data_mut(id, |prop_data| {
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
        });
        if let Some(evaluator) = evaluator {
            let new_value = evaluator();
            self.with_prop_data_mut(id, |prop_data| {
                prop_data.value = new_value;
            });
        }
    }

    /// Main function to get access to a reference inside of a property.
    /// Makes sure the value is up to date before calling the provided closure
    fn get_value_ref<T: PropVal, V>(&self, id: PropId, f: impl FnOnce(&T) -> V) -> V {
        self.update_value(id);
        self.with_prop_data(id, |prop_data| {
            let value = prop_data.value.downcast_ref::<T>().expect("correct type");
            f(value)
        })
    }

    // WARNING: read the below before using this function
    // NOTE: This NEVER updates the value before access, do that before if needed
    // by calling self.update_value(..)
    // NOTE: This always assumes the underlying data was changed, and marks
    // it and it's dependents as dirty irrespective of actual modification
    fn get_value_mut<T: PropVal, V>(&self, id: PropId, f: impl FnOnce(&mut T) -> V) -> V {
        let mut to_dirty = vec![];
        let ret_value = self.with_prop_data_mut(id, |prop_data| {
            let value = prop_data.value.downcast_mut().expect("correct type");
            let ret = f(value);
            to_dirty.extend_from_slice(&prop_data.subscribers);
            ret
        });
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
            });
        }
        ret_value
    }

    fn remove_entry(&self, id: PropId) {
        let prop_data = {
            let mut sm = self.entries.borrow_mut();
            let prop_data = sm.remove(id).expect("tried to remove non-existent prop");
            prop_data
        }
        .into_inner();
        for sub in prop_data.subscribers {
            self.with_prop_data_mut(sub, |s| {
                if let PropType::Expr { subscriptions, .. } = &mut s.prop_type {
                    subscriptions.retain(|s| s != &id);
                }
            });
        }
        if let PropType::Expr { subscriptions, .. } = prop_data.prop_type {
            for subscription in subscriptions {
                self.with_prop_data_mut(subscription, |sub| {
                    sub.subscribers.retain(|v| v != &id);
                });
            }
        }
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
pub struct ErasedProperty {
    id: PropId,
}

impl ErasedProperty {
    pub fn get<T: PropVal>(&self) -> Property<T> {
        Property {
            id: self.id,
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
        ErasedProperty { id: self.id }
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
        let prop = Property::<i32>::computed(|| 42, &[]);
        assert_eq!(prop.get(), 42);
    }

    #[test]
    fn test_computed_dependent_on_literal() {
        let prop_1 = Property::literal(2);
        let p1 = prop_1.clone();
        let prop_2 = Property::<i32>::computed(move || p1.get() * 5, &[&prop_1]);

        assert_eq!(prop_2.get(), 10);
        prop_1.set(3);
        assert_eq!(prop_2.get(), 15);
    }

    #[test]
    fn test_read() {
        let prop_1 = Property::literal(2);
        let p1 = prop_1.clone();
        let prop_2 = Property::<i32>::computed(move || p1.get() * 5, &[&prop_1]);

        assert_eq!(prop_2.get(), 10);
        prop_1.set(3);
        prop_2.read(|p2| {
            // make sure prop_2 recomputes if needed before
            // exposing access to user
            assert_eq!(*p2, 15);
        });
    }

    #[test]
    fn test_update() {
        let prop_1 = Property::literal(2);
        let p1 = prop_1.clone();
        let prop_2 = Property::<i32>::computed(move || p1.get() * 5, &[&prop_1]);

        assert_eq!(prop_2.get(), 10);
        prop_1.set(3);
        prop_2.update(|p2| {
            // make sure prop_2 recomputes if needed before
            // exposing access to user
            assert_eq!(*p2, 15);
        });
        prop_1.update(|p1| {
            *p1 = 4;
        });
        // make sure the above update
        // marked prop_2 as dirty
        assert_eq!(prop_2.get(), 20);
    }

    #[test]
    fn test_property_replacement() {
        let prop_1 = Property::literal(2);
        let p1 = prop_1.clone();
        let prop_2 = Property::computed(move || p1.get(), &[&prop_1]);

        let prop_3 = Property::literal(6);
        let p3 = prop_3.clone();
        let prop_4 = Property::computed(move || p3.get(), &[&prop_3]);

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
        let prop_3 = Property::computed(move || p1.get() * p2.get(), &[&prop_1, &prop_2]);
        let p1 = prop_1.clone();
        let p3 = prop_3.clone();
        let prop_4 = Property::computed(move || p1.get() + p3.get(), &[&prop_1, &prop_3]);

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
