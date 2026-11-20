use super::{
    private::PropertyId,
    properties_table::{PropertyType, PROPERTY_TABLE},
    PropertyValue,
};

/// Reactive untyped property. Shallow clones can be cheaply made. Manages
/// refcounting and deletion of underlying data when all instances with a
/// specific PropertyId has been dropped.
///
/// Untyped graph handles retain their owner-thread restriction:
///
/// ```compile_fail
/// use pax_runtime_api::LocalProperty;
/// let value = LocalProperty::new(1_u32).untyped();
/// std::thread::spawn(move || drop(value));
/// ```
///
/// A lock cannot make a thread-local graph handle transferable:
///
/// ```compile_fail
/// use pax_runtime_api::LocalProperty;
/// use std::sync::{Arc, Mutex};
/// let value = Arc::new(Mutex::new(LocalProperty::new(1_u32)));
/// std::thread::spawn(move || value.lock().unwrap().get());
/// ```
#[derive(Debug)]
pub struct UntypedProperty {
    // Slotmap id for the backing property table entry.
    pub(crate) id: PropertyId,
    // The key is meaningful only in the creating thread's property table.
    owner: std::marker::PhantomData<std::rc::Rc<()>>,
}

/// Callback-free notifications for consumers that resolve changed dependencies
/// cooperatively. A notification is not a settled-value witness: consumers must
/// revalidate their versions before publishing. Dropping a subscription removes
/// its graph ownership without running an evaluator or retaining a property.
#[doc(hidden)]
#[derive(Default)]
pub struct DependencyEventQueue(
    std::rc::Rc<
        std::cell::RefCell<std::collections::VecDeque<(u64, std::rc::Weak<DependencyEventInner>)>>,
    >,
);

pub(super) struct DependencyEventInner {
    queue: std::rc::Weak<
        std::cell::RefCell<std::collections::VecDeque<(u64, std::rc::Weak<DependencyEventInner>)>>,
    >,
    key: u64,
    queued: std::cell::Cell<bool>,
}

// A shared viewport property can have thousands of readers. Compact on an
// amortized schedule so registering them costs linear work and a property
// that never changes can still release retired registrations.
#[derive(Default)]
pub(super) struct DependencyReaders {
    entries: Vec<std::rc::Weak<DependencyEventInner>>,
    until_compaction: usize,
}

impl DependencyReaders {
    fn register(&mut self, reader: &std::rc::Rc<DependencyEventInner>) {
        if self.until_compaction == 0 {
            self.entries.retain(|entry| entry.strong_count() != 0);
            self.until_compaction = self.entries.len().max(8);
        }
        self.until_compaction -= 1;
        self.entries.push(std::rc::Rc::downgrade(reader));
    }

    pub(super) fn notify(&mut self) {
        self.entries.retain(|entry| {
            entry.upgrade().is_some_and(|reader| {
                reader.invalidate();
                true
            })
        });
        self.until_compaction = self.entries.len().max(8);
    }
}

#[doc(hidden)]
pub struct DependencyEventSubscription {
    inner: std::rc::Rc<DependencyEventInner>,
    // Generational IDs do not keep captured properties alive.
    sources: Vec<PropertyId>,
}

impl DependencyEventInner {
    pub(super) fn invalidate(self: &std::rc::Rc<Self>) {
        if let Some(queue) = self.queue.upgrade() {
            if !self.queued.replace(true) {
                queue
                    .borrow_mut()
                    .push_back((self.key, std::rc::Rc::downgrade(self)));
            }
        }
    }
}

impl DependencyEventSubscription {
    fn source_ids(properties: &[UntypedProperty]) -> Vec<PropertyId> {
        let mut ids: Vec<_> = properties.iter().map(UntypedProperty::get_id).collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    fn register(inner: DependencyEventInner, sources: Vec<PropertyId>) -> Self {
        let subscription = std::rc::Rc::new(inner);
        PROPERTY_TABLE.with(|table| {
            for &id in &sources {
                table.with_property_data_mut(id, |data| {
                    let events = data
                        .invalidation_events
                        .get_or_insert_with(Default::default);
                    events.register(&subscription);
                    if data.dirty {
                        subscription.invalidate();
                    }
                });
            }
        });
        Self {
            inner: subscription,
            sources,
        }
    }

    /// Refresh a capture's reads, preserving the registration for an equal set.
    pub fn refresh(&mut self, properties: &[UntypedProperty]) {
        let sources = Self::source_ids(properties);
        if sources == self.sources {
            // A consumer can drain its notification while a lazy input is
            // still dirty. Preserve the same wake-up as a fresh registration.
            if !self.inner.queued.get()
                && PROPERTY_TABLE.with(|table| {
                    sources
                        .iter()
                        .any(|&id| table.with_property_data(id, |data| data.dirty))
                })
            {
                self.inner.invalidate();
            }
        } else {
            *self = Self::register(
                DependencyEventInner {
                    queue: self.inner.queue.clone(),
                    key: self.inner.key,
                    queued: std::cell::Cell::new(false),
                },
                sources,
            );
        }
    }
}

impl DependencyEventQueue {
    pub fn watch(&self, properties: &[UntypedProperty], key: u64) -> DependencyEventSubscription {
        DependencyEventSubscription::register(
            DependencyEventInner {
                queue: std::rc::Rc::downgrade(&self.0),
                key,
                queued: std::cell::Cell::new(false),
            },
            DependencyEventSubscription::source_ids(properties),
        )
    }
    /// Pops one bounded work unit. A retired subscription can leave a harmless
    /// stale key; the consumer separately validates mounted/source identity.
    pub fn pop(&self) -> Option<u64> {
        let (key, subscription) = self.0.borrow_mut().pop_front()?;
        if let Some(subscription) = subscription.upgrade() {
            subscription.queued.set(false);
        }
        Some(key)
    }
    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }
}

impl Clone for UntypedProperty {
    fn clone(&self) -> Self {
        PROPERTY_TABLE.with(|t| {
            t.increase_ref_count(self.id);
        });
        UntypedProperty {
            id: self.id,
            owner: std::marker::PhantomData,
        }
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
    pub(super) fn try_from_id(id: PropertyId) -> Option<Self> {
        PROPERTY_TABLE.with(|table| {
            if !table.has_live_entry(id) {
                return None;
            }
            table.increase_ref_count(id);
            Some(Self {
                id,
                owner: std::marker::PhantomData,
            })
        })
    }
    // Allocates a new table entry and returns its untyped handle.
    pub(crate) fn new<T: PropertyValue>(
        val: T,
        inbound: Vec<PropertyId>,
        data: PropertyType<T>,
        debug_name: Option<&str>,
    ) -> Self {
        UntypedProperty {
            id: PROPERTY_TABLE.with(|t| t.add_entry(val, inbound, data, debug_name)),
            owner: std::marker::PhantomData,
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
