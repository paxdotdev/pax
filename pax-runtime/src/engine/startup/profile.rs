use pax_runtime_api::properties::{ReactiveWorkProfile, ReactiveWorkReport};
use serde::Serialize;
use std::collections::BTreeMap;
use std::{cell::RefCell, rc::Rc};
use web_time::Instant;

/// CPU wall time for complete startup units, including their synchronous reads.
#[derive(Default, Serialize)]
pub struct StartupUnitProfile {
    pub count: usize,
    pub total_ms: f64,
    pub max_ms: f64,
}

impl StartupUnitProfile {
    pub(super) fn record(&mut self, ms: f64) {
        self.count += 1;
        self.total_ms += ms;
        self.max_ms = self.max_ms.max(ms);
    }
}

/// Time spent in an instrumented node phase. `self_ms` excludes nested phases;
/// `total_ms` includes them. Neither should be added to service or reactive totals.
#[derive(Default, Serialize)]
pub struct StartupPhaseProfile {
    pub count: usize,
    pub total_ms: f64,
    pub self_ms: f64,
    pub max_ms: f64,
}

#[derive(Default)]
pub(super) struct StartupPhases {
    phases: BTreeMap<&'static str, StartupPhaseProfile>,
    children_ms: Vec<f64>,
}

impl StartupPhases {
    fn finish(&mut self, name: &'static str, ms: f64) {
        let children_ms = self.children_ms.pop().expect("balanced startup phase");
        if let Some(parent) = self.children_ms.last_mut() {
            *parent += ms;
        }
        let phase = self.phases.entry(name).or_default();
        phase.count += 1;
        phase.total_ms += ms;
        phase.self_ms += (ms - children_ms).max(0.);
        phase.max_ms = phase.max_ms.max(ms);
    }
}

pub(crate) struct StartupPhaseGuard {
    phases: Rc<RefCell<StartupPhases>>,
    name: &'static str,
    start: Instant,
}

impl StartupPhaseGuard {
    pub(super) fn new(phases: Rc<RefCell<StartupPhases>>, name: &'static str) -> Self {
        phases.borrow_mut().children_ms.push(0.);
        Self {
            phases,
            name,
            start: Instant::now(),
        }
    }
}

impl Drop for StartupPhaseGuard {
    fn drop(&mut self) {
        let ms = self.start.elapsed().as_secs_f64() * 1000.;
        self.phases.borrow_mut().finish(self.name, ms);
    }
}

/// Opt-in attribution of resumable startup. Units and service exclude root
/// construction; node phases include it. Native measurement/application, host
/// waits and GPU time are excluded. Reactive evaluator and phase times overlap
/// unit/service times; do not add them.
#[derive(Serialize)]
pub struct StartupProfileReport {
    pub services: usize,
    pub service_ms: f64,
    pub settlement_requests: usize,
    pub presented_checkpoints: usize,
    pub mounts_completed: usize,
    pub mounts_deferred: usize,
    pub mounts_stale: usize,
    pub units: BTreeMap<&'static str, StartupUnitProfile>,
    /// Completed mount units grouped by containing template component and node type.
    pub mounts: BTreeMap<String, StartupUnitProfile>,
    /// Nested node construction/mount phases, including synchronous dependency reads.
    pub phases: BTreeMap<&'static str, StartupPhaseProfile>,
    pub reactive: ReactiveWorkReport,
}

#[derive(Default)]
pub(super) struct StartupProfile {
    pub services: usize,
    pub service_ms: f64,
    pub settlement_requests: usize,
    pub presented_checkpoints: usize,
    pub mounts_completed: usize,
    pub mounts_deferred: usize,
    pub mounts_stale: usize,
    pub units: BTreeMap<&'static str, StartupUnitProfile>,
    pub mounts: BTreeMap<String, StartupUnitProfile>,
    pub phases: Rc<RefCell<StartupPhases>>,
    pub reactive: ReactiveWorkProfile,
}

impl StartupProfile {
    pub fn into_report(self) -> StartupProfileReport {
        StartupProfileReport {
            services: self.services,
            service_ms: self.service_ms,
            settlement_requests: self.settlement_requests,
            presented_checkpoints: self.presented_checkpoints,
            mounts_completed: self.mounts_completed,
            mounts_deferred: self.mounts_deferred,
            mounts_stale: self.mounts_stale,
            units: self.units,
            mounts: self.mounts,
            phases: std::mem::take(&mut self.phases.borrow_mut().phases),
            reactive: self.reactive.into_report(64),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_and_recursive_phases_count_exclusive_work_once() {
        let mut phases = StartupPhases::default();
        phases.children_ms.extend([0., 0., 0.]);
        phases.finish("child", 2.);
        phases.finish("parent", 5.);
        phases.finish("parent", 11.);
        assert_eq!(phases.phases["child"].self_ms, 2.);
        assert_eq!(phases.phases["parent"].count, 2);
        assert_eq!(phases.phases["parent"].total_ms, 16.);
        assert_eq!(phases.phases["parent"].self_ms, 9.);
        assert_eq!(phases.phases["parent"].max_ms, 11.);
        assert!(phases.children_ms.is_empty());
    }
}
