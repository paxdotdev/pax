//! Optional Tokio integration for native Pax applications, without a custom main.
//!
//! Register [`TokioService`] with `AppBuilder::provide_managed`. Tasks use an
//! explicit runtime handle and a Pax lifetime. No Tokio context is entered around
//! UI callbacks. Browser applications use their browser executor instead.
#![cfg(not(target_arch = "wasm32"))]
use futures_util::FutureExt;
use pax_runtime_api::{
    async_runtime::{panic_message, AsyncScopeSource},
    properties::SharedPropertyValue,
    AppContext, AppResult, AsyncError, AsyncScope, ManagedService, Property, ShutdownComplete,
    ShutdownTicket, TaskControl, TaskOutcome, TaskStatus,
};
use std::{
    cell::RefCell,
    future::Future,
    panic::AssertUnwindSafe,
    sync::{mpsc, Arc, Mutex},
};
use tokio::runtime::{Handle, Runtime, RuntimeFlavor};

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}
#[derive(Default)]
struct Tracking {
    closed: bool,
    active: usize,
    next_id: u64,
    aborts: std::collections::HashMap<u64, tokio::task::AbortHandle>,
    completed: Option<ShutdownComplete>,
}
struct Running(Arc<Mutex<Tracking>>, u64);
impl Drop for Running {
    fn drop(&mut self) {
        let signal = {
            let mut tracking = lock(&self.0);
            tracking.active -= 1;
            tracking.aborts.remove(&self.1);
            if tracking.active == 0 {
                tracking.completed.take()
            } else {
                None
            }
        };
        if let Some(signal) = signal {
            signal.complete(Ok(()));
        }
    }
}

/// Application-owned Tokio client. Owned mode holds the runtime on a dedicated
/// owner thread so even fallback destruction never waits on the UI thread.
/// Borrowed mode never shuts down the external runtime; its owner must keep it
/// alive and continuously driven, including when it has current-thread flavor.
pub struct TokioService {
    handle: Handle,
    shutdown: RefCell<Option<mpsc::Sender<ShutdownComplete>>>,
    tracking: Arc<Mutex<Tracking>>,
}
impl TokioService {
    /// Own a multithread runtime. A current-thread runtime needs a continuously
    /// driven service thread and the borrowed-handle constructor instead.
    pub fn owned(runtime: Runtime) -> AppResult<Self> {
        if runtime.handle().runtime_flavor() != RuntimeFlavor::MultiThread {
            // Consuming an invalid runtime must not block the UI either.
            runtime.shutdown_background();
            return Err("TokioService::owned requires a multi-thread runtime; continuously drive a current-thread runtime externally and use borrowed(handle)".into());
        }
        let handle = runtime.handle().clone();
        let (shutdown, requests) = mpsc::channel::<ShutdownComplete>();
        // Keep a fallback owner outside the closure in case thread creation
        // fails before the runtime can be transferred to that thread.
        let transfer = Arc::new(Mutex::new(Some(runtime)));
        let worker = transfer.clone();
        if let Err(error) = std::thread::Builder::new()
            .name("pax-tokio-owner".into())
            .spawn(move || {
                let runtime = lock(&worker).take().unwrap();
                let signal = requests.recv().ok();
                drop(runtime);
                if let Some(signal) = signal {
                    signal.complete(Ok(()));
                }
            })
        {
            if let Some(runtime) = lock(&transfer).take() {
                runtime.shutdown_background();
            }
            return Err(error.into());
        }
        Ok(Self {
            handle,
            shutdown: RefCell::new(Some(shutdown)),
            tracking: Arc::new(Mutex::new(Tracking::default())),
        })
    }
    /// Borrow an explicitly supplied, continuously driven runtime. A Handle
    /// alone does not keep the Runtime alive and does not drive current-thread I/O.
    pub fn borrowed(handle: Handle) -> Self {
        Self {
            handle,
            shutdown: RefCell::new(None),
            tracking: Arc::new(Mutex::new(Tracking::default())),
        }
    }
    /// Advanced untracked work: raw tasks are neither staged nor cancelled by
    /// Pax scopes. Prefer the bound helpers for component/application work.
    pub fn handle(&self) -> Result<Handle, AsyncError> {
        if lock(&self.tracking).closed {
            Err(AsyncError::Closed)
        } else {
            Ok(self.handle.clone())
        }
    }
    pub fn for_application(&self, app: &AppContext) -> Result<BoundTasks, AsyncError> {
        self.for_scope(app.async_scope()?)
    }
    /// Uses this exact node's context, not the component owning a handler's self.
    pub fn for_node(&self, node: &impl AsyncScopeSource) -> Result<BoundTasks, AsyncError> {
        self.for_scope(node.work_scope()?)
    }
    pub fn for_scope(&self, scope: AsyncScope) -> Result<BoundTasks, AsyncError> {
        self.handle()?;
        if scope.is_closed() {
            return Err(AsyncError::Closed);
        }
        Ok(BoundTasks {
            handle: self.handle.clone(),
            tracking: self.tracking.clone(),
            scope,
        })
    }
}
impl ManagedService for TokioService {
    fn begin_shutdown(&self) -> ShutdownTicket {
        let mut tracking = lock(&self.tracking);
        if tracking.closed {
            return ShutdownTicket::ready(Err("Tokio service shutdown was requested twice".into()));
        }
        tracking.closed = true;
        let aborts: Vec<_> = tracking.aborts.values().cloned().collect();
        let (ticket, completed) = ShutdownTicket::pending();
        if let Some(shutdown) = self.shutdown.borrow_mut().take() {
            drop(tracking);
            // The runtime owner acknowledges only after Runtime::drop returns,
            // including any started blocking operations that cannot be aborted.
            if let Err(error) = shutdown.send(completed) {
                error
                    .0
                    .complete(Err("Tokio runtime owner exited unexpectedly".into()));
            }
        } else if tracking.active == 0 {
            drop(tracking);
            completed.complete(Ok(()));
        } else {
            tracking.completed = Some(completed);
            drop(tracking);
        }
        for abort in aborts {
            abort.abort();
        }
        ticket
    }
}
impl Drop for TokioService {
    fn drop(&mut self) {
        lock(&self.tracking).closed = true;
        // Disconnecting the owner also initiates cleanup after failed setup.
        // Its Runtime and any waiting destructor remain on that owner thread.
        self.shutdown.get_mut().take();
    }
}

