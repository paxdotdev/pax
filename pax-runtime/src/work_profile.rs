//! Explicit-window attribution for reactive effects, callbacks, and retained rendering.

use crate::{node_inspection::NodeInspection, RuntimeContext};
use pax_runtime_api::properties::{ReactiveWorkCounts, ReactiveWorkProfile};
use serde::Serialize;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

#[derive(Default)]
pub(crate) struct WorkProfile {
    pub active: bool,
    // Taken while draining so no RefCell borrow crosses application callbacks.
    pub reactive: Option<ReactiveWorkProfile>,
    pub lifecycle: HashMap<(u32, &'static str), LifecycleWork>,
    pub scopes: HashMap<(u32, &'static str), ScopedWork>,
    pub rendering: HashMap<(usize, usize, Option<u32>, &'static str), RenderWorkCounts>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pax_runtime_api::{Platform, RenderWorkObservation, OS};

    #[test]
    fn render_observer_is_weak_and_capture_windows_reset() {
        let engine = crate::test_support::empty_engine(
            (320., 240.),
            Platform::Web,
            OS::Mac,
            Box::new(|| 0),
            crate::layer_tiling::ScrollerTilingPolicy::default(),
        );
        let runtime = &engine.runtime_context;
        let observer = runtime.render_work_observer();
        let emit = || {
            observer(RenderWorkObservation {
                layer: 2,
                surface: 3,
                node: Some(999),
                operation: "test render",
                count: 1,
                pixels: 64,
                cpu_ms: 0.25,
            })
        };
        emit();
        assert!(!runtime.begin_work_profile());
        runtime.enable_work_profiling();
        emit();
        assert!(runtime.take_work_profile().is_none());
        assert!(runtime.begin_work_profile());
        emit();
        emit();
        let report = runtime.take_work_profile().unwrap();
        assert_eq!(report.rendering.len(), 1);
        let row = &report.rendering[0];
        assert_eq!((row.layer, row.surface, row.node), (2, 3, Some(999)));
        assert_eq!(
            (row.work.count, row.work.pixels, row.work.cpu_ms),
            (2, 128, 0.5)
        );
        assert!(
            report.nodes.is_empty(),
            "unknown nodes are omitted without evaluation"
        );
        emit();
        assert!(runtime.begin_work_profile());
        assert!(runtime.take_work_profile().unwrap().rendering.is_empty());
        assert!(runtime.begin_work_profile());
        emit();
        assert!(runtime.begin_work_profile());
        assert!(runtime.take_work_profile().unwrap().rendering.is_empty());
        let weak = Rc::downgrade(runtime);
        drop(engine);
        assert!(
            weak.upgrade().is_none(),
            "the sink must not keep the runtime alive"
        );
        emit();
    }
}

/// Aggregated backend work within an explicit capture. Pixel counts estimate
/// processed area, not elapsed GPU time; nested spans must not be summed.
#[derive(Default, Serialize)]
pub struct RenderWorkCounts {
    pub count: u64,
    pub pixels: u64,
    pub cpu_ms: f64,
}

/// Backend work attributed to a physical surface and, when known, an engine node.
#[derive(Serialize)]
pub struct RenderWorkRow {
    pub layer: usize,
    pub surface: usize,
    pub node: Option<u32>,
    pub operation: &'static str,
    pub work: RenderWorkCounts,
}

/// Inclusive CPU work for one lifecycle target and phase. Context
/// construction, handler execution, and teardown are disjoint; dependency reads
/// belong to their caller.
#[derive(Default, Serialize)]
pub struct LifecycleWork {
    pub calls: usize,
    pub context_ms: f64,
    pub handler_ms: f64,
    pub teardown_ms: f64,
    pub max_ms: f64,
}

/// Inclusive timing for a named runtime operation, including nested dependency reads.
#[derive(Default, Serialize)]
pub struct ScopedWork {
    pub calls: usize,
    pub inclusive_ms: f64,
}

/// An opt-in timing scope. Dropping it records work only while capture is active.
pub struct WorkProfileScope {
    context: Weak<RuntimeContext>,
    node: u32,
    name: &'static str,
    started: web_time::Instant,
}

impl Drop for WorkProfileScope {
    fn drop(&mut self) {
        let elapsed = self.started.elapsed().as_secs_f64() * 1000.;
        let Some(context) = self.context.upgrade() else {
            return;
        };
        let mut slot = context.work_profile.borrow_mut();
        if let Some(profile) = slot.as_mut().filter(|p| p.active) {
            let work = profile.scopes.entry((self.node, self.name)).or_default();
            work.calls += 1;
            work.inclusive_ms += elapsed;
        }
    }
}

/// A callback target, with source identity when it is still mounted at capture end.
#[derive(Serialize)]
pub struct LifecycleWorkRow {
    pub node: u32,
    pub phase: &'static str,
    pub identity: Option<NodeInspection>,
    pub work: LifecycleWork,
}

/// Explicit-window wall-clock attribution. These inclusive spans overlap tick
/// phase timings and must not be added to them. Effects are queued evaluations;
/// nested dependency reads are included but not counted separately.
#[derive(Serialize)]
pub struct WorkProfileReport {
    pub reactive_totals: ReactiveWorkCounts,
    pub effect_categories: Vec<(String, ReactiveWorkCounts)>,
    /// Up to 128 property labels ranked by inclusive evaluator time.
    pub slowest_properties: Vec<(String, ReactiveWorkCounts)>,
    pub omitted_properties: usize,
    /// All lifecycle targets, ranked by context, handler, and teardown time.
    pub lifecycle: Vec<LifecycleWorkRow>,
    /// Nested operation timings; these overlap queued effects and lifecycle spans.
    pub scopes: Vec<(u32, &'static str, ScopedWork)>,
    /// Geometry, capture reuse, surface encoding and lighting invalidations.
    pub rendering: Vec<RenderWorkRow>,
    /// Source identities for named scopes and backend observations. Missing
    /// entries have unmounted before the capture ended.
    pub nodes: Vec<NodeInspection>,
}

impl RuntimeContext {
    /// A weak diagnostic sink, scoped by begin/take_work_profile. Emission after
    /// the capture or runtime lifetime is harmless and never evaluates geometry.
    pub fn render_work_observer(self: &Rc<Self>) -> pax_runtime_api::RenderWorkObserver {
        let context = Rc::downgrade(self);
        Rc::new(move |sample| {
            let Some(context) = context.upgrade() else {
                return;
            };
            let mut slot = context.work_profile.borrow_mut();
            let Some(profile) = slot.as_mut().filter(|p| p.active) else {
                return;
            };
            let work = profile
                .rendering
                .entry((sample.layer, sample.surface, sample.node, sample.operation))
                .or_default();
            work.count += sample.count;
            work.pixels += sample.pixels;
            work.cpu_ms += sample.cpu_ms;
        })
    }

    pub(crate) fn record_node_work(&self, node: u32, name: &'static str, elapsed_ms: f64) {
        if let Some(profile) = self.work_profile.borrow_mut().as_mut().filter(|p| p.active) {
            let work = profile.scopes.entry((node, name)).or_default();
            work.calls += 1;
            work.inclusive_ms += elapsed_ms;
        }
    }
    /// Time a named operation during an explicitly enabled capture. Returns None
    /// without reading the clock when inactive. Scopes must not cross capture windows.
    pub fn profile_node_work(
        self: &Rc<Self>,
        node: u32,
        name: &'static str,
    ) -> Option<WorkProfileScope> {
        self.work_profile_active().then(|| WorkProfileScope {
            context: Rc::downgrade(self),
            node,
            name,
            started: web_time::Instant::now(),
        })
    }
    /// Enable CPU-work capture before mounting to retain complete diagnostic names.
    /// Collection begins only after `begin_work_profile`, in debug or release builds.
    pub fn enable_work_profiling(&self) {
        self.property_graph.set_diagnostic_names_enabled(true);
        *self.work_profile.borrow_mut() = Some(WorkProfile::default());
    }

    pub(crate) fn work_profiling_enabled(&self) -> bool {
        self.work_profile.borrow().is_some()
    }

    pub(crate) fn work_profile_active(&self) -> bool {
        self.work_profile
            .borrow()
            .as_ref()
            .is_some_and(|p| p.active)
    }

    /// Start or reset an explicit capture window. Returns false unless enabled.
    pub fn begin_work_profile(&self) -> bool {
        let mut slot = self.work_profile.borrow_mut();
        let Some(profile) = slot.as_mut() else {
            return false;
        };
        *profile = WorkProfile {
            active: true,
            reactive: Some(ReactiveWorkProfile::default()),
            ..Default::default()
        };
        true
    }

    pub(crate) fn record_lifecycle_work(
        &self,
        node: u32,
        phase: &'static str,
        context_ms: f64,
        handler_ms: f64,
        teardown_ms: f64,
    ) {
        let mut slot = self.work_profile.borrow_mut();
        let Some(profile) = slot.as_mut().filter(|p| p.active) else {
            return;
        };
        let work = profile.lifecycle.entry((node, phase)).or_default();
        work.calls += 1;
        work.context_ms += context_ms;
        work.handler_ms += handler_ms;
        work.teardown_ms += teardown_ms;
        work.max_ms = work.max_ms.max(context_ms + handler_ms + teardown_ms);
    }

    /// Stop and consume the current capture. Returns None when inactive. Source
    /// inspection does not evaluate geometry; names remain available for another window.
    pub fn take_work_profile(&self) -> Option<WorkProfileReport> {
        let profile = {
            let mut slot = self.work_profile.borrow_mut();
            let profile = slot.as_mut().filter(|p| p.active)?;
            std::mem::take(profile)
        };
        let reactive = profile.reactive.unwrap_or_default().into_report(usize::MAX);
        let mut properties = reactive.top_properties;
        properties.sort_by(|a, b| {
            b.1.inclusive_ms
                .total_cmp(&a.1.inclusive_ms)
                .then(a.0.cmp(&b.0))
        });
        let omitted_properties = properties.len().saturating_sub(128);
        properties.truncate(128);
        let mut lifecycle: Vec<_> = profile
            .lifecycle
            .into_iter()
            .map(|((node, phase), work)| LifecycleWorkRow {
                node,
                phase,
                identity: self.inspect_node(node),
                work,
            })
            .collect();
        lifecycle.sort_by(|a, b| {
            (b.work.context_ms + b.work.handler_ms + b.work.teardown_ms)
                .total_cmp(&(a.work.context_ms + a.work.handler_ms + a.work.teardown_ms))
                .then(a.node.cmp(&b.node))
                .then(a.phase.cmp(b.phase))
        });
        let mut scopes: Vec<_> = profile
            .scopes
            .into_iter()
            .map(|((node, name), work)| (node, name, work))
            .collect();
        scopes.sort_by(|a, b| {
            b.2.inclusive_ms
                .total_cmp(&a.2.inclusive_ms)
                .then(a.0.cmp(&b.0))
                .then(a.1.cmp(b.1))
        });
        let mut rendering: Vec<_> = profile
            .rendering
            .into_iter()
            .map(|((layer, surface, node, operation), work)| RenderWorkRow {
                layer,
                surface,
                node,
                operation,
                work,
            })
            .collect();
        rendering.sort_by_key(|row| (row.layer, row.surface, row.node, row.operation));
        let mut node_ids: Vec<_> = scopes
            .iter()
            .map(|row| row.0)
            .chain(rendering.iter().filter_map(|row| row.node))
            .collect();
        node_ids.sort_unstable();
        node_ids.dedup();
        let nodes = node_ids
            .into_iter()
            .filter_map(|id| self.inspect_node(id))
            .collect();
        Some(WorkProfileReport {
            reactive_totals: reactive.totals,
            effect_categories: reactive.categories,
            slowest_properties: properties,
            omitted_properties,
            lifecycle,
            scopes,
            rendering,
            nodes,
        })
    }
}
