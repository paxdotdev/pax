use std::{any::Any, cell::RefCell, rc::Rc};

use slotmap::{SlotMap, SparseSecondaryMap};

use super::{private::PropertyId, PropertyValue};

thread_local! {
    /// Global property table used to store data backing dirty-dag
    pub static PROPERTY_TABLE: PropertyTable = PropertyTable::default();
}

/// The main collection of data associated with a specific property id
pub struct PropertyData {
    // The cached value of this property
    pub value: Box<dyn Any>,
    // List of properties that depend on this value
    pub outbound: Vec<PropertyId>,
    // Specialization data (computed/literal etc)
    pub property_type: PropertyType,
}

/// Specialization data only needed for different kinds of properties
#[derive(Clone)]
pub(crate) enum PropertyType {
    Literal,
    Computed {
        // Information needed to recompute on change
        evaluator: Rc<dyn Fn() -> Box<dyn Any>>,
        // Dirty bit set if a depency further up in the dirty-dag tree
        // has been changed
        dirty: bool,
        // The inbound connections, ie dependencies, of this computed value
        inbound: Vec<PropertyId>,
    },
}

/// Main propertytable, containing data associated with each property
#[derive(Default)]
pub(crate) struct PropertyTable {
    // Main property table containing property data
    pub(crate) property_map: RefCell<SlotMap<PropertyId, (ReferenceCount, Option<PropertyData>)>>,
    debug_names: RefCell<SparseSecondaryMap<PropertyId, String>>,
}

pub struct ReferenceCount(usize);

impl PropertyTable {
    /// Main function to get access to a value inside of a property.
    /// Makes sure the value is up to date before returning in the case
    /// of computed properties.
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

    // Main function to set a value of a property.
    // NOTE: This always assumes the underlying data was changed, and marks
    // it and it's dependents as dirty irrespective of actual modification
    pub fn set_value<T: PropertyValue>(&self, id: PropertyId, new_val: T) {
        self.with_property_data_mut(id, |prop_data: &mut PropertyData| {
            let new_val = Box::new(new_val);
            // drops old value
            prop_data.value = new_val;
        });
        self.dirtify_outbound(id);
    }

    /// Adds a new untyped property entry
    pub fn add_entry(
        &self,
        start_val: Box<dyn Any>,
        data: PropertyType,
        debug_name: Option<&str>,
    ) -> PropertyId {
        let id = {
            let Ok(mut sm) = self.property_map.try_borrow_mut() else {
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
                    property_type: data,
                }),
            ))
        };
        if let Some(name) = debug_name {
            self.debug_names.borrow_mut().insert(id, name.to_owned());
        }
        self.connect_inbound(id);
        id
    }

    /// Gives mutable access to a entry in the property table
    /// WARNING: this function is dangerous, f can not drop, create, set, get
    /// or in any other way modify the global property table or this will panic
    /// with multiple mutable borrows. Letting f contain any form of userland
    /// code is NOT a good idea.
    fn with_entry_mut<V>(
        &self,
        id: PropertyId,
        f: impl FnOnce(&mut (ReferenceCount, Option<PropertyData>)) -> V,
    ) -> V {
        let mut sm = self.property_map.borrow_mut();
        let mut data = sm.get_mut(id).unwrap();
        let return_value = f(&mut data);
        return_value
    }

    /// Allows mutable access to the data associated with a property id.
    /// WARNING while this method is being run, the entry corresponding to id
    /// is not present in the table, and methods such as .get(), .set(), and
    /// possibly others on the property with the id parameter bellow will panic.
    pub fn with_property_data_mut<V>(
        &self,
        id: PropertyId,
        f: impl FnOnce(&mut PropertyData) -> V,
    ) -> V {
        // take the value out of the table
        let mut property_data = self.with_entry_mut(id, |(_, existing_property_data)| {
            existing_property_data
                .take()
                .expect("property data should not have already been taken")
        });

        // run f, without table being borrowed
        // NOTE: need to make sure with_property_data_mut is not run recursively
        // with the same id
        let res = f(&mut property_data);

        // return value to the table that was taken
        self.with_entry_mut(id, |(_, existing_property_data)| {
            *existing_property_data = Some(property_data);
        });

        res
    }

    /// Allows access to the data associated with a property id.
    /// WARNING while this method is being run, the entry corresponding to id
    /// is not present in the table, and methods such as .get(), .set(), and
    /// possibly others on the property with the id parameter bellow will panic.
    pub fn with_property_data<V>(&self, id: PropertyId, f: impl FnOnce(&PropertyData) -> V) -> V {
        self.with_property_data_mut(id, |prop_data| f(&*prop_data))
    }

    /// Increase the ref count of a property
    pub fn increase_ref_count(&self, id: PropertyId) -> usize {
        self.with_entry_mut(id, |(ReferenceCount(ref_count), _)| {
            *ref_count += 1;
            *ref_count
        })
    }

    /// Decrease the ref count of a property
    pub fn decrease_ref_count(&self, id: PropertyId) -> usize {
        self.with_entry_mut(id, |(ReferenceCount(ref_count), _)| {
            *ref_count -= 1;
            *ref_count
        })
    }

    /// Replaces the way the source parameters property is being
    /// computed / it's value to the way target does.
    /// NOTE: source_id and target_id need to both contain
    /// the type T, or else this panics
    pub fn replace_property_keep_outbound_connections<T: Clone + 'static>(
        &self,
        source_id: PropertyId,
        target_id: PropertyId,
    ) {
        // disconnect self from it's dependents, in preparation of overwriting
        // with targets inbound. (only does something for computed values)
        self.disconnect_inbound(source_id);

        // copy nessesary internal state from target to source
        self.with_property_data_mut(source_id, |source_property_data| {
            self.with_property_data(target_id, |target_property_data| {
                // make sure source is correct type
                source_property_data
                    .value
                    .downcast_ref::<T>()
                    .expect("source type should be T");

                source_property_data.value = Box::new(
                    target_property_data
                        .value
                        .downcast_ref::<T>()
                        .expect("target type should be T ")
                        .clone(),
                );
                source_property_data.property_type = target_property_data.property_type.clone();
            });
        });

        // connect self to it's new dependents (found in property_types Expr
        // type as inbound) (only does something for computed values)
        self.connect_inbound(source_id);

        // make sure dependencies of self
        // know that something has changed
        self.dirtify_outbound(source_id);

        // overwrite with more descriptive name
        let target_name = self.debug_name(target_id);
        let mut names = self.debug_names.borrow_mut();
        names.insert(source_id, format!("{}", target_name));
    }

    // re-computes the value if dirty
    pub fn update_value(&self, id: PropertyId) {
        let evaluator =
            self.with_property_data_mut(id, |prop_data| match &mut prop_data.property_type {
                PropertyType::Computed {
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
            // WARNING: the evaluator should not be run while the table is in
            // an invalid state (borrowed, in with_property_data closure, etc.)
            // as this function is provided by a user of the property system and
            // can do arbitrary sets/ gets/drops etc (that need the prop data)
            let new_value = evaluator();
            self.with_property_data_mut(id, |prop_data| {
                // NOTE: drops old value, potentially containing
                // properties.
                prop_data.value = new_value;
            })
        }
    }

    /// drop a properties underlying data, making any subsequent calls invalid by panic
    pub fn remove_entry(&self, id: PropertyId) {
        let res = {
            self.disconnect_outbound(id);
            self.disconnect_inbound(id);
            let Ok(mut sm) = self.property_map.try_borrow_mut() else {
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

    pub(crate) fn total_properties_count(&self) -> usize {
        self.property_map.borrow().len()
    }
}
