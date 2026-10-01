use serde::{Deserialize, Serialize};
use std::{marker::PhantomData, rc::Rc};

mod graph_operations;
mod local_property;
mod properties_table;
mod shared_graph;
mod shared_property;
#[cfg(test)]
mod shared_tests;
#[cfg(test)]
mod tests;
mod untyped_property;

use crate::{Duration, EasingCurve, Interpolatable, TransitionQueueEntry};

use self::properties_table::{PropertyType, PROPERTY_MILLIS, PROPERTY_TIME};
pub use local_property::LocalProperty;
pub use properties_table::EffectDrainReport;
use properties_table::PROPERTY_TABLE;
#[doc(hidden)]
pub use shared_graph::{BindingScope, BindingScopeGuard};
pub use shared_graph::{PropertyGraph, PropertyGraphGuard};
pub use shared_property::{Property, Published, SharedPropertyValue};
pub use untyped_property::UntypedProperty;

/// Sealed PropertyId needed for slotmap (strictly internal)
mod private {
    slotmap::new_key_type!(
        pub struct PropertyId;
    );
}

/// Value operations shared by local and transferable properties.
///
/// Values must be cloneable for `.get()`, interpolatable for transitions, and
/// `'static` because properties are stored in the runtime graph.
pub trait PropertyValue: Default + Clone + Interpolatable + 'static {}
impl<T: Default + Clone + Interpolatable + 'static> PropertyValue for T {}

/// Owner-thread binding operations used by generated component factories.
///
/// Shared fields publish values while their evaluators stay in the entered
/// [`PropertyGraph`]. Local fields retain ordinary lazy graph semantics.
pub trait PropertyBinding<T: PropertyValue>: Sized {
    /// Creates independent literal state of this field's ownership kind.
    fn from_value(value: T) -> Self;
    /// Obtains the owner-thread view used by generated bindings.
    fn local(&self) -> LocalProperty<T>;
    /// Aliases a local binding, preserving a shared source when one exists.
    fn from_local(source: LocalProperty<T>) -> Self;
    /// Reads a field for conversion, using local snapshots during an explicit
    /// graph conversion and ordinary value semantics otherwise.
    #[doc(hidden)]
    fn value_for_conversion(&self) -> T;
}

thread_local! {
    static GRAPH_CONVERSION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(crate) fn with_graph_conversion<R>(f: impl FnOnce() -> R) -> R {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            GRAPH_CONVERSION.with(|active| active.set(self.0));
        }
    }
    let _restore = Restore(GRAPH_CONVERSION.with(|active| active.replace(true)));
    f()
}

// Utility method to inspect total entry count in property table.
pub fn property_table_total_properties_count() -> usize {
    PROPERTY_TABLE.with(|t| t.total_properties_count())
}

#[doc(hidden)]
pub fn register_effect_property(prop: &LocalProperty<()>) {
    PROPERTY_TABLE.with(|t| t.register_effect(prop.untyped.id));
}

#[doc(hidden)]
pub fn register_effect_property_with_name(prop: &LocalProperty<()>, debug_name: &str) {
    PROPERTY_TABLE.with(|t| t.register_effect_with_name(prop.untyped.id, Some(debug_name)));
}

#[doc(hidden)]
pub fn drain_effects(max_iterations: usize) -> usize {
    PROPERTY_TABLE.with(|t| t.drain_effects(max_iterations))
}

#[doc(hidden)]
pub fn drain_effects_with_report(max_iterations: usize) -> EffectDrainReport {
    PROPERTY_TABLE.with(|t| t.drain_effects_with_report(max_iterations))
}

#[doc(hidden)]
pub fn property_outbound_debug_names(prop: &UntypedProperty) -> Vec<String> {
    PROPERTY_TABLE.with(|t| t.outbound_debug_names(prop.id))
}

#[doc(hidden)]
pub fn property_has_direct_outbound(source: &UntypedProperty, outbound: &UntypedProperty) -> bool {
    PROPERTY_TABLE.with(|t| t.has_direct_outbound(source.id, outbound.id))
}

// Registers the runtime clock property used by transition/easing machinery.
pub fn register_time(prop: &LocalProperty<u64>) {
    PROPERTY_TIME.with_borrow_mut(|time| *time = prop.clone());
}

// Registers the runtime wall clock property used by time-based transition/easing machinery.
pub fn register_millis(prop: &LocalProperty<u64>) {
    PROPERTY_MILLIS.with_borrow_mut(|time| *time = prop.clone());
}

/// Current runtime frame used to distinguish initial binding from later changes.
#[doc(hidden)]
pub fn current_frame() -> u64 {
    PROPERTY_TIME.with_borrow(|time| time.get())
}
