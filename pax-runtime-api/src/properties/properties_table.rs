use std::{any::Any, cell::RefCell, rc::Rc};

use slotmap::{SlotMap, SparseSecondaryMap};

use super::{private::PropertyId, PropertyValue};

thread_local! {
    pub static PROPERTY_TABLE: PropertyTable = PropertyTable::new();
}

pub struct PropertyData {
    pub value: Option<Box<dyn Any>>,
    pub subscribers: Vec<PropertyId>,
    pub ref_count: usize,
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
        subscriptions: Vec<PropertyId>,
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
            if let PropertyType::Expr { subscriptions, .. } = &prop_data.prop_type {
                for dep_id in subscriptions {
                    self.with_property_value_mut(*dep_id, |dep_prop| {
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

    /// You cannot use .get() or .set() on the mutably borrowed property
    pub fn with_property_value_mut<V>(
        &self,
        id: PropertyId,
        f: impl FnOnce(&mut Box<dyn Any>) -> V,
    ) -> Result<V, String> {
        let mut requested_data = {
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
            prop_data.value.take()
        }.expect("value should always be present");

        let return_value = f(&mut requested_data);
    
        // Refactor to another method
        {
            let mut sm = self.entries.try_borrow_mut().map_err(|_| {
                format!(
                    "failed to borrow table for use by property \"{}\" - already mutably borrowed",
                    self.debug_name(id),
                )
            })?;
            let mut prop_data = sm.get_mut(id).ok_or(&format!(
                "tried to get prop_data for property \"{}\" that doesn't exist anymore",
                self.debug_name(id)
            ))?;
            prop_data.borrow_mut().value = Some(requested_data);
        }
        Ok(ret)
    }

    pub fn with_prop_data<V>(
        &self,
        id: PropertyId,
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
    pub fn update_value(&self, id: PropertyId) {
        let evaluator = self
            .with_property_value_mut(id, |prop_data| {
                if let PropertyType::Expr {
                    dirty: true, evaluator, ..
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
            self.with_property_value_mut(id, |prop_data| {
                // WARNING: don't remove this. old value needs to be returned outside of this prop_data mut before being dropped
                std::mem::replace(&mut prop_data.value, new_value)
            })
            .map_err(|e| format!("update_value error: {}", e))
            .unwrap();
        }
    }

    /// Main function to get access to a reference inside of a property.
    /// Makes sure the value is up to date before calling the provided closure
    pub fn get_value_ref<T: PropertyValue, V>(&self, id: PropertyId, f: impl FnOnce(&T) -> V) -> V {
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
    /// TODO Change this to set with value : T 
    pub fn get_value_mut<T: PropertyValue, V>(&self, id: PropertyId, f: impl FnOnce(&mut T) -> V) -> V {
        let mut to_dirty = vec![];
        let ret_value = self
            .with_property_value_mut(id, |prop_data: &mut PropertyData| {
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
        // put 
        while let Some(dep_id) = to_dirty.pop() {
            self.with_property_value_mut(dep_id, |dep_data| {
                if dep_id == id {
                    unreachable!(
                        "property cycles should never happen with literals/computeds being a DAG"
                    );
                }
                let PropertyType::Expr { ref mut dirty, .. } = dep_data.prop_type else {
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

    pub fn remove_entry(&self, id: PropertyId) {
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
            self.with_property_value_mut(sub, |s| {
                if let PropertyType::Expr { subscriptions, .. } = &mut s.prop_type {
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
        if let PropertyType::Expr { subscriptions, .. } = prop_data.prop_type {
            for subscription in subscriptions {
                self.with_property_value_mut(subscription, |sub| {
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
