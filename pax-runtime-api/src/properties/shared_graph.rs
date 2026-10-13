use super::{
    properties_table::PROPERTY_TABLE,
    shared_property::{lock, next_id, SharedCell},
    LocalProperty, Property, SharedPropertyValue,
};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    rc::Rc,
    sync::{Arc, Mutex, Weak},
};

#[derive(Default)]
struct InboxState {
    attached: HashSet<u64>,
    pending: HashSet<u64>,
    order: VecDeque<u64>,
    retired: HashSet<u64>,
    closed: bool,
}

pub(super) struct GraphInbox {
    state: Mutex<InboxState>,
    wake: Mutex<Arc<dyn Fn() + Send + Sync>>,
}

impl GraphInbox {
    pub(super) fn retire(&self, id: u64) {
        let wake = {
            let mut state = lock(&self.state);
            if state.closed {
                return;
            }
            let wake = state.pending.is_empty() && state.retired.is_empty();
            state.retired.insert(id);
            wake
        };
        if wake {
            self.wake();
        }
    }

    pub fn publish(&self, id: u64) {
        let wake = {
            let mut state = lock(&self.state);
            if state.closed || !state.attached.contains(&id) {
                return;
            }
            let wake = state.pending.is_empty() && state.retired.is_empty();
            if state.pending.insert(id) {
                state.order.push_back(id);
            }
            wake
        };
        if wake {
            self.wake();
        }
    }

    fn wake(&self) {
        let wake = lock(&self.wake).clone();
        wake();
    }

    fn attach(&self, id: u64) {
        lock(&self.state).attached.insert(id);
    }

    fn detach(&self, id: u64) {
        let mut state = lock(&self.state);
        state.attached.remove(&id);
        state.pending.remove(&id);
        state.order.retain(|queued| *queued != id);
    }
}

thread_local! {
    static ACTIVE_GRAPH: RefCell<Option<PropertyGraph>> = const { RefCell::new(None) };
    static ACTIVE_SCOPE: RefCell<Option<std::rc::Weak<BindingScopeInner>>> = const { RefCell::new(None) };
}

pub(super) fn with_current_graph<R>(f: impl FnOnce(&PropertyGraph) -> R) -> R {
    let graph = ACTIVE_GRAPH
        .with(|active| active.borrow().clone())
        .expect("shared property graph access requires PropertyGraph::enter on its owner thread");
    f(&graph)
}

pub(super) fn current_graph_id() -> Option<u64> {
    ACTIVE_GRAPH.with(|active| active.borrow().as_ref().map(|graph| graph.inner.id))
}

pub(super) fn current_binding_scope() -> Option<std::rc::Weak<BindingScopeInner>> {
    ACTIVE_SCOPE.with(|active| active.borrow().clone())
}

// A local evaluator belongs to the graph that created it, even if another
// application on the same UI thread happens to drain the shared effect queue.
pub(super) fn owner_evaluator<T: 'static>(evaluator: impl Fn() -> T + 'static) -> impl Fn() -> T {
    let owner = ACTIVE_GRAPH.with(|active| {
        active
            .borrow()
            .as_ref()
            .map(|graph| Rc::downgrade(&graph.inner))
    });
    let scope = ACTIVE_SCOPE.with(|active| active.borrow().clone());
    move || {
        let graph = owner.as_ref().map(|owner| PropertyGraph {
            inner: owner
                .upgrade()
                .expect("cannot evaluate a binding after its graph has been destroyed"),
        });
        let _entered = graph.as_ref().map(PropertyGraph::enter);
        // Pure local dependencies may still be read during reconciliation or
        // by another mounted view. Restore their attachment context without
        // reviving it; attach() treats a closed/missing owner as detached.
        let previous = ACTIVE_SCOPE.with(|active| active.replace(scope.clone()));
        let _scope = BindingScopeGuard { previous };
        evaluator()
    }
}

