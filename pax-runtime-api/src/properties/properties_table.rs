use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc};

use crate::{Property, TransitionManager, TransitionQueueEntry};

use super::PropertyValue;

use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

thread_local! {
    /// Global property table used to store data backing dirty-dag
    pub(crate) static PROPERTY_TABLE: PropertyTable = PropertyTable::default();
    /// Property time variable, to be used by
    pub(crate) static PROPERTY_TIME: RefCell<Property<u64>> = RefCell::new(Property::new(0));
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct PropertyId {
    id: u64,
}

impl PropertyId {
    pub fn new() -> Self {
        Self {
            id: COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }
}

/// The main collection of data associated with a specific property id
pub struct PropertyData {
    // Data associated with the property
    pub value: Box<dyn Any>,
    // Transition manager for easing transitions
    pub transition_manager: Option<Box<dyn Any>>,
    // Information about the type of property (computed/literal etc)
    pub property_type: PropertyType,
    // List of properties that this property depends on
    pub inbound: Vec<PropertyId>,
    // List of properties that depend on this value
    pub outbound: Vec<PropertyId>,
    // Version number to track updates for local property value caching
    pub version: u64,
}

impl PropertyData {
    pub fn get_value<T: PropertyValue>(&self) -> T {
        self.value.downcast_ref::<T>().unwrap().clone()
    }

    pub fn set_value<T: PropertyValue>(&mut self, new_val: T) {
        self.value = Box::new(new_val);
    }

    pub fn get_transition_manager<T: PropertyValue>(&mut self) -> Option<&mut TransitionManager<T>> {
        self.transition_manager.as_mut().map(|tm| tm.downcast_mut::<TransitionManager<T>>().unwrap())
    }

    pub fn clone_value<T: PropertyValue>(&self) -> Box<dyn Any> {
        Box::new(self.get_value::<T>().clone())
    }
}


/// Specialization data only needed for different kinds of properties
#[derive(Clone)]
pub(crate) enum PropertyType {
    Literal,
    Computed {
        // Information needed to recompute on change
        evaluator: Rc<dyn Fn() -> Box<dyn Any>>,
    },
}

/// Main propertytable, containing data associated with each property
#[derive(Default)]
pub(crate) struct PropertyTable {
    // Main property table containing property data
    // Box<dyn Any> is of type Box<Entry<T>> where T is the proptype
    pub(crate) property_map: RefCell<HashMap<PropertyId, Entry>>,
    // List of properties that have been set in the last tick
    pub(crate) enqueued_sets: RefCell<Vec<PropertyId>>,
}

pub struct Entry {
    pub ref_count: usize,
    pub data: Option<PropertyData>,
}

impl PropertyTable {

    // Returns version number of a property
    // This is used to invalidate the cached value of a property
    pub fn get_version(&self, id: PropertyId) -> u64 {
        self.with_property_data(id, |property_data| {
            property_data.version
        })
    }

    // Retrieves an owned value of the property with the given id
    pub fn get_value<T: PropertyValue>(&self, id: PropertyId) -> T {
        self.with_property_data(id, |property_data| {
            property_data.get_value::<T>()
        })
    }


    // Main function to set a value of a property.
    // NOTE: This always assumes the underlying data was changed, and marks
    // it and it's dependents as dirty irrespective of actual modification
    pub fn set_value<T: PropertyValue>(&self, id: PropertyId, new_val: T) {
        self.with_property_data_mut(id, |property_data: &mut PropertyData| {
            property_data.version += 1;
            property_data.set_value(new_val);
        });
        self.enqueued_sets.borrow_mut().push(id);
    }

    /// Adds a new untyped property entry
    pub fn add_entry<T: PropertyValue>(
        &self,
        start_val: T,
        inbound: Vec<PropertyId>,
        data: PropertyType,
    ) -> PropertyId {
        let id = {
            let Ok(mut sm) = self.property_map.try_borrow_mut() else {
                panic!("table already borrowed");
            };
            let entry = Entry {
                ref_count: 1,
                data: Some(PropertyData {
                    inbound,
                    value: Box::new(start_val),
                    property_type: data,
                    transition_manager: None,
                    outbound: Vec::with_capacity(0),
                    version: 0,
                }),
            };
            let id = PropertyId::new();
            sm.insert(id, entry);
            id
        };
        self.connect_inbound(id);
        self.enqueued_sets.borrow_mut().push(id);
        id
    }

    // Add a transition to the transitionmanager, making the value slowly change over time
    // Currently this only transitions the literal value of the property (and updates dependends accordingly)
    // This has no special interactions with computed properties
    pub fn transition<T: PropertyValue>(
        &self,
        id: PropertyId,
        transition: TransitionQueueEntry<T>,
        overwrite: bool,
    ) {
        let mut should_connect_to_time = false;
        let (time_id, curr_time) = PROPERTY_TIME.with_borrow(|time| (time.untyped.id, time.get().clone()));
        self.with_property_data_mut(id, |property_data: &mut PropertyData| {
            let value = property_data.get_value::<T>();
            let mut manager = property_data.get_transition_manager::<T>();
            let mut new_manager = TransitionManager::new(value, curr_time);
            let transition_manager = manager
                .get_or_insert_with(|| &mut new_manager);
            if overwrite {
                transition_manager.reset_transitions(curr_time);
            }
            transition_manager.push_transition(transition);
            if !property_data.inbound.contains(&time_id) {
                should_connect_to_time = true;
                property_data.inbound.push(time_id);
            }
        });
        if should_connect_to_time {
            self.connect_inbound(id);
        }
    }

    /// Gives mutable access to a entry in the property table
    /// WARNING: this function is dangerous, f can not drop, create, set, get
    /// or in any other way modify the global property table or this will panic
    /// with multiple mutable borrows. Letting f contain any form of userland
    /// code is NOT a good idea.
    fn with_entry_mut<V>(&self, id: PropertyId, f: impl FnOnce(&mut Entry) -> V) -> V {
        let mut sm = self.property_map.borrow_mut();
        let data = sm.get_mut(&id).unwrap();
        let return_value = f(data);
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
        log::warn!("with_property_data_mut {:?}", id);
        let mut property_data = self.with_entry_mut(id, |entry| {
            entry
                .data
                .take()
                .expect("property data should not have already been taken")
        });

        // run f, without table being borrowed
        // NOTE: need to make sure with_property_data_mut is not run recursively
        // with the same id
        let res = f(&mut property_data);

        // return value to the table that was taken
        self.with_entry_mut(id, |entry| {
            entry.data = Some(property_data);
        });
        log::warn!("with_property_data_mut done {:?}", id);
        res
    }

    /// Allows access to the data associated with a property id.
    /// WARNING while this method is being run, the entry corresponding to id
    /// is not present in the table, and methods such as .get(), .set(), and
    /// possibly others on the property with the id parameter bellow will panic.
    pub fn with_property_data<V>(&self, id: PropertyId, f: impl FnOnce(&PropertyData) -> V) -> V {
        self.with_property_data_mut(id, |property_data| f(&*property_data))
    }



    /// Allows access to the data associated with a property id.
    /// WARNING while this method is being run, the entry corresponding to id
    /// is not present in the table, and methods such as .get(), .set(), and
    /// possibly others on the property with the id parameter bellow will panic.
    // pub fn with_property_data<V>(&self, id: PropertyId, f: impl FnOnce(&PropertyData) -> V) -> V {
    //     self.with_property_data_mut(id, |property_data| f(&*property_data))
    // }

    /// Increase the ref count of a property
    pub fn increase_ref_count(&self, id: PropertyId) -> usize {
        log::warn!("increasing ref count of property {:?}", id);
        let res = self.with_entry_mut(id, |entry| {
            entry.ref_count += 1;
            entry.ref_count
        });
        log::warn!("increased ref count of property {:?}", id);
        res
    }

    /// Decrease the ref count of a property
    pub fn decrease_ref_count(&self, id: PropertyId) -> usize {
        log::warn!("decreasing ref count of property {:?}", id);
        let res = self.with_entry_mut(id, |entry| {
            entry.ref_count -= 1;
            entry.ref_count
        });
        log::warn!("decreased ref count of property {:?}", id);
        res
    }

    /// Replaces the way the source parameters property is being
    /// computed / it's value to the way target does.
    /// NOTE: source_id and target_id need to both contain
    /// the type T, or else this panics
    pub fn replace_property_keep_outbound_connections<T: PropertyValue + 'static>(
        &self,
        source_id: PropertyId,
        target_id: PropertyId,
    ) {
        // disconnect self from it's dependents, in preparation of overwriting
        // with targets inbound. (only does something for computed values)
        self.disconnect_inbound(source_id);

        // copy nessesary internal state from target to source
        self.with_property_data_mut(source_id, |source_property_data| {
            self.with_property_data_mut(target_id, |target_property_data| {
                // Copy over inbound, dirty state, and current value to source
                source_property_data.inbound = target_property_data.inbound.clone();
                source_property_data.value = target_property_data.clone_value::<T>();
                source_property_data.property_type = target_property_data.property_type.clone();
            });
        });

        // connect self to it's new dependents (found in property_types Expr
        // type as inbound) (only does something for computed values)
        self.connect_inbound(source_id);

        // enqueue source so its dependents are updated
        self.enqueued_sets.borrow_mut().push(source_id);
    }

    // re-computes the value if dirty
    pub fn update_value(&self, id: PropertyId) {
        log::warn!("updating value of property {:?}", id);
        let mut remove_dep_from_literal = false;
        let evaluator = self.with_property_data_mut(id, |property_data| {
            //short circuit if the value is still up to date
            match &mut property_data.property_type {
                PropertyType::Computed { evaluator, .. } => Some(Rc::clone(&evaluator)),
                PropertyType::Literal => {
                    // let tm = property_data.get_transition_manager::<T>()?;
                    // let curr_time = PROPERTY_TIME.with_borrow(|time| time.get());
                    // let value = tm.compute_eased_value(*curr_time);
                    // if let Some(interp_value) = value {
                    //     property_data.value = Box::new(interp_value);
                    // } else {
                    //     //transition must be over, let's remove dependencies
                    //     remove_dep_from_literal = true;
                    //     property_data.transition_manager = None;
                    // }
                    None
                }
            }
        });

        if remove_dep_from_literal {
            self.disconnect_inbound(id);
            self.with_property_data_mut(id, |property_data| {
                property_data.inbound.clear();
            });
        }

        if let Some(evaluator) = evaluator {
            // WARNING: the evaluator should not be run while the table is in
            // an invalid state (borrowed, in with_property_data closure, etc.)
            // as this function is provided by a user of the property system and
            // can do arbitrary sets/ gets/drops etc (that need the prop data)

            let new_value = { evaluator() };
            self.with_property_data_mut(id, |property_data| {
                property_data.value = new_value;
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
                    "failed to remove property - propertytable already borrowed"
                );
            };
            sm.remove(&id).expect("tried to remove non-existent prop")
        };
        drop(res);
    }

    pub(crate) fn total_properties_count(&self) -> usize {
        self.property_map.borrow().len()
    }
}
