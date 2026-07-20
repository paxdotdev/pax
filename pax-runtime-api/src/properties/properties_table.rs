use std::{
    any::Any,
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    rc::Rc,
};

use slotmap::SlotMap;
#[cfg(debug_assertions)]
use slotmap::SparseSecondaryMap;

use crate::{Property, TransitionManager, TransitionQueueEntry};

use super::{private::PropertyId, PropertyValue};

#[derive(Clone, Debug, Default)]
pub struct EffectDrainReport {
    pub ran: usize,
    pub popped: usize,
    pub skipped_clean: usize,
    pub skipped_unregistered: usize,
    pub skipped_missing: usize,
    pub remaining: usize,
    pub top_effects: Vec<(String, usize)>,
}

thread_local! {
    // Global property table used to store data backing dirty-dag
    pub(crate) static PROPERTY_TABLE: PropertyTable = PropertyTable::default();
    // Global frame timestamp.
    pub(crate) static PROPERTY_TIME: RefCell<Property<u64>> = RefCell::new(Property::new(0));
    // Global wall-clock timestamp in milliseconds.
    pub(crate) static PROPERTY_MILLIS: RefCell<Property<u64>> = RefCell::new(Property::new(0));
}

// The main collection of data associated with a specific property id
pub struct PropertyData {
    // typed data for this property,
    // can always be downcast to TypedPropertyData<T>
    // where T matches the property type
    typed_data: Box<dyn Any>,
    // List of properties that this property depends on
    pub inbound: Vec<PropertyId>,
    // List of properties that depend on this value
    pub outbound: Vec<PropertyId>,
    // Dirty bit set if a dependency further up in the dirty-dag tree
    // has been changed. For computed this can be any other props,
    // for literals, only time variable
    pub dirty: bool,
}

impl PropertyData {
    fn typed_data<T: 'static>(&mut self) -> &mut TypedPropertyData<T> {
        self.typed_data
            .downcast_mut::<TypedPropertyData<T>>()
            .unwrap_or_else(|| panic!("Failed to downcast to TypedPropertyData<{}>. The actual type does not match the expected type.", std::any::type_name::<T>()))
    }
}

// Type-specialized payload for one `Property<T>` table entry.
pub struct TypedPropertyData<T> {
    value: T,
    transition_manager: Option<TransitionManager<T>>,
    // Specialization data (computed/literal etc)
    property_type: PropertyType<T>,
}

// Specialization data only needed for different kinds of properties
#[derive(Clone)]
pub(crate) enum PropertyType<T> {
    Literal,
    Computed {
        // Information needed to recompute on change
        evaluator: Rc<dyn Fn() -> T>,
    },
}

// Main propertytable, containing data associated with each property
#[derive(Default)]
pub(crate) struct PropertyTable {
    // Main property table containing property data
    // Box<dyn Any> is of type Box<Entry<T>> where T is the proptype
    pub(crate) property_map: RefCell<SlotMap<PropertyId, Entry>>,
    #[cfg(debug_assertions)]
    debug_names: RefCell<SparseSecondaryMap<PropertyId, String>>,
    effect_debug_names: RefCell<HashMap<PropertyId, String>>,
    effect_properties: RefCell<HashSet<PropertyId>>,
    queued_effects: RefCell<VecDeque<PropertyId>>,
    queued_effect_set: RefCell<HashSet<PropertyId>>,
}

// Reference-counted slotmap entry for one property id.
pub struct Entry {
    ref_count: usize,
    data: Option<PropertyData>,
}

impl PropertyTable {
    // Main function to get access to a value inside of a property.
    // Makes sure the value is up to date before returning in the case
    // of computed properties.
    pub fn get_value<T: PropertyValue>(&self, id: PropertyId) -> T {
        self.update_value::<T>(id);
        self.with_property_data_mut(id, |property_data| {
            property_data.typed_data::<T>().value.clone()
        })
    }