// Deferred graph machinery must be allocated in its original owner context,
// including an explicitly unowned context, regardless of who first reads it.
// Weak owners avoid extending a mounted node or application's lifetime.
pub(crate) fn owner_initializer<T: 'static>(
    initialize: impl FnOnce() -> T + 'static,
) -> impl FnOnce() -> T {
    let owner =
        ACTIVE_GRAPH.with(|active| active.borrow().as_ref().map(|g| Rc::downgrade(&g.inner)));
    let scope = ACTIVE_SCOPE.with(|active| active.borrow().clone());
    move || {
        let graph = owner.map(|owner| PropertyGraph {
            inner: owner
                .upgrade()
                .expect("cannot initialize a binding after its graph has been destroyed"),
        });
        if let Some(graph) = &graph {
            assert!(!graph.inner.closed.get(), "property graph is closed");
        }
        let previous = ACTIVE_GRAPH.with(|active| active.replace(graph));
        let previous_scope = ACTIVE_SCOPE.with(|active| active.replace(scope));
        let _context = PropertyGraphGuard {
            previous,
            previous_scope,
        };
        initialize()
    }
}

trait Attachment {
    fn as_any(&self) -> &dyn Any;
    fn prepare_import(&self) -> Option<Box<dyn FnOnce()>>;
    fn live(&self) -> bool;
    fn into_dormant(self: Box<Self>) -> Box<dyn DormantAttachment>;
}

trait DormantAttachment {
    fn live(&self) -> bool;
    fn restore(&self, graph: &PropertyGraph);
    fn local_id(&self) -> super::private::PropertyId;
}

struct Dormant<T: SharedPropertyValue> {
    shared: Weak<SharedCell<T>>,
    local: super::private::PropertyId,
    lifetime: RefCell<Option<AttachmentLifetime<T>>>,
    graph: u64,
    id: u64,
}

// Keep the drop notification registered while publication is suspended. Neither
// this lease nor the graph retains the shared application value.
struct AttachmentLifetime<T> {
    shared: Weak<SharedCell<T>>,
    graph: u64,
    id: u64,
    inbox: Weak<GraphInbox>,
}

impl<T: SharedPropertyValue> Drop for Dormant<T> {
    fn drop(&mut self) {
        let _ = PROPERTY_TABLE.try_with(|table| {
            table.unwatch_retirement(self.local, self.graph, self.id);
        });
    }
}

impl<T> Drop for AttachmentLifetime<T> {
    fn drop(&mut self) {
        if let Some(shared) = self.shared.upgrade() {
            shared.unsubscribe(self.graph);
        }
    }
}

impl<T: SharedPropertyValue> DormantAttachment for Dormant<T> {
    fn local_id(&self) -> super::private::PropertyId {
        self.local
    }
    fn live(&self) -> bool {
        self.shared.strong_count() != 0
            && PROPERTY_TABLE.with(|table| table.has_live_entry(self.local))
    }
    fn restore(&self, graph: &PropertyGraph) {
        let Some(cell) = self.shared.upgrade() else {
            return;
        };
        let Some(untyped) = super::UntypedProperty::try_from_id(self.local) else {
            return;
        };
        let local = LocalProperty::new_from_untyped(untyped);
        let property = Property { cell };
        graph.inner.inbox.attach(property.cell.id);
        property.cell.subscribe(graph.inner.id, &graph.inner.inbox);
        let snapshot = property.cell.snapshot();
        PROPERTY_TABLE.with(|table| table.import_shared(self.local, (*snapshot.value).clone()));
        PROPERTY_TABLE.with(|table| table.invalidate(self.local));
        graph.insert(
            &property,
            local,
            snapshot.revision,
            self.lifetime.borrow_mut().take(),
        );
    }
}

struct TypedAttachment<T: SharedPropertyValue> {
    local: LocalProperty<T>,
    revision: Rc<Cell<u64>>,
    _publication_effect: LocalProperty<()>,
    lifetime: Option<AttachmentLifetime<T>>,
}

