//! Executor-neutral task lifetimes and bounded delivery to the UI thread.
//!
//! A scope is local authority. Only its producers, guarded publishers and task
//! reporters can cross threads. A successful send means accepted, not delivered.
use crate::{properties::SharedPropertyValue, Interpolatable, Property};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    future::Future,
    marker::PhantomData,
    pin::Pin,
    rc::{Rc, Weak},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, MutexGuard,
    },
    task::{Context, Poll, Waker},
    time::Duration,
};

pub(crate) fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value.lock().unwrap_or_else(|error| error.into_inner())
}

/// Application resource limits. Counts do not bound the byte size of user values.
#[derive(Clone, Debug)]
pub struct AsyncLimits {
    pub scopes: usize,
    pub tasks: usize,
    pub publishers: usize,
    pub endpoints: usize,
    pub message_slots: usize,
    pub channel_capacity: usize,
    pub callbacks_per_turn: usize,
    pub callback_time: Duration,
}
impl Default for AsyncLimits {
    fn default() -> Self {
        Self {
            scopes: 1024,
            tasks: 1024,
            publishers: 1024,
            endpoints: 1024,
            message_slots: 4096,
            channel_capacity: 32,
            callbacks_per_turn: 64,
            callback_time: Duration::from_millis(2),
        }
    }
}

/// New work was rejected before it could start or reserve resources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AsyncError {
    Closed,
    Limit(&'static str),
    ZeroCapacity,
}
impl std::fmt::Display for AsyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed => f.write_str("the work lifetime is closed"),
            Self::Limit(name) => write!(f, "application {name} limit reached"),
            Self::ZeroCapacity => f.write_str("a UI channel needs at least one slot"),
        }
    }
}
impl std::error::Error for AsyncError {}

/// A rejected publication retains the caller's value.
#[derive(Debug, PartialEq, Eq)]
pub struct Closed<T>(pub T);
/// A nonblocking channel send either retains the value or accepts it exactly once.
#[derive(Debug, PartialEq, Eq)]
pub enum TrySendError<T> {
    Full(T),
    Closed(T),
}

type Payload = Box<dyn Any + Send>;
type Wake = Arc<dyn Fn() + Send + Sync>;

// The chain is locked from the application toward the leaf. Revocation only
// needs its own gate: it cannot return while an earlier commit holds that gate.
#[derive(Clone)]
pub(crate) struct Authority(Vec<Arc<Mutex<bool>>>);
impl Authority {
    fn root() -> Self {
        Self(vec![Arc::new(Mutex::new(true))])
    }
    fn child(&self) -> Self {
        let mut gates = self.0.clone();
        gates.push(Arc::new(Mutex::new(true)));
        Self(gates)
    }
    pub(crate) fn acquire(&self) -> Option<Vec<MutexGuard<'_, bool>>> {
        let mut guards = Vec::with_capacity(self.0.len());
        for gate in &self.0 {
            let guard = lock(gate);
            if !*guard {
                return None;
            }
            guards.push(guard);
        }
        Some(guards)
    }
    fn live(&self) -> bool {
        self.acquire().is_some()
    }
    fn revoke(&self) {
        *lock(self.0.last().unwrap()) = false;
    }
}

#[derive(Default)]
struct Ready {
    order: VecDeque<u64>,
    set: HashSet<u64>,
}
struct Hub {
    ready: Mutex<Ready>,
    wake: Mutex<Wake>,
}
impl Hub {
    fn notify(&self, id: u64) {
        let wake = {
            let mut ready = lock(&self.ready);
            let was_empty = ready.order.is_empty();
            if ready.set.insert(id) {
                ready.order.push_back(id);
            }
            was_empty
        };
        if wake {
            self.wake();
        }
    }
    fn wake(&self) {
        let wake = lock(&self.wake).clone();
        wake();
    }
    fn forget(&self, id: u64) {
        let mut ready = lock(&self.ready);
        ready.set.remove(&id);
        ready.order.retain(|queued| *queued != id);
    }
    fn prepend(&self, ids: impl IntoIterator<Item = u64>) {
        let ids: Vec<_> = ids.into_iter().collect();
        let mut ready = lock(&self.ready);
        for id in ids.into_iter().rev() {
            ready.order.retain(|queued| *queued != id);
            ready.set.insert(id);
            ready.order.push_front(id);
        }
    }
    fn take(&self) -> Vec<u64> {
        let mut ready = lock(&self.ready);
        ready.set.clear();
        ready.order.drain(..).collect()
    }
}

