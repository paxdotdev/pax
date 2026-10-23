//! Cooperative admission for drawing capture and surface submission. A drawing
//! capture or GPU submission is atomic; the allowance limits starting more work.

/// CPU-time allowance for preparing settled drawings between application ticks.
/// Actual elapsed CPU time limits further work; at least one eligible job can
/// make progress. Unrelated expensive jobs do not predict the next job's cost.
pub struct SurfaceWorkBudget {
    allowance_ms: f64,
}

impl SurfaceWorkBudget {
    pub fn new(allowance_ms: f64) -> Self {
        Self {
            allowance_ms: allowance_ms.max(0.0),
        }
    }

    pub fn admits(&self, completed: usize, elapsed_ms: f64) -> bool {
        completed == 0 || elapsed_ms < self.allowance_ms
    }
}

#[derive(Clone, Default, serde::Serialize)]
pub struct PreparationStageTiming {
    pub count: usize,
    pub total_ms: f64,
    pub max_ms: f64,
}

/// Fixed-stage, opt-in CPU attribution for cold preparation. Image replay is a
/// subphase of replay; neither submission nor presentation measures GPU completion.
#[derive(Clone, Default, serde::Serialize)]
pub struct PreparationDiagnostics {
    pub stages: std::collections::BTreeMap<&'static str, PreparationStageTiming>,
    pub targeted_surfaces: usize,
    /// Already-presented dirty surfaces included alongside the selected targets.
    pub additional_surfaces: usize,
    pub image_uploads: usize,
    pub image_upload_bytes: usize,
}

impl PreparationDiagnostics {
    fn accumulate(&mut self, next: Self) {
        for (name, sample) in next.stages {
            let current = self.stages.entry(name).or_default();
            current.count += sample.count;
            current.total_ms += sample.total_ms;
            current.max_ms = current.max_ms.max(sample.max_ms);
        }
        self.targeted_surfaces += next.targeted_surfaces;
        self.additional_surfaces += next.additional_surfaces;
        self.image_uploads += next.image_uploads;
        self.image_upload_bytes += next.image_upload_bytes;
    }
}

#[derive(Clone, Default, serde::Serialize)]
pub struct SurfaceWorkReport {
    pub initializations: usize,
    pub recorded: usize,
    pub fallbacks: usize,
    pub submitted: usize,
    /// Visible initialization/submission units serviced in this display epoch.
    /// Diagnostic only: starting initialization is not presented content.
    pub urgent_units: usize,
    pub deferred: usize,
    pub cache_misses: usize,
    pub dirty_layers: usize,
    pub unfiltered_layers: usize,
    pub blocked: bool,
    pub blocker: Option<&'static str>,
    pub elapsed_ms: f64,
    /// Absent unless runtime diagnostics are enabled. Fixed-size stage aggregates,
    /// never per-node or per-command logs; no effect on admission or readiness.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<PreparationDiagnostics>,
}

impl SurfaceWorkReport {
    pub(super) fn accumulate(&mut self, next: Self) {
        self.initializations += next.initializations;
        self.recorded += next.recorded;
        self.fallbacks += next.fallbacks;
        self.submitted += next.submitted;
        self.urgent_units += next.urgent_units;
        // Deferred is the queue snapshot from the latest opportunity, not work
        // completed; the other counts describe attempts across opportunities.
        self.deferred = next.deferred;
        self.cache_misses += next.cache_misses;
        self.dirty_layers += next.dirty_layers;
        self.unfiltered_layers += next.unfiltered_layers;
        self.blocked |= next.blocked;
        self.blocker = next.blocker.or(self.blocker);
        self.elapsed_ms += next.elapsed_ms;
        if let Some(diagnostics) = next.diagnostics {
            self.diagnostics
                .get_or_insert_with(Default::default)
                .accumulate(diagnostics);
        }
    }
}

/// Per-display-frame accounting for cooperative rendering. This is not a scene
/// snapshot or work queue: pending nodes, layers and surfaces remain authoritative
/// in the runtime/backend and are selected again at every service boundary.
#[derive(Default)]
pub struct RenderWorkState {
    epoch: Option<u32>,
    pub(super) setup_complete: bool,
    pub(super) urgent_complete: bool,
    pub(super) preparation: SurfaceWorkReport,
    // Failed layers wait for a later tick rather than spinning between host tasks.
    pub(super) attempted_layers: std::collections::HashSet<usize>,
    // Lighting uniforms can need submission without any canvas-content redraw.
    // Keep unsubmitted layer IDs across ticks; they are not captured scene data.
    pub(super) pending_lighting_flushes: std::collections::HashSet<usize>,
}

impl RenderWorkState {
    /// Begin drawing after a settled tick. Multiple ticks/completions in one
    /// display epoch share preparation accounting, including its first-job grant.
    pub fn begin(&mut self, epoch: u32) {
        if self.epoch != Some(epoch) {
            self.preparation = SurfaceWorkReport::default();
            self.epoch = Some(epoch);
        }
        self.setup_complete = false;
        self.urgent_complete = false;
        self.attempted_layers.clear();
    }
}

/// One synchronous service result. `pending` requests another host task;
/// `blocked` waits for a normal settlement/completion opportunity, not a busy loop.
/// CPU times include atomic overruns and do not measure GPU completion.
#[derive(Default, serde::Serialize)]
pub struct RenderWorkReport {
    pub pending: bool,
    pub blocked: bool,
    pub blocker: Option<&'static str>,
    pub allowance_exhausted: bool,
    pub ordinary_layers: usize,
    pub ordinary_drawn_layers: usize,
    pub ordinary_flush_only_layers: usize,
    pub atomic_fallbacks: usize,
    pub units: usize,
    pub elapsed_ms: f64,
    /// Setup and ordinary drawing are measured, not charged to preparation.
    pub setup_ms: f64,
    pub ordinary_ms: f64,
    pub ordinary_drawn_ms: f64,
    pub ordinary_flush_only_ms: f64,
    pub max_unit_ms: f64,
    pub preparation: SurfaceWorkReport,
    /// Opt-in setup attribution: (layer, dirty before reconciliation, pending nodes).
    /// At most 32 layers per service; empty when runtime diagnostics are disabled.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub layer_reconciliation: Vec<(usize, bool, usize)>,
    /// Opt-in draw attribution: (layer, current priority, selected nodes, still dirty).
    /// At most 32 units per service, without retaining scene nodes between calls.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub layer_draws: Vec<(usize, Option<i32>, usize, usize)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn continuation_and_extra_ticks_do_not_replenish_frame_allowance() {
        let mut state = RenderWorkState::default();
        state.begin(7);
        state.preparation.submitted = 1;
        state.preparation.elapsed_ms = 6.0;
        state.setup_complete = true;
        state.begin(7);
        assert_eq!(state.preparation.submitted, 1);
        assert_eq!(state.preparation.elapsed_ms, 6.0);
        assert!(!state.setup_complete);
        assert!(!SurfaceWorkBudget::new(4.0).admits(1, state.preparation.elapsed_ms));
        state.begin(8);
        assert_eq!(state.preparation.submitted, 0);
        assert_eq!(state.preparation.elapsed_ms, 0.0);
    }
    #[test]
    fn atomic_overrun_stops_more_work_but_never_starves_first_job() {
        let budget = SurfaceWorkBudget::new(4.0);
        assert!(budget.admits(0, 0.0));
        assert!(!budget.admits(1, 8.0));
        assert!(budget.admits(0, 0.0));
        // A later epoch with cheap work is not poisoned by the overrun.
        assert!(budget.admits(1, 0.5));
        assert!(!budget.admits(2, 4.0));
    }
}
