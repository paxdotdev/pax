use std::{
    any::Any,
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

use slotmap::{SlotMap, SparseSecondaryMap};
use web_time::Instant;

use crate::{Property, TransitionManager, TransitionQueueEntry};

use super::{private::PropertyId, PropertyValue};

thread_local! {
    /// Global property table used to store data backing dirty-dag
    pub(crate) static PROPERTY_TABLE: PropertyTable = PropertyTable::default();
    /// Property time variable, to be used by
    pub(crate) static PROPERTY_TIME: RefCell<Property<u64>> = RefCell::new(Property::new(0));
}
pub static TIME_GET: AtomicU64 = AtomicU64::new(0);
pub static TIME_SET: AtomicU64 = AtomicU64::new(0);
pub static TIME_GENERAL: AtomicU64 = AtomicU64::new(0);

/// The main collection of data associated with a specific property id
pub struct PropertyData {
    // typed data for this property,
    // can always be downcast to TypedPropertyData<T>
    // where T matches the property type
    typed_data: Box<dyn Any>,
    // List of properties that this property depends on
    pub inbound: Vec<PropertyId>,
    // List of properties that depend on this value
    pub outbound: Vec<PropertyId>,
    // Dirty bit set if a depency further up in the dirty-dag tree
    // has been changed. For computed this can be any other props,
    // for literals, only time variable
    pub dirty: bool,
}

impl PropertyData {
    fn typed_data<T: 'static>(&mut self) -> &mut TypedPropertyData<T> {
        let typed_data = self
            .typed_data
            .downcast_mut::<TypedPropertyData<T>>()
            .expect("TypedPropertyData<T> should have correct type T during downcast");
        typed_data
    }
}

pub struct TypedPropertyData<T> {
    value: T,
    transition_manager: Option<TransitionManager<T>>,
    // Specialization data (computed/literal etc)
    property_type: PropertyType<T>,
}

/// Specialization data only needed for different kinds of properties
#[derive(Clone)]
pub(crate) enum PropertyType<T> {
    Literal,
    Computed {
        // Information needed to recompute on change
        evaluator: Rc<dyn Fn() -> T>,
    },
}

/// Main propertytable, containing data associated with each property
#[derive(Default)]
pub(crate) struct PropertyTable {
    // Main property table containing property data
    // Box<dyn Any> is of type Box<Entry<T>> where T is the proptype
    pub(crate) property_map: RefCell<SlotMap<PropertyId, Entry>>,
    debug_names: RefCell<SparseSecondaryMap<PropertyId, String>>,
}

pub struct Entry {
    ref_count: usize,
    data: Option<PropertyData>,
}

impl PropertyTable {
    /// Main function to get access to a value inside of a property.
    /// Makes sure the value is up to date before returning in the case
    /// of computed properties.
    pub fn get_value<T: PropertyValue>(&self, id: PropertyId) -> T {
        let get_start = Instant::now();
        self.update_value::<T>(id);
        let start = Instant::now();
        let res = self.with_property_data_mut(id, |property_data| {
            property_data.typed_data::<T>().value.clone()
        });
        let get_end = Instant::now();
        TIME_GET.fetch_add(
            get_end.duration_since(get_start).as_nanos() as u64,
            Ordering::Relaxed,
        );
        TIME_GENERAL.fetch_add(
            get_end.duration_since(start).as_nanos() as u64,
            Ordering::Relaxed,
        );
        res
    }

    // Main function to set a value of a property.
    // NOTE: This always assumes the underlying data was changed, and marks
    // it and it's dependents as dirty irrespective of actual modification
    pub fn set_value<T: PropertyValue>(&self, id: PropertyId, new_val: T) {
        let get_start = Instant::now();
        self.with_property_data_mut(id, |property_data: &mut PropertyData| {
            let typed_data = property_data.typed_data();
            typed_data.value = new_val;
        });
        self.dirtify_outbound(id);
        let get_end = Instant::now();
        TIME_SET.fetch_add(
            get_end.duration_since(get_start).as_nanos() as u64,
            Ordering::Relaxed,
        );
    }