struct QueueState {
    values: VecDeque<Payload>,
    closed: bool,
    producers: usize,
    waiters: HashMap<usize, Waker>,
    next_waiter: usize,
}
struct Queue {
    id: u64,
    capacity: usize,
    authority: Authority,
    state: Mutex<QueueState>,
    hub: std::sync::Weak<Hub>,
}
impl Queue {
    fn notify(&self) {
        if lock(&self.state).closed {
            return;
        }
        if let Some(hub) = self.hub.upgrade() {
            hub.notify(self.id);
        }
    }
    fn send(&self, value: Payload) -> Result<(), TrySendError<Payload>> {
        let Some(authority) = self.authority.acquire() else {
            return Err(TrySendError::Closed(value));
        };
        let mut state = lock(&self.state);
        if state.closed {
            return Err(TrySendError::Closed(value));
        }
        if state.values.len() == self.capacity {
            return Err(TrySendError::Full(value));
        }
        state.values.push_back(value);
        drop(state);
        drop(authority);
        self.notify();
        Ok(())
    }
    fn pop(&self) -> Option<Payload> {
        let (value, waiters) = {
            let mut state = lock(&self.state);
            (state.values.pop_front(), std::mem::take(&mut state.waiters))
        };
        for waiter in waiters.into_values() {
            waiter.wake();
        }
        value
    }
    fn close(&self) {
        let (values, waiters) = {
            let mut state = lock(&self.state);
            state.closed = true;
            (
                std::mem::take(&mut state.values),
                std::mem::take(&mut state.waiters),
            )
        };
        drop(values);
        for waiter in waiters.into_values() {
            waiter.wake();
        }
    }
    fn finished(&self) -> bool {
        let state = lock(&self.state);
        state.closed || (state.producers == 0 && state.values.is_empty())
    }
}

/// Transferable producer for a bounded UI channel. Its callback stays local.
pub struct UiSender<T> {
    queue: Arc<Queue>,
    _payload: PhantomData<Mutex<T>>,
}
impl<T> Clone for UiSender<T> {
    fn clone(&self) -> Self {
        lock(&self.queue.state).producers += 1;
        Self {
            queue: self.queue.clone(),
            _payload: PhantomData,
        }
    }
}
impl<T> Drop for UiSender<T> {
    fn drop(&mut self) {
        let last = {
            let mut state = lock(&self.queue.state);
            state.producers -= 1;
            state.producers == 0
        };
        if last {
            self.queue.notify();
        }
    }
}
impl<T: Send + 'static> UiSender<T> {
    /// Enqueue immediately, reporting Full or Closed without losing the value.
    pub fn try_send(&self, value: T) -> Result<(), TrySendError<T>> {
        self.queue
            .send(Box::new(value))
            .map_err(|error| match error {
                TrySendError::Full(value) => TrySendError::Full(*value.downcast().unwrap()),
                TrySendError::Closed(value) => TrySendError::Closed(*value.downcast().unwrap()),
            })
    }
    /// Wait for capacity without blocking the UI or an executor thread.
    pub fn send(&self, value: T) -> SendFuture<'_, T> {
        SendFuture {
            sender: self,
            value: Some(value),
            waiter: None,
        }
    }
}

/// Capacity wait owned by one send attempt; dropping it unregisters its waker.
pub struct SendFuture<'a, T> {
    sender: &'a UiSender<T>,
    value: Option<T>,
    waiter: Option<usize>,
}
impl<T> Unpin for SendFuture<'_, T> {}
impl<T> Drop for SendFuture<'_, T> {
    fn drop(&mut self) {
        if let Some(id) = self.waiter {
            let removed = lock(&self.sender.queue.state).waiters.remove(&id);
            drop(removed);
        }
    }
}
impl<T: Send + 'static> Future for SendFuture<'_, T> {
    type Output = Result<(), Closed<T>>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let queue = self.sender.queue.clone();
        let waker = cx.waker().clone();
        let Some(authority) = queue.authority.acquire() else {
            return Poll::Ready(Err(Closed(
                self.value.take().expect("send polled after completion"),
            )));
        };
        let mut state = lock(&queue.state);
        if state.closed {
            return Poll::Ready(Err(Closed(
                self.value.take().expect("send polled after completion"),
            )));
        }
        if state.values.len() < queue.capacity {
            let previous = self.waiter.take().and_then(|id| state.waiters.remove(&id));
            state.values.push_back(Box::new(
                self.value.take().expect("send polled after completion"),
            ));
            drop(state);
            drop(authority);
            drop(previous);
            queue.notify();
            return Poll::Ready(Ok(()));
        }
        let id = *self.waiter.get_or_insert_with(|| {
            state.next_waiter += 1;
            state.next_waiter
        });
        let previous = state.waiters.insert(id, waker);
        drop(state);
        drop(authority);
        drop(previous);
        Poll::Pending
    }
}

/// Single-use producer. Dropping it unused releases the callback on the UI turn.
pub struct Completion<T>(UiSender<T>);
impl<T: Send + 'static> Completion<T> {
    pub fn complete(self, value: T) -> Result<(), Closed<T>> {
        self.0.try_send(value).map_err(|error| match error {
            TrySendError::Closed(value) | TrySendError::Full(value) => Closed(value),
        })
    }
}

/// Keeps a UI channel subscribed. Drop closes it and releases its local callback.
pub struct UiSubscription {
    owner: Weak<DispatcherInner>,
    id: u64,
}
impl Drop for UiSubscription {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.upgrade() {
            owner.remove_endpoint(self.id);
        }
    }
}

