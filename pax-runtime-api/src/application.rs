//! Application configuration and services, independent of any async executor.
use crate::application_shutdown;
use crate::async_runtime::monotonic_now;
use crate::properties::PropertyGraphGuard;
use crate::{async_runtime::AsyncDispatcher, AsyncError, AsyncLimits, AsyncScope};
use crate::{ManagedService, PropertyGraph, ShutdownTicket, TargetInfo};
use std::time::Duration;
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
};

/// A fallible application configuration result.
pub type AppResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Hooks selected by `#[application(Type)]` on the active root component.
/// Configuration runs before root defaults. Hooks run on the UI thread and
/// must not block it on network work or executor shutdown.
pub trait Application: 'static {
    /// Register services and configure process facilities before root defaults.
    fn configure(_app: &mut AppBuilder) -> AppResult {
        Ok(())
    }
    /// Called once after the host commits activation of the mounted instance.
    fn started(_app: &AppContext) {}
    /// Notify services during teardown; never wait for workers here.
    fn stopping(_app: &AppContext, _reason: StopReason) {}
}

/// The empty configuration used when the root omits the application attribute.
pub struct EmptyApplication;
impl Application for EmptyApplication {}

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

/// A service lookup or registration error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceError {
    Duplicate(&'static str),
    Missing(&'static str),
    Closed,
}
impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Duplicate(name) => write!(f, "application service {name} is already registered"),
            Self::Missing(name) => write!(f, "application service {name} is not registered"),
            Self::Closed => f.write_str("application is closed"),
        }
    }
}
impl std::error::Error for ServiceError {}

/// Configure immutable, application-wide service registrations.
pub struct AppBuilder {
    /// Target facts available before any node or root state exists.
    pub target: TargetInfo,
    graph: PropertyGraph,
    /// Bounded task and UI delivery resources for this application.
    pub async_limits: AsyncLimits,
    services: HashMap<TypeId, Rc<dyn Any>>,
    managed: Vec<Rc<dyn ManagedService>>,
    /// Time before the host reports unfinished teardown. Dependencies remain
    /// retained after expiry; this is not a cancellation guarantee.
    pub shutdown_deadline: Duration,
}
impl AppBuilder {
    /// Process-local identity allocated before configuration; never serialized.
    pub fn id(&self) -> u64 {
        self.graph.id()
    }
    /// Register a plain service. Its destructor runs on the UI thread; resources
    /// that can wait during destruction must not be registered as plain services.
    pub fn provide<T: 'static>(&mut self, service: T) -> Result<(), ServiceError> {
        let id = TypeId::of::<T>();
        if self.services.contains_key(&id) {
            return Err(ServiceError::Duplicate(std::any::type_name::<T>()));
        }
        self.services.insert(id, Rc::new(service));
        Ok(())
    }
    /// Register a service that can acknowledge asynchronous cleanup. Register
    /// dependencies before dependents; teardown runs in reverse order.
    pub fn provide_managed<T: ManagedService>(&mut self, service: T) -> Result<(), ServiceError> {
        let id = TypeId::of::<T>();
        if self.services.contains_key(&id) {
            return Err(ServiceError::Duplicate(std::any::type_name::<T>()));
        }
        let service = Rc::new(service);
        self.services.insert(id, service.clone());
        self.managed.push(service);
        Ok(())
    }
}

impl Drop for AppBuilder {
    fn drop(&mut self) {
        if self.managed.is_empty() {
            return;
        }
        let dispatcher = AsyncDispatcher::new(AsyncLimits::default());
        let scope = dispatcher.scope().expect("default scope limit");
        let app = ApplicationInstance {
            inner: Rc::new(AppInner {
                phase: Cell::new(AppPhase::Preparing),
                target: self.target,
                graph: self.graph.clone(),
                dispatcher,
                scope,
                services: RefCell::new(std::mem::take(&mut self.services)),
                managed: RefCell::new(std::mem::take(&mut self.managed)),
                started: EmptyApplication::started,
                stopping: EmptyApplication::stopping,
                shutdown_ticket: RefCell::new(None),
                shutdown_deadline: self.shutdown_deadline,
                shutdown_started: Cell::new(None),
                deadline_reported: Cell::new(false),
                shutdown_errors: RefCell::new(Vec::new()),
            }),
        };
        app.finish_close(StopReason::PreparationDiscarded);
    }
}

