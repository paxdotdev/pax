//! Root initialization and application lifetime, independent of any async executor.
use crate::application_shutdown;
use crate::async_runtime::monotonic_now;
use crate::properties::PropertyGraphGuard;
use crate::{application_shutdown::Cleanup, PropertyGraph, ShutdownTicket, TargetInfo};
use crate::{async_runtime::AsyncDispatcher, AsyncError, AsyncLimits, AsyncScope};
use std::time::Duration;
use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

/// A fallible application configuration result.
pub type AppResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Maximum encoded startup, request or reply size across a host boundary.
pub const HOST_MESSAGE_LIMIT: usize = 64 * 1024;
type HostHandler = Rc<dyn Fn(Vec<u8>) -> AppResult<Vec<u8>>>;
type ReplyCallback = Box<dyn FnOnce(Result<Vec<u8>, String>)>;

// The dispatcher's UI callback owns this reply. Revoking its scope drops the
// callback and acknowledges cancellation, even if the host stopped drawing.
struct HostReply(RefCell<Option<ReplyCallback>>);
impl HostReply {
    fn finish(&self, result: Result<Vec<u8>, String>) {
        let callback = self.0.borrow_mut().take();
        if let Some(callback) = callback {
            callback(result);
        }
    }
}
impl Drop for HostReply {
    fn drop(&mut self) {
        self.finish(Err("application closed before replying".into()));
    }
}

/// Resource limits and cleanup diagnostics for one application instance.
/// These options never construct an executor or register application services.
#[derive(Clone)]
pub struct AppOptions {
    pub async_limits: AsyncLimits,
    /// Opaque application-defined bytes, copied by the host before preparation.
    /// At most [`HOST_MESSAGE_LIMIT`] bytes. Pax does not choose their encoding.
    pub startup_data: Vec<u8>,
    /// Diagnostic deadline only; unfinished cleanup retains its dependencies.
    pub shutdown_deadline: Duration,
}
impl Default for AppOptions {
    fn default() -> Self {
        Self {
            async_limits: AsyncLimits::default(),
            startup_data: Vec::new(),
            shutdown_deadline: Duration::from_secs(2),
        }
    }
}

/// Context supplied to the ordinary root initializer on the UI owner thread.
/// The application's property graph is entered for the duration of the call.
/// Construct the root promptly, then use scoped work for asynchronous loading.
pub struct InitContext {
    pub target: TargetInfo,
    lifetime: AppContext,
    startup_data: Vec<u8>,
}
impl InitContext {
    /// Cancellation and retained cleanup boundary for explicitly owned executors.
    pub fn lifetime(&self) -> &AppContext {
        &self.lifetime
    }
    /// Read application-defined startup bytes. Decode and validate them in the
    /// initializer; the host never receives access to the resulting Rust values.
    pub fn startup_data(&self) -> &[u8] {
        &self.startup_data
    }
    /// Install the application's one host request handler during initialization.
    /// Requests run on the UI owner through the scoped delivery queue after
    /// activation. Return promptly; start scoped async work for longer operations.
    /// Requests and replies are bounded byte buffers with an app-defined encoding.
    /// Closing cancels queued requests. The handler is released before cleanup.
    pub fn on_host_request(
        &self,
        handler: impl Fn(Vec<u8>) -> AppResult<Vec<u8>> + 'static,
    ) -> AppResult {
        let inner = self
            .lifetime
            .inner
            .upgrade()
            .ok_or("application is closed")?;
        if inner.phase.get() != AppPhase::Preparing {
            return Err("register host requests during initialization".into());
        }
        let mut slot = inner.host_handler.borrow_mut();
        if slot.is_some() {
            return Err("host request handler already installed".into());
        }
        *slot = Some(Rc::new(handler));
        Ok(())
    }
}

/// The activation and teardown state of one cartridge instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppPhase {
    Preparing,
    Active,
    Closing,
    Closed,
}

/// Why an application is leaving its active host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    HostClosed,
    Replaced,
    PreparationDiscarded,
}