struct PublisherLease {
    count: Arc<AtomicUsize>,
}
impl Drop for PublisherLease {
    fn drop(&mut self) {
        self.count.fetch_sub(1, Ordering::Relaxed);
    }
}
/// A shared property writer whose commit is atomic with lifetime revocation.
/// Cloning this handle does not acquire another registration. Raw `Property`
/// clones remain deliberately unguarded.
#[derive(Clone)]
pub struct GuardedPublisher<T> {
    property: Property<T>,
    authority: Authority,
    _lease: Arc<PublisherLease>,
}
impl<T: SharedPropertyValue> GuardedPublisher<T> {
    pub fn set(&self, value: T) -> Result<(), Closed<T>> {
        self.property.publish_guarded(value, &self.authority)
    }
    pub fn is_closed(&self) -> bool {
        !self.authority.live()
    }
}

type CallbackContext = Rc<dyn Fn(&mut dyn FnMut())>;
struct ScopeInner {
    id: u64,
    owner: Weak<DispatcherInner>,
    authority: Authority,
    callback_context: RefCell<Option<CallbackContext>>,
    _parent: Option<AsyncScope>,
}
impl Drop for ScopeInner {
    fn drop(&mut self) {
        self.authority.revoke();
        if let Some(owner) = self.owner.upgrade() {
            owner.scopes.borrow_mut().remove(&self.id);
        }
    }
}
/// Local lifetime controller. Cancellation revokes descendants before requesting
/// task abortion. Keep a component's scope from its mount context when handlers
/// later receive contexts for child controls.
///
/// ```compile_fail
/// use pax_runtime_api::{application::{ApplicationInstance, EmptyApplication}, TargetInfo};
/// let app = ApplicationInstance::prepare::<EmptyApplication>(TargetInfo::default()).unwrap();
/// let scope = app.context().async_scope().unwrap();
/// std::thread::spawn(move || scope.cancel());
/// ```
#[derive(Clone)]
pub struct AsyncScope(Rc<ScopeInner>);
impl AsyncScope {
    fn owner(&self) -> Result<Rc<DispatcherInner>, AsyncError> {
        if !self.0.authority.live() {
            return Err(AsyncError::Closed);
        }
        self.0.owner.upgrade().ok_or(AsyncError::Closed)
    }
    /// Restores a mounted node's property ownership around local delivery.
    #[doc(hidden)]
    pub fn set_callback_context(&self, enter: impl Fn(&mut dyn FnMut()) + 'static) {
        *self.0.callback_context.borrow_mut() = Some(Rc::new(enter));
    }
    pub fn id(&self) -> u64 {
        self.0.id
    }
    pub fn is_closed(&self) -> bool {
        !self.0.authority.live() || self.0.owner.strong_count() == 0
    }
    /// Create an independent operation underneath this lifetime.
    pub fn child(&self) -> Result<Self, AsyncError> {
        let owner = self.owner()?;
        owner.scope(Some(self.clone()))
    }
    pub fn cancel(&self) {
        self.0.authority.revoke();
        if let Some(owner) = self.0.owner.upgrade() {
            owner.sweep_cancelled();
        }
    }
    /// Revoke the previous request before publishing replacement state and
    /// creating its new operation. Registration failure leaves the old work closed.
    pub fn replace<T: SharedPropertyValue>(
        &self,
        previous: &mut Option<AsyncScope>,
        property: &Property<T>,
        initial: T,
    ) -> Result<AsyncScope, AsyncError> {
        if let Some(old) = previous.take() {
            old.cancel();
        }
        self.owner()?;
        property.set(initial);
        let next = self.child()?;
        *previous = Some(next.clone());
        Ok(next)
    }
    pub fn publisher<T: SharedPropertyValue>(
        &self,
        property: &Property<T>,
    ) -> Result<GuardedPublisher<T>, AsyncError> {
        let owner = self.owner()?;
        owner
            .publishers
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                (count < owner.limits.publishers).then_some(count + 1)
            })
            .map_err(|_| AsyncError::Limit("publisher"))?;
        Ok(GuardedPublisher {
            property: property.clone(),
            authority: self.0.authority.clone(),
            _lease: Arc::new(PublisherLease {
                count: owner.publishers.clone(),
            }),
        })
    }
    pub fn completion<T: Send + 'static>(
        &self,
        callback: impl FnOnce(T) + 'static,
    ) -> Result<Completion<T>, AsyncError> {
        let mut callback = Some(callback);
        let sender = self.endpoint(1, true, move |value| callback.take().unwrap()(value))?;
        Ok(Completion(sender))
    }
    pub fn channel<T: Send + 'static>(
        &self,
        capacity: usize,
        callback: impl FnMut(T) + 'static,
    ) -> Result<(UiSender<T>, UiSubscription), AsyncError> {
        let sender = self.endpoint(capacity, false, callback)?;
        let subscription = UiSubscription {
            owner: self.0.owner.clone(),
            id: sender.queue.id,
        };
        Ok((sender, subscription))
    }
    pub fn default_channel<T: Send + 'static>(
        &self,
        callback: impl FnMut(T) + 'static,
    ) -> Result<(UiSender<T>, UiSubscription), AsyncError> {
        self.channel(self.owner()?.limits.channel_capacity, callback)
    }
    fn endpoint<T: Send + 'static>(
        &self,
        capacity: usize,
        once: bool,
        mut callback: impl FnMut(T) + 'static,
    ) -> Result<UiSender<T>, AsyncError> {
        if capacity == 0 {
            return Err(AsyncError::ZeroCapacity);
        }
        let owner = self.owner()?;
        if owner.endpoints.borrow().len() >= owner.limits.endpoints {
            return Err(AsyncError::Limit("endpoint"));
        }
        if capacity > owner.limits.message_slots.saturating_sub(owner.slots.get()) {
            return Err(AsyncError::Limit("message slot"));
        }
        let id = owner.next_id();
        let queue = Arc::new(Queue {
            id,
            capacity,
            authority: self.0.authority.clone(),
            hub: Arc::downgrade(&owner.hub),
            state: Mutex::new(QueueState {
                values: VecDeque::new(),
                closed: false,
                producers: 1,
                waiters: HashMap::new(),
                next_waiter: 0,
            }),
        });
        owner.slots.set(owner.slots.get() + capacity);
        owner.endpoints.borrow_mut().insert(
            id,
            Rc::new(Endpoint {
                scope: self.clone(),
                queue: queue.clone(),
                once,
                callback: RefCell::new(Box::new(move |payload| {
                    callback(*payload.downcast().unwrap())
                })),
            }),
        );
        Ok(UiSender {
            queue,
            _payload: PhantomData,
        })
    }
    /// Adapter entry point. `start` runs only after activation and returns a
    /// nonblocking cancellation request. Retaining the control is optional.
    pub fn register_task(
        &self,
        start: impl FnOnce(TaskReporter) -> Box<dyn FnOnce()> + 'static,
    ) -> Result<TaskControl, AsyncError> {
        let owner = self.owner()?;
        if owner.tasks.borrow().len() >= owner.limits.tasks {
            return Err(AsyncError::Limit("task"));
        }
        let id = owner.next_id();
        let status = Property::new(TaskStatus::Staged);
        owner.tasks.borrow_mut().insert(
            id,
            TaskEntry {
                scope: self.clone(),
                status: status.clone(),
                start: Some(Box::new(start)),
                cancel: None,
            },
        );
        if owner.active.get() {
            owner.start_task(id);
        }
        Ok(TaskControl {
            scope: self.clone(),
            status,
        })
    }
}