impl<T: SharedPropertyValue> Attachment for TypedAttachment<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn live(&self) -> bool {
        self.lifetime
            .as_ref()
            .is_some_and(|lifetime| lifetime.shared.strong_count() != 0)
    }
    fn into_dormant(mut self: Box<Self>) -> Box<dyn DormantAttachment> {
        let lifetime = self.lifetime.take().expect("attached publication lifetime");
        PROPERTY_TABLE.with(|table| {
            table.watch_retirement(
                self.local.untyped.get_id(),
                lifetime.graph,
                lifetime.id,
                lifetime.inbox.clone(),
            )
        });
        Box::new(Dormant::<T> {
            shared: lifetime.shared.clone(),
            local: self.local.untyped.get_id(),
            graph: lifetime.graph,
            id: lifetime.id,
            lifetime: RefCell::new(Some(lifetime)),
        })
    }
    fn prepare_import(&self) -> Option<Box<dyn FnOnce()>> {
        let snapshot = self.lifetime.as_ref()?.shared.upgrade()?.snapshot();
        if snapshot.revision <= self.revision.get() {
            return None;
        }
        let local = self.local.clone();
        let revision = self.revision.clone();
        Some(Box::new(move || {
            PROPERTY_TABLE.with(|table| {
                table.import_shared(local.untyped.get_id(), (*snapshot.value).clone())
            });
            revision.set(snapshot.revision);
        }))
    }
}

impl<T: SharedPropertyValue> Drop for TypedAttachment<T> {
    fn drop(&mut self) {
        let _ =
            PROPERTY_TABLE.try_with(|table| table.clear_shared::<T>(self.local.untyped.get_id()));
    }
}

struct GraphInner {
    id: u64,
    inbox: Arc<GraphInbox>,
    attachments: RefCell<HashMap<u64, Box<dyn Attachment>>>,
    dormant: RefCell<HashMap<u64, Box<dyn DormantAttachment>>>,
    owners: RefCell<HashMap<u64, HashSet<u64>>>,
    scopes: RefCell<HashMap<u64, std::rc::Weak<BindingScopeInner>>>,
    closed: Cell<bool>,
}

impl Drop for GraphInner {
    fn drop(&mut self) {
        let mut state = lock(&self.inbox.state);
        state.closed = true;
        state.attached.clear();
        state.pending.clear();
        state.order.clear();
        state.retired.clear();
    }
}

/// Owner-thread bridge between shared values and Pax's local reactive graph.
///
/// The wake callback may run on any publishing thread. It must schedule host
/// work, never enter the engine inline. Enter this graph while constructing
/// bindings; call [`Self::import`] at a safe UI update boundary before draining
/// local effects. Imports coalesce by property and freeze a snapshot before any
/// local evaluator runs. Graphs and their local views cannot cross threads.
#[derive(Clone)]
pub struct PropertyGraph {
    inner: Rc<GraphInner>,
}

