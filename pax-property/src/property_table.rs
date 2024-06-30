use nohash_hasher::BuildNoHashHasher;
use std::{
    any::Any,
    cell::{RefCell, RefMut},
    collections::{HashMap, HashSet},
};
use web_sys::{window, Performance};

use std::rc::Rc;

use crate::{
    generate_untyped_closure,
    transitions::{Interpolatable, TransitionManager, TransitionQueueEntry},
    Property, PropertyId, PropertyValue,
};

thread_local! {
    /// Global property table used to store data backing dirty-dag
    pub(crate) static PROPERTY_TABLE: PropertyTable = PropertyTable::default();
    /// Property time variable, to be used by
    pub(crate) static PROPERTY_TIME: RefCell<Property<u64>> = RefCell::new(Property::time());
     /// Statistics for tracking get requests
    static STATISTICS: RefCell<HashMap<String, f64>> = RefCell::default();

    pub static PERFORMANCE : Performance = window().unwrap().performance().unwrap();
}

pub fn stat_add(key: &str, val: f64) {
    STATISTICS.with_borrow_mut(|stats| {
        *stats.entry(key.to_owned()).or_default() += val;
    });
}

pub fn print_stats() {
    log::debug!("------- Stats -------");
    STATISTICS.with_borrow_mut(|stats| {
        for (k, v) in stats.drain() {
            log::debug!("{k}: {v}")
        }
    });
    log::debug!("---------------------");
}

#[derive(Default, Clone)]
pub struct PropScope {
    created_ids: Vec<PropertyId>,
}

pub struct PropertyTable {
    pub properties: RefCell<HashMap<PropertyId, Entry, BuildNoHashHasher<u64>>>,
    pub property_scopes: RefCell<Vec<PropScope>>,
}

pub struct Entry {
    pub data: PropertyData,
}

/// Specialization data only needed for different kinds of properties
#[derive(Clone)]
pub(crate) enum PropertyType {
    Literal,
    Expression {
        // Information needed to recompute on change
        evaluator: Rc<dyn Fn() -> Box<dyn Any>>,
    },
    Time {
        // List of currently transitioning properties
        transitioning: HashMap<PropertyId, TransitionCleanupInfo>,
    },
}

/// Information needed to cleanup a transitioning subscription to tick
#[derive(Clone)]
pub(crate) struct TransitionCleanupInfo {
    sub_id: SubscriptionId,
    is_finished: Rc<dyn Fn() -> bool>,
}

impl TransitionCleanupInfo {
    pub fn new(sub_id: SubscriptionId, is_finished: Rc<dyn Fn() -> bool>) -> Self {
        Self {
            sub_id,
            is_finished,
        }
    }

    pub fn cleanup(&self) {
        if (self.is_finished)() {
            PROPERTY_TIME.with(|t| t.borrow_mut().unsubscribe(self.sub_id.clone()));
        }
    }
}

#[derive(Clone, Default)]
pub struct PropertyScopeManager {
    pub property_ids: RefCell<Vec<PropertyId>>,
    // need to keep track if this has been cloned, and only destroy props
    // if last propscopemanager
    pub ref_count: Rc<()>,
}

impl PropertyScopeManager {
    pub fn new() -> Self {
        Self {
            property_ids: RefCell::default(),
            ref_count: Rc::new(()),
        }
    }

    pub fn start_scope(&self) {
        PROPERTY_TABLE.with(|t| {
            t.start_scope();
        });
    }
    pub fn end_scope(&self) {
        PROPERTY_TABLE.with(|t| {
            let scope = t.end_scope();
            self.property_ids.borrow_mut().extend(scope.created_ids);
        });
    }

    pub fn run_with_scope<V>(&self, f: impl FnOnce() -> V) -> V {
        self.start_scope();
        let res = f();
        self.end_scope();
        res
    }

    pub fn drop_scope(&self) {
        PROPERTY_TABLE.with(|t| {
            for id in self.property_ids.borrow_mut().drain(0..) {
                t.remove_entry(id);
            }
        });
    }
}

