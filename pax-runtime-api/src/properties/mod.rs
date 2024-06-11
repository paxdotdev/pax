use serde::{Deserialize, Serialize};
use std::{any::Any, cell::{Ref, RefCell}, marker::PhantomData, rc::Rc, sync::atomic::AtomicI64};

mod graph_operations;
mod properties_table;
// #[cfg(test)]
// mod tests;
mod untyped_property;

use crate::{EasingCurve, Interpolatable, TransitionQueueEntry};

use self::properties_table::{PropertyType, PROPERTY_TIME};
use properties_table::PROPERTY_TABLE;
pub use untyped_property::UntypedProperty;


/// PropertyValue represents a restriction on valid generic types that a property
/// can contain. All T need to be Clone (to enable .get()) + 'static (no
/// references/ lifetimes)
pub trait PropertyValue: Default + Clone + Interpolatable + 'static {}
impl<T: Default + Clone + Interpolatable + 'static> PropertyValue for T {}

impl<T: PropertyValue> Interpolatable for Property<T> {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        let cp_self = self.clone();
        let cp_other = other.clone();
        Property::computed(
            move || cp_self.get().interpolate(&cp_other.get(), t),
            &[self.untyped(), other.untyped()],
        )
    }
}

pub struct CachedData<T> {
    _cached_version: u64,
    _cached_value: T,
}

/// A typed wrapper over a UntypedProperty that casts to/from an untyped
/// property on get/set
#[derive(Clone)]
pub struct Property<T> {
    untyped: UntypedProperty,
    _cached_data : Rc<RefCell<CachedData<T>>>,
}

impl<T> std::fmt::Debug for Property<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Property")
            .field("untyped", &self.untyped)
            .finish()
    }
}

impl<T: PropertyValue> Property<T> {
    pub fn new(val: T) -> Self {
        Self {
            untyped: UntypedProperty::new(val.clone(), Vec::with_capacity(0), PropertyType::Literal),
            _cached_data: Rc::new(RefCell::new(CachedData {
                _cached_version: 0,
                _cached_value: val,
            })),
        }
    }

    pub fn computed(evaluator: impl Fn() -> T + 'static, dependents: &[UntypedProperty]) -> Self {
        let inbound: Vec<_> = dependents.iter().map(|v| v.get_id()).collect();
        let start_val = T::default();
        let evaluator = Rc::new(generate_untyped_closure(evaluator));
        Self {
            untyped: UntypedProperty::new(
                start_val.clone(),
                inbound,
                PropertyType::Computed { evaluator },
            ),
            _cached_data: Rc::new(RefCell::new(CachedData {
                _cached_version: 0,
                _cached_value: start_val,
            })),
        }
    }

    pub fn ease_to(&self, end_val: T, time: u64, curve: EasingCurve) {
        self.ease_to_value(end_val, time, curve, true);
    }

    pub fn ease_to_later(&self, end_val: T, time: u64, curve: EasingCurve) {
        self.ease_to_value(end_val, time, curve, false);
    }

    fn ease_to_value(&self, end_val: T, time: u64, curve: EasingCurve, overwrite: bool) {
        PROPERTY_TABLE.with(|t| {
            t.transition(
                self.untyped.id,
                TransitionQueueEntry {
                    duration_frames: time,
                    curve,
                    ending_value: end_val,
                },
                overwrite,
            )
        })
    }

   
    pub fn get(&self) -> T {
        PROPERTY_TABLE.with(|t| t.get_value(self.untyped.id))
    }


    /// Gets the currently stored value. Only clones from table if the version
    /// has changed since last retrieved
    // pub fn get(&self) -> Ref<T> {
    //     let current_version = PROPERTY_TABLE.with(|t| t.get_version(self.untyped.id));
    //     {
    //         let cached_data = self._cached_data.borrow();
    //         if cached_data._cached_version != current_version {
    //             let mut cached_data = self._cached_data.borrow_mut();
    //             let new_val = PROPERTY_TABLE.with(|t| t.get_value(self.untyped.id));
    //             cached_data._cached_value = new_val;
    //             cached_data._cached_version = current_version;
    //         }
    //     }
    //     Ref::map(self._cached_data.borrow(), |cached_data| &cached_data._cached_value)
    // }

    /// Sets this properties value and sets the drity bit recursively of all of
    /// it's dependencies if not already set
    pub fn set(&self, val: T) {
        PROPERTY_TABLE.with(|t| t.set_value(self.untyped.id, val));
    }


    /// replaces a properties evaluation/inbounds/value to be the same as
    /// target, while keeping it's dependents.
    /// WARNING: this method can introduce circular dependencies if one is not careful.
    /// Using it wrongly can introduce memory leaks and inconsistent property behaviour.
    /// This method can be used to replace an inner value from for example a literal to
    /// a computed computed, while keeping the link to it's dependents
    pub fn replace_with(&self, target: Property<T>) {
        PROPERTY_TABLE.with(|t| {
            // we know self contains T, and that target contains T, so this should never panic
            t.replace_property_keep_outbound_connections::<T>(self.untyped.id, target.untyped.id)
        })
    }

    /// Casts this property to it's untyped version
    pub fn untyped(&self) -> UntypedProperty {
        self.untyped.clone()
    }
}

impl<T: PropertyValue> Default for Property<T> {
    fn default() -> Self {
        Property::new(T::default())
    }
}

/// Serialization and deserialization fully disconnects properties,
/// and only loads them back in as literals.
impl<'de, T: PropertyValue + Deserialize<'de>> Deserialize<'de> for Property<T> {
    fn deserialize<D>(deserializer: D) -> Result<Property<T>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = T::deserialize(deserializer)?;
        Ok(Property::new(value))
    }
}

impl<T: PropertyValue + Serialize> Serialize for Property<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // TODO check if literal or computed, error on computed?
        self.get().serialize(serializer)
    }
}

/// Utility method to inspect total entry count in property table
pub fn property_table_total_properties_count() -> usize {
    PROPERTY_TABLE.with(|t| t.total_properties_count())
}

pub fn register_time(prop: &Property<u64>) {
    PROPERTY_TIME.with_borrow_mut(|time| *time = prop.clone());
}

pub fn update_properties() {
    PROPERTY_TABLE.with(|t| t.update_affected());
    log::warn!("updated properties");
}

fn generate_untyped_closure<T: 'static + Any>(evaluator: impl Fn() -> T + 'static) -> impl Fn() -> Box<dyn Any> {
    move || Box::new(evaluator()) as Box<dyn Any>
}