pub(crate) struct AppInner {
    pub(crate) phase: Cell<AppPhase>,
    target: TargetInfo,
    pub(crate) graph: PropertyGraph,
    dispatcher: AsyncDispatcher,
    scope: AsyncScope,
    initialized: Cell<bool>,
    startup_data: Vec<u8>,
    host_handler: RefCell<Option<HostHandler>>,
    retained: RefCell<Vec<Rc<dyn Any>>>,
    cleanup: RefCell<Vec<Rc<dyn Cleanup>>>,
    pub(crate) shutdown_ticket: RefCell<Option<ShutdownTicket>>,
    shutdown_deadline: Duration,
    shutdown_started: Cell<Option<Duration>>,
    deadline_reported: Cell<bool>,
    shutdown_errors: RefCell<Vec<String>>,
    close_waiters: RefCell<Vec<Weak<RefCell<Option<Waker>>>>>,
}

/// Weak access to the application that owns a node. Cloning this context does
/// not keep an application active. This context stays local and provides no dependency lookup.
#[derive(Clone)]
pub struct AppContext {
    inner: Weak<AppInner>,
}
impl AppContext {
    /// Host adapter entry: copy a request into the bounded UI delivery queue.
    /// The reply is called once on the owner thread, including rejection and
    /// cancellation. Adapters must defer user callbacks to prevent reentrancy
    /// into an engine that is delivering or revoking this request.
    #[doc(hidden)]
    pub fn request(&self, bytes: Vec<u8>, reply: impl FnOnce(Result<Vec<u8>, String>) + 'static) {
        let reply = Rc::new(HostReply(RefCell::new(Some(Box::new(reply)))));
        let enqueue = || -> Result<(), String> {
            if bytes.len() > HOST_MESSAGE_LIMIT {
                return Err("host request exceeds 64 KiB".into());
            }
            let inner = self.inner.upgrade().ok_or("application is closed")?;
            let handler = inner
                .host_handler
                .borrow()
                .clone()
                .ok_or("application has no host request handler")?;
            let scope = self.async_scope().map_err(|e| e.to_string())?;
            let delivered_reply = reply.clone();
            let completion = scope
                .completion(move |bytes| {
                    let result =
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handler(bytes)))
                            .map_err(|panic| crate::async_runtime::panic_message(panic))
                            .and_then(|result| result.map_err(|e| e.to_string()))
                            .and_then(|bytes| {
                                if bytes.len() <= HOST_MESSAGE_LIMIT {
                                    Ok(bytes)
                                } else {
                                    Err("host reply exceeds 64 KiB".into())
                                }
                            })
                            .map_err(|error| {
                                if error.len() <= HOST_MESSAGE_LIMIT {
                                    error
                                } else {
                                    "host error reply exceeds 64 KiB".into()
                                }
                            });
                    delivered_reply.finish(result);
                })
                .map_err(|e| e.to_string())?;
            completion
                .complete(bytes)
                .map_err(|_| "application is closed".to_string())
        };
        if let Err(error) = enqueue() {
            reply.finish(Err(error));
        }
    }
    /// Application lifetime, independent of individual mounted views.
    pub fn async_scope(&self) -> Result<AsyncScope, AsyncError> {
        let inner = self.inner.upgrade().ok_or(AsyncError::Closed)?;
        if matches!(inner.phase.get(), AppPhase::Closing | AppPhase::Closed) {
            return Err(AsyncError::Closed);
        }
        Ok(inner.scope.clone())
    }
    /// Observe unhandled task panics on the UI thread, including dropped controls.
    /// Reporting continues while managed shutdown retains the application. The
    /// sink must not touch detached UI state once the phase is Closing.
    pub fn set_task_error_sink(&self, sink: impl Fn(String) + 'static) -> Result<(), AsyncError> {
        let inner = self.inner.upgrade().ok_or(AsyncError::Closed)?;
        if matches!(inner.phase.get(), AppPhase::Closing | AppPhase::Closed) {
            return Err(AsyncError::Closed);
        }
        inner.dispatcher.set_error_sink(sink);
        Ok(())
    }
    /// Process-local identity of this application instance while retained.
    pub fn id(&self) -> Option<u64> {
        self.inner.upgrade().map(|inner| inner.graph.id())
    }
    /// Adapter integration: retain a cleanup owner until its acknowledgement.
    /// This is not a type registry; callers cannot look up retained resources.
    #[doc(hidden)]
    pub fn retain_cleanup(&self, cleanup: Rc<dyn Cleanup>) -> Result<(), AsyncError> {
        if let Some(inner) = self.inner.upgrade() {
            if !matches!(inner.phase.get(), AppPhase::Closing | AppPhase::Closed) {
                inner.cleanup.borrow_mut().push(cleanup);
                return Ok(());
            }
        }
        // Rejected ownership still needs a retained acknowledgement path. In
        // particular, dropping a newly supplied runtime must not unload its
        // cartridge while its off-thread destructor or workers are running.
        let orphan = ApplicationInstance::prepare(TargetInfo::default(), AppOptions::default())
            .expect("default lifecycle limits are valid");
        orphan.inner.cleanup.borrow_mut().push(cleanup);
        orphan.finish_close(StopReason::PreparationDiscarded);
        Err(AsyncError::Closed)
    }
    /// Current lifecycle state; a released owner reports Closed.
    pub fn phase(&self) -> AppPhase {
        self.inner
            .upgrade()
            .map_or(AppPhase::Closed, |inner| inner.phase.get())
    }
    /// Observe disposal completion without driving cleanup. The host's wake
    /// scheduler keeps cleanup progressing even when this future is not polled.
    pub fn closed(&self) -> AppClosed {
        let wake = Rc::new(RefCell::new(None));
        if let Some(inner) = self.inner.upgrade() {
            let mut waiters = inner.close_waiters.borrow_mut();
            waiters.retain(|waiter| waiter.strong_count() != 0);
            waiters.push(Rc::downgrade(&wake));
        }
        AppClosed {
            context: self.clone(),
            wake,
        }
    }
    /// Target facts selected by the owning host.
    pub fn target(&self) -> Result<TargetInfo, AsyncError> {
        self.inner
            .upgrade()
            .map(|inner| inner.target)
            .ok_or(AsyncError::Closed)
    }
}