pub(crate) struct AppInner {
    pub(crate) phase: Cell<AppPhase>,
    target: TargetInfo,
    pub(crate) graph: PropertyGraph,
    dispatcher: AsyncDispatcher,
    scope: AsyncScope,
    services: RefCell<HashMap<TypeId, Rc<dyn Any>>>,
    started: fn(&AppContext),
    stopping: fn(&AppContext, StopReason),
    managed: RefCell<Vec<Rc<dyn ManagedService>>>,
    pub(crate) shutdown_ticket: RefCell<Option<ShutdownTicket>>,
    shutdown_deadline: Duration,
    shutdown_started: Cell<Option<Duration>>,
    deadline_reported: Cell<bool>,
    shutdown_errors: RefCell<Vec<String>>,
}

/// Weak access to the application that owns a node. Cloning this context does
/// not keep an application active. Service handles and this context stay local.
#[derive(Clone)]
pub struct AppContext {
    inner: Weak<AppInner>,
}
impl AppContext {
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
    /// Look up a registered concrete service without extending app authority.
    pub fn service<T: 'static>(&self) -> Result<Rc<T>, ServiceError> {
        let inner = self.inner.upgrade().ok_or(ServiceError::Closed)?;
        if inner.phase.get() == AppPhase::Closed {
            return Err(ServiceError::Closed);
        }
        let services = inner.services.borrow();
        services
            .get(&TypeId::of::<T>())
            .cloned()
            .ok_or(ServiceError::Missing(std::any::type_name::<T>()))?
            .downcast()
            .map_err(|_| ServiceError::Missing(std::any::type_name::<T>()))
    }
    /// Current lifecycle state; a released owner reports Closed.
    pub fn phase(&self) -> AppPhase {
        self.inner
            .upgrade()
            .map_or(AppPhase::Closed, |inner| inner.phase.get())
    }
    /// Target facts selected by the owning host.
    pub fn target(&self) -> Result<TargetInfo, ServiceError> {
        self.inner
            .upgrade()
            .map(|inner| inner.target)
            .ok_or(ServiceError::Closed)
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
    pub fn prepare<A: Application>(target: TargetInfo) -> AppResult<Self> {
        let graph = PropertyGraph::new(|| {});
        let _graph = graph.enter();
        let mut builder = AppBuilder {
            target,
            graph: graph.clone(),
            services: HashMap::new(),
            async_limits: AsyncLimits::default(),
            managed: Vec::new(),
            shutdown_deadline: Duration::from_secs(2),
        };
        A::configure(&mut builder)?;
        if builder.async_limits.callbacks_per_turn == 0
            || builder.async_limits.callback_time.is_zero()
        {
            return Err("async delivery budgets must be positive".into());
        }
        let dispatcher = AsyncDispatcher::new(builder.async_limits.clone());
        let scope = dispatcher.scope()?;
        Ok(Self {
            inner: Rc::new(AppInner {
                phase: Cell::new(AppPhase::Preparing),
                target,
                graph,
                dispatcher,
                scope,
                services: RefCell::new(std::mem::take(&mut builder.services)),
                managed: RefCell::new(std::mem::take(&mut builder.managed)),
                shutdown_ticket: RefCell::new(None),
                shutdown_deadline: builder.shutdown_deadline,
                shutdown_started: Cell::new(None),
                deadline_reported: Cell::new(false),
                shutdown_errors: RefCell::new(Vec::new()),
                started: A::started,
                stopping: A::stopping,
            }),
        })
    }
    pub fn for_runtime(target: TargetInfo) -> Self {
        PREPARING_APP
            .with(|app| app.borrow().as_ref().and_then(Weak::upgrade))
            .map(|inner| Self { inner })
            .unwrap_or_else(|| {
                Self::prepare::<EmptyApplication>(target).expect("empty configuration cannot fail")
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
        (self.inner.started)(&self.context());
        self.inner.dispatcher.activate();
    }
    pub fn begin_close(&self) -> bool {
        match self.inner.phase.get() {
            AppPhase::Closing | AppPhase::Closed => false,
            _ => {
                self.inner.phase.set(AppPhase::Closing);
                self.inner.dispatcher.close();
                // Task completion can outlive the view's wake token. Retired
                // owners still report failures and release task records on the UI.
                let wake = application_shutdown::wake_route();
                self.inner.dispatcher.set_waker(move || wake());
                self.inner.graph.stop_publications();
                true
            }
        }
    }
    pub fn finish_close(&self, reason: StopReason) {
        if self.inner.phase.get() == AppPhase::Closed || self.inner.shutdown_started.get().is_some()
        {
            return;
        }
        self.begin_close();
        let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (self.inner.stopping)(&self.context(), reason)
        }));
        self.inner.graph.close();
        self.inner.shutdown_started.set(Some(monotonic_now()));
        application_shutdown::retire(self.inner.clone());
        self.advance_shutdown();
        if self.inner.phase.get() != AppPhase::Closed {
            application_shutdown::deadline_wake(self.inner.shutdown_deadline);
        }
        application_shutdown::pump_shutdowns();
        if let Err(panic) = stopped {
            std::panic::resume_unwind(panic);
        }
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
                        log::error!("managed service shutdown failed: {error}");
                        self.inner.shutdown_errors.borrow_mut().push(error);
                    }
                    self.inner.managed.borrow_mut().pop();
                }
                None => {}
            }
            let service = self.inner.managed.borrow().last().cloned();
            let Some(service) = service else {
                let services = std::mem::take(&mut *self.inner.services.borrow_mut());
                self.inner.phase.set(AppPhase::Closed);
                drop(services);
                break;
            };
            let ticket =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| service.begin_shutdown()))
                    .unwrap_or_else(|panic| {
                        let error = crate::async_runtime::panic_message(panic);
                        log::error!("managed shutdown panicked; retaining dependencies: {error}");
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
            log::error!("application shutdown deadline expired; services and dependencies remain retained until acknowledgement");
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
    thread_local! { static CALLS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) }; }
    struct Hooks;
    struct Config(u32);
    impl Application for Hooks {
        fn configure(app: &mut AppBuilder) -> AppResult {
            CALLS.with(|calls| calls.borrow_mut().push("configure"));
            app.provide(Config(42))?;
            Ok(())
        }
        fn started(app: &AppContext) {
            assert_eq!(app.phase(), AppPhase::Active);
            assert_eq!(app.service::<Config>().unwrap().0, 42);
            CALLS.with(|calls| calls.borrow_mut().push("started"));
        }
        fn stopping(app: &AppContext, _: StopReason) {
            assert_eq!(app.phase(), AppPhase::Closing);
            assert_eq!(app.service::<Config>().unwrap().0, 42);
            CALLS.with(|calls| calls.borrow_mut().push("stopping"));
        }
    }
    #[test]
    fn configure_precedes_root_construction_and_activation_is_once() {
        CALLS.with(|calls| calls.borrow_mut().clear());
        let app = ApplicationInstance::prepare::<Hooks>(TargetInfo::default()).unwrap();
        let context = app.context();
        {
            let _entered = app.enter();
            let runtime_app = ApplicationInstance::for_runtime(TargetInfo::default());
            assert_eq!(runtime_app.context().phase(), AppPhase::Preparing);
            assert_eq!(runtime_app.context().service::<Config>().unwrap().0, 42);
            CALLS.with(|calls| calls.borrow_mut().push("root"));
        }
        app.activate();
        app.activate();
        app.begin_close();
        app.finish_close(StopReason::HostClosed);
        app.finish_close(StopReason::HostClosed);
        assert_eq!(context.phase(), AppPhase::Closed);
        assert!(matches!(
            context.service::<Config>(),
            Err(ServiceError::Closed)
        ));
        CALLS.with(|calls| {
            assert_eq!(
                *calls.borrow(),
                ["configure", "root", "started", "stopping"]
            )
        });
    }
    #[test]
    fn duplicate_services_fail_and_context_does_not_own_application() {
        struct Duplicate;
        impl Application for Duplicate {
            fn configure(app: &mut AppBuilder) -> AppResult {
                app.provide(1_u32)?;
                app.provide(2_u32)?;
                Ok(())
            }
        }
        assert!(ApplicationInstance::prepare::<Duplicate>(TargetInfo::default()).is_err());
        let app = ApplicationInstance::prepare::<EmptyApplication>(TargetInfo::default()).unwrap();
        let context = app.context();
        assert!(matches!(
            context.service::<u32>(),
            Err(ServiceError::Missing(_))
        ));
        drop(app);
        assert_eq!(context.phase(), AppPhase::Closed);
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
    note = "Start asynchronous work through an application service or scoped task helper; async fn handlers are not awaited."
)]
pub trait SynchronousHandlerReturn: handler_return::Sealed {}
impl SynchronousHandlerReturn for () {}
#[doc(hidden)]
pub fn synchronous_handler<T: SynchronousHandlerReturn>(_: T) {}