/// Observable task state. A cancellation request is not proof that work stopped.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TaskStatus {
    #[default]
    Staged,
    Running,
    CancelRequested,
    Completed,
    Cancelled,
    Panicked(String),
}
impl Interpolatable for TaskStatus {}
/// The callback result keeps application `Result<T, E>` separate from a task panic.
#[derive(Debug)]
pub enum TaskOutcome<T> {
    Completed(T),
    Panicked(String),
}
/// UI-owned task control; dropping it does not detach work from its scope.
#[derive(Clone)]
pub struct TaskControl {
    scope: AsyncScope,
    status: Property<TaskStatus>,
}
impl TaskControl {
    pub fn cancel(&self) {
        self.scope.cancel();
    }
    pub fn status(&self) -> TaskStatus {
        self.status.get()
    }
    /// A snapshot property for reactive status displays; applications should not
    /// mutate this adapter-owned state.
    pub fn status_property(&self) -> crate::properties::Published<TaskStatus> {
        self.status.published()
    }
}
/// Transferable completion signal owned by an executor adapter. Drop means the
/// task stopped without normal completion, usually after an abort request.
pub struct TaskReporter {
    id: u64,
    status: Property<TaskStatus>,
    hub: std::sync::Weak<Hub>,
    finished: bool,
}
impl TaskReporter {
    pub fn finish(mut self, outcome: TaskStatus) {
        if let TaskStatus::Panicked(error) = &outcome {
            log::error!("async task panicked: {error}");
        }
        self.status.set(outcome);
        self.finished = true;
        if let Some(hub) = self.hub.upgrade() {
            hub.notify(self.id);
        }
    }
}
impl Drop for TaskReporter {
    fn drop(&mut self) {
        if !self.finished {
            self.status.set(TaskStatus::Cancelled);
            if let Some(hub) = self.hub.upgrade() {
                hub.notify(self.id);
            }
        }
    }
}