    /// Adds a new untyped property entry
    pub fn add_entry<T: PropertyValue>(
        &self,
        start_val: T,
        inbound: Vec<PropertyId>,
        data: PropertyType<T>,
        debug_name: Option<&str>,
    ) -> PropertyId {
        let id = {
            let Ok(mut sm) = self.property_map.try_borrow_mut() else {
                panic!(
                    "couldn't create new property \"{}\"- table already borrowed",
                    debug_name.unwrap_or("<no name>")
                );
            };
            let entry = Entry {
                ref_count: 1,
                data: Some(PropertyData {
                    inbound,
                    dirty: true,
                    typed_data: Box::new(TypedPropertyData {
                        value: start_val,
                        property_type: data,
                        transition_manager: None,
                    }),
                    outbound: Vec::with_capacity(0),
                }),
            };
            sm.insert(entry)
        };
        if let Some(name) = debug_name {
            self.debug_names.borrow_mut().insert(id, name.to_owned());
        }
        self.connect_inbound(id);
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
        let (time_id, curr_time) = PROPERTY_TIME.with_borrow(|time| (time.untyped.id, time.get()));
        self.with_property_data_mut(id, |property_data: &mut PropertyData| {
            let typed_data = property_data.typed_data::<T>();
            let transition_manager = typed_data
                .transition_manager
                .get_or_insert_with(|| TransitionManager::new(typed_data.value.clone(), curr_time));
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
        let data = sm.get_mut(id).unwrap();
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

        res
    }

    /// Allows access to the data associated with a property id.
    /// WARNING while this method is being run, the entry corresponding to id
    /// is not present in the table, and methods such as .get(), .set(), and
    /// possibly others on the property with the id parameter bellow will panic.
    pub fn with_property_data<V>(&self, id: PropertyId, f: impl FnOnce(&PropertyData) -> V) -> V {
        self.with_property_data_mut(id, |property_data| f(&*property_data))
    }

    /// Increase the ref count of a property
    pub fn increase_ref_count(&self, id: PropertyId) -> usize {
        self.with_entry_mut(id, |entry| {
            entry.ref_count += 1;
            entry.ref_count
        })
    }

    /// Decrease the ref count of a property
    pub fn decrease_ref_count(&self, id: PropertyId) -> usize {
        self.with_entry_mut(id, |entry| {
            entry.ref_count -= 1;
            entry.ref_count
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
            self.with_property_data_mut(target_id, |target_property_data| {
                // Copy over inbound, dirty state, and current value to source
                source_property_data.inbound = target_property_data.inbound.clone();
                source_property_data.dirty = target_property_data.dirty;
                let source_typed = source_property_data.typed_data::<T>();
                let target_typed = target_property_data.typed_data::<T>();
                source_typed.value = target_typed.value.clone();
                source_typed.property_type = target_typed.property_type.clone();
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
    pub fn update_value<T: PropertyValue>(&self, id: PropertyId) {
        let start = Instant::now();
        let mut remove_dep_from_literal = false;
        let evaluator = self.with_property_data_mut(id, |property_data| {
            //short circuit if the value is still up to date
            if property_data.dirty == false {
                return None;
            }
            property_data.dirty = false;
            let typed_data = property_data.typed_data::<T>();
            match &mut typed_data.property_type {
                PropertyType::Computed { evaluator, .. } => Some(Rc::clone(&evaluator)),
                PropertyType::Literal => {
                    let tm = typed_data.transition_manager.as_mut()?;
                    let curr_time = PROPERTY_TIME.with_borrow(|time| time.get());
                    let value = tm.compute_eased_value(curr_time);
                    if let Some(interp_value) = value {
                        typed_data.value = interp_value;
                    } else {
                        //transition must be over, let's remove dependencies
                        remove_dep_from_literal = true;
                        typed_data.transition_manager = None;
                    }
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
        let end = Instant::now();
        TIME_GENERAL.fetch_add(
            end.duration_since(start).as_nanos() as u64,
            Ordering::Relaxed,
        );

        if let Some(evaluator) = evaluator {
            // WARNING: the evaluator should not be run while the table is in
            // an invalid state (borrowed, in with_property_data closure, etc.)
            // as this function is provided by a user of the property system and
            // can do arbitrary sets/ gets/drops etc (that need the prop data)

            let eval_start = Instant::now();
            //TRACING: this is the point we do not want included in a span, it is outside of prop system
            let new_value = { evaluator() };
            let eval_end = Instant::now();
            TIME_GET.fetch_sub(
                eval_end.duration_since(eval_start).as_nanos() as u64,
                Ordering::Relaxed,
            );
            let start = Instant::now();
            self.with_property_data_mut(id, |property_data| {
                let typed_data = property_data.typed_data();
                typed_data.value = new_value;
            });
            let end = Instant::now();
            TIME_GENERAL.fetch_add(
                end.duration_since(start).as_nanos() as u64,
                Ordering::Relaxed,
            );
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