/// Completion of retained application cleanup. This observer does not own the
/// application and never blocks the UI thread.
pub struct AppClosed {
    context: AppContext,
    wake: Rc<RefCell<Option<Waker>>>,
}
impl Future for AppClosed {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let Some(inner) = self.context.inner.upgrade() else {
            return Poll::Ready(());
        };
        if inner.phase.get() == AppPhase::Closed {
            return Poll::Ready(());
        }
        *self.wake.borrow_mut() = Some(cx.waker().clone());
        Poll::Pending
    }
}

thread_local! {
    static PREPARING_APP: RefCell<Option<Weak<AppInner>>> = const { RefCell::new(None) };
}

/// Host-owned application instance. Generated startup selects the application
/// before constructing root defaults; hosts activate it after a successful mount.
#[doc(hidden)]
#[derive(Clone)]
pub struct ApplicationInstance {
    pub(crate) inner: Rc<AppInner>,
}
impl Drop for ApplicationInstance {
    fn drop(&mut self) {
        if Rc::strong_count(&self.inner) == 1 && self.inner.phase.get() != AppPhase::Closed {
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let reason = if self.inner.phase.get() == AppPhase::Preparing {
                    StopReason::PreparationDiscarded
                } else {
                    StopReason::HostClosed
                };
                self.begin_close();
                self.finish_close(reason);
            }))
            .is_err()
            {
                log::error!("discarded application teardown panicked");
            }
        }
    }
}
impl ApplicationInstance {
    pub fn prepare(target: TargetInfo, options: AppOptions) -> AppResult<Self> {
        if options.startup_data.len() > HOST_MESSAGE_LIMIT {
            return Err("startup data exceeds 64 KiB".into());
        }
        if options.async_limits.callbacks_per_turn == 0
            || options.async_limits.callback_time.is_zero()
        {
            return Err("async delivery budgets must be positive".into());
        }
        let graph = PropertyGraph::new(|| {});
        let dispatcher = AsyncDispatcher::new(options.async_limits);
        let scope = dispatcher.scope()?;
        Ok(Self {
            inner: Rc::new(AppInner {
                phase: Cell::new(AppPhase::Preparing),
                target,
                graph,
                dispatcher,
                scope,
                initialized: Cell::new(false),
                startup_data: options.startup_data,
                host_handler: RefCell::new(None),
                retained: RefCell::new(Vec::new()),
                cleanup: RefCell::new(Vec::new()),
                shutdown_ticket: RefCell::new(None),
                shutdown_deadline: options.shutdown_deadline,
                shutdown_started: Cell::new(None),
                deadline_reported: Cell::new(false),
                shutdown_errors: RefCell::new(Vec::new()),
                close_waiters: RefCell::new(Vec::new()),
            }),
        })
    }
    /// Invoke the selected initializer once, preserving its actual return value.
    /// Failed or panicking initialization uses the same retained cleanup as close.
    pub fn initialize<R>(&self, init: impl FnOnce(&InitContext) -> AppResult<R>) -> AppResult<R> {
        if self.inner.phase.get() != AppPhase::Preparing || self.inner.initialized.replace(true) {
            return Err("application initializer can only run once before mounting".into());
        }
        let _entered = self.enter();
        let cx = InitContext {
            target: self.inner.target,
            lifetime: self.context(),
            startup_data: self.inner.startup_data.clone(),
        };
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| init(&cx))) {
            Ok(Ok(root)) => Ok(root),
            Ok(Err(error)) => {
                self.finish_close(StopReason::PreparationDiscarded);
                Err(error)
            }
            Err(panic) => {
                self.finish_close(StopReason::PreparationDiscarded);
                std::panic::resume_unwind(panic)
            }
        }
    }
    /// Keep the supplied root and its ordinary Rust dependencies through cleanup.
    /// Host code must pass the same allocation that is mounted, never a snapshot.
    pub fn retain_root<T: 'static>(&self, root: Rc<T>) {
        assert_eq!(self.inner.phase.get(), AppPhase::Preparing);
        self.inner.retained.borrow_mut().push(root);
    }
    pub fn for_runtime(target: TargetInfo) -> Self {
        PREPARING_APP
            .with(|app| app.borrow().as_ref().and_then(Weak::upgrade))
            .map(|inner| Self { inner })
            .unwrap_or_else(|| {
                Self::prepare(target, AppOptions::default())
                    .expect("empty configuration cannot fail")
            })
    }
    pub fn context(&self) -> AppContext {
        AppContext {
            inner: Rc::downgrade(&self.inner),
        }
    }
    pub fn dispatcher(&self) -> AsyncDispatcher {
        self.inner.dispatcher.clone()
    }
    pub fn has_pending_work(&self) -> bool {
        self.inner.graph.has_pending() || self.inner.dispatcher.has_pending()
    }
    pub fn set_waker(&self, wake: impl Fn() + Send + Sync + 'static) {
        let wake = std::sync::Arc::new(wake);
        let delivery_wake = wake.clone();
        self.inner.graph.set_waker(move || wake());
        self.inner.dispatcher.set_waker(move || delivery_wake());
    }
    pub fn property_graph(&self) -> PropertyGraph {
        self.inner.graph.clone()
    }
    pub fn enter(&self) -> ApplicationGuard {
        let previous = PREPARING_APP.with(|app| app.replace(Some(Rc::downgrade(&self.inner))));
        ApplicationGuard {
            previous,
            _graph: self.inner.graph.enter(),
        }
    }
    pub fn activate(&self) {
        if self.inner.phase.get() != AppPhase::Preparing {
            return;
        }
        self.inner.phase.set(AppPhase::Active);
        let _entered = self.enter();
        self.inner.dispatcher.activate();
    }
    pub fn begin_close(&self) -> bool {
        match self.inner.phase.get() {
            AppPhase::Closing | AppPhase::Closed => false,
            _ => {
                self.inner.phase.set(AppPhase::Closing);
                self.inner.dispatcher.close();
                self.inner.host_handler.borrow_mut().take();
                // Task completion can outlive the view's wake token. Retired
                // owners still report failures and release task records on the UI.
                let wake = application_shutdown::wake_route();
                self.inner.dispatcher.set_waker(move || wake());
                self.inner.graph.stop_publications();
                true
            }
        }
    }
    pub fn finish_close(&self, _reason: StopReason) {
        if self.inner.phase.get() == AppPhase::Closed || self.inner.shutdown_started.get().is_some()
        {
            return;
        }
        self.begin_close();
        self.inner.graph.close();
        self.inner.shutdown_started.set(Some(monotonic_now()));
        application_shutdown::retire(self.inner.clone());
        self.advance_shutdown();
        if self.inner.phase.get() != AppPhase::Closed {
            application_shutdown::deadline_wake(self.inner.shutdown_deadline);
        }
        application_shutdown::pump_shutdowns();
    }
    pub(crate) fn advance_shutdown(&self) {
        loop {
            // All delivery scopes are closed; only terminal task reporting runs.
            if let Err(error) = self.inner.dispatcher.drain(|| {}) {
                log::error!("task error sink panicked during shutdown: {error}");
            }
            let result = self
                .inner
                .shutdown_ticket
                .borrow()
                .as_ref()
                .map(ShutdownTicket::result);
            match result {
                Some(None) => break,
                Some(Some(result)) => {
                    self.inner.shutdown_ticket.borrow_mut().take();
                    if let Err(error) = result {
                        log::error!("application cleanup failed: {error}");
                        self.inner.shutdown_errors.borrow_mut().push(error);
                    }
                    self.inner.cleanup.borrow_mut().pop();
                }
                None => {}
            }
            let service = self.inner.cleanup.borrow().last().cloned();
            let Some(service) = service else {
                let retained = std::mem::take(&mut *self.inner.retained.borrow_mut());
                self.inner.phase.set(AppPhase::Closed);
                drop(retained);
                let waiters = std::mem::take(&mut *self.inner.close_waiters.borrow_mut());
                for waiter in waiters {
                    if let Some(slot) = waiter.upgrade() {
                        let wake = slot.borrow_mut().take();
                        if let Some(wake) = wake {
                            wake.wake();
                        }
                    }
                }
                break;
            };
            let ticket =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| service.begin_shutdown()))
                    .unwrap_or_else(|panic| {
                        let error = crate::async_runtime::panic_message(panic);
                        log::error!(
                            "application cleanup panicked; retaining dependencies: {error}"
                        );
                        self.inner.shutdown_errors.borrow_mut().push(error);
                        ShutdownTicket::unacknowledged()
                    });
            ticket.set_waker(application_shutdown::wake_route());
            *self.inner.shutdown_ticket.borrow_mut() = Some(ticket);
        }
        if self.inner.phase.get() != AppPhase::Closed
            && !self.inner.deadline_reported.get()
            && self.inner.shutdown_started.get().is_some_and(|started| {
                monotonic_now().saturating_sub(started) >= self.inner.shutdown_deadline
            })
        {
            self.inner.deadline_reported.set(true);
            log::error!("application shutdown deadline expired; resources and dependencies remain retained until acknowledgement");
        }
    }
}

