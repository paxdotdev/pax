use pax_kit::*;
mod application;
mod request_panel;
mod work_service;
use application::{WorkbenchApplication, WorkbenchState};
#[allow(unused_imports)]
use request_panel::RequestPanel;

#[pax]
#[main]
#[application(WorkbenchApplication)]
#[file("lib.pax")]
pub struct AsyncWorkbench {
    pub clicks: Property<usize>,
    pub shared_value: Property<usize>,
    pub starts: Property<usize>,
    pub mounts: Property<usize>,
    pub phase: Property<String>,
    pub mounted: Property<bool>,
    pub backend: Property<String>,
    pub history: LocalProperty<String>,
}

impl AsyncWorkbench {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let state = ctx.application().service::<WorkbenchState>().unwrap();
        fn follow<T: pax_kit::api::properties::SharedPropertyValue>(
            target: &Property<T>,
            source: &Property<T>,
        ) {
            let local = source.local();
            let deps = [local.untyped()];
            target.replace_with(LocalProperty::computed(move || local.get(), &deps));
        }
        follow(&self.shared_value, &state.value);
        follow(&self.starts, &state.starts);
        follow(&self.mounts, &state.mounts);
        follow(&self.phase, &state.phase);
        let history = state.history.local();
        let deps = [history.untyped()];
        self.history.replace_with(LocalProperty::computed(
            move || {
                history
                    .get()
                    .iter()
                    .rev()
                    .take(12)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n")
            },
            &deps,
        ));
        self.mounted.set(true);
        self.backend.set(
            if ctx.platform == Platform::Web {
                "Browser futures · bounded delivery"
            } else {
                "Tokio · 2 workers · owned runtime"
            }
            .into(),
        );
    }
    pub fn click(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.clicks.update(|n| *n += 1);
    }
    pub fn toggle_panel(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.mounted.update(|mounted| *mounted = !*mounted);
    }
    pub fn publish(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        ctx.application()
            .service::<WorkbenchState>()
            .unwrap()
            .value
            .update(|n| *n += 1);
    }
    pub fn burst(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        let state = ctx.application().service::<WorkbenchState>().unwrap();
        let value = state.value.clone();
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::sync::atomic::Ordering;
            if state
                .burst_workers
                .compare_exchange(0, 2, Ordering::SeqCst, Ordering::SeqCst)
                .is_err()
            {
                return;
            }
            for _ in 0..2 {
                let value = value.clone();
                let workers = state.burst_workers.clone();
                std::thread::spawn(move || {
                    for _ in 0..500 {
                        value.update(|n| *n += 1);
                    }
                    workers.fetch_sub(1, Ordering::SeqCst);
                });
            }
        }
        #[cfg(target_arch = "wasm32")]
        for _ in 0..1000 {
            value.update(|n| *n += 1);
        }
        work_service::record(&state.history, "burst +1000 · raw application data".into());
    }
}