type StartTask = Box<dyn FnOnce(TaskReporter) -> Box<dyn FnOnce()>>;
struct TaskEntry {
    scope: AsyncScope,
    status: Property<TaskStatus>,
    start: Option<StartTask>,
    cancel: Option<Box<dyn FnOnce()>>,
}
struct Endpoint {
    scope: AsyncScope,
    queue: Arc<Queue>,
    once: bool,
    callback: RefCell<Box<dyn FnMut(Payload)>>,
}
struct DispatcherInner {
    limits: AsyncLimits,
    active: Cell<bool>,
    root: Authority,
    sequence: Cell<u64>,
    hub: Arc<Hub>,
    scopes: RefCell<HashMap<u64, Weak<ScopeInner>>>,
    endpoints: RefCell<HashMap<u64, Rc<Endpoint>>>,
    tasks: RefCell<HashMap<u64, TaskEntry>>,
    slots: Cell<usize>,
    publishers: Arc<AtomicUsize>,
    error_sink: RefCell<Rc<dyn Fn(String)>>,
    draining: Cell<bool>,
}
impl DispatcherInner {
    fn next_id(&self) -> u64 {
        let id = self
            .sequence
            .get()
            .checked_add(1)
            .expect("async identity exhausted");
        self.sequence.set(id);
        id
    }
    fn scope(self: &Rc<Self>, parent: Option<AsyncScope>) -> Result<AsyncScope, AsyncError> {
        if !self.root.live() {
            return Err(AsyncError::Closed);
        }
        if self.scopes.borrow().len() >= self.limits.scopes {
            return Err(AsyncError::Limit("scope"));
        }
        let id = self.next_id();
        let authority = parent
            .as_ref()
            .map_or_else(|| self.root.child(), |parent| parent.0.authority.child());
        let callback_context = parent
            .as_ref()
            .and_then(|parent| parent.0.callback_context.borrow().clone());
        let inner = Rc::new(ScopeInner {
            id,
            owner: Rc::downgrade(self),
            authority,
            callback_context: RefCell::new(callback_context),
            _parent: parent,
        });
        self.scopes.borrow_mut().insert(id, Rc::downgrade(&inner));
        Ok(AsyncScope(inner))
    }
    fn remove_endpoint(&self, id: u64) {
        let endpoint = self.endpoints.borrow_mut().remove(&id);
        if let Some(endpoint) = endpoint {
            self.slots.set(self.slots.get() - endpoint.queue.capacity);
            endpoint.queue.close();
            self.hub.forget(id);
            drop(endpoint);
        }
    }
    fn start_task(&self, id: u64) {
        let start = {
            let mut tasks = self.tasks.borrow_mut();
            let Some(task) = tasks.get_mut(&id) else {
                return;
            };
            if task.scope.is_closed() {
                return;
            }
            task.start.take().map(|start| (start, task.status.clone()))
        };
        if let Some((start, status)) = start {
            status.set(TaskStatus::Running);
            let reporter = TaskReporter {
                id,
                status: status.clone(),
                hub: Arc::downgrade(&self.hub),
                finished: false,
            };
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| start(reporter))) {
                Ok(cancel) => {
                    let mut cancel = Some(cancel);
                    if let Some(task) = self.tasks.borrow_mut().get_mut(&id) {
                        if !task.scope.is_closed() {
                            task.cancel = cancel.take();
                        }
                    }
                    if let Some(cancel) = cancel {
                        cancel();
                    }
                }
                Err(panic) => {
                    status.set(TaskStatus::Panicked(panic_message(panic)));
                    self.hub.notify(id);
                }
            }
        }
    }
    fn sweep_cancelled(&self) {
        let endpoints: Vec<_> = self
            .endpoints
            .borrow()
            .iter()
            .filter(|(_, endpoint)| endpoint.scope.is_closed())
            .map(|(id, _)| *id)
            .collect();
        for id in endpoints {
            self.remove_endpoint(id);
        }
        let cancelled: Vec<_> = self
            .tasks
            .borrow()
            .iter()
            .filter(|(_, task)| task.scope.is_closed())
            .map(|(id, _)| *id)
            .collect();
        for id in cancelled {
            let (staged, cancel, status) = {
                let mut tasks = self.tasks.borrow_mut();
                let Some(task) = tasks.get_mut(&id) else {
                    continue;
                };
                (task.start.take(), task.cancel.take(), task.status.clone())
            };
            if staged.is_some() {
                status.set(TaskStatus::Cancelled);
                drop(staged);
                self.hub.notify(id);
            } else if let Some(cancel) = cancel {
                if matches!(status.get(), TaskStatus::Running) {
                    status.set(TaskStatus::CancelRequested);
                }
                cancel();
            }
        }
    }
}
impl Drop for DispatcherInner {
    fn drop(&mut self) {
        self.root.revoke();
        for endpoint in self.endpoints.get_mut().values() {
            endpoint.queue.close();
        }
        for task in self.tasks.get_mut().values_mut() {
            if let Some(cancel) = task.cancel.take() {
                cancel();
            }
        }
    }
}

