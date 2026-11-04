//! Opt-in attribution of work actually evaluated by the reactive queue.
use super::properties_table::CutoffEvaluation;
use serde::Serialize;
use std::collections::HashMap;

/// Queued evaluations only. Synchronous dependency reads are included in their
/// caller's inclusive time, but are not counted as additional queued evaluations.
#[derive(Clone, Debug, Default, Serialize)]
pub struct ReactiveWorkCounts {
    pub effects: usize,
    pub cutoffs_initialized: usize,
    pub cutoffs_suppressed: usize,
    pub cutoffs_propagated: usize,
    /// CPU wall time inside evaluators, including nested reads; not queue overhead.
    pub inclusive_ms: f64,
}

impl ReactiveWorkCounts {
    pub fn evaluations(&self) -> usize {
        self.effects + self.cutoffs_initialized + self.cutoffs_suppressed + self.cutoffs_propagated
    }

    fn add(&mut self, other: &Self) {
        self.effects += other.effects;
        self.cutoffs_initialized += other.cutoffs_initialized;
        self.cutoffs_suppressed += other.cutoffs_suppressed;
        self.cutoffs_propagated += other.cutoffs_propagated;
        self.inclusive_ms += other.inclusive_ms;
    }
}

/// Cumulative queue attribution. Categories and totals include every evaluation;
/// only `top_properties` is truncated, ranked by evaluation count then name.
#[derive(Debug, Serialize)]
pub struct ReactiveWorkReport {
    pub totals: ReactiveWorkCounts,
    pub categories: Vec<(String, ReactiveWorkCounts)>,
    pub top_properties: Vec<(String, ReactiveWorkCounts)>,
    pub distinct_labels: usize,
    pub omitted_labels: usize,
}

/// Collector for an explicitly profiled drain. Normal drains allocate no collector.
#[derive(Default)]
pub struct ReactiveWorkProfile {
    properties: HashMap<String, ReactiveWorkCounts>,
}

impl ReactiveWorkProfile {
    pub(super) fn record(&mut self, name: &str, outcome: CutoffEvaluation, elapsed_ms: f64) {
        // A property is typically evaluated many times. Allocate its label once.
        if !self.properties.contains_key(name) {
            self.properties.insert(name.to_owned(), Default::default());
        }
        let counts = self.properties.get_mut(name).unwrap();
        match outcome {
            CutoffEvaluation::NotCutoff => counts.effects += 1,
            CutoffEvaluation::Initialized => counts.cutoffs_initialized += 1,
            CutoffEvaluation::Suppressed => counts.cutoffs_suppressed += 1,
            CutoffEvaluation::Propagated => counts.cutoffs_propagated += 1,
        }
        counts.inclusive_ms += elapsed_ms;
    }

    pub fn into_report(self, property_limit: usize) -> ReactiveWorkReport {
        let mut totals = ReactiveWorkCounts::default();
        let mut categories: HashMap<String, ReactiveWorkCounts> = HashMap::new();
        for (name, counts) in &self.properties {
            totals.add(counts);
            let category = name
                .split_once(" (")
                .map_or(name.as_str(), |(name, _)| name);
            categories
                .entry(category.to_owned())
                .or_default()
                .add(counts);
        }
        let sort = |rows: &mut Vec<(String, ReactiveWorkCounts)>| {
            rows.sort_by(|(a, x), (b, y)| y.evaluations().cmp(&x.evaluations()).then(a.cmp(b)));
        };
        let mut categories: Vec<_> = categories.into_iter().collect();
        sort(&mut categories);
        let mut top_properties: Vec<_> = self.properties.into_iter().collect();
        let distinct_labels = top_properties.len();
        sort(&mut top_properties);
        top_properties.truncate(property_limit);
        ReactiveWorkReport {
            totals,
            categories,
            omitted_labels: distinct_labels - top_properties.len(),
            distinct_labels,
            top_properties,
        }
    }
}
