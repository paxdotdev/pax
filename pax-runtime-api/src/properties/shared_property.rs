use super::{LocalProperty, PropertyBinding, PropertyValue, UntypedProperty};
use crate::Interpolatable;
use serde::{Deserialize, Serialize};
use std::{
    cell::Cell,
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, MutexGuard, Weak,
    },
};

use super::shared_graph::{with_current_graph, GraphInbox};

/// Values that can be published by a shared [`Property`].
///
/// Local computations and values containing `Rc` belong in [`LocalProperty`].
pub trait SharedPropertyValue: PropertyValue + Send + Sync {}
impl<T: PropertyValue + Send + Sync> SharedPropertyValue for T {}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub(super) fn next_id() -> u64 {
    NEXT_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .expect("property identity space exhausted")
}

pub(super) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // Writers build a private draft before committing. A panicking update
    // leaves the published snapshot intact, so poisoning does not lose data.
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

thread_local! {
    static MUTATING: Cell<bool> = const { Cell::new(false) };
}

struct MutationGuard;

impl MutationGuard {
    fn enter() -> Self {
        MUTATING.with(|active| {
            assert!(
                !active.replace(true),
                "property mutation callbacks must not mutate another property recursively"
            );
        });
        Self
    }
}

impl Drop for MutationGuard {
    fn drop(&mut self) {
        MUTATING.with(|active| active.set(false));
    }
}

pub(super) struct Snapshot<T> {
    pub revision: u64,
    pub value: Arc<T>,
}

pub(super) struct SharedCell<T> {
    pub id: u64,
    value: Mutex<Snapshot<T>>,
    writer: Mutex<()>,
    subscribers: Mutex<HashMap<u64, Weak<GraphInbox>>>,
}

impl<T> Drop for SharedCell<T> {
    fn drop(&mut self) {
        // The last shared owner can disappear on a worker. Wake the owner to
        // release its local binding; never destroy an evaluator on this thread.
        let subscribers = self
            .subscribers
            .get_mut()
            .unwrap_or_else(|e| e.into_inner());
        for inbox in subscribers.values().filter_map(Weak::upgrade) {
            inbox.publish(self.id);
        }
    }
}

impl<T: SharedPropertyValue> SharedCell<T> {
    pub fn snapshot(&self) -> Snapshot<T> {
        let snapshot = lock(&self.value);
        Snapshot {
            revision: snapshot.revision,
            value: snapshot.value.clone(),
        }
    }

    fn change(&self, origin: Option<u64>, update: impl FnOnce(&T) -> Option<T>) -> (bool, u64) {
        let mutation = MutationGuard::enter();
        let writer = lock(&self.writer);
        let before = self.snapshot();
        let Some(value) = update(&before.value) else {
            return (false, before.revision);
        };
        let revision = before
            .revision
            .checked_add(1)
            .expect("property revision exhausted");
        let value = Arc::new(value);
        let previous = {
            let mut current = lock(&self.value);
            std::mem::replace(&mut *current, Snapshot { revision, value })
        };
        drop(writer);
        drop(mutation);
        // Value destructors and wake callbacks are arbitrary code. They must
        // run after all publication locks and the mutation guard are released.
        drop(previous);
        drop(before);
        self.notify(origin);
        (true, revision)
    }

    fn notify(&self, origin: Option<u64>) {
        let subscribers: Vec<_> = {
            let mut subscribers = lock(&self.subscribers);
            subscribers.retain(|_, inbox| inbox.strong_count() != 0);
            subscribers
                .iter()
                .filter(|(id, _)| Some(**id) != origin)
                .filter_map(|(_, inbox)| inbox.upgrade())
                .collect()
        };
        for inbox in subscribers {
            inbox.publish(self.id);
        }
    }

    pub fn publish_local(&self, value: T, graph: u64) -> u64 {
        self.change(Some(graph), |_| Some(value)).1
    }

    pub fn subscribe(&self, id: u64, inbox: &Arc<GraphInbox>) {
        lock(&self.subscribers).insert(id, Arc::downgrade(inbox));
    }

    pub fn unsubscribe(&self, id: u64) {
        lock(&self.subscribers).remove(&id);
    }
}

/// Thread-safe application state with snapshot reads and atomic publication.
///
/// Clones share the value, including when moved into a worker or Tokio task.
/// Reading never evaluates a UI expression. Use [`Self::local`] inside an
/// entered [`super::PropertyGraph`] to connect to owner-thread computations.
/// A write publishes immediately; attached graphs observe it on their next
/// import. Intermediate revisions may coalesce. Use a channel for events that
/// must each be delivered.
pub struct Property<T> {
    pub(super) cell: Arc<SharedCell<T>>,
}

impl<T> Clone for Property<T> {
    fn clone(&self) -> Self {
        Self {
            cell: self.cell.clone(),
        }
    }
}

