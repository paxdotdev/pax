use std::collections::{HashMap, HashSet, VecDeque};

use super::{properties_table::{PropertyId, PropertyTable, PropertyType}, PropertyValue};

impl PropertyTable {

    /// Removes id from it's dependents
    /// NOTE: does NOT modify the outbound list
    pub fn disconnect_outbound(&self, id: PropertyId) {
        self.with_property_data(id, |property_data| {
            for outbound_id in &property_data.outbound {
                self.with_property_data_mut(*outbound_id, |property_dependent| {
                    property_dependent.inbound.retain(|s| s != &id);
                });
            }
        });
    }

    /// Removes id from it's dependencies
    /// NOTE: does NOT modify the inbound list
    pub fn disconnect_inbound(&self, id: PropertyId) {
        self.with_property_data_mut(id, |property_data| {
            for inbound_id in &property_data.inbound {
                self.with_property_data_mut(*inbound_id, |property_dependency| {
                    property_dependency.outbound.retain(|s| s != &id);
                });
            }
        });
    }

    /// Adds it's own PropertyId to the outbound list
    /// of it's dependencies, letting them know to dirty
    /// it if it changes
    /// NOTE: does NOT modify the inbound list of self (id), only
    /// uses it to hook up dependencies
    pub fn connect_inbound(&self, id: PropertyId) {
        self.with_property_data(id, |property_data| {
            for inbound_id in &property_data.inbound {
                self.with_property_data_mut(*inbound_id, |property_dependency| {
                    property_dependency.outbound.push(id);
                });
            }
        });
    }
    fn find_affected_properties(
        &self,
        changed_properties: &HashSet<PropertyId>,
    ) -> HashSet<PropertyId> {
        let mut affected = HashSet::new();
        let mut queue = VecDeque::new();

        for prop in changed_properties {
            queue.push_back(*prop);
            affected.insert(*prop);
        }

        while let Some(prop) = queue.pop_front() {
            if let Some(entry) = self.property_map.borrow().get(&prop) {
                if let Some(data) = &entry.data {
                    for &dep in &data.outbound {
                        if affected.insert(dep) {
                            queue.push_back(dep);
                        }
                    }
                }
            }
        }

        affected
    }

    fn topological_sort_affected(
        &self,
        affected: &HashSet<PropertyId>,
    ) -> Result<Vec<PropertyId>, String> {
        let mut in_degree = HashMap::new();
        let mut sorted = Vec::new();
        let mut queue = VecDeque::new();

        for &prop in affected {
            if let Some(entry) = self.property_map.borrow().get(&prop) {
                if let Some(data) = &entry.data {
                    *in_degree.entry(prop).or_insert(0) += data.inbound.len();
                }
            }
        }
        for (prop, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(*prop);
            }
        }

        while let Some(prop) = queue.pop_front() {
            sorted.push(prop);

            if let Some(entry) = self.property_map.borrow().get(&prop) {
                if let Some(data) = &entry.data {
                    for &dep in &data.outbound {
                        if affected.contains(&dep) {
                            if let Some(degree) = in_degree.get_mut(&dep) {
                                *degree -= 1;
                                if *degree == 0 {
                                    queue.push_back(dep);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(sorted)
        // if sorted.len() == affected.len() {
        //     Ok(sorted)
        // } else {
        //     Err(format!("Subgraph has cycles: sorted len is {:?} and affected len is {:?}", sorted.len(), affected.len()))
        // }
    }


    pub fn update_affected(&self) {
        let enqueued_sets: Vec<PropertyId> = self.enqueued_sets.borrow_mut().drain(..).collect();
        let changed_properties: HashSet<PropertyId> = enqueued_sets.into_iter().collect();

        let affected_properties = self.find_affected_properties(&changed_properties);

        match self.topological_sort_affected(&affected_properties) {
            Ok(order) => {
                for id in order {
                    log::warn!("starting update property: {:?}", id);
                    self.update_value(id);
                    log::warn!("finished update property: {:?}", id);
                }
            }
            Err(e) => {
                log::warn!("Error: {}", e);
            }
        }
        log::warn!("finished update_affected");
    }
}