impl PropertyGraph {
    /// Creates a bridge with a host-owned, transferable wake callback.
    pub fn new(wake: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            inner: Rc::new(GraphInner {
                id: next_id(),
                inbox: Arc::new(GraphInbox {
                    state: Mutex::new(InboxState::default()),
                    wake: Mutex::new(Arc::new(wake)),
                }),
                attachments: RefCell::new(HashMap::new()),
                dormant: RefCell::new(HashMap::new()),
                owners: RefCell::new(HashMap::new()),
                scopes: RefCell::new(HashMap::new()),
                closed: Cell::new(false),
            }),
        }
    }

    /// Unique attachment identity for host scheduling. It never identifies an
    /// engine pointer and is not reused after this graph closes.
    pub fn id(&self) -> u64 {
        self.inner.id
    }

    /// Whether publications or retirements still need owner-thread servicing.
    /// Hosts recheck after a queued wake because an intervening frame may settle them.
    pub fn has_pending(&self) -> bool {
        let state = lock(&self.inner.inbox.state);
        !state.closed && (!state.pending.is_empty() || !state.retired.is_empty())
    }

    /// Installs the host scheduler after construction and wakes it if work
    /// arrived before registration. The callback must only schedule work.
    pub fn set_waker(&self, wake: impl Fn() + Send + Sync + 'static) {
        *lock(&self.inner.inbox.wake) = Arc::new(wake);
        let pending = {
            let state = lock(&self.inner.inbox.state);
            !state.closed && (!state.pending.is_empty() || !state.retired.is_empty())
        };
        if pending {
            self.inner.inbox.wake();
        }
    }

    /// Selects this graph for shared-to-local bindings on this thread.
    pub fn enter(&self) -> PropertyGraphGuard {
        assert!(!self.inner.closed.get(), "property graph is closed");
        let previous = ACTIVE_GRAPH.with(|active| active.replace(Some(self.clone())));
        let previous_scope = ACTIVE_SCOPE.with(|active| active.take());
        PropertyGraphGuard {
            previous,
            previous_scope,
        }
    }

    pub(super) fn attach<T: SharedPropertyValue>(
        &self,
        property: &Property<T>,
    ) -> LocalProperty<T> {
        assert!(!self.inner.closed.get(), "property graph is closed");
        let detached = ACTIVE_SCOPE.with(|active| {
            active
                .borrow()
                .as_ref()
                .is_some_and(|scope| scope.upgrade().is_none_or(|scope| scope.closed.get()))
        });
        if detached {
            // Retained local computations can read their last projection, but
            // cannot re-subscribe a view after unmount. Raw data remains data.
            if let Some(attachment) = self.inner.attachments.borrow().get(&property.cell.id) {
                return attachment
                    .as_any()
                    .downcast_ref::<TypedAttachment<T>>()
                    .unwrap()
                    .local
                    .clone();
            }
            if let Some(dormant) = self.inner.dormant.borrow().get(&property.cell.id) {
                if let Some(local) = super::UntypedProperty::try_from_id(dormant.local_id()) {
                    return LocalProperty::new_from_untyped(local);
                }
            }
            return LocalProperty::new(property.get());
        }
        self.record_owner(property.cell.id);
        self.restore(property.cell.id);
        if let Some(attachment) = self.inner.attachments.borrow().get(&property.cell.id) {
            return attachment
                .as_any()
                .downcast_ref::<TypedAttachment<T>>()
                .expect("shared property type changed")
                .local
                .clone();
        }
        // Subscribe before sampling: a racing writer is either in the initial
        // snapshot or leaves a pending notification for the next import.
        self.inner.inbox.attach(property.cell.id);
        property.cell.subscribe(self.inner.id, &self.inner.inbox);
        let snapshot = property.cell.snapshot();
        let local = LocalProperty::new((*snapshot.value).clone());
        self.insert(property, local.clone(), snapshot.revision, None);
        local
    }

    pub(super) fn observed_revision<T: SharedPropertyValue>(
        &self,
        property: &Property<T>,
    ) -> Option<u64> {
        self.inner
            .attachments
            .borrow()
            .get(&property.cell.id)
            .map(|entry| {
                entry
                    .as_any()
                    .downcast_ref::<TypedAttachment<T>>()
                    .expect("shared property type changed")
                    .revision
                    .get()
            })
    }

    pub(super) fn share<T: SharedPropertyValue>(&self, local: LocalProperty<T>) -> Property<T> {
        if let Some(cell) =
            PROPERTY_TABLE.with(|table| table.shared_storage::<T>(local.untyped.get_id()))
        {
            self.record_owner(cell.id);
            return Property { cell };
        }
        let property = Property::new(local.get());
        self.record_owner(property.cell.id);
        self.inner.inbox.attach(property.cell.id);
        property.cell.subscribe(self.inner.id, &self.inner.inbox);
        self.insert(&property, local, 0, None);
        property
    }

    fn insert<T: SharedPropertyValue>(
        &self,
        property: &Property<T>,
        local: LocalProperty<T>,
        revision: u64,
        lifetime: Option<AttachmentLifetime<T>>,
    ) {
        let seen = Rc::new(Cell::new(revision));
        let weak = Arc::downgrade(&property.cell);
        let weak_writer = weak.clone();
        let writer_seen = seen.clone();
        let graph = self.inner.id;
        PROPERTY_TABLE.with(|table| {
            table.set_shared(
                local.untyped.get_id(),
                weak.clone(),
                Rc::new(move |value: &T| {
                    if let Some(shared) = weak_writer.upgrade() {
                        writer_seen.set(shared.publish_local(value.clone(), graph));
                    }
                }),
            )
        });
        // The projection belongs to every view that attaches it. Its eager
        // publication effect must survive the first view's unmount.
        let _graph = self.enter();
        let observed = local.clone();
        let effect = LocalProperty::computed(
            move || {
                observed.get();
            },
            &[local.untyped()],
        );
        super::register_effect_property(&effect);
        self.inner.attachments.borrow_mut().insert(
            property.cell.id,
            Box::new(TypedAttachment {
                local,
                revision: seen,
                _publication_effect: effect,
                lifetime: Some(lifetime.unwrap_or_else(|| AttachmentLifetime {
                    shared: Arc::downgrade(&property.cell),
                    graph,
                    id: property.cell.id,
                    inbox: Arc::downgrade(&self.inner.inbox),
                })),
            }),
        );
    }

    /// Imports a finite snapshot of pending publications without running user
    /// evaluators. Publications racing this import stay pending for a later turn.
    /// Returns the number of imported properties.
    pub fn import(&self) -> usize {
        self.import_batch(1024)
    }

    /// Imports at most `limit` queued identities in FIFO order and requests a
    /// further host turn if work remains. Repeated writes occupy one slot.
    pub fn import_batch(&self, limit: usize) -> usize {
        self.import_batch_with_observer(limit, |_| {})
    }

    /// Reports publication import and retirement cleanup separately, without
    /// changing the import boundary. The observer must not re-enter the graph.
    #[doc(hidden)]
    pub fn import_batch_with_observer(
        &self,
        limit: usize,
        mut completed: impl FnMut(&'static str),
    ) -> usize {
        if self.inner.closed.get() {
            return 0;
        }
        let _entered = self.enter();
        let pending: Vec<_> = {
            let mut state = lock(&self.inner.inbox.state);
            let count = limit.min(state.order.len());
            let pending: Vec<_> = state.order.drain(..count).collect();
            for id in &pending {
                state.pending.remove(id);
            }
            pending
        };
        let actions: Vec<_> = {
            let attachments = self.inner.attachments.borrow();
            pending
                .into_iter()
                .filter_map(|id| attachments.get(&id)?.prepare_import())
                .collect()
        };
        let count = actions.len();
        for action in actions {
            action();
        }
        completed("property_import");
        self.collect();
        completed("property_cleanup");
        if self.has_pending() {
            self.inner.inbox.wake();
        }
        count
    }

    /// Releases bindings notified by shared-owner or suspended-local destruction.
    /// Work is proportional to retirement notifications, not all live bindings.
    pub fn collect(&self) {
        let candidates = std::mem::take(&mut lock(&self.inner.inbox.state).retired);
        let dead: Vec<_> = candidates
            .into_iter()
            .filter(|id| {
                // A local retirement can race a same-thread remount. Only retire
                // the current attachment if its own owners are gone.
                let attached = self.inner.attachments.borrow();
                if let Some(attachment) = attached.get(id) {
                    return !attachment.live();
                }
                self.inner
                    .dormant
                    .borrow()
                    .get(id)
                    .is_some_and(|entry| !entry.live())
            })
            .collect();
        let retired: Vec<_> = {
            let mut attachments = self.inner.attachments.borrow_mut();
            dead.into_iter()
                .filter_map(|id| {
                    self.inner.inbox.detach(id);
                    self.inner.dormant.borrow_mut().remove(&id);
                    if let Some(owners) = self.inner.owners.borrow_mut().remove(&id) {
                        let scopes = self.inner.scopes.borrow();
                        for owner in owners {
                            if let Some(scope) = scopes.get(&owner).and_then(std::rc::Weak::upgrade)
                            {
                                scope.properties.borrow_mut().remove(&id);
                            }
                        }
                    }
                    attachments.remove(&id)
                })
                .collect()
        };
        drop(retired);
    }

    /// Detaches every graph binding on the owner thread. Retained shared values
    /// remain usable but can no longer wake or evaluate this graph.
    #[doc(hidden)]
    pub fn stop_publications(&self) {
        let mut state = lock(&self.inner.inbox.state);
        state.closed = true;
        state.attached.clear();
        state.pending.clear();
        state.order.clear();
        state.retired.clear();
    }

    pub fn close(&self) {
        self.inner.closed.set(true);
        {
            let mut state = lock(&self.inner.inbox.state);
            state.closed = true;
            state.attached.clear();
            state.pending.clear();
            state.order.clear();
            state.retired.clear();
        }
        let retired = std::mem::take(&mut *self.inner.attachments.borrow_mut());
        self.inner.dormant.borrow_mut().clear();
        self.inner.owners.borrow_mut().clear();
        self.inner.scopes.borrow_mut().clear();
        drop(retired);
    }

    /// Creates an internal owner for view bindings. Unmount suspends its graph
    /// attachments while allowing retained shared data to survive independently.
    #[doc(hidden)]
    pub fn binding_scope(&self) -> BindingScope {
        let scope = BindingScope {
            inner: Rc::new(BindingScopeInner {
                graph: Rc::downgrade(&self.inner),
                id: next_id(),
                closed: Cell::new(false),
                properties: RefCell::new(HashSet::new()),
            }),
        };
        self.inner
            .scopes
            .borrow_mut()
            .insert(scope.inner.id, Rc::downgrade(&scope.inner));
        scope
    }

    fn restore(&self, id: u64) {
        let dormant = self.inner.dormant.borrow_mut().remove(&id);
        if let Some(dormant) = dormant {
            dormant.restore(self);
        }
    }

    fn record_owner(&self, id: u64) {
        let scope =
            ACTIVE_SCOPE.with(|scope| scope.borrow().as_ref().and_then(std::rc::Weak::upgrade));
        let mut owners = self.inner.owners.borrow_mut();
        let owners = owners.entry(id).or_default();
        if let Some(scope) = scope.filter(|scope| scope.graph.ptr_eq(&Rc::downgrade(&self.inner))) {
            assert!(
                !scope.closed.get(),
                "cannot attach properties for an unmounted view"
            );
            scope.properties.borrow_mut().insert(id);
            owners.insert(scope.id);
        } else if owners.is_empty() {
            owners.insert(0);
        }
    }

    fn release_owner(&self, scope: &BindingScopeInner) {
        let retired: Vec<_> = scope
            .properties
            .borrow()
            .iter()
            .filter_map(|id| {
                let mut owners = self.inner.owners.borrow_mut();
                let set = owners.get_mut(id)?;
                set.remove(&scope.id);
                if !set.is_empty() {
                    return None;
                }
                owners.remove(id);
                self.inner
                    .attachments
                    .borrow_mut()
                    .remove(id)
                    .map(|attachment| (*id, attachment))
            })
            .collect();
        for (id, attachment) in retired {
            self.inner.inbox.detach(id);
            let dormant = attachment.into_dormant();
            self.inner.dormant.borrow_mut().insert(id, dormant);
        }
    }
}

