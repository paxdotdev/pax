use std::{any::Any, marker::PhantomData};

use crate::Property;

use super::{
    private::PropertyId,
    properties_table::{PropertyType, PROPERTY_TABLE},
    PropertyValue,
};

/// Reactive property type. Shallow clones can be cheaply made.
#[derive(Debug)]
pub struct UntypedProperty {
    pub id: PropertyId,
}

impl Clone for UntypedProperty {
    fn clone(&self) -> Self {
        PROPERTY_TABLE.with(|t| {
            t.with_ref_count_mut(self.id, |ref_count| {
                *ref_count += 1;
            })
        });
        UntypedProperty { id: self.id }
    }
}

impl Drop for UntypedProperty {
    fn drop(&mut self) {
        PROPERTY_TABLE.with(|t| {
            let ref_count = t.with_ref_count_mut(self.id, |ref_count| {
                *ref_count -= 1;
                *ref_count
            });
            if ref_count == 0 {
                t.remove_entry(self.id);
            }
        });
    }
}

impl UntypedProperty {
    pub fn new(val: Box<dyn Any>, data: PropertyType, debug_name: Option<&str>) -> Self {
        UntypedProperty {
            id: PROPERTY_TABLE.with(|t| t.add_entry(val, data, debug_name)),
        }
    }

    pub fn as_typed<T: PropertyValue>(&self) -> Property<T> {
        Property {
            untyped: self.clone(),
            _phantom: PhantomData,
        }
    }
    pub fn get_id(&self) -> PropertyId {
        self.id
    }
}
