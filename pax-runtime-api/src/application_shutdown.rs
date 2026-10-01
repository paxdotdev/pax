//! Asynchronous service cleanup with UI-thread ownership and host wakeups.
use crate::{
    application::{AppInner, ApplicationInstance},
    async_runtime::lock,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

type Wake = Arc<dyn Fn() + Send + Sync>;
struct TicketState {
    result: Option<Result<(), String>>,
    wake: Wake,
}
/// Nonblocking acknowledgement of managed cleanup. A pending ticket retains its
/// service and all earlier dependencies; a deadline never fabricates completion.
/// An explicit Err acknowledges quiescence with a cleanup error. Dropping the
/// signal is not acknowledgement and leaves dependencies retained.
pub struct ShutdownTicket(Arc<Mutex<TicketState>>);
/// Transferable signal. Dropping it without acknowledgement reports failure.
pub struct ShutdownComplete(Option<Arc<Mutex<TicketState>>>);
impl ShutdownTicket {
    pub fn pending() -> (Self, ShutdownComplete) {
        let state = Arc::new(Mutex::new(TicketState {
            result: None,
            wake: Arc::new(|| {}),
        }));
        (Self(state.clone()), ShutdownComplete(Some(state)))
    }
    pub fn ready(result: Result<(), String>) -> Self {
        Self(Arc::new(Mutex::new(TicketState {
            result: Some(result),
            wake: Arc::new(|| {}),
        })))
    }
    pub(crate) fn set_waker(&self, wake: Wake) {
        let ready = {
            let mut state = lock(&self.0);
            state.wake = wake.clone();
            state.result.is_some()
        };
        if ready {
            wake();
        }
    }
    pub(crate) fn unacknowledged() -> Self {
        Self(Arc::new(Mutex::new(TicketState {
            result: None,
            wake: Arc::new(|| {}),
        })))
    }
    pub fn result(&self) -> Option<Result<(), String>> {
        lock(&self.0).result.clone()
    }
}
impl ShutdownComplete {
    pub fn complete(mut self, result: Result<(), String>) {
        if let Some(state) = self.0.take() {
            let wake = {
                let mut state = lock(&state);
                state.result = Some(result);
                state.wake.clone()
            };
            wake();
        }
    }
}
impl Drop for ShutdownComplete {
    fn drop(&mut self) {
        if let Some(state) = self.0.take() {
            let wake = lock(&state).wake.clone();
            log::error!("shutdown signal dropped without acknowledgement; retaining dependencies");
            wake();
        }
    }
}
/// A service whose teardown may wait. `begin_shutdown` runs once on the UI
/// thread and must immediately reject new work, start cleanup without blocking,
/// and return a ticket. Dependencies registered earlier remain allocated until
/// all later services acknowledge. Destruction itself must never block the UI.
pub trait ManagedService: 'static {
    fn begin_shutdown(&self) -> ShutdownTicket;
}

thread_local! {
    static RETIRED: RefCell<HashMap<u64, Rc<AppInner>>> = RefCell::new(HashMap::new());
    static HOST_WAKE: RefCell<Wake> = RefCell::new(Arc::new(|| {}));
}
pub(crate) fn retire(inner: Rc<AppInner>) {
    RETIRED.with(|retired| retired.borrow_mut().insert(inner.graph.id(), inner));
}
pub(crate) fn wake_route() -> Wake {
    HOST_WAKE.with(|wake| wake.borrow().clone())
}

/// Install a host-owned UI scheduler before configuring a cartridge. Its token
/// remains valid for retired applications after engine/window disposal. The
/// callback schedules `pump_shutdowns`; it must not call it synchronously.
#[doc(hidden)]
pub fn set_shutdown_waker(wake: impl Fn() + Send + Sync + 'static) {
    let wake: Wake = Arc::new(wake);
    HOST_WAKE.with(|host| *host.borrow_mut() = wake.clone());
    let retired: Vec<_> = RETIRED.with(|retired| retired.borrow().values().cloned().collect());
    for inner in retired {
        if let Some(ticket) = inner.shutdown_ticket.borrow().as_ref() {
            ticket.set_waker(wake.clone());
        }
    }
    if pending_shutdowns() != 0 {
        wake();
    }
}
/// Advance acknowledged services in reverse registration order on the owner
/// thread. A deadline is diagnostic; unfinished applications remain retained.
#[doc(hidden)]
pub fn pump_shutdowns() -> usize {
    let retired: Vec<_> = RETIRED.with(|retired| retired.borrow().values().cloned().collect());
    for inner in retired {
        let app = ApplicationInstance { inner };
        app.advance_shutdown();
        if app.inner.phase.get() == crate::AppPhase::Closed {
            RETIRED.with(|retired| retired.borrow_mut().remove(&app.inner.graph.id()));
        }
    }
    pending_shutdowns()
}
#[doc(hidden)]
pub fn pending_shutdowns() -> usize {
    RETIRED.with(|retired| retired.borrow().len())
}