impl Drop for PropertyScopeManager {
    fn drop(&mut self) {
        // if this is the last ref being dropped, clean up the props
        if Rc::strong_count(&self.ref_count) == 1 {
            self.drop_scope();
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct SubscriptionId(usize);

#[derive(Clone)]
pub struct Subscriptions {
    pub subscriptions: HashMap<SubscriptionId, Rc<dyn Fn()>>,
    pub cached_subscriptions: Vec<Rc<dyn Fn()>>,
    pub next_id: SubscriptionId,
}

impl Default for Subscriptions {
    fn default() -> Self {
        Self {
            subscriptions: HashMap::new(),
            cached_subscriptions: Vec::new(),
            next_id: SubscriptionId(0),
        }
    }
}

impl Subscriptions {
    pub fn add(&mut self, sub: Rc<dyn Fn()>) -> SubscriptionId {
        let id = self.next_id.clone();
        self.subscriptions.insert(id.clone(), sub);
        self.next_id = SubscriptionId(id.0 + 1);
        self.update_cached_subscriptions();
        id
    }

    pub fn remove(&mut self, id: SubscriptionId) {
        self.subscriptions.remove(&id);
        self.update_cached_subscriptions();
    }

    fn update_cached_subscriptions(&mut self) {
        self.cached_subscriptions = self.subscriptions.values().cloned().collect();
    }

    pub fn get_cloned_subscriptions(&self) -> Vec<Rc<dyn Fn()>> {
        self.cached_subscriptions.clone()
    }
}

pub struct PropertyData {
    // Data associated with the property
    pub value: Box<dyn Any>,
    // Closures to run when this property is set
    pub subscriptions: Subscriptions,
    // The type of the property
    pub property_type: PropertyType,
    // List of properties that this property depends on
    pub inbound: HashSet<PropertyId>,
    // List of properties that depend on this value
    pub outbound: HashSet<PropertyId>,
    // Topologically sorted dependencies (None if not computed yet)
    pub dependents_to_update: Option<Vec<PropertyId>>,
    // Type agnostic transition manager
    transition_manager: Option<TransitionManagerWrapper>,
}

#[derive(Clone)]
struct TransitionManagerWrapper {
    manager: Rc<dyn Any>,
    queue_length_closure: Rc<dyn Fn() -> bool>,
}

impl TransitionManagerWrapper {
    pub fn new<T: Interpolatable + 'static>(value: T, current_time: u64) -> Self {
        let manager = Rc::new(RefCell::new(TransitionManager::new(value, current_time)));
        let cloned_manager = manager.clone();
        Self {
            manager,
            queue_length_closure: Rc::new(move || cloned_manager.borrow().is_finished()),
        }
    }

    pub fn get_manager_as_mut<T: Interpolatable + 'static>(&self) -> RefMut<TransitionManager<T>> {
        self.manager
            .downcast_ref::<RefCell<TransitionManager<T>>>()
            .unwrap()
            .borrow_mut()
    }
}

impl PropertyData {
    pub fn get_value<T: PropertyValue>(&self) -> T {
        match self.value.downcast_ref::<T>() {
            Some(value) => value.clone(),
            None => {
                panic!(
                    "Failed to downcast to the requested type: {}",
                    std::any::type_name::<T>()
                );
            }
        }
    }

    pub fn set_value<T: PropertyValue>(&mut self, new_val: T) {
        self.value = Box::new(new_val);
    }
}

impl PropertyTable {
    fn start_scope(&self) {
        let mut scopes = self.property_scopes.borrow_mut();
        scopes.push(PropScope::default());
    }

    fn end_scope(&self) -> PropScope {
        let mut scopes = self.property_scopes.borrow_mut();
        let scope = scopes.pop().expect("No scope to end");
        scope
    }

    pub fn push_to_scope_if_exists(&self, id: PropertyId) {
        let mut scopes = self.property_scopes.borrow_mut();
        if let Some(scope) = scopes.last_mut() {
            scope.created_ids.push(id);
        } else {
            panic!("properties can only be created inside a drop scope handler");
        }
    }

