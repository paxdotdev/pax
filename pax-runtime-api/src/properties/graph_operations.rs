use super::{
    private::PropertyId,
    properties_table::{PropertyTable, PropertyType},
};

impl PropertyTable {
    /// marks dependencies of self dirty recursively
    pub fn dirtify_outbound(&self, id: PropertyId) {
        let mut to_dirtify =
            self.with_property_data_mut(id, |prop_data| prop_data.outbound.clone());

        while let Some(dep_id) = to_dirtify.pop() {
            self.with_property_data_mut(dep_id, |dep_data| {
                if dep_id == id {
                    unreachable!("property cycle");
                }
                let PropertyType::Computed { ref mut dirty, .. } = dep_data.property_type else {
                    unreachable!("literal depends on property")
                };
                if !*dirty {
                    *dirty = true;
                    to_dirtify.extend_from_slice(&dep_data.outbound);
                }
            });
        }
    }

    /// Removes id from it's dependencies
    /// NOTE: does NOT modify the inbound list
    pub fn disconnect_inbound(&self, id: PropertyId) {
        self.with_property_data_mut(id, |property_data| {
            if let PropertyType::Computed { inbound, .. } = &mut property_data.property_type {
                for subscription in inbound {
                    self.with_property_data_mut(*subscription, |sub| {
                        sub.outbound.retain(|s| s != &id);
                    });
                }
            }
        });
    }

    /// Removes id from it's dependencies
    /// NOTE: does NOT modify the outbound list
    pub fn disconnect_outbound(&self, id: PropertyId) {
        self.with_property_data(id, |property_data| {
            for sub_id in &property_data.outbound {
                self.with_property_data_mut(*sub_id, |subscriber| {
                    if let PropertyType::Computed {
                        inbound: subscriptions,
                        ..
                    } = &mut subscriber.property_type
                    {
                        subscriptions.retain(|s| s != &id);
                    }
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
        self.with_property_data(id, |prop_data| {
            if let PropertyType::Computed { inbound, .. } = &prop_data.property_type {
                for dep_id in inbound {
                    self.with_property_data_mut(*dep_id, |dep_prop| {
                        dep_prop.outbound.push(id);
                    });
                }
            }
        });
    }
}