/// Chassis-owned delivery scheduler. Each turn snapshots eligible queue lengths,
/// visits channels round-robin, and settles structural changes before every call.
#[doc(hidden)]
#[derive(Clone)]
pub struct AsyncDispatcher(Rc<DispatcherInner>);
impl AsyncDispatcher {
    pub fn new(limits: AsyncLimits) -> Self {
        Self(Rc::new(DispatcherInner {
            limits,
            active: Cell::new(false),
            root: Authority::root(),
            sequence: Cell::new(0),
            hub: Arc::new(Hub {
                ready: Mutex::new(Ready::default()),
                wake: Mutex::new(Arc::new(|| {})),
            }),
            scopes: RefCell::new(HashMap::new()),
            endpoints: RefCell::new(HashMap::new()),
            tasks: RefCell::new(HashMap::new()),
            slots: Cell::new(0),
            publishers: Arc::new(AtomicUsize::new(0)),
            error_sink: RefCell::new(Rc::new(|error| log::error!("async task failed: {error}"))),
            draining: Cell::new(false),
        }))
    }
    pub fn scope(&self) -> Result<AsyncScope, AsyncError> {
        self.0.scope(None)
    }
    pub fn set_waker(&self, wake: impl Fn() + Send + Sync + 'static) {
        *lock(&self.0.hub.wake) = Arc::new(wake);
        if self.has_pending() {
            self.0.hub.wake();
        }
    }
    pub fn set_error_sink(&self, sink: impl Fn(String) + 'static) {
        *self.0.error_sink.borrow_mut() = Rc::new(sink);
    }
    pub fn has_pending(&self) -> bool {
        self.0.active.get() && !lock(&self.0.hub.ready).order.is_empty()
    }
    pub fn activate(&self) {
        if self.0.active.replace(true) || !self.0.root.live() {
            return;
        }
        let tasks: Vec<_> = self.0.tasks.borrow().keys().copied().collect();
        for id in tasks {
            self.0.start_task(id);
        }
        if self.has_pending() {
            self.0.hub.wake();
        }
    }
    pub fn close(&self) {
        self.0.root.revoke();
        self.0.sweep_cancelled();
    }
    pub fn task_count(&self) -> usize {
        self.0.tasks.borrow().len()
    }
    pub fn counts(&self) -> (usize, usize, usize, usize) {
        (
            self.0.scopes.borrow().len(),
            self.0.tasks.borrow().len(),
            self.0.endpoints.borrow().len(),
            self.0.publishers.load(Ordering::Relaxed),
        )
    }
    pub fn drain(&self, mut settle: impl FnMut()) -> Result<usize, String> {
        if !self.0.active.get() || self.0.draining.replace(true) {
            return Ok(0);
        }
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _reset = Reset(&self.0.draining);
        let mut ready = VecDeque::new();
        for id in self.0.hub.take() {
            let task = {
                let mut tasks = self.0.tasks.borrow_mut();
                if tasks.get(&id).is_some_and(|task| {
                    matches!(
                        task.status.get(),
                        TaskStatus::Completed | TaskStatus::Cancelled | TaskStatus::Panicked(_)
                    )
                }) {
                    tasks.remove(&id)
                } else {
                    None
                }
            };
            if let Some(task) = task {
                if let TaskStatus::Panicked(error) = task.status.get() {
                    let sink = self.0.error_sink.borrow().clone();
                    if let Err(panic) =
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sink(error)))
                    {
                        self.close();
                        return Err(panic_message(panic));
                    }
                }
                drop(task);
            }
            let endpoint = self.0.endpoints.borrow().get(&id).cloned();
            if let Some(endpoint) = endpoint {
                let quota = lock(&endpoint.queue.state).values.len();
                if quota == 0 && endpoint.queue.finished() {
                    self.0.remove_endpoint(id);
                } else if quota > 0 {
                    ready.push_back((id, quota));
                }
            }
        }
        let started = monotonic_now();
        let mut delivered = 0;
        while !ready.is_empty()
            && delivered < self.0.limits.callbacks_per_turn
            && (delivered == 0
                || monotonic_now().saturating_sub(started) < self.0.limits.callback_time)
        {
            settle();
            let (id, quota) = ready.pop_front().unwrap();
            let endpoint = self.0.endpoints.borrow().get(&id).cloned();
            let Some(endpoint) = endpoint else {
                continue;
            };
            if endpoint.scope.is_closed() {
                self.0.remove_endpoint(id);
                continue;
            }
            if let Some(value) = endpoint.queue.pop() {
                let context = endpoint.scope.0.callback_context.borrow().clone();
                let mut value = Some(value);
                let mut deliver = || {
                    (endpoint.callback.borrow_mut())(
                        value.take().expect("callback context invoked twice"),
                    )
                };
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if let Some(context) = context {
                        context(&mut deliver);
                    } else {
                        deliver();
                    }
                }));
                if let Err(panic) = result {
                    self.close();
                    return Err(panic_message(panic));
                }
                delivered += 1;
                if endpoint.once || endpoint.queue.finished() {
                    self.0.remove_endpoint(id);
                } else if quota > 1 {
                    ready.push_back((id, quota - 1));
                }
            }
        }
        // Preserve the next round across budgets. Enqueues during callbacks
        // remain behind this round and never run reentrantly in the same drain.
        self.0.hub.prepend(ready.into_iter().map(|(id, _)| id));
        if self.has_pending() {
            self.0.hub.wake();
        }
        Ok(delivered)
    }
}

/// Converts an unwinding panic into an observable task/callback failure.
pub fn panic_message(panic: Box<dyn Any + Send>) -> String {
    panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            panic
                .downcast_ref::<&str>()
                .map(|value| (*value).to_owned())
        })
        .unwrap_or_else(|| "task panicked with a non-string payload".into())
}