    pub fn insert<T: PropertyValue>(
        &self,
        property_type: PropertyType,
        value: T,
        inbound: Vec<PropertyId>,
        is_static: bool,
    ) -> PropertyId {
        let id = {
            let Ok(mut sm) = self.properties.try_borrow_mut() else {
                panic!("Failed to borrow property table");
            };
            let id = PropertyId::new();
            sm.insert(
                id,
                Entry {
                    data: PropertyData {
                        value: Box::new(value),
                        subscriptions: Subscriptions::default(),
                        property_type,
                        inbound: inbound.clone().into_iter().collect(),
                        outbound: HashSet::new(),
                        dependents_to_update: None,
                        transition_manager: None,
                    },
                },
            );
            for i in &inbound {
                // Connect the new property to its dependencies
                sm.get_mut(i).map(|entry| {
                    entry.data.outbound.insert(id);
                });
            }
            id
        };

        if !is_static {
            self.push_to_scope_if_exists(id);
        }
        self.clear_memoized_dependents(id);
        id
    }

    pub fn get<T: PropertyValue>(&self, id: PropertyId) -> T {
        let sm = self.properties.borrow();
        if !sm.contains_key(&id) {
            return T::default();
        }

        let entry = sm.get(&id).expect("Property not found");
        let value = entry.data.get_value();
        value
    }

    pub fn set<T: PropertyValue>(&self, id: PropertyId, new_val: T) {
        if !self.properties.borrow().contains_key(&id) {
            log::warn!("tried to set property that doesn't exist");
            return;
        }

        let mut all_subscriptions = Vec::new();

        // update value of property and grab dependencies to update
        let (mut deps, current_node_subscriptions) = {
            let mut sm = self.properties.borrow_mut();
            let entry = sm.get_mut(&id).expect("Property not found");
            entry.data.set_value(new_val);
            (
                entry.data.dependents_to_update.clone(),
                entry.data.subscriptions.get_cloned_subscriptions(),
            )
        };

        all_subscriptions.extend(current_node_subscriptions);

        if deps.is_none() {
            deps = Some(self.topological_sort_affected(id));
            {
                let mut sm = self.properties.borrow_mut();
                let entry = sm.get_mut(&id).expect("Property not found");
                entry.data.dependents_to_update = deps.clone();
            }
        }

        // update all dependent properties & collect subscriptions
        for dep_id in deps.unwrap() {
            {
                let mut sm = self.properties.borrow_mut();
                let entry = sm.get_mut(&dep_id).expect("Property not found");
                all_subscriptions.extend(entry.data.subscriptions.get_cloned_subscriptions());
            }
            self.recompute_expression(dep_id);
        }

        for sub in all_subscriptions {
            sub();
        }
    }

    fn recompute_expression(&self, id: PropertyId) {
        let evaluator: Option<Rc<dyn Fn() -> Box<dyn Any>>> = {
            let sm = self.properties.borrow();
            let entry = sm.get(&id).expect("Property not found");
            match &entry.data.property_type {
                PropertyType::Expression { evaluator } => Some(evaluator.clone()),
                _ => None,
            }
        };
        let new_value = evaluator.expect("Literal cannot be recomputed")();
        {
            let mut sm = self.properties.borrow_mut();
            let entry = sm.get_mut(&id).expect("Property not found");
            entry.data.value = new_value;
        }
    }

