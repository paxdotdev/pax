use pax_kit::*;

pub struct WorkbenchApplication;

#[derive(Default)]
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

impl Application for WorkbenchApplication {
    fn configure(app: &mut AppBuilder) -> AppResult {
        crate::work_service::configure(app)?;
        app.provide(WorkbenchState {
            application_id: app.id(),
            phase: Property::new("Prepared before root mount".into()),
            ..Default::default()
        })?;
        Ok(())
    }
    fn started(app: &AppContext) {
        let state = app
            .service::<WorkbenchState>()
            .expect("registered during configure");
        state.starts.update(|count| *count += 1);
        state
            .phase
            .set("Active · setup once per application".into());
    }
    fn stopping(app: &AppContext, _: StopReason) {
        let state = app
            .service::<WorkbenchState>()
            .expect("services remain readable while stopping");
        state.phase.set("Closing".into());
    }
}
