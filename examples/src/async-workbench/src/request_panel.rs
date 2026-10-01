use crate::{
    application::WorkbenchState,
    work_service::{self, BoundTasks, Executor},
};
use pax_kit::*;

#[pax]
#[file("request_panel.pax")]
pub struct RequestPanel {
    pub value: Property<usize>,
    pub description: LocalProperty<String>,
    pub progress: Property<f64>,
    pub result: Property<String>,
    pub task_status: LocalProperty<String>,
    pub stream_count: LocalProperty<usize>,
}
struct PanelSession {
    scope: AsyncScope,
    executor: std::rc::Rc<Executor>,
    operation: Option<AsyncScope>,
    task: Option<TaskControl>,
    subscription: Option<UiSubscription>,
    sequence: usize,
    mount: usize,
    application: u64,
    history: Property<Vec<String>>,
}
impl Store for PanelSession {}
impl PanelSession {
    fn replace(&mut self, result: &Property<String>, label: &str) -> (AsyncScope, BoundTasks) {
        self.sequence += 1;
        self.subscription.take();
        let scope = self
            .scope
            .replace(
                &mut self.operation,
                result,
                format!("Loading · {label} #{}", self.sequence),
            )
            .unwrap();
        self.task.take();
        self.log(format!("{label} started"));
        let tasks = self.executor.for_scope(scope.clone()).unwrap();
        (scope, tasks)
    }
    fn log(&self, message: String) {
        work_service::record(
            &self.history,
            format!(
                "app {} / mount {} / op {} · {message}",
                self.application, self.mount, self.sequence
            ),
        );
    }
    fn observe(&mut self, task: TaskControl, display: &LocalProperty<String>) {
        let status = task.status_property().local();
        let deps = [status.untyped()];
        display.replace_with(LocalProperty::computed(
            move || format!("Task: {:?}", status.get()),
            &deps,
        ));
        self.task = Some(task);
    }
}
impl RequestPanel {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let app = ctx.application();
        let state = app.service::<WorkbenchState>().unwrap();
        state.mounts.update(|n| *n += 1);
        let shared = self.value.clone();
        let value = shared.local();
        let deps = [value.untyped()];
        self.description.replace_with(LocalProperty::computed(
            move || {
                format!(
                    "Shared → local: {} · revisions observed {} / published {}",
                    value.get(),
                    shared.observed_revision().unwrap_or(0),
                    shared.revision()
                )
            },
            &deps,
        ));
        self.result.set("Ready to start a request".into());
        self.task_status.set("No task running".into());
        let scope = ctx.async_scope().unwrap();
        let session = PanelSession {
            scope,
            executor: app.service::<Executor>().unwrap(),
            operation: None,
            task: None,
            subscription: None,
            sequence: 0,
            mount: state.mounts.get(),
            application: state.application_id,
            history: state.history.clone(),
        };
        session.log("mounted".into());
        ctx.provide_store(session)
            .expect("request panel is mounting");
    }
    pub fn run(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        ctx.with_store::<PanelSession, _>(|session| {
            let (_, tasks) = session.replace(&self.result, "timer");
            let task = tasks
                .spawn_into(&self.result, async {
                    work_service::delay(600).await;
                    "Ready · 600 ms timer completed".into()
                })
                .unwrap();
            session.observe(task, &self.task_status);
        })
        .unwrap();
    }
    pub fn fail(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        ctx.with_store::<PanelSession, _>(|session| {
            let (_, tasks) = session.replace(&self.result, "controlled error");
            let result = self.result.clone();
            let task = tasks
                .spawn_with_completion(
                    async {
                        work_service::delay(600).await;
                        Err::<String, _>("Fixture error: retry is safe".to_owned())
                    },
                    move |outcome| {
                        result.set(match outcome {
                            TaskOutcome::Completed(Ok(value)) => value,
                            TaskOutcome::Completed(Err(error)) => error,
                            TaskOutcome::Panicked(error) => format!("Task panic: {error}"),
                        });
                    },
                )
                .unwrap();
            session.observe(task, &self.task_status);
        })
        .unwrap();
    }
    pub fn replace(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        ctx.with_store::<PanelSession, _>(|session| {
            let (old, _) = session.replace(&self.result, "slow A");
            let stale = old.publisher(&self.result).unwrap();
            let (_, tasks) = session.replace(&self.result, "fast B");
            let task = tasks
                .spawn_into(&self.result, async {
                    work_service::delay(200).await;
                    "Ready · B wins; A has been revoked".into()
                })
                .unwrap();
            session.observe(task, &self.task_status);
            // Deliberately retain A's producer in independent monitored work.
            // It proves revocation even when the old producer ignores cancellation.
            let history = session.history.clone();
            session
                .executor
                .for_scope(session.scope.clone())
                .unwrap()
                .spawn(async move {
                    work_service::delay(900).await;
                    let rejected = stale.set("Incorrect stale result A".into()).is_err();
                    work_service::record(
                        &history,
                        format!("retained A publication rejected: {rejected}"),
                    );
                })
                .unwrap();
        })
        .unwrap();
    }
    pub fn cancel(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        ctx.with_store::<PanelSession, _>(|session| {
            if let Some(operation) = session.operation.take() {
                operation.cancel();
            }
            session.subscription.take();
            self.result
                .set("Cancelled · result authority revoked".into());
            session.log("cancelled".into());
        })
        .unwrap();
    }
    pub fn stream(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        self.stream_count.set(0);
        self.progress.set(0.0);
        ctx.with_store::<PanelSession, _>(|session| {
            let (scope, tasks) = session.replace(&self.result, "bounded stream");
            let count = self.stream_count.clone();
            let result = self.result.clone();
            let history = session.history.clone();
            let (sender, subscription) = scope
                .channel(4, move |n: usize| {
                    assert_eq!(n, count.get() + 1, "lossless channel ordering");
                    count.set(n);
                    if n == 100 {
                        result.set("Ready · all 100 events delivered in order".into());
                        work_service::record(&history, "stream delivered 100 / 100".into());
                    }
                })
                .unwrap();
            // Fill it before the worker starts: the fifth item must report Full.
            for n in 1..=4 {
                sender.try_send(n).unwrap();
            }
            assert!(matches!(sender.try_send(5), Err(TrySendError::Full(5))));
            session.log("capacity 4: Full observed, producer will await".into());
            let progress = scope.publisher(&self.progress).unwrap();
            let task = tasks
                .spawn(async move {
                    for n in 5..=100 {
                        if sender.send(n).await.is_err() {
                            break;
                        }
                        if progress.set(n as f64 / 100.0).is_err() {
                            break;
                        }
                        work_service::delay(8).await;
                    }
                })
                .unwrap();
            session.subscription = Some(subscription);
            session.observe(task, &self.task_status);
        })
        .unwrap();
    }
    pub fn io(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        ctx.with_store::<PanelSession, _>(|session| {
            let (_, tasks) = session.replace(&self.result, "local I/O");
            let task = tasks
                .spawn_into(&self.result, async {
                    work_service::delay(200).await;
                    match work_service::local_io().await {
                        Ok(value) => format!("Ready · {value}"),
                        Err(error) => format!("I/O error · {error}"),
                    }
                })
                .unwrap();
            session.observe(task, &self.task_status);
        })
        .unwrap();
    }
    pub fn plain_thread(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        ctx.with_store::<PanelSession, _>(|session| {
            let (scope, _) = session.replace(&self.result, "plain completion");
            let local = self.stream_count.clone();
            let result = self.result.clone();
            let completion = scope
                .completion(move |value| {
                    local.set(value);
                    result.set("Ready · local callback ran on the UI owner".into());
                })
                .unwrap();
            #[cfg(not(target_arch = "wasm32"))]
            {
                let task = scope
                    .register_task(move |reporter| {
                        std::thread::spawn(move || {
                            let _ = completion.complete(42usize);
                            reporter.finish(TaskStatus::Completed);
                        });
                        Box::new(|| {})
                    })
                    .unwrap();
                session.observe(task, &self.task_status);
            }
            #[cfg(target_arch = "wasm32")]
            {
                completion.complete(42usize).unwrap();
                self.task_status
                    .set("Browser callback queued locally".into());
            }
        })
        .unwrap();
    }
    pub fn progress_burst(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        self.progress.set(0.0);
        ctx.with_store::<PanelSession, _>(|session| {
            let (scope, tasks) = session.replace(&self.result, "progress burst");
            let progress = scope.publisher(&self.progress).unwrap();
            let task = tasks
                .spawn_into(&self.result, async move {
                    for n in 1..=1000 {
                        if progress.set(n as f64 / 1000.0).is_err() {
                            break;
                        }
                    }
                    "Ready · 1000 progress publications; intermediate frames may coalesce".into()
                })
                .unwrap();
            session.observe(task, &self.task_status);
        })
        .unwrap();
    }
    pub fn blocking(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::sync::{
                atomic::{AtomicBool, AtomicUsize, Ordering},
                Arc,
            };
            struct Slot(Arc<AtomicUsize>);
            impl std::ops::Drop for Slot {
                fn drop(&mut self) {
                    self.0.store(0, Ordering::SeqCst);
                }
            }
            let state = ctx.application().service::<WorkbenchState>().unwrap();
            if state
                .blocking_jobs
                .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
                .is_err()
            {
                self.result
                    .set("One blocking probe is already running".into());
                return;
            }
            let slot = Slot(state.blocking_jobs.clone());
            ctx.with_store::<PanelSession, _>(|session| {
                let (scope, _) = session.replace(&self.result, "cooperative blocking work");
                let result = scope.publisher(&self.result).unwrap();
                let handle = session.executor.handle().unwrap();
                let cancelled = Arc::new(AtomicBool::new(false));
                let stop = cancelled.clone();
                let task = scope
                    .register_task(move |reporter| {
                        let job = handle.spawn_blocking(move || {
                            let _slot = slot;
                            for _ in 0..60 {
                                if cancelled.load(Ordering::SeqCst) {
                                    reporter.finish(TaskStatus::Cancelled);
                                    return;
                                }
                                std::thread::sleep(std::time::Duration::from_millis(10));
                            }
                            let status = if result
                                .set("Ready · bounded blocking work finished".into())
                                .is_ok()
                            {
                                TaskStatus::Completed
                            } else {
                                TaskStatus::Cancelled
                            };
                            reporter.finish(status);
                        });
                        let abort = job.abort_handle();
                        drop(job);
                        Box::new(move || {
                            stop.store(true, Ordering::SeqCst);
                            abort.abort();
                        })
                    })
                    .unwrap();
                session.observe(task, &self.task_status);
            })
            .unwrap();
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = ctx;
            self.result.set(
                "Blocking work is native-only; browser futures must keep the UI thread free."
                    .into(),
            );
        }
    }
    pub fn panic_task(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        #[cfg(not(target_arch = "wasm32"))]
        ctx.with_store::<PanelSession, _>(|session| {
            let (_, tasks) = session.replace(&self.result, "panic probe");
            let task = tasks
                .spawn(async {
                    work_service::delay(200).await;
                    panic!("expected workbench task panic");
                })
                .unwrap();
            session.observe(task, &self.task_status);
        })
        .unwrap();
        #[cfg(target_arch = "wasm32")]
        {
            let _ = ctx;
            self.result.set("Panic probe is native-only; this Wasm build aborts on panic. Use Fail for a recoverable error.".into());
        }
    }
}