pub(super) struct BindingScopeInner {
    graph: std::rc::Weak<GraphInner>,
    id: u64,
    pub(super) closed: Cell<bool>,
    properties: RefCell<HashSet<u64>>,
}

impl Drop for BindingScopeInner {
    fn drop(&mut self) {
        if let Some(inner) = self.graph.upgrade() {
            inner.scopes.borrow_mut().remove(&self.id);
            PropertyGraph { inner }.release_owner(self);
        }
    }
}

/// Internal owner-thread attachment lifetime used by mounted nodes.
#[doc(hidden)]
#[derive(Clone)]
pub struct BindingScope {
    inner: Rc<BindingScopeInner>,
}

impl BindingScope {
    /// Evaluate an owner-thread node access without reviving a detached view.
    pub fn with_graph<R>(&self, body: impl FnOnce() -> R) -> R {
        let graph = PropertyGraph {
            inner: self
                .inner
                .graph
                .upgrade()
                .expect("binding graph was destroyed"),
        };
        let _graph = graph.enter();
        let previous = ACTIVE_SCOPE.with(|active| active.replace(Some(Rc::downgrade(&self.inner))));
        let _scope = BindingScopeGuard { previous };
        body()
    }

    pub fn enter(&self) -> BindingScopeGuard {
        assert!(
            !self.inner.closed.get(),
            "cannot evaluate bindings for an unmounted view"
        );
        let previous = ACTIVE_SCOPE.with(|active| active.replace(Some(Rc::downgrade(&self.inner))));
        BindingScopeGuard { previous }
    }

