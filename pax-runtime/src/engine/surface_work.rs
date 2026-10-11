//! Cooperative admission for drawing capture and surface submission. A drawing
//! capture or GPU submission is atomic; the allowance limits starting more work.

/// CPU-time allowance for preparing settled drawings between application ticks.
/// The cost estimate adapts across passes; at least one eligible job can make progress.
pub struct SurfaceWorkBudget {
    allowance_ms: f64,
    estimate_ms: f64,
}

impl SurfaceWorkBudget {
    pub fn new(allowance_ms: f64) -> Self {
        Self {
            allowance_ms: allowance_ms.max(0.0),
            estimate_ms: 0.0,
        }
    }

    pub fn admits(&self, completed: usize, elapsed_ms: f64) -> bool {
        completed == 0 || elapsed_ms + self.estimate_ms <= self.allowance_ms
    }

    pub fn observe(&mut self, elapsed_ms: f64) {
        // Rise immediately after an expensive job; decay gradually so one cheap
        // tile does not cause a following burst of expensive image uploads.
        self.estimate_ms = elapsed_ms.max(self.estimate_ms * 0.8);
    }
}

#[derive(Default, serde::Serialize)]
pub struct SurfaceWorkReport {
    pub initializations: usize,
    pub recorded: usize,
    pub fallbacks: usize,
    pub submitted: usize,
    pub deferred: usize,
    pub cache_misses: usize,
    pub dirty_layers: usize,
    pub unfiltered_layers: usize,
    pub blocked: bool,
    pub blocker: Option<&'static str>,
    pub elapsed_ms: f64,
}

impl SurfaceWorkReport {
    pub(super) fn accumulate(&mut self, next: Self) {
        self.initializations += next.initializations;
        self.recorded += next.recorded;
        self.fallbacks += next.fallbacks;
        self.submitted += next.submitted;
        // Deferred is the queue snapshot from the latest opportunity, not work
        // completed; the other counts describe attempts across opportunities.
        self.deferred = next.deferred;
        self.cache_misses += next.cache_misses;
        self.dirty_layers += next.dirty_layers;
        self.unfiltered_layers += next.unfiltered_layers;
        self.blocked |= next.blocked;
        self.blocker = next.blocker.or(self.blocker);
        self.elapsed_ms += next.elapsed_ms;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atomic_overrun_stops_more_work_but_never_starves_first_job() {
        let mut budget = SurfaceWorkBudget::new(4.0);
        assert!(budget.admits(0, 0.0));
        budget.observe(8.0);
        assert!(!budget.admits(1, 8.0));
        assert!(budget.admits(0, 0.0));
        budget.observe(0.5);
        assert!(!budget.admits(1, 0.5));
        for _ in 0..20 {
            budget.observe(0.5);
        }
        assert!(budget.admits(1, 0.5));
        assert!(!budget.admits(2, 4.0));
    }
}