// One deadline notification, never a periodic executor poll. Retired native
// cartridges remain loaded while this closure or any service can still run.
pub(crate) fn deadline_wake(delay: Duration) {
    let wake = wake_route();
    #[cfg(not(target_arch = "wasm32"))]
    if let Err(error) = std::thread::Builder::new()
        .name("pax-shutdown-deadline".into())
        .spawn(move || {
            std::thread::sleep(delay);
            wake();
        })
    {
        log::error!("could not schedule shutdown deadline: {error}");
    }
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;
        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = globalThis, js_name = setTimeout)]
            fn set_timeout(callback: &JsValue, millis: i32);
        }
        let callback = wasm_bindgen::closure::Closure::once_into_js(move || wake());
        set_timeout(&callback, delay.as_millis().min(i32::MAX as u128) as i32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AppBuilder, AppContext, AppPhase, AppResult, Application, ManagedService, StopReason,
        TargetInfo,
    };
    thread_local! {
        static EVENTS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
        static SIGNAL: RefCell<Option<ShutdownComplete>> = const { RefCell::new(None) };
    }
    struct Dependency;
    impl ManagedService for Dependency {
        fn begin_shutdown(&self) -> ShutdownTicket {
            EVENTS.with(|events| events.borrow_mut().push("dependency shutdown"));
            ShutdownTicket::ready(Ok(()))
        }
    }
    impl Drop for Dependency {
        fn drop(&mut self) {
            EVENTS.with(|events| events.borrow_mut().push("dependency drop"));
        }
    }
    struct Dependent;
    impl ManagedService for Dependent {
        fn begin_shutdown(&self) -> ShutdownTicket {
            EVENTS.with(|events| events.borrow_mut().push("dependent shutdown"));
            let (ticket, signal) = ShutdownTicket::pending();
            SIGNAL.with(|slot| *slot.borrow_mut() = Some(signal));
            ticket
        }
    }
    struct Hooks;
    impl Application for Hooks {
        fn configure(app: &mut AppBuilder) -> AppResult {
            app.shutdown_deadline = Duration::ZERO;
            app.provide_managed(Dependency)?;
            app.provide_managed(Dependent)?;
            Ok(())
        }
        fn stopping(app: &AppContext, _: StopReason) {
            assert!(app.async_scope().is_err());
            assert!(app.service::<Dependency>().is_ok());
            EVENTS.with(|events| events.borrow_mut().push("stopping"));
        }
    }
    #[test]
    fn shutdown_is_sequential_and_retains_dependencies_after_deadline_and_engine_drop() {
        EVENTS.with(|events| events.borrow_mut().clear());
        let app = ApplicationInstance::prepare::<Hooks>(TargetInfo::default()).unwrap();
        let context = app.context();
        app.activate();
        app.begin_close();
        app.finish_close(StopReason::HostClosed);
        drop(app);
        assert_eq!(context.phase(), AppPhase::Closing);
        EVENTS.with(|events| assert_eq!(*events.borrow(), ["stopping", "dependent shutdown"]));
        assert_eq!(pump_shutdowns(), 1);
        SIGNAL.with(|signal| signal.borrow_mut().take().unwrap().complete(Ok(())));
        assert_eq!(pump_shutdowns(), 0);
        assert_eq!(context.phase(), AppPhase::Closed);
        EVENTS.with(|events| {
            assert_eq!(
                *events.borrow(),
                [
                    "stopping",
                    "dependent shutdown",
                    "dependency shutdown",
                    "dependency drop"
                ]
            )
        });
    }
    #[test]
    fn retired_app_reports_late_task_panics_through_the_independent_host_wake() {
        let wakes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let woke = wakes.clone();
        set_shutdown_waker(move || {
            woke.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        let app = ApplicationInstance::prepare::<Hooks>(TargetInfo::default()).unwrap();
        let errors = Rc::new(RefCell::new(Vec::new()));
        let observed = errors.clone();
        app.context()
            .set_task_error_sink(move |error| observed.borrow_mut().push(error))
            .unwrap();
        app.activate();
        let reporter = Rc::new(RefCell::new(None));
        let capture = reporter.clone();
        let task = app
            .context()
            .async_scope()
            .unwrap()
            .register_task(move |signal| {
                *capture.borrow_mut() = Some(signal);
                Box::new(|| {})
            })
            .unwrap();
        drop(task);
        app.begin_close();
        app.finish_close(StopReason::HostClosed);
        drop(app);
        let before = wakes.load(std::sync::atomic::Ordering::SeqCst);
        let reporter = reporter.borrow_mut().take().unwrap();
        std::thread::spawn(move || {
            reporter.finish(crate::TaskStatus::Panicked("late task failure".into()))
        })
        .join()
        .unwrap();
        assert!(wakes.load(std::sync::atomic::Ordering::SeqCst) > before);
        SIGNAL.with(|signal| signal.borrow_mut().take().unwrap().complete(Ok(())));
        assert_eq!(pump_shutdowns(), 0);
        assert_eq!(*errors.borrow(), ["late task failure"]);
        set_shutdown_waker(|| {});
    }
    #[test]
    fn failed_configuration_uses_the_same_retained_cleanup_order() {
        EVENTS.with(|events| events.borrow_mut().clear());
        struct Fails;
        impl Application for Fails {
            fn configure(app: &mut AppBuilder) -> AppResult {
                app.provide_managed(Dependency)?;
                app.provide_managed(Dependent)?;
                Err("expected configuration failure".into())
            }
        }
        assert!(ApplicationInstance::prepare::<Fails>(TargetInfo::default()).is_err());
        EVENTS.with(|events| assert_eq!(*events.borrow(), ["dependent shutdown"]));
        SIGNAL.with(|signal| signal.borrow_mut().take().unwrap().complete(Ok(())));
        assert_eq!(pump_shutdowns(), 0);
        EVENTS.with(|events| {
            assert_eq!(
                *events.borrow(),
                [
                    "dependent shutdown",
                    "dependency shutdown",
                    "dependency drop"
                ]
            )
        });
    }
}