    pub fn close(&self) {
        if self.inner.closed.replace(true) {
            return;
        }
        if let Some(inner) = self.inner.graph.upgrade() {
            PropertyGraph { inner }.release_owner(&self.inner);
        }
    }

    pub fn resume(&self) {
        if !self.inner.closed.replace(false) {
            return;
        }
        if let Some(inner) = self.inner.graph.upgrade() {
            let graph = PropertyGraph { inner };
            let _entered = graph.enter();
            let _scope = self.enter();
            self.inner.properties.borrow_mut().retain(|id| {
                graph.inner.attachments.borrow().contains_key(id)
                    || graph
                        .inner
                        .dormant
                        .borrow()
                        .get(id)
                        .is_some_and(|entry| entry.live())
            });
            let properties: Vec<_> = self.inner.properties.borrow().iter().copied().collect();
            for id in properties {
                graph.record_owner(id);
                graph.restore(id);
            }
        }
    }
}

#[doc(hidden)]
pub struct BindingScopeGuard {
    previous: Option<std::rc::Weak<BindingScopeInner>>,
}
impl Drop for BindingScopeGuard {
    fn drop(&mut self) {
        ACTIVE_SCOPE.with(|active| {
            active.replace(self.previous.take());
        });
    }
}

/// Restores the previous owner-thread graph when dropped.
#[must_use]
pub struct PropertyGraphGuard {
    previous: Option<PropertyGraph>,
    previous_scope: Option<std::rc::Weak<BindingScopeInner>>,
}

