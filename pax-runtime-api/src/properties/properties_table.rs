use std::{any::Any, cell::RefCell, rc::Rc};

use slotmap::{SlotMap, SparseSecondaryMap};

use super::{private::PropertyId, PropertyValue};

thread_local! {
    pub static PROPERTY_TABLE: PropertyTable = PropertyTable::new();
}

pub struct PropertyData {
    pub value: Box<dyn Any>,
    pub outbound: Vec<PropertyId>,
    pub prop_type: PropertyType,
}

pub struct ReferenceCount(usize);

#[derive(Clone)]
pub enum PropertyType {
    Literal,
    Expr {
        // Information needed to recompute on change
        evaluator: Rc<dyn Fn() -> Box<dyn Any>>,
        dirty: bool,
        inbound: Vec<PropertyId>,
    },
}

pub struct PropertyTable {
    pub entries: RefCell<SlotMap<PropertyId, (ReferenceCount, Option<PropertyData>)>>,
    pub debug_names: RefCell<SparseSecondaryMap<PropertyId, String>>,
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
                    .map(|(k, (_, v))| (k, v.as_ref().map(|v| v.outbound.to_owned())))
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

    pub fn add_entry(
        &self,
        start_val: Box<dyn Any>,
        data: PropertyType,
        debug_name: Option<&str>,
    ) -> PropertyId {
        let id = {
            let Ok(mut sm) = self.entries.try_borrow_mut() else {
                panic!(
                    "couldn't create new property \"{}\"- table already borrowed",
                    debug_name.unwrap_or("<no name>")
                );
            };
            sm.insert((
                ReferenceCount(1),
                Some(PropertyData {
                    value: start_val,
                    outbound: Vec::with_capacity(0),
                    prop_type: data,
                }),
            ))
        };
        if let Some(name) = debug_name {
            self.debug_names.borrow_mut().insert(id, name.to_owned());
        }
        self.connect_dependents(id);
        id
    }

    /// WARNING this function is dangerous, f can not drop, create, set, get
    /// or in any other way modify the global table or this will panic with multiple mutable borrows
    fn with_entry_mut<V>(
        &self,
        id: PropertyId,
        f: impl FnOnce(&mut (ReferenceCount, Option<PropertyData>)) -> V,
    ) -> V {
        let mut sm = self.entries.borrow_mut();
        let mut data = sm.get_mut(id).unwrap();

        let return_value = f(&mut data);
        return_value
    }

    /// WARNING You cannot use .get() or .set() on the mutably borrowed property
    /// or this will panic
    pub fn with_property_data_mut<V>(
        &self,
        id: PropertyId,
        f: impl FnOnce(&mut PropertyData) -> V,
    ) -> V {
        let mut property_data = self.with_entry_mut(id, |(_, existing_property_data)| {
            existing_property_data
                .take()
                .expect("property data has not been taken")
        });
        let res = f(&mut property_data);
        self.with_entry_mut(id, |(_, existing_property_data)| {
            *existing_property_data = Some(property_data);
        });
        res
    }

    pub fn with_property_data<V>(&self, id: PropertyId, f: impl FnOnce(&PropertyData) -> V) -> V {
        self.with_property_data_mut(id, |prop_data| f(&*prop_data))
    }

    //WARNING borrows entire table
    pub fn with_ref_count_mut<V>(&self, id: PropertyId, f: impl FnOnce(&mut usize) -> V) -> V {
        let res = self.with_entry_mut(id, |(ref_count, _)| f(&mut ref_count.0));
        res
    }

    // re-computes the value and all of it's dependencies if dirty
    pub fn update_value(&self, id: PropertyId) {
        let evaluator =
            self.with_property_data_mut(id, |prop_data| match &mut prop_data.prop_type {
                PropertyType::Expr {
                    evaluator,
                    dirty: ref mut is_dirty @ true,
                    ..
                } => {
                    *is_dirty = false;
                    Some(Rc::clone(&evaluator))
                }
                _ => None,
            });
        if let Some(evaluator) = evaluator {
            // WARNING: the evaluator should not be run inside of a
            // with_property_data closure, as this function is provided by the user
            // and can do arbitrary sets/gets/drops etc (that need the prop data)
            let new_value = evaluator();
            self.with_property_data_mut(id, |prop_data| {
                // drops old value
                prop_data.value = new_value;
            })
        }
    }

    /// Main function to get access to a reference inside of a property.
    /// Makes sure the value is up to date before calling the provided closure
    pub fn get_value<T: PropertyValue>(&self, id: PropertyId) -> T {
        self.update_value(id);
        self.with_property_data(id, |prop_data| {
            prop_data
                .value
                .downcast_ref::<T>()
                .expect("value should contain correct type")
                .clone()
        })
    }

    // WARNING: read the below before using this function
    // NOTE: This NEVER updates the value before access, do that before if needed
    // by calling self.update_value(..)
    // NOTE: This always assumes the underlying data was changed, and marks
    // it and it's dependents as dirty irrespective of actual modification
    pub fn set_value<T: PropertyValue>(&self, id: PropertyId, new_val: T) {
        self.with_property_data_mut(id, |prop_data: &mut PropertyData| {
            // drops old value
            prop_data.value = Box::new(new_val);
        });
        self.dirtify_dependencies(id);
    }

    pub fn remove_entry(&self, id: PropertyId) {
        let res = {
            self.disconnect_dependencies(id);
            self.disconnect_dependents(id);
            let Ok(mut sm) = self.entries.try_borrow_mut() else {
                panic!(
                    "failed to remove property \"{}\" - propertytable already borrowed",
                    self.debug_name(id),
                );
            };
            sm.remove(id).expect("tried to remove non-existent prop")
        };
        drop(res);
    }

    pub fn debug_name(&self, id: PropertyId) -> String {
        self.debug_names
            .borrow()
            .get(id)
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("<NO DEBUG NAME>")
            .to_owned()
    }
}
