use super::{private::PropertyId, properties_table::PropertyTable};

impl PropertyTable {
    // marks dependencies of self dirty recursively
    pub fn dirtify_outbound(&self, id: PropertyId) {
        let mut to_dirtify = self.with_property_data_mut(id, |data| {
            data.notify_readers();
            data.outbound.clone()
        });

        while let Some(dep_id) = to_dirtify.pop() {
            let (newly_dirty, is_cutoff) = self.with_property_data_mut(dep_id, |dep_data| {
                if dep_id == id {
                    unreachable!("property cycle");
                }
                if !dep_data.dirty {
                    dep_data.dirty = true;
                    dep_data.notify_readers();
                    let is_cutoff = dep_data.cutoff_settler.is_some();
                    if !is_cutoff {
                        to_dirtify.extend_from_slice(&dep_data.outbound);
                    }
                    (true, is_cutoff)
                } else {
                    (false, dep_data.cutoff_settler.is_some())
                }
            });
            if newly_dirty {
                if is_cutoff {
                    self.enqueue_cutoff(dep_id);
                } else {
                    self.enqueue_effect_if_registered(dep_id);
                }
            }
        }
    }

    // Removes id from its dependents
    // NOTE: does NOT modify the outbound list
    pub fn disconnect_outbound(&self, id: PropertyId) {
        self.with_property_data(id, |property_data| {
            for outbound_id in &property_data.outbound {
                self.with_property_data_mut(*outbound_id, |property_dependent| {
                    property_dependent.inbound.retain(|s| s != &id);
                });
            }
        });
    }

    // Removes id from its dependencies
    // NOTE: does NOT modify the inbound list
    pub fn disconnect_inbound(&self, id: PropertyId) {
        self.with_property_data_mut(id, |property_data| {
            for &source in &property_data.inbound {
                self.disconnect_edge(source, id);
            }
        });
    }

    fn disconnect_edge(&self, source: PropertyId, dependent: PropertyId) {
        self.with_property_data_mut(source, |data| {
            // Remove one edge per inbound occurrence, preserving scheduling order.
            // Short-lived handler contexts usually own the last edge.
            if let Some(index) = data
                .outbound
                .iter()
                .rposition(|&output| output == dependent)
            {
                data.outbound.remove(index);
            }
        });
    }

    fn connect_edge(&self, source: PropertyId, dependent: PropertyId) {
        let queue_cutoff = self.with_property_data_mut(source, |data| {
            data.outbound.push(dependent);
            data.dirty && data.cutoff_settler.is_some()
        });
        if queue_cutoff {
            self.enqueue_cutoff(source);
        }
    }

    // Only the evaluating slot changes its subscriptions. replace_with can copy
    // the same evaluator into several slots whose active edges differ.
    pub(super) fn set_optional_input(&self, id: PropertyId, source: PropertyId, enabled: bool) {
        self.with_property_data_mut(id, |data| {
            let existing = data.inbound.iter().position(|&input| input == source);
            match (enabled, existing) {
                (true, None) => {
                    data.inbound.push(source);
                    self.connect_edge(source, id);
                }
                (false, Some(index)) => {
                    data.inbound.remove(index);
                    self.disconnect_edge(source, id);
                }
                _ => {}
            }
        });
    }

    // Adds its own PropertyId to the outbound list
    // of its dependencies, letting them know to dirty
    // it if it changes
    // NOTE: does NOT modify the inbound list of self (id), only
    // uses it to hook up dependencies
    pub fn connect_inbound(&self, id: PropertyId) {
        // These operations only edit upstream graph metadata and queue work;
        // they never evaluate user code. Borrow the dependency slice just as
        // disconnect_inbound does, without allocating a temporary ID vector.
        self.with_property_data(id, |property_data| {
            for &inbound_id in &property_data.inbound {
                self.connect_edge(inbound_id, id);
            }
        });
    }
}