/// A local owner that can bind work to its lifetime without exposing a node to
/// an executor. Implemented by `AppContext` and runtime `NodeContext`.
pub trait AsyncScopeSource {
    fn work_scope(&self) -> Result<AsyncScope, AsyncError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, sync::atomic::AtomicBool, task::Wake};
    fn dispatcher() -> (AsyncDispatcher, AsyncScope) {
        let dispatcher = AsyncDispatcher::new(AsyncLimits::default());
        let scope = dispatcher.scope().unwrap();
        dispatcher.activate();
        (dispatcher, scope)
    }
    struct Flag(AtomicBool);
    impl Wake for Flag {
        fn wake(self: Arc<Self>) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    #[test]
    fn worker_completion_runs_and_drops_its_local_capture_only_on_the_owner() {
        let (dispatcher, scope) = dispatcher();
        let owner = std::thread::current().id();
        let value = Rc::new(Cell::new(0));
        let captured = value.clone();
        let completion = scope
            .completion(move |result| {
                assert_eq!(std::thread::current().id(), owner);
                captured.set(result);
            })
            .unwrap();
        std::thread::spawn(move || completion.complete(42).unwrap())
            .join()
            .unwrap();
        assert_eq!(value.get(), 0);
        dispatcher.drain(|| {}).unwrap();
        assert_eq!(value.get(), 42);
        assert_eq!(Rc::strong_count(&value), 1);
        assert_eq!(dispatcher.counts().2, 0);
    }
    #[test]
    fn unused_producer_drop_releases_callback_on_next_owner_turn() {
        let (dispatcher, scope) = dispatcher();
        let local = Rc::new(());
        let capture = local.clone();
        let completion = scope.completion(move |_: ()| drop(capture)).unwrap();
        std::thread::spawn(move || drop(completion)).join().unwrap();
        assert_eq!(Rc::strong_count(&local), 2);
        dispatcher.drain(|| {}).unwrap();
        assert_eq!(Rc::strong_count(&local), 1);
    }
    #[test]
    fn structural_settlement_precedes_every_callback_and_revokes_queued_work() {
        let (dispatcher, scope) = dispatcher();
        let node = scope.child().unwrap();
        let removed = Rc::new(Cell::new(false));
        let remove = removed.clone();
        let first = scope.completion(move |_: ()| remove.set(true)).unwrap();
        let second = node
            .completion(|_: ()| panic!("removed node callback must not start"))
            .unwrap();
        first.complete(()).unwrap();
        second.complete(()).unwrap();
        assert_eq!(
            dispatcher
                .drain(|| {
                    if removed.get() {
                        node.cancel();
                    }
                })
                .unwrap(),
            1
        );
        assert_eq!(dispatcher.counts().2, 0);
    }
    #[test]
    fn callbacks_enqueue_for_a_later_drain_and_channels_rotate_under_budget() {
        let mut limits = AsyncLimits::default();
        limits.callbacks_per_turn = 2;
        let dispatcher = AsyncDispatcher::new(limits);
        let scope = dispatcher.scope().unwrap();
        dispatcher.activate();
        let history = Rc::new(RefCell::new(Vec::new()));
        let record = history.clone();
        let (a, _a) = scope
            .channel(4, move |value| record.borrow_mut().push(value))
            .unwrap();
        let record = history.clone();
        let a2 = a.clone();
        let (b, _b) = scope
            .channel(2, move |value| {
                record.borrow_mut().push(value);
                a2.try_send(9).unwrap();
            })
            .unwrap();
        a.try_send(1).unwrap();
        a.try_send(2).unwrap();
        b.try_send(3).unwrap();
        dispatcher.drain(|| {}).unwrap();
        assert_eq!(*history.borrow(), [1, 3]);
        dispatcher.drain(|| {}).unwrap();
        assert_eq!(*history.borrow(), [1, 3, 2, 9]);
    }
    #[test]
    fn closure_wakes_capacity_waiters_and_returns_their_values() {
        let (dispatcher, scope) = dispatcher();
        let (sender, subscription) = scope.channel(1, |_: usize| {}).unwrap();
        sender.try_send(1).unwrap();
        assert_eq!(sender.try_send(2), Err(TrySendError::Full(2)));
        let wake = Arc::new(Flag(AtomicBool::new(false)));
        let waker = Waker::from(wake.clone());
        let mut cx = Context::from_waker(&waker);
        let mut future = Box::pin(sender.send(3));
        assert!(future.as_mut().poll(&mut cx).is_pending());
        drop(subscription);
        assert!(wake.0.load(Ordering::SeqCst));
        assert_eq!(future.as_mut().poll(&mut cx), Poll::Ready(Err(Closed(3))));
        dispatcher.drain(|| {}).unwrap();
        assert_eq!(dispatcher.counts().2, 0);
    }
    #[test]
    fn late_guarded_writes_cannot_overwrite_replacement_even_from_retained_workers() {
        let (_dispatcher, scope) = dispatcher();
        let target = Property::new(0usize);
        let mut previous = None;
        let old = scope.replace(&mut previous, &target, 1).unwrap();
        let publisher = old.publisher(&target).unwrap();
        publisher.set(2).unwrap();
        let next = scope.replace(&mut previous, &target, 3).unwrap();
        assert_eq!(
            std::thread::spawn(move || publisher.set(99))
                .join()
                .unwrap(),
            Err(Closed(99))
        );
        assert_eq!(target.get(), 3);
        next.publisher(&target).unwrap().set(4).unwrap();
        scope.cancel();
        assert_eq!(target.get(), 4);
    }
    #[test]
    fn revocation_racing_a_worker_commit_cannot_overwrite_replacement_state() {
        let (_dispatcher, scope) = dispatcher();
        let target = Property::new(0usize);
        for _ in 0..128 {
            let operation = scope.child().unwrap();
            let publisher = operation.publisher(&target).unwrap();
            let start = Arc::new(std::sync::Barrier::new(2));
            let worker_start = start.clone();
            let worker = std::thread::spawn(move || {
                worker_start.wait();
                let _ = publisher.set(99);
            });
            start.wait();
            operation.cancel();
            target.set(7);
            worker.join().unwrap();
            assert_eq!(target.get(), 7);
        }
    }
    #[test]
    fn guarded_commit_releases_authority_before_waking_arbitrary_host_code() {
        let (_dispatcher, scope) = dispatcher();
        let authority = scope.0.authority.clone();
        let graph = crate::PropertyGraph::new(move || authority.revoke());
        let _entered = graph.enter();
        let target = Property::new(0usize);
        let _view = target.local();
        let publisher = scope.publisher(&target).unwrap();
        publisher.set(1).unwrap();
        assert!(publisher.is_closed());
        assert_eq!(target.get(), 1);
    }
    #[test]
    fn prepared_tasks_are_staged_and_discarded_without_starting() {
        let dispatcher = AsyncDispatcher::new(AsyncLimits::default());
        let scope = dispatcher.scope().unwrap();
        let task = scope
            .register_task(|_| panic!("discarded candidate executed a task"))
            .unwrap();
        assert_eq!(task.status(), TaskStatus::Staged);
        dispatcher.close();
        dispatcher.activate();
        assert_eq!(task.status(), TaskStatus::Cancelled);
    }
    #[test]
    fn dropping_controls_does_not_detach_tasks_and_panics_reach_the_sink() {
        let (dispatcher, scope) = dispatcher();
        let errors = Rc::new(RefCell::new(Vec::new()));
        let captured = errors.clone();
        dispatcher.set_error_sink(move |error| captured.borrow_mut().push(error));
        let reporter = Rc::new(RefCell::new(None));
        let captured = reporter.clone();
        let control = scope
            .child()
            .unwrap()
            .register_task(move |signal| {
                *captured.borrow_mut() = Some(signal);
                Box::new(|| {})
            })
            .unwrap();
        drop(control);
        assert_eq!(dispatcher.task_count(), 1);
        reporter
            .borrow_mut()
            .take()
            .unwrap()
            .finish(TaskStatus::Panicked("expected failure".into()));
        dispatcher.drain(|| {}).unwrap();
        assert_eq!(*errors.borrow(), ["expected failure"]);
        assert_eq!(dispatcher.task_count(), 0);
        assert_eq!(dispatcher.counts().0, 1);
    }
    #[test]
    fn resources_are_bounded_and_reusable_after_completion_or_drop() {
        let mut limits = AsyncLimits::default();
        limits.endpoints = 1;
        limits.message_slots = 2;
        limits.publishers = 1;
        limits.tasks = 1;
        let dispatcher = AsyncDispatcher::new(limits);
        let scope = dispatcher.scope().unwrap();
        dispatcher.activate();
        assert!(matches!(
            scope.channel(3, |_: ()| {}),
            Err(AsyncError::Limit("message slot"))
        ));
        let completion = scope.completion(|_: ()| {}).unwrap();
        assert!(matches!(
            scope.completion(|_: ()| {}),
            Err(AsyncError::Limit("endpoint"))
        ));
        drop(completion);
        dispatcher.drain(|| {}).unwrap();
        let completion = scope.completion(|_: ()| {}).unwrap();
        drop(completion);
        dispatcher.drain(|| {}).unwrap();
        let value = Property::new(0usize);
        let publisher = scope.publisher(&value).unwrap();
        assert!(matches!(
            scope.publisher(&value),
            Err(AsyncError::Limit("publisher"))
        ));
        drop(publisher);
        assert!(scope.publisher(&value).is_ok());
    }
    #[test]
    fn cancelling_parent_closes_descendants_and_waiters_before_returning() {
        let (_dispatcher, root) = dispatcher();
        let child = root.child().unwrap();
        let (sender, _subscription) = child.channel(1, |_: usize| {}).unwrap();
        sender.try_send(1).unwrap();
        let wake = Arc::new(Flag(AtomicBool::new(false)));
        let waker = Waker::from(wake.clone());
        let mut cx = Context::from_waker(&waker);
        let mut send = Box::pin(sender.send(2));
        assert!(send.as_mut().poll(&mut cx).is_pending());
        root.cancel();
        assert!(wake.0.load(Ordering::SeqCst));
        assert_eq!(send.as_mut().poll(&mut cx), Poll::Ready(Err(Closed(2))));
        assert!(child.is_closed());
    }
}

/// Browser and native monotonic time for dispatch budgets and shutdown deadlines.
pub(crate) fn monotonic_now() -> Duration {
    #[cfg(not(target_arch = "wasm32"))]
    {
        static ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        ORIGIN.get_or_init(std::time::Instant::now).elapsed()
    }
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;
        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = performance, js_name = now)]
            fn performance_now() -> f64;
        }
        Duration::from_secs_f64(performance_now() / 1000.0)
    }
}
