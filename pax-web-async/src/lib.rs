//! Scoped browser futures with the same publication/delivery contracts as Pax's
//! native adapter. This crate does not provide Tokio networking in browser Wasm.
#![cfg(any(target_arch = "wasm32", doc))]
use futures_util::{
    future::{AbortHandle, Abortable},
    FutureExt,
};
use pax_runtime_api::{
    async_runtime::{panic_message, AsyncScopeSource},
    properties::SharedPropertyValue,
    AppContext, AsyncError, AsyncScope, ManagedService, Property, ShutdownComplete, ShutdownTicket,
    TaskControl, TaskOutcome, TaskStatus,
};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    panic::AssertUnwindSafe,
    rc::Rc,
};

#[derive(Default)]
struct Inner {
    closed: Cell<bool>,
    next_id: Cell<u64>,
    aborts: RefCell<std::collections::HashMap<u64, AbortHandle>>,
    shutdown: RefCell<Option<ShutdownComplete>>,
}
struct Running(Rc<Inner>, u64);
impl Drop for Running {
    fn drop(&mut self) {
        self.0.aborts.borrow_mut().remove(&self.1);
        if self.0.aborts.borrow().is_empty() {
            let completed = self.0.shutdown.borrow_mut().take();
            if let Some(completed) = completed {
                completed.complete(Ok(()));
            }
        }
    }
}
/// Managed browser executor service. Futures stay on the browser's thread;
/// component cancellation aborts their polling and revokes result authority.
#[derive(Default)]
pub struct BrowserService(Rc<Inner>);
impl BrowserService {
    pub fn for_application(&self, app: &AppContext) -> Result<BoundTasks, AsyncError> {
        self.for_scope(app.async_scope()?)
    }
    pub fn for_node(&self, node: &impl AsyncScopeSource) -> Result<BoundTasks, AsyncError> {
        self.for_scope(node.work_scope()?)
    }
    pub fn for_scope(&self, scope: AsyncScope) -> Result<BoundTasks, AsyncError> {
        if self.0.closed.get() || scope.is_closed() {
            return Err(AsyncError::Closed);
        }
        Ok(BoundTasks {
            inner: self.0.clone(),
            scope,
        })
    }
}
impl ManagedService for BrowserService {
    fn begin_shutdown(&self) -> ShutdownTicket {
        if self.0.closed.replace(true) {
            return ShutdownTicket::ready(Err("browser executor shutdown requested twice".into()));
        }
        if self.0.aborts.borrow().is_empty() {
            return ShutdownTicket::ready(Ok(()));
        }
        let (ticket, completed) = ShutdownTicket::pending();
        *self.0.shutdown.borrow_mut() = Some(completed);
        let aborts: Vec<_> = self.0.aborts.borrow().values().cloned().collect();
        for abort in aborts {
            abort.abort();
        }
        ticket
    }
}
/// Local bound executor. Dropping a task control does not detach its lifetime.
#[derive(Clone)]
pub struct BoundTasks {
    inner: Rc<Inner>,
    scope: AsyncScope,
}
impl BoundTasks {
    fn operation(&self) -> Result<AsyncScope, AsyncError> {
        if self.inner.closed.get() {
            return Err(AsyncError::Closed);
        }
        self.scope.child()
    }
    fn start(
        &self,
        operation: AsyncScope,
        future: impl Future<Output = TaskStatus> + 'static,
    ) -> Result<TaskControl, AsyncError> {
        let inner = self.inner.clone();
        operation.register_task(move |reporter| {
            if inner.closed.get() {
                reporter.finish(TaskStatus::Cancelled);
                return Box::new(|| {});
            }
            let id = inner
                .next_id
                .get()
                .checked_add(1)
                .expect("browser task identity exhausted");
            inner.next_id.set(id);
            let (abort, registration) = AbortHandle::new_pair();
            inner.aborts.borrow_mut().insert(id, abort.clone());
            let running = Running(inner, id);
            wasm_bindgen_futures::spawn_local(async move {
                let _running = running;
                match Abortable::new(AssertUnwindSafe(future).catch_unwind(), registration).await {
                    Ok(result) => reporter.finish(
                        result.unwrap_or_else(|panic| TaskStatus::Panicked(panic_message(panic))),
                    ),
                    Err(_) => reporter.finish(TaskStatus::Cancelled),
                }
            });
            Box::new(move || abort.abort())
        })
    }
    pub fn spawn(
        &self,
        future: impl Future<Output = ()> + 'static,
    ) -> Result<TaskControl, AsyncError> {
        self.start(self.operation()?, async move {
            future.await;
            TaskStatus::Completed
        })
    }
    pub fn spawn_into<T: SharedPropertyValue>(
        &self,
        target: &Property<T>,
        future: impl Future<Output = T> + 'static,
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
    pub fn spawn_with_completion<T: Send + 'static>(
        &self,
        future: impl Future<Output = T> + 'static,
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