impl<T: SharedPropertyValue> Property<T> {
    /// Creates independent shared state. No UI thread or graph is required.
    pub fn new(value: T) -> Self {
        Self {
            cell: Arc::new(SharedCell {
                id: next_id(),
                value: Mutex::new(Snapshot {
                    revision: 0,
                    value: Arc::new(value),
                }),
                writer: Mutex::new(()),
                subscribers: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Export read-only shared snapshots, including owner-thread computed output.
    pub fn published(&self) -> Published<T> {
        Published(self.clone())
    }

    /// Returns a clone of the latest published value on any thread.
    pub fn get(&self) -> T {
        self.read(Clone::clone)
    }

    /// Reads an immutable snapshot without holding a storage lock during `f`.
    /// Reentrant reads and writes are permitted here.
    pub fn read<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let snapshot = self.cell.snapshot();
        f(&snapshot.value)
    }

    /// Revision imported or published by this thread's entered UI graph.
    /// Returns None if this property has no active attachment in that graph.
    /// This diagnostic never imports data or evaluates a computation.
    pub fn observed_revision(&self) -> Option<u64> {
        with_current_graph(|graph| graph.observed_revision(self))
    }

    /// Atomically publishes a value and requests work from attached graphs.
    pub fn set(&self, value: T) {
        self.cell.change(None, |_| Some(value));
    }

    pub(crate) fn publish_guarded(
        &self,
        value: T,
        authority: &crate::async_runtime::Authority,
    ) -> Result<(), crate::async_runtime::Closed<T>> {
        let mutation = MutationGuard::enter();
        let Some(authority) = authority.acquire() else {
            return Err(crate::async_runtime::Closed(value));
        };
        let writer = lock(&self.cell.writer);
        let previous = {
            let mut current = lock(&self.cell.value);
            let revision = current
                .revision
                .checked_add(1)
                .expect("property revision exhausted");
            std::mem::replace(
                &mut *current,
                Snapshot {
                    revision,
                    value: Arc::new(value),
                },
            )
        };
        drop(writer);
        drop(authority);
        drop(mutation);
        // Revocation and commit have finished before arbitrary drop/wake code.
        drop(previous);
        self.cell.notify(None);
        Ok(())
    }

    /// Updates one property atomically, invoking `f` exactly once.
    ///
    /// The closure edits a private draft. A panic leaves the published value
    /// unchanged. Neither this closure nor value cloning/equality invoked
    /// during mutation may recursively mutate a property; this is diagnosed
    /// with a panic before acquiring another write lock. Snapshot reads are
    /// allowed. Keep callbacks short and never wait for a worker or UI task.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        self.cell.change(None, |old| {
            let mut draft = old.clone();
            f(&mut draft);
            Some(draft)
        });
    }

    /// Compares and publishes under the same single-property write operation.
    pub fn set_if_neq(&self, value: T) -> bool
    where
        T: PartialEq,
    {
        self.cell
            .change(None, |old| (old != &value).then_some(value))
            .0
    }

    /// Monotonic revision of the currently published value.
    pub fn revision(&self) -> u64 {
        self.cell.snapshot().revision
    }

    /// Obtains this graph's local view of the shared value.
    ///
    /// Requires an entered [`super::PropertyGraph`]. The view participates in
    /// lazy computation and two-way binding on its owner thread. Worker writes
    /// become visible to the view when that graph imports publications.
    pub fn local(&self) -> LocalProperty<T> {
        with_current_graph(|graph| graph.attach(self))
    }

    /// Returns a local dependency handle. This handle cannot cross threads.
    pub fn untyped(&self) -> UntypedProperty {
        self.local().untyped()
    }

    /// Installs an owner-thread binding while retaining this shared identity.
    pub fn replace_with(&self, source: LocalProperty<T>) {
        let local = self.local();
        local.replace_with(source);
    }
}

impl<T: SharedPropertyValue> Default for Property<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: SharedPropertyValue + std::fmt::Debug> std::fmt::Debug for Property<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.read(|value| f.debug_tuple("Property").field(value).finish())
    }
}

impl<T: SharedPropertyValue> Interpolatable for Property<T> {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Self::new(self.get().interpolate(&other.get(), t))
    }
}

impl<T: SharedPropertyValue + Serialize> Serialize for Property<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.read(|value| value.serialize(serializer))
    }
}

impl<'de, T: SharedPropertyValue + Deserialize<'de>> Deserialize<'de> for Property<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Self::new)
    }
}

impl<T: SharedPropertyValue> PropertyBinding<T> for Property<T> {
    fn from_value(value: T) -> Self {
        Self::new(value)
    }
    fn local(&self) -> LocalProperty<T> {
        self.local()
    }
    fn from_local(source: LocalProperty<T>) -> Self {
        with_current_graph(|graph| graph.share(source))
    }
    fn value_for_conversion(&self) -> T {
        if super::GRAPH_CONVERSION.with(Cell::get) {
            self.local().get()
        } else {
            self.get()
        }
    }
}

/// A transferable read-only view of published state. Reading never evaluates a
/// UI binding. Its local computation is independent: rebinding or mutating that
/// local result cannot publish into the original shared cell.
///
/// ```compile_fail
/// let published = pax_runtime_api::Property::new(1u32).published();
/// published.set(2);
/// ```
#[derive(Clone)]
pub struct Published<T>(Property<T>);
impl<T: SharedPropertyValue> Published<T> {
    pub fn get(&self) -> T {
        self.0.get()
    }
    pub fn read<R>(&self, callback: impl FnOnce(&T) -> R) -> R {
        self.0.read(callback)
    }
    pub fn revision(&self) -> u64 {
        self.0.revision()
    }
    pub fn local(&self) -> LocalProperty<T> {
        let source = self.0.local();
        let deps = [source.untyped()];
        LocalProperty::computed(move || source.get(), &deps)
    }
}
