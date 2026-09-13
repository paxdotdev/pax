use super::{
    private::PropertyId,
    properties_table::{PropertyType, PROPERTY_TABLE},
    PropertyValue,
};

/// Reactive untyped property. Shallow clones can be cheaply made. Manages
/// refcounting and deletion of underlying data when all instances with a
/// specific PropertyId has been dropped.
#[derive(Debug)]
pub struct UntypedProperty {
    // Slotmap id for the backing property table entry.
    pub(crate) id: PropertyId,
}

impl Clone for UntypedProperty {
    fn clone(&self) -> Self {
        PROPERTY_TABLE.with(|t| {
            t.increase_ref_count(self.id);
        });
        UntypedProperty { id: self.id }
    }
}

impl Drop for UntypedProperty {
    fn drop(&mut self) {
        // Process shutdown can drop properties while the thread-local property table
        // itself is being destroyed. In that phase, bookkeeping is no longer useful
        // and re-entering the TLS would panic.
        let _ = PROPERTY_TABLE.try_with(|t| {
            let ref_count = t.decrease_ref_count(self.id);
            if ref_count == 0 {
                t.remove_entry(self.id);
            }
        });
    }
}

impl UntypedProperty {
    // Allocates a new table entry and returns its untyped handle.
    pub(crate) fn new<T: PropertyValue>(
        val: T,
        inbound: Vec<PropertyId>,
        data: PropertyType<T>,
        debug_name: Option<&str>,
    ) -> Self {
        UntypedProperty {
            id: PROPERTY_TABLE.with(|t| t.add_entry(val, inbound, data, debug_name)),
        }
    }

    // Returns the table id backing this untyped property handle.
    pub fn get_id(&self) -> PropertyId {
        self.id
    }

    // Inspect storage as well as the adapter's declared type before rewrapping.
    pub(crate) fn has_value_type<T: PropertyValue>(&self) -> bool {
        PROPERTY_TABLE.with(|table| table.has_value_type::<T>(self.id))
    }
}