    // Reads a property value by reference after updating dirty dependencies.
    pub fn read_value<T: PropertyValue, V>(&self, id: PropertyId, f: impl FnOnce(&T) -> V) -> V {
        self.update_value::<T>(id);
        self.with_property_data_mut(id, |property_data| {
            f(&property_data.typed_data::<T>().value)
        })
    }

    // Main function to set a value of a property.
    // NOTE: This always assumes the underlying data was changed, and marks
    // it and its dependents as dirty irrespective of actual modification
    pub fn set_value<T: PropertyValue>(&self, id: PropertyId, new_val: T) {
        self.with_property_data_mut(id, |property_data: &mut PropertyData| {
            let typed_data = property_data.typed_data();
            typed_data.value = new_val;
            property_data.dirty = false;
        });
        self.dirtify_outbound(id);
    }

    pub fn invalidate(&self, id: PropertyId) {
        let newly_dirty = self.with_property_data_mut(id, |property_data| {
            if property_data.dirty {
                false
            } else {
                property_data.dirty = true;
                true
            }
        });
        if newly_dirty {
            self.enqueue_effect_if_registered(id);
            self.dirtify_outbound(id);
        }
    }

    // Adds a new untyped property entry
    pub fn add_entry<T: PropertyValue>(
        &self,
        start_val: T,
        inbound: Vec<PropertyId>,
        data: PropertyType<T>,
        debug_name: Option<&str>,
    ) -> PropertyId {
        #[cfg(not(debug_assertions))]
        let _ = debug_name;

        let id = {
            let Ok(mut sm) = self.property_map.try_borrow_mut() else {
                #[cfg(debug_assertions)]
                panic!(
                    "couldn't create new property \"{}\"- table already borrowed",
                    debug_name.unwrap_or("<no name>")
                );
                #[cfg(not(debug_assertions))]
                panic!("couldn't create new property - table already borrowed");
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
        #[cfg(debug_assertions)]
        {
            if let Some(name) = debug_name {
                self.debug_names.borrow_mut().insert(id, name.to_owned());
            }
        }
        self.connect_inbound(id);
        id
    }

    // Add a transition to the transitionmanager, making the value slowly change over time
    // Currently this only transitions the literal value of the property (and updates dependents accordingly)
    // This has no special interactions with computed properties
    pub fn transition<T: PropertyValue>(
        &self,
        id: PropertyId,
        transition: TransitionQueueEntry<T>,
        overwrite: bool,
    ) {
        let mut should_reconnect_inbound = false;
        let (time_id, curr_time) = PROPERTY_TIME.with_borrow(|time| (time.untyped.id, time.get()));
        let (millis_id, curr_millis) =
            PROPERTY_MILLIS.with_borrow(|time| (time.untyped.id, time.get()));
        self.with_property_data_mut(id, |property_data: &mut PropertyData| {
            let typed_data = property_data.typed_data::<T>();
            let transition_manager = typed_data.transition_manager.get_or_insert_with(|| {
                TransitionManager::new(typed_data.value.clone(), curr_time, curr_millis)
            });
            if overwrite {
                transition_manager.reset_transitions(curr_time, curr_millis);
            }
            transition_manager.push_transition(transition);
            if !property_data.inbound.contains(&time_id) {
                should_reconnect_inbound = true;
                property_data.inbound.push(time_id);
            }
            if !property_data.inbound.contains(&millis_id) {
                should_reconnect_inbound = true;
                property_data.inbound.push(millis_id);
            }
        });
        if should_reconnect_inbound {
            self.disconnect_inbound(id);
            self.connect_inbound(id);
        }
    }

    // Gives mutable access to a entry in the property table
    // WARNING: this function is dangerous, f can not drop, create, set, get
    // or in any other way modify the global property table or this will panic
    // with multiple mutable borrows. Letting f contain any form of userland
    // code is NOT a good idea.
    fn with_entry_mut<V>(&self, id: PropertyId, f: impl FnOnce(&mut Entry) -> V) -> V {
        let mut sm = self.property_map.borrow_mut();
        let data = sm.get_mut(id).unwrap();
        let return_value = f(data);
        return_value
    }

    // Allows mutable access to the data associated with a property id.
    // WARNING while this method is being run, the entry corresponding to id
    // is not present in the table, and methods such as .get(), .set(), and
    // possibly others on the property with the id parameter bellow will panic.
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

    // Allows access to the data associated with a property id.
    // WARNING while this method is being run, the entry corresponding to id
    // is not present in the table, and methods such as .get(), .set(), and
    // possibly others on the property with the id parameter bellow will panic.
    pub fn with_property_data<V>(&self, id: PropertyId, f: impl FnOnce(&PropertyData) -> V) -> V {
        self.with_property_data_mut(id, |property_data| f(&*property_data))
    }

    // Increase the ref count of a property
    pub fn increase_ref_count(&self, id: PropertyId) -> usize {
        self.with_entry_mut(id, |entry| {
            entry.ref_count += 1;
            entry.ref_count
        })
    }

    // Decrease the ref count of a property
    pub fn decrease_ref_count(&self, id: PropertyId) -> usize {
        self.with_entry_mut(id, |entry| {
            entry.ref_count -= 1;
            entry.ref_count
        })
    }

    // Replaces the way the source parameters property is being
    // computed / its value to the way target does.
    // NOTE: source_id and target_id need to both contain
    // the type T, or else this panics
    pub fn replace_property_keep_outbound_connections<T: Clone + 'static>(
        &self,
        source_id: PropertyId,
        target_id: PropertyId,
    ) {
        if source_id == target_id {
            return;
        }

        // disconnect self from its dependents, in preparation of overwriting
        // with targets inbound. (only does something for computed values)
        self.disconnect_inbound(source_id);

        // copy necessary internal state from target to source
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

        // connect self to its new dependents (found in property_types Expr
        // type as inbound) (only does something for computed values)
        self.connect_inbound(source_id);

        // make sure dependencies of self
        // know that something has changed
        self.dirtify_outbound(source_id);

        #[cfg(debug_assertions)]
        {
            // overwrite with more descriptive name
            let target_name = self.debug_name(target_id);
            let mut names = self.debug_names.borrow_mut();
            names.insert(source_id, format!("{}", target_name));
        }

        self.enqueue_effect_if_registered(source_id);
    }

    // re-computes the value if dirty
    pub fn update_value<T: PropertyValue>(&self, id: PropertyId) {
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
                    let curr_millis = PROPERTY_MILLIS.with_borrow(|time| time.get());
                    let value = tm.compute_eased_value(curr_time, curr_millis);
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

        if let Some(evaluator) = evaluator {
            // WARNING: the evaluator should not be run while the table is in
            // an invalid state (borrowed, in with_property_data closure, etc.)
            // as this function is provided by a user of the property system and
            // can do arbitrary sets/ gets/drops etc (that need the prop data)
            let new_value = { evaluator() };
            self.with_property_data_mut(id, |property_data| {
                let typed_data = property_data.typed_data();
                typed_data.value = new_value;
            })
        }
    }

    // drop a properties underlying data, making any subsequent calls invalid by panic
    pub fn remove_entry(&self, id: PropertyId) {
        let res = {
            self.unregister_effect(id);
            self.disconnect_outbound(id);
            self.disconnect_inbound(id);
            let Ok(mut sm) = self.property_map.try_borrow_mut() else {
                #[cfg(debug_assertions)]
                panic!(
                    "failed to remove property \"{}\" - propertytable already borrowed",
                    self.debug_name(id),
                );
                #[cfg(not(debug_assertions))]
                panic!("failed to remove property - propertytable already borrowed");
            };
            sm.remove(id).expect("tried to remove non-existent prop")
        };
        drop(res);
    }

    // Returns a human-readable name for diagnostics, falling back to the raw id.
    #[cfg(debug_assertions)]
    pub fn debug_name(&self, id: PropertyId) -> String {
        self.debug_names
            .borrow()
            .get(id)
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("<NO DEBUG NAME>")
            .to_owned()
    }

    // Returns the number of live property-table slots.
    pub(crate) fn total_properties_count(&self) -> usize {
        self.property_map.borrow().len()
    }

    pub(crate) fn register_effect(&self, id: PropertyId) {
        self.register_effect_with_name(id, None);
    }

    pub(crate) fn register_effect_with_name(&self, id: PropertyId, debug_name: Option<&str>) {
        if let Some(debug_name) = debug_name {
            self.effect_debug_names
                .borrow_mut()
                .insert(id, debug_name.to_owned());
        }
        self.effect_properties.borrow_mut().insert(id);
        self.enqueue_effect_if_registered(id);
    }

    pub(crate) fn drain_effects(&self, max_iterations: usize) -> usize {
        let mut ran = 0;
        while ran < max_iterations {
            let Some(id) = self.pop_queued_effect() else {
                break;
            };
            if !self.is_registered_effect(id) || !self.has_live_entry(id) {
                continue;
            }
            let dirty = self.with_property_data(id, |property_data| property_data.dirty);
            if dirty {
                self.update_value::<()>(id);
                ran += 1;
            }
        }
        ran
    }

    pub(crate) fn drain_effects_with_report(&self, max_iterations: usize) -> EffectDrainReport {
        let mut report = EffectDrainReport::default();
        let mut effect_counts = HashMap::new();

        while report.ran < max_iterations {
            let Some(id) = self.pop_queued_effect() else {
                break;
            };
            report.popped += 1;

            if !self.is_registered_effect(id) {
                report.skipped_unregistered += 1;
                continue;
            }
            if !self.has_live_entry(id) {
                report.skipped_missing += 1;
                continue;
            }

            let dirty = self.with_property_data(id, |property_data| property_data.dirty);
            if dirty {
                let effect_name = self.debug_name_for_diagnostics(id);
                self.update_value::<()>(id);
                report.ran += 1;
                *effect_counts.entry(effect_name).or_insert(0) += 1;
            } else {
                report.skipped_clean += 1;
            }
        }

        report.remaining = self.queued_effects.borrow().len();
        report.top_effects = effect_counts.into_iter().collect();
        report
            .top_effects
            .sort_by(|(left_name, left_count), (right_name, right_count)| {
                right_count
                    .cmp(left_count)
                    .then_with(|| left_name.cmp(right_name))
            });
        report.top_effects.truncate(8);
        report
    }

    pub(crate) fn enqueue_effect_if_registered(&self, id: PropertyId) {
        if self.is_registered_effect(id) {
            self.enqueue_effect(id);
        }
    }

    fn enqueue_effect(&self, id: PropertyId) {
        let mut queued_set = self.queued_effect_set.borrow_mut();
        if queued_set.insert(id) {
            self.queued_effects.borrow_mut().push_back(id);
        }
    }

    fn pop_queued_effect(&self) -> Option<PropertyId> {
        let id = self.queued_effects.borrow_mut().pop_front()?;
        self.queued_effect_set.borrow_mut().remove(&id);
        Some(id)
    }

    fn unregister_effect(&self, id: PropertyId) {
        self.effect_properties.borrow_mut().remove(&id);
        self.effect_debug_names.borrow_mut().remove(&id);
        self.queued_effect_set.borrow_mut().remove(&id);
    }

    fn is_registered_effect(&self, id: PropertyId) -> bool {
        self.effect_properties.borrow().contains(&id)
    }

    fn has_live_entry(&self, id: PropertyId) -> bool {
        self.property_map
            .borrow()
            .get(id)
            .and_then(|entry| entry.data.as_ref())
            .is_some()
    }

    fn debug_name_for_diagnostics(&self, id: PropertyId) -> String {
        if let Some(name) = self.effect_debug_names.borrow().get(&id) {
            return name.clone();
        }

        #[cfg(debug_assertions)]
        {
            if let Some(name) = self.debug_names.borrow().get(id) {
                return name.clone();
            }
        }

        format!("{id:?}")
    }
}
