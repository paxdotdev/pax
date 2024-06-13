use std::{
    any::Any,
    cell::{Ref, RefCell},
    collections::HashMap,
    rc::Rc,
};

use property_table::{PropertyType, SubscriptionId, GET_STATISTICS, PROPERTY_TABLE, PROPERTY_TIME};

mod graph;
pub mod property_id;
mod property_table;
pub mod transitions;

pub use property_id::PropertyId;
use serde::{Deserialize, Serialize};
use transitions::{EasingCurve, Interpolatable, TransitionQueueEntry};
pub trait PropertyValue: Default + Clone + 'static {}
impl<T: Default + Clone + 'static> PropertyValue for T {}

#[derive(Clone, Copy)]
pub struct Property<T> {
    id: PropertyId,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> std::fmt::Debug for Property<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Property").field("id", &self.id).finish()
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

impl<T: PropertyValue + Interpolatable> Property<T> {
    /// Eases the property to a new value over a given time right away
    pub fn ease_to(&self, end_val: T, time: u64, curve: EasingCurve) {
        self.ease_to_value(end_val, time, curve, true);
    }

    /// Eases the property to new value over time after queued transitions have finished
    pub fn ease_to_later(&self, end_val: T, time: u64, curve: EasingCurve) {
        self.ease_to_value(end_val, time, curve, false);
    }

    fn ease_to_value(&self, end_val: T, time: u64, curve: EasingCurve, overwrite: bool) {
        PROPERTY_TABLE.with(|t| {
            t.transition(
                self.id,
                TransitionQueueEntry {
                    duration_frames: time,
                    curve,
                    ending_value: end_val,
                },
                overwrite,
            )
        })
    }
}

impl<T: PropertyValue> Property<T> {
    pub fn new(val: T) -> Self {
        Self {
            id: PROPERTY_TABLE.with(|t| t.insert(PropertyType::Literal, val.clone(), Vec::new())),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn expression(evaluator: impl Fn() -> T + 'static, dependents: &[PropertyId]) -> Self {
        let inbound = dependents.to_vec();
        let start_val = evaluator();
        let evaluator = Rc::new(generate_untyped_closure(evaluator));
        Self {
            id: PROPERTY_TABLE.with(|t| {
                t.insert(
                    PropertyType::Expression { evaluator },
                    start_val.clone(),
                    inbound,
                )
            }),
            _phantom: std::marker::PhantomData,
        }
    }

    pub(crate) fn time() -> Self {
        Self {
            id: PROPERTY_TABLE.with(|t| {
                t.insert(
                    PropertyType::Time {
                        transitioning: HashMap::new(),
                    },
                    0,
                    Vec::new(),
                )
            }),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Gets the value of the property
    pub fn get(&self) -> T {
        PROPERTY_TABLE.with(|t| t.get(self.id))
    }

    /// Sets the value of the property and runs relevant subscriptions
    pub fn set(&self, val: T) {
        PROPERTY_TABLE.with(|t| t.set(self.id, val));
    }

    /// Adds a callback to be run when the property is set
    pub fn subscribe(&self, sub: impl Fn() + 'static) -> SubscriptionId {
        PROPERTY_TABLE.with(|t| t.subscribe(self.id, Rc::new(sub)))
    }

    /// Removes a subscription
    pub fn unsubscribe(&self, sub_id: SubscriptionId) {
        PROPERTY_TABLE.with(|t| t.unsubscribe(self.id, sub_id))
    }

    pub fn get_id(&self) -> PropertyId {
        self.id
    }

    pub fn new_from_id(id: PropertyId) -> Self {
        Self {
            id,
            _phantom: std::marker::PhantomData,
        }
    }
}

fn generate_untyped_closure<T: 'static + Any>(
    evaluator: impl Fn() -> T + 'static,
) -> impl Fn() -> Box<dyn Any> {
    move || Box::new(evaluator()) as Box<dyn Any>
}

pub fn print_graph() {
    PROPERTY_TABLE.with(|t| t.render_graph_to_file("/Users/warfa/Documents/Projects/Pax/paxcorp/pax/examples/src/fireworks/graph.svg"));
}

pub fn set_time(frames_elapsed: u64) {
    PROPERTY_TABLE.with(|t| t.cleanup_finished_transitions());
    PROPERTY_TIME.with(|t| t.borrow().set(frames_elapsed));
    // if frames_elapsed % 1 == 0 {
    //     log::info!("Time: {}", frames_elapsed);
    //     GET_STATISTICS.with(|s| s.borrow_mut().print_stats());
    // }
}

fn get_time() -> u64 {
    PROPERTY_TIME.with(|t| t.borrow().get())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expression() {
        let p1 = Property::new(5);
        let p2 = Property::expression(move || 2 * p1.get(), &[p1.get_id()]);
        assert_eq!(p2.get(), 10);
        p1.set(10);
        assert_eq!(p2.get(), 20);
    }

    #[test]
    fn test_single_subscription() {
        let p1 = Property::new(0);
        let p2 = Property::new(0);
        p1.subscribe(move || {
            p2.set(p1.get());
        });
        p1.set(5);
        assert_eq!(p2.get(), 5);
    }

    #[test]
    fn test_expression_subscription() {
        let p1 = Property::new(5);
        let p2 = Property::expression(move || 2 * p1.get(), &[p1.get_id()]);
        let p3 = Property::expression(move || 3 * p1.get(), &[p1.get_id()]);
        let p4 = Property::expression(move || p2.get() + p3.get(), &[p2.get_id(), p3.get_id()]);
        let p5 = Property::new(0);
        let _ = p4.subscribe(move || p5.set(p4.get()));
        assert_eq!(p5.get(), 0);
        p1.set(10);
        assert_eq!(p5.get(), 50);
        p1.set(100);
        assert_eq!(p5.get(), 500);
    }

    #[test]
    fn test_unsubscribe() {
        let p1 = Property::new(5);
        let p2 = Property::expression(move || 2 * p1.get(), &[p1.get_id()]);
        let p3 = Property::expression(move || 3 * p1.get(), &[p1.get_id()]);
        let p4 = Property::expression(move || p2.get() + p3.get(), &[p2.get_id(), p3.get_id()]);
        let p5 = Property::new(0);
        let sub_id = p4.subscribe(move || p5.set(p4.get()));
        assert_eq!(p5.get(), 0);
        p1.set(10);
        assert_eq!(p5.get(), 50);
        p4.unsubscribe(sub_id);
        p1.set(100);
        assert_eq!(p5.get(), 50);
    }

    #[test]
    fn test_ease_to() {
        set_time(0);
        let p1 = Property::new(5);
        assert_eq!(p1.get(), 5);
        p1.ease_to(10, 5, EasingCurve::Linear);
        assert_eq!(p1.get(), 5);
        set_time(1);
        assert_eq!(get_time(), 1);
        print_graph();
        assert_eq!(p1.get(), 6);
        set_time(5);
        assert_eq!(p1.get(), 10);
    }

    #[test]
    fn test_ease_to_later() {
        set_time(0);
        let p1 = Property::new(5);
        assert_eq!(p1.get(), 5);
        p1.ease_to(10, 5, EasingCurve::Linear);
        p1.ease_to_later(20, 10, EasingCurve::Linear);
        assert_eq!(p1.get(), 5);
        set_time(1);
        assert_eq!(p1.get(), 6);
        set_time(5);
        assert_eq!(p1.get(), 10);
        set_time(6);
        assert_eq!(p1.get(), 11);
        set_time(15);
        assert_eq!(p1.get(), 20);
        set_time(17);
        assert_eq!(p1.get(), 20);
        set_time(20);
    }
}
