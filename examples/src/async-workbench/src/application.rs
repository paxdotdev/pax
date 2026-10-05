use pax_kit::*;

use crate::{work_service::Executor, AsyncWorkbench};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct WorkbenchResources {
    pub state: Rc<WorkbenchState>,
    pub executor: Executor,
}
impl Store for WorkbenchResources {}

#[derive(Default, Debug)]
pub struct WorkbenchState {
    pub value: Property<usize>,
    pub starts: Property<usize>,
    pub mounts: Property<usize>,
    pub phase: Property<String>,
    pub history: Property<Vec<String>>,
    pub application_id: u64,
    #[cfg(not(target_arch = "wasm32"))]
    pub blocking_jobs: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    #[cfg(not(target_arch = "wasm32"))]
    pub burst_workers: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

/// Root construction runs once inside the application's property graph.
pub fn init(cx: &InitContext) -> AppResult<AsyncWorkbench> {
    let executor = crate::work_service::executor(cx)?;
    let state = Rc::new(WorkbenchState {
        application_id: cx.lifetime().id().expect("initializer has an owner"),
        starts: Property::new(1),
        value: Property::new(if cx.startup_data().is_empty() {
            0
        } else {
            std::str::from_utf8(cx.startup_data())?.parse()?
        }),
        phase: Property::new("Loading · initialized before root mount".into()),
        ..Default::default()
    });
    let value = state.value.clone();
    cx.on_host_request(move |bytes| {
        match bytes.as_slice() {
            b"increment" => value.set(value.get() + 1),
            b"read" => {}
            _ => return Err("expected UTF-8 increment or read".into()),
        }
        Ok(value.get().to_string().into_bytes())
    })?;
    executor
        .for_application(cx.lifetime())?
        .spawn_into(&state.phase, async {
            crate::work_service::delay(600).await;
            "Ready · async initialization completed".to_owned()
        })?;
    Ok(AsyncWorkbench {
        mounted: Property::new(true),
        shared_value: state.value.clone(),
        starts: state.starts.clone(),
        mounts: state.mounts.clone(),
        phase: state.phase.clone(),
        resources: Some(Rc::new(WorkbenchResources { state, executor })),
        ..Default::default()
    })
}