/// Restores startup selection; never hold this guard across an await.
#[doc(hidden)]
pub struct ApplicationGuard {
    previous: Option<Weak<AppInner>>,
    _graph: PropertyGraphGuard,
}
impl Drop for ApplicationGuard {
    fn drop(&mut self) {
        PREPARING_APP.with(|app| {
            app.replace(self.previous.take());
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_requests_are_staged_bounded_and_cancelled_with_the_application() {
        let app = ApplicationInstance::prepare(
            TargetInfo::default(),
            AppOptions {
                startup_data: b"configured by caller".to_vec(),
                ..Default::default()
            },
        )
        .unwrap();
        let calls = Rc::new(Cell::new(0));
        app.initialize(|cx| {
            assert_eq!(cx.startup_data(), b"configured by caller");
            let calls = calls.clone();
            cx.on_host_request(move |bytes| {
                calls.set(calls.get() + 1);
                Ok(bytes)
            })?;
            assert!(cx.on_host_request(|bytes| Ok(bytes)).is_err());
            Ok(())
        })
        .unwrap();
        let replies = Rc::new(RefCell::new(Vec::new()));
        let observed = replies.clone();
        app.context().request(b"one".to_vec(), move |result| {
            observed.borrow_mut().push(result)
        });
        app.dispatcher().drain(|| {}).unwrap();
        assert_eq!(calls.get(), 0);
        app.activate();
        app.dispatcher().drain(|| {}).unwrap();
        assert_eq!(&*replies.borrow(), &[Ok(b"one".to_vec())]);
        let observed = replies.clone();
        app.context()
            .request(vec![0; HOST_MESSAGE_LIMIT + 1], move |result| {
                observed.borrow_mut().push(result)
            });
        assert!(replies.borrow()[1].as_ref().unwrap_err().contains("64 KiB"));
        let observed = replies.clone();
        app.context().request(b"cancel".to_vec(), move |result| {
            observed.borrow_mut().push(result)
        });
        app.finish_close(StopReason::HostClosed);
        assert_eq!(calls.get(), 1);
        assert_eq!(replies.borrow().len(), 3);
        assert!(replies.borrow()[2].is_err());
    }
    #[test]
    fn host_request_failure_is_observable_without_unwinding_into_host() {
        let app =
            ApplicationInstance::prepare(TargetInfo::default(), AppOptions::default()).unwrap();
        app.initialize(|cx| cx.on_host_request(|_| panic!("request failed")))
            .unwrap();
        app.activate();
        let reply = Rc::new(RefCell::new(None));
        let received = reply.clone();
        app.context().request(Vec::new(), move |result| {
            *received.borrow_mut() = Some(result)
        });
        app.dispatcher().drain(|| {}).unwrap();
        assert!(reply
            .borrow()
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap_err()
            .contains("request failed"));
    }
    #[test]
    fn host_error_replies_obey_the_boundary_size_limit() {
        let app =
            ApplicationInstance::prepare(TargetInfo::default(), AppOptions::default()).unwrap();
        app.initialize(|cx| cx.on_host_request(|_| Err("x".repeat(HOST_MESSAGE_LIMIT + 1).into())))
            .unwrap();
        app.activate();
        let reply = Rc::new(RefCell::new(None));
        let received = reply.clone();
        app.context().request(Vec::new(), move |result| {
            *received.borrow_mut() = Some(result)
        });
        app.dispatcher().drain(|| {}).unwrap();
        let reply = reply.borrow();
        let error = reply.as_ref().unwrap().as_ref().unwrap_err();
        assert_eq!(error, "host error reply exceeds 64 KiB");
    }
    #[test]
    fn initializer_runs_once_in_its_graph_and_preserves_property_identity() {
        let app =
            ApplicationInstance::prepare(TargetInfo::default(), AppOptions::default()).unwrap();
        let context = app.context();
        let calls = Cell::new(0);
        let root = app
            .initialize(|cx| {
                calls.set(calls.get() + 1);
                assert_eq!(cx.lifetime().id(), context.id());
                assert_eq!(
                    ApplicationInstance::for_runtime(cx.target).context().id(),
                    context.id()
                );
                Ok(crate::Property::new(42usize))
            })
            .unwrap();
        let shared = root.clone();
        shared.set(43);
        assert_eq!(root.get(), 43);
        assert!(app
            .initialize(|_| {
                calls.set(99);
                Ok(())
            })
            .is_err());
        assert_eq!(calls.get(), 1);
        app.activate();
        app.activate();
        assert_eq!(context.phase(), AppPhase::Active);
        app.finish_close(StopReason::HostClosed);
        assert_eq!(context.phase(), AppPhase::Closed);
    }
    #[test]
    fn context_does_not_own_application_and_dropped_preparation_closes() {
        let app =
            ApplicationInstance::prepare(TargetInfo::default(), AppOptions::default()).unwrap();
        let context = app.context();
        drop(app);
        assert_eq!(context.phase(), AppPhase::Closed);
        assert!(context.async_scope().is_err());
    }
}

impl crate::async_runtime::AsyncScopeSource for AppContext {
    fn work_scope(&self) -> Result<AsyncScope, AsyncError> {
        self.async_scope()
    }
}

mod handler_return {
    pub trait Sealed {}
    impl Sealed for () {}
}
/// Internal handler return bound used by generated event bindings.
///
/// ```compile_fail
/// async fn load() {}
/// pax_runtime_api::application::synchronous_handler(load());
/// ```
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "Pax event handlers must return () synchronously",
    note = "Start asynchronous work through an explicitly owned executor or scoped task helper; async fn handlers are not awaited."
)]
pub trait SynchronousHandlerReturn: handler_return::Sealed {}
impl SynchronousHandlerReturn for () {}
#[doc(hidden)]
pub fn synchronous_handler<T: SynchronousHandlerReturn>(_: T) {}
