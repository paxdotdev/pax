use super::{
    private::PropertyId,
    properties_table::{PropertyTable, PROPERTY_TIME},
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
                if !dep_data.dirty {
                    dep_data.dirty = true;
                    to_dirtify.extend_from_slice(&dep_data.outbound);
                }
            });
        }
    }

    /// Removes id from it's dependencies
    /// NOTE: does NOT modify the inbound list
    pub fn disconnect_inbound(&self, id: PropertyId) {
        self.with_property_data_mut(id, |property_data| {
            for subscription in &property_data.inbound {
                self.with_property_data_mut(*subscription, |sub| {
                    sub.outbound.retain(|s| s != &id);
                });
            }
        });
    }

    /// Removes id from it's dependents
    /// NOTE: does NOT modify the outbound list
    pub fn disconnect_outbound(&self, id: PropertyId) {
        self.with_property_data(id, |property_data| {
            for sub_id in &property_data.outbound {
                self.with_property_data_mut(*sub_id, |subscriber| {
                    subscriber.inbound.retain(|s| s != &id);
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
            for dep_id in &prop_data.inbound {
                self.with_property_data_mut(*dep_id, |dep_prop| {
                    dep_prop.outbound.push(id);
                });
            }
        });
    }
}