    pub fn replace_with<T: PropertyValue>(
        &self,
        older_property: PropertyId,
        new_property: PropertyId,
    ) {
        let old_inbound = {
            let mut sm = self.properties.borrow_mut();
            let new_property_entry = sm.get_mut(&new_property).expect("Property not found");
            new_property_entry.data.outbound.insert(older_property);

            let old_property_entry = sm.get_mut(&older_property).expect("Property not found");
            let ret = old_property_entry.data.inbound.clone();
            old_property_entry.data.inbound = HashSet::new();
            old_property_entry.data.inbound.insert(new_property);

            let new_property: Property<T> = new_property.get_property();
            let mirror_closure = move || new_property.get();
            let untyped_closure = generate_untyped_closure(mirror_closure);
            let property_type = PropertyType::Expression {
                evaluator: Rc::new(untyped_closure),
            };
            old_property_entry.data.property_type = property_type;
            ret
        };

        for id in old_inbound.clone() {
            {
                let mut sm = self.properties.borrow_mut();
                let entry = sm.get_mut(&id).expect("Property not found");
                entry.data.outbound.remove(&older_property);
            }
        }
        self.clear_memoized_dependents(older_property);
    }

    pub fn print_outbound(&self, id: PropertyId) {
        let sm = self.properties.borrow();
        let entry = sm.get(&id).expect("Property not found");
        log::warn!(
            "Outbound for property: {:?}",
            entry.data.outbound.iter().collect::<Vec<&PropertyId>>()
        );
    }

    pub fn print_inbound(&self, id: PropertyId) {
        let sm = self.properties.borrow();
        let entry = sm.get(&id).expect("Property not found");
        log::warn!(
            "Inbound for property: {:?}",
            entry.data.inbound.iter().collect::<Vec<&PropertyId>>()
        );
    }

    pub fn subscribe(&self, id: PropertyId, sub: Rc<dyn Fn()>) -> SubscriptionId {
        sub();
        let mut sm = self.properties.borrow_mut();
        let entry = sm.get_mut(&id).expect("Property not found");
        entry.data.subscriptions.add(sub)
    }

    pub fn unsubscribe(&self, id: PropertyId, sub_id: SubscriptionId) {
        let mut sm = self.properties.borrow_mut();
        let entry = sm.get_mut(&id).expect("Property not found");
        entry.data.subscriptions.remove(sub_id);
    }

    pub fn transition<T: PropertyValue + Interpolatable>(
        &self,
        id: PropertyId,
        transition: TransitionQueueEntry<T>,
        overwrite: bool,
    ) {
        // get current value
        let value = {
            let sm = self.properties.borrow();
            let entry = sm.get(&id).expect("Property not found");
            entry.data.get_value::<T>()
        };

        let current_time = PROPERTY_TIME.with(|t| t.borrow().get());

        // add transition to transition manager
        {
            let mut sm = self.properties.borrow_mut();
            let entry: &mut Entry = sm.get_mut(&id).expect("Property not found");
            if let Some(transition_manager) = &entry.data.transition_manager {
                if overwrite {
                    transition_manager
                        .get_manager_as_mut::<T>()
                        .reset_transitions(current_time);
                }
                transition_manager
                    .get_manager_as_mut::<T>()
                    .push_transition(transition);
            } else {
                let manager = TransitionManagerWrapper::new(value, current_time);
                entry.data.transition_manager = Some(manager);
                entry
                    .data
                    .transition_manager
                    .as_mut()
                    .unwrap()
                    .get_manager_as_mut::<T>()
                    .push_transition(transition);
            }
        }

        // add subscription to time property
        self.add_transitioning_subscription::<T>(id);
    }

    pub fn cleanup_finished_transitions(&self) {
        let time = &PROPERTY_TIME.with(|t| t.borrow().get_id());
        let mut cleanups = Vec::new();
        // collect finished transitions and the information necessary to clean them up
        {
            let mut sm = self.properties.borrow_mut();
            let entry = sm.get_mut(time).expect("Property not found");

            match &mut entry.data.property_type {
                PropertyType::Time { transitioning } => {
                    let mut to_remove = Vec::new();
                    for (id, cleanup_info) in transitioning.iter() {
                        if (cleanup_info.is_finished)() {
                            to_remove.push(*id);
                            cleanups.push(cleanup_info.clone());
                        }
                    }
                    for id in to_remove {
                        transitioning.remove(&id);
                    }
                }
                _ => panic!("Property is not a time property"),
            }
        }
        for cleanup in cleanups {
            cleanup.cleanup();
        }
    }