/// UI-local bound executor. Each spawn creates an independently cancellable
/// operation below the owner. Keep a component-bound helper from on_mount when
/// later event contexts belong to controls inside that component.
#[derive(Clone)]
pub struct BoundTasks {
    handle: Handle,
    tracking: Arc<Mutex<Tracking>>,
    scope: AsyncScope,
}
impl BoundTasks {
    fn operation(&self) -> Result<AsyncScope, AsyncError> {
        if lock(&self.tracking).closed {
            return Err(AsyncError::Closed);
        }
        self.scope.child()
    }
    fn start(
        &self,
        operation: AsyncScope,
        future: impl Future<Output = TaskStatus> + Send + 'static,
    ) -> Result<TaskControl, AsyncError> {
        let handle = self.handle.clone();
        let tracking = self.tracking.clone();
        operation.register_task(move |reporter| {
            let id = {
                let mut state = lock(&tracking);
                if state.closed {
                    reporter.finish(TaskStatus::Cancelled);
                    return Box::new(|| {});
                }
                state.active += 1;
                state.next_id += 1;
                state.next_id
            };
            let running = Running(tracking.clone(), id);
            let mut state = lock(&tracking);
            let job = handle.spawn(future);
            let abort = job.abort_handle();
            state.aborts.insert(id, abort.clone());
            drop(state);
            // Monitor the JoinHandle separately so a panic while dropping an
            // aborted future is still observable, not mistaken for cancellation.
            let monitor = handle.spawn(async move {
                let _running = running;
                let status = match job.await {
                    Ok(status) => status,
                    Err(error) if error.is_panic() => {
                        TaskStatus::Panicked(panic_message(error.into_panic()))
                    }
                    Err(_) => TaskStatus::Cancelled,
                };
                reporter.finish(status);
            });
            drop(monitor);
            Box::new(move || abort.abort())
        })
    }
    /// Monitor ordinary work. Raw Property clones captured here remain writable
    /// after cancellation; use spawn_into for revocable result publication.
    pub fn spawn(
        &self,
        future: impl Future<Output = ()> + Send + 'static,
    ) -> Result<TaskControl, AsyncError> {
        self.start(self.operation()?, async move {
            future.await;
            TaskStatus::Completed
        })
    }
    /// Publish the returned value only if this task's operation is still live.
    /// The future receives no raw clone of the target.
    pub fn spawn_into<T: SharedPropertyValue>(
        &self,
        target: &Property<T>,
        future: impl Future<Output = T> + Send + 'static,
    ) -> Result<TaskControl, AsyncError> {
        let operation = self.operation()?;
        let publisher = operation.publisher(target)?;
        self.start(operation, async move {
            if publisher.set(future.await).is_ok() {
                TaskStatus::Completed
            } else {
                TaskStatus::Cancelled
            }
        })
    }
    /// Return a result or task panic to a UI-local closure, never running that
    /// closure inline on the executor. Cancellation suppresses queued delivery.
    pub fn spawn_with_completion<T: Send + 'static>(
        &self,
        future: impl Future<Output = T> + Send + 'static,
        callback: impl FnOnce(TaskOutcome<T>) + 'static,
    ) -> Result<TaskControl, AsyncError> {
        let operation = self.operation()?;
        let completion = operation.completion(callback)?;
        self.start(operation, async move {
            let (outcome, status) = match AssertUnwindSafe(future).catch_unwind().await {
                Ok(value) => (TaskOutcome::Completed(value), TaskStatus::Completed),
                Err(panic) => {
                    let error = panic_message(panic);
                    (
                        TaskOutcome::Panicked(error.clone()),
                        TaskStatus::Panicked(error),
                    )
                }
            };
            let _ = completion.complete(outcome);
            status
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pax_runtime_api::async_runtime::AsyncDispatcher;
    use pax_runtime_api::AsyncLimits;
    use std::{cell::Cell, rc::Rc, sync::mpsc, time::Duration};
    fn runtime() -> Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap()
    }
    fn wait_until(mut ready: impl FnMut() -> bool) {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !ready() {
            assert!(std::time::Instant::now() < deadline, "timed out");
            std::thread::yield_now();
        }
    }
    #[test]
    fn managed_owned_timer_publishes_without_a_ui_tokio_context() {
        let dispatcher = AsyncDispatcher::new(AsyncLimits::default());
        let scope = dispatcher.scope().unwrap();
        let service = TokioService::owned(runtime()).unwrap();
        let tasks = service.for_scope(scope.clone()).unwrap();
        let result = Property::new(0usize);
        let task = tasks
            .spawn_into(&result, async {
                tokio::time::sleep(Duration::from_millis(1)).await;
                42
            })
            .unwrap();
        assert_eq!(task.status(), TaskStatus::Staged);
        assert_eq!(result.get(), 0);
        dispatcher.activate();
        wait_until(|| task.status() == TaskStatus::Completed);
        assert_eq!(result.get(), 42);
        scope.cancel();
        let ticket = service.begin_shutdown();
        wait_until(|| ticket.result().is_some());
        assert_eq!(ticket.result(), Some(Ok(())));
    }
    #[test]
    fn cancel_suppresses_callback_and_borrowed_shutdown_leaves_runtime_alive() {
        let runtime = runtime();
        let service = TokioService::borrowed(runtime.handle().clone());
        let dispatcher = AsyncDispatcher::new(AsyncLimits::default());
        let scope = dispatcher.scope().unwrap();
        dispatcher.activate();
        let calls = Rc::new(Cell::new(0));
        let captured = calls.clone();
        let (started, observed) = mpsc::channel();
        let task = service
            .for_scope(scope.clone())
            .unwrap()
            .spawn_with_completion(
                async move {
                    started.send(()).unwrap();
                    std::future::pending::<usize>().await
                },
                move |_| captured.set(captured.get() + 1),
            )
            .unwrap();
        observed.recv_timeout(Duration::from_secs(2)).unwrap();
        task.cancel();
        wait_until(|| task.status() == TaskStatus::Cancelled);
        dispatcher.drain(|| {}).unwrap();
        assert_eq!(calls.get(), 0);
        scope.cancel();
        let ticket = service.begin_shutdown();
        wait_until(|| ticket.result().is_some());
        assert_eq!(runtime.block_on(async { 9 }), 9);
    }
    #[test]
    fn panic_is_observable_in_control_and_ui_completion() {
        let service = TokioService::owned(runtime()).unwrap();
        let dispatcher = AsyncDispatcher::new(AsyncLimits::default());
        let scope = dispatcher.scope().unwrap();
        dispatcher.activate();
        let result = Rc::new(RefCell::new(None));
        let captured = result.clone();
        let task = service
            .for_scope(scope.clone())
            .unwrap()
            .spawn_with_completion(
                async {
                    panic!("expected task panic");
                    #[allow(unreachable_code)]
                    1usize
                },
                move |outcome| *captured.borrow_mut() = Some(outcome),
            )
            .unwrap();
        wait_until(|| matches!(task.status(), TaskStatus::Panicked(_)));
        dispatcher.drain(|| {}).unwrap();
        assert!(
            matches!(result.borrow().as_ref(), Some(TaskOutcome::Panicked(error)) if error == "expected task panic")
        );
        scope.cancel();
        let ticket = service.begin_shutdown();
        wait_until(|| ticket.result().is_some());
    }
    #[test]
    fn panic_during_aborted_future_drop_remains_observable() {
        struct PanicOnDrop;
        impl Drop for PanicOnDrop {
            fn drop(&mut self) {
                panic!("expected abort drop panic");
            }
        }
        let service = TokioService::owned(runtime()).unwrap();
        let dispatcher = AsyncDispatcher::new(AsyncLimits::default());
        let scope = dispatcher.scope().unwrap();
        dispatcher.activate();
        let (started, observed) = mpsc::channel();
        let task = service
            .for_scope(scope.clone())
            .unwrap()
            .spawn(async move {
                let _guard = PanicOnDrop;
                started.send(()).unwrap();
                std::future::pending::<()>().await;
            })
            .unwrap();
        observed.recv_timeout(Duration::from_secs(2)).unwrap();
        task.cancel();
        wait_until(|| matches!(task.status(), TaskStatus::Panicked(_)));
        assert_eq!(
            task.status(),
            TaskStatus::Panicked("expected abort drop panic".into())
        );
        scope.cancel();
        let ticket = service.begin_shutdown();
        wait_until(|| ticket.result().is_some());
    }
    #[test]
    fn owned_current_thread_runtime_is_rejected() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        assert!(TokioService::owned(runtime).is_err());
    }
    #[test]
    fn native_loopback_io_is_driven_on_workers() {
        let service = TokioService::owned(runtime()).unwrap();
        let dispatcher = AsyncDispatcher::new(AsyncLimits::default());
        let scope = dispatcher.scope().unwrap();
        dispatcher.activate();
        let output = Property::new(String::new());
        let task = service
            .for_scope(scope.clone())
            .unwrap()
            .spawn_into(&output, async {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                tokio::time::timeout(Duration::from_secs(2), async {
                    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                    let address = listener.local_addr().unwrap();
                    let server = async {
                        let (mut stream, _) = listener.accept().await.unwrap();
                        stream.write_all(b"pax loopback").await.unwrap();
                    };
                    let client = async {
                        let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
                        let mut value = String::new();
                        stream.read_to_string(&mut value).await.unwrap();
                        value
                    };
                    let (_, value) = tokio::join!(server, client);
                    value
                })
                .await
                .unwrap()
            })
            .unwrap();
        wait_until(|| task.status() == TaskStatus::Completed);
        assert_eq!(output.get(), "pax loopback");
        scope.cancel();
        let ticket = service.begin_shutdown();
        wait_until(|| ticket.result().is_some());
    }
    #[test]
    fn dedicated_current_thread_localset_retains_non_send_state_off_ui() {
        let (handle_sender, handle_receiver) = mpsc::sync_channel(1);
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let published = Property::new(0usize);
        let worker_value = published.clone();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let local = tokio::task::LocalSet::new();
            handle_sender.send(runtime.handle().clone()).unwrap();
            local.block_on(&runtime, async move {
                let local_state = Rc::new(Cell::new(40));
                tokio::task::spawn_local(async move {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                    worker_value.set(local_state.get() + 2);
                });
                let _ = stopped.await;
            });
        });
        let service = TokioService::borrowed(handle_receiver.recv().unwrap());
        let dispatcher = AsyncDispatcher::new(AsyncLimits::default());
        let scope = dispatcher.scope().unwrap();
        dispatcher.activate();
        let value = Property::new(0usize);
        let task = service
            .for_scope(scope.clone())
            .unwrap()
            .spawn_into(&value, async {
                tokio::time::sleep(Duration::from_millis(1)).await;
                7
            })
            .unwrap();
        wait_until(|| task.status() == TaskStatus::Completed && published.get() == 42);
        assert_eq!(value.get(), 7);
        scope.cancel();
        let ticket = service.begin_shutdown();
        wait_until(|| ticket.result().is_some());
        stop.send(()).unwrap();
        thread.join().unwrap();
    }
    #[test]
    fn blocking_work_keeps_owned_shutdown_pending_until_actual_completion() {
        let service = TokioService::owned(runtime()).unwrap();
        let (started, start) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        service.handle().unwrap().spawn_blocking(move || {
            started.send(()).unwrap();
            wait.recv().unwrap();
        });
        start.recv_timeout(Duration::from_secs(2)).unwrap();
        let ticket = service.begin_shutdown();
        assert!(ticket.result().is_none());
        release.send(()).unwrap();
        wait_until(|| ticket.result().is_some());
        assert_eq!(ticket.result(), Some(Ok(())));
    }
}