impl Drop for PropertyGraphGuard {
    fn drop(&mut self) {
        ACTIVE_GRAPH.with(|active| {
            active.replace(self.previous.take());
        });
        ACTIVE_SCOPE.with(|active| {
            active.replace(self.previous_scope.take());
        });
    }
}

#[cfg(test)]
mod retirement_tests {
    use super::*;

    #[test]
    fn empty_import_does_not_visit_live_attachments() {
        struct Counted(Rc<Cell<usize>>);
        impl Attachment for Counted {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn prepare_import(&self) -> Option<Box<dyn FnOnce()>> {
                None
            }
            fn live(&self) -> bool {
                self.0.set(self.0.get() + 1);
                true
            }
            fn into_dormant(self: Box<Self>) -> Box<dyn DormantAttachment> {
                unreachable!()
            }
        }
        let graph = PropertyGraph::new(|| {});
        let visits = Rc::new(Cell::new(0));
        for id in 0..1000 {
            graph
                .inner
                .attachments
                .borrow_mut()
                .insert(id, Box::new(Counted(visits.clone())));
        }
        graph.import();
        graph.collect();
        assert_eq!(visits.get(), 0);
        graph.inner.inbox.retire(40);
        graph.collect();
        assert_eq!(visits.get(), 1, "only the notified identity is checked");
    }