    fn get_transition_manager(&self, id: PropertyId) -> Option<TransitionManagerWrapper> {
        let sm = self.properties.borrow();
        let entry = sm.get(&id).expect("Property not found");
        entry.data.transition_manager.clone()
    }

    pub fn get_currently_running_transitions(&self) -> HashMap<PropertyId, TransitionCleanupInfo> {
        let time = &PROPERTY_TIME.with(|t| t.borrow().get_id());
        let sm = self.properties.borrow();
        let entry = sm.get(time).expect("Property not found");
        match &entry.data.property_type {
            PropertyType::Time { transitioning } => transitioning.clone(),
            _ => panic!("Property is not a time property"),
        }
    }

    pub fn add_to_currently_running_transitions(
        &self,
        id: PropertyId,
        cleanup_info: TransitionCleanupInfo,
    ) {
        let time = &PROPERTY_TIME.with(|t| t.borrow().get_id());
        let mut sm = self.properties.borrow_mut();
        let entry = sm.get_mut(time).expect("Property not found");
        match &mut entry.data.property_type {
            PropertyType::Time { transitioning } => {
                transitioning.insert(id, cleanup_info);
            }
            _ => panic!("Property is not a time property"),
        }
    }

    pub fn add_transitioning_subscription<T: PropertyValue + Interpolatable>(
        &self,
        id: PropertyId,
    ) {
        let time: &PropertyId = &PROPERTY_TIME.with(|t| t.borrow().get_id());

        // get transitioning properties
        let current_transitions = self.get_currently_running_transitions();

        // transitioning property exists return otherwise add subscription
        if current_transitions.contains_key(&id) {
            return;
        }

        let transition_manager = self.get_transition_manager(id);

        if let Some(transition_manager) = transition_manager {
            let cloned_transition_manager = transition_manager.clone();
            let sub_id = self.subscribe(
                *time,
                Rc::new(move || {
                    let time = PROPERTY_TIME.with(|t| t.borrow().get());
                    let mut manager = cloned_transition_manager.get_manager_as_mut::<T>();
                    let eased_value = manager.compute_eased_value(time);
                    if let Some(new_val) = eased_value {
                        PROPERTY_TABLE.with(|t| t.set(id, new_val));
                    }
                }),
            );

            let cleanup_info =
                TransitionCleanupInfo::new(sub_id, transition_manager.queue_length_closure);
            self.add_to_currently_running_transitions(id, cleanup_info);
        }
    }

    pub fn remove_entry(&self, id: PropertyId) {
        self.clear_memoized_dependents(id);
        {
            let mut sm = self.properties.borrow_mut();
            let (outbound, inbound) = {
                let entry = sm.get(&id).expect("Property not found");
                (entry.data.outbound.clone(), entry.data.inbound.clone())
            };

            for outbound_id in outbound {
                if let Some(entry) = sm.get_mut(&outbound_id) {
                    entry.data.inbound.remove(&id);
                }
            }
            for inbound_id in inbound.clone() {
                if let Some(entry) = sm.get_mut(&inbound_id) {
                    entry.data.outbound.remove(&id);
                }
            }
            sm.remove(&id);
        }
    }
}

impl Default for PropertyTable {
    fn default() -> Self {
        PropertyTable {
            properties: RefCell::new(HashMap::with_capacity_and_hasher(
                100,
                BuildNoHashHasher::default(),
            )),
            property_scopes: RefCell::new(Vec::new()),
        }
    }
}

pub fn print_number_of_properties() {
    PROPERTY_TABLE.with(|t| {
        let num_properties = t.properties.borrow().len();
        log::info!("Number of properties: {}", num_properties);
    });
}

pub fn print_scope_stack() {
    PROPERTY_TABLE.with(|t| {
        let scopes = t.property_scopes.borrow();
        log::info!("Scope stack: {:?}", scopes.len());
    });
}
