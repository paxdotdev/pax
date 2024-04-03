use std::{any::Any, marker::PhantomData};

use crate::Property;

use super::{private::PropertyId, properties_table::{PropertyType, PROPERTY_TABLE}, PropertyValue};

/// Reactive property type. Shallow clones can be cheaply made.
#[derive(Debug)]
pub struct ErasedProperty {
    pub id: PropertyId,
}

impl Clone for ErasedProperty {
    fn clone(&self) -> Self {
        PROPERTY_TABLE.with(|t| {
            t.with_prop_data_mut(self.id, |prop_data| {
                prop_data.ref_count += 1;
            })
            .map_err(|e| format!("couldn't increase ref count: {}", e))
            .unwrap();
        });
        ErasedProperty { id: self.id }
    }
}

impl Drop for ErasedProperty {
    fn drop(&mut self) {
        PROPERTY_TABLE.with(|t| {
            let ref_count = t
                .with_prop_data_mut(self.id, |prop_data| {
                    prop_data.ref_count -= 1;
                    prop_data.ref_count
                })
                .map_err(|e| format!("coun't decrease ref count: {}", e))
                .unwrap();
            if ref_count == 0 {
                t.remove_entry(self.id);
            }
        });
    }
}


impl ErasedProperty {
    pub fn new(val: Box<dyn Any>, data: PropertyType, debug_name: Option<&str>) -> Self {
        ErasedProperty {
            id: PROPERTY_TABLE.with(|t| t.add_entry(val, data, debug_name)),
        }
    }

    pub fn get<T: PropertyValue>(&self) -> Property<T> {
        Property {
            erased: self.clone(),
            _phantom: PhantomData,
        }
    }
    pub fn get_id(&self) -> PropertyId {
        self.id
    }
}