    #[test]
    fn retirement_is_independent_of_publication_batch_limit() {
        let graph = PropertyGraph::new(|| {});
        let _graph = graph.enter();
        let retained = Property::new(1_u32);
        let retained_local = retained.local();
        let values: Vec<_> = (0..1500)
            .map(|n| {
                let value = Property::new(n);
                value.local();
                value
            })
            .collect();
        std::thread::spawn(move || drop(values)).join().unwrap();
        retained.set(2);
        assert!(graph.has_pending());
        assert_eq!(graph.import_batch(0), 0);
        assert_eq!(graph.inner.attachments.borrow().len(), 1);
        assert_eq!(retained_local.get(), 1);
        assert_eq!(graph.import(), 1);
        assert_eq!(retained_local.get(), 2);
        assert!(!graph.has_pending());
    }

    #[test]
    fn suspended_local_drop_retires_metadata_without_dropping_shared_value() {
        let graph = PropertyGraph::new(|| {});
        let _graph = graph.enter();
        let shared = Property::new(1_u32);
        let scope = graph.binding_scope();
        let local = {
            let _scope = scope.enter();
            shared.local()
        };
        scope.close();
        assert_eq!(graph.inner.dormant.borrow().len(), 1);
        drop(local);
        assert!(graph.has_pending());
        graph.import();
        assert!(graph.inner.dormant.borrow().is_empty());
        shared.set(2);
        assert!(!graph.has_pending());
        scope.resume();
        let _scope = scope.enter();
        assert_eq!(shared.local().get(), 2);
    }

    #[test]
    fn suspended_shared_drop_on_worker_retires_but_preserves_retained_local() {
        let graph = PropertyGraph::new(|| {});
        let _graph = graph.enter();
        let shared = Property::new(1_u32);
        let scope = graph.binding_scope();
        let local = {
            let _scope = scope.enter();
            shared.local()
        };
        scope.close();
        std::thread::spawn(move || drop(shared)).join().unwrap();
        graph.collect();
        assert!(graph.inner.dormant.borrow().is_empty());
        assert_eq!(local.get(), 1);
        scope.resume();
        assert!(scope.inner.properties.borrow().is_empty());
    }

    #[test]
    fn delayed_local_retirement_does_not_detach_a_new_projection() {
        let graph = PropertyGraph::new(|| {});
        let _graph = graph.enter();
        let shared = Property::new(1_u32);
        let scope = graph.binding_scope();
        let local = {
            let _scope = scope.enter();
            shared.local()
        };
        scope.close();
        drop(local);
        // Reattach before servicing the old local's destruction notification.
        scope.resume();
        let _scope = scope.enter();
        let replacement = shared.local();
        graph.collect();
        shared.set(3);
        assert_eq!(graph.import(), 1);
        assert_eq!(replacement.get(), 3);
    }

    #[test]
    fn repeated_suspend_resume_keeps_drop_notifications_and_publication() {
        let graph = PropertyGraph::new(|| {});
        let _graph = graph.enter();
        let shared = Property::new(0_u32);
        let scope = graph.binding_scope();
        let local = {
            let _scope = scope.enter();
            shared.local()
        };
        for n in 1..10 {
            scope.close();
            shared.set(n);
            assert!(!graph.has_pending());
            scope.resume();
            assert_eq!(local.get(), n);
            shared.set(n + 100);
            assert_eq!(graph.import(), 1);
            assert_eq!(local.get(), n + 100);
        }
        drop(shared);
        graph.collect();
        assert!(graph.inner.attachments.borrow().is_empty());
        assert_eq!(local.get(), 109);
    }
}
