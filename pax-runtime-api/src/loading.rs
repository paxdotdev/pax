//! Read-only state and actions for an application loading component.
//!
//! Download progress counts decoded application-module bytes, not compressed
//! network traffic. Transfer completion does not imply presentation readiness.

use crate::{Interpolatable, LocalProperty, PaxValue, ToPaxValue, Variable};
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// Stage of one application startup attempt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoadingPhase {
    /// Fetching and compiling the deferred application module.
    #[default]
    Downloading,
    /// Constructing, mounting, measuring and preparing the application.
    Preparing,
    /// The application's first presentation is ready.
    Ready,
    /// The loader is exiting over the prepared application; input remains gated.
    Revealing,
    /// The application has been presented and activated.
    Active,
    /// Startup failed; consult the error and retry capability.
    Failed,
    /// This startup attempt was cancelled.
    Cancelled,
}

impl LoadingPhase {
    /// Stable lowercase spelling exposed as `@loading.phase` in Pax.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Downloading => "downloading",
            Self::Preparing => "preparing",
            Self::Ready => "ready",
            Self::Revealing => "revealing",
            Self::Active => "active",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Application-module transfer counters. Unknown totals remain `None`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoadingDownload {
    /// Decoded bytes received during the current attempt.
    pub received_bytes: u64,
    /// Expected decoded size, when the build manifest or transport provides it.
    pub total_bytes: Option<u64>,
}

impl LoadingDownload {
    /// Download fraction in 0..=1, or `None` without a positive denominator.
    /// This is not an estimate of compilation, mounting or rendering work.
    pub fn fraction(&self) -> Option<f64> {
        self.total_bytes
            .filter(|total| *total > 0)
            .map(|total| (self.received_bytes as f64 / total as f64).min(1.0))
    }
}

/// A snapshot of startup state. Editing a snapshot never changes the host.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoadingState {
    /// Current startup stage.
    pub phase: LoadingPhase,
    /// Transfer counters, independent of preparation/readiness.
    pub download: LoadingDownload,
    /// Human-readable failure information, when startup has failed.
    pub error: Option<String>,
    /// Whether the host can safely retry this attempt.
    pub can_retry: bool,
    /// Whether the host currently accepts cancellation.
    pub can_cancel: bool,
}

impl Interpolatable for LoadingState {}
impl ToPaxValue for LoadingState {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(
            vec![
                (
                    "phase".into(),
                    self.phase.as_str().to_string().to_pax_value(),
                ),
                (
                    "download".into(),
                    PaxValue::Object(
                        vec![
                            (
                                "received_bytes".into(),
                                self.download.received_bytes.to_pax_value(),
                            ),
                            (
                                "total_bytes".into(),
                                self.download.total_bytes.to_pax_value(),
                            ),
                            ("fraction".into(), self.download.fraction().to_pax_value()),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                ),
                ("error".into(), self.error.to_pax_value()),
                ("can_retry".into(), self.can_retry.to_pax_value()),
                ("can_cancel".into(), self.can_cancel.to_pax_value()),
            ]
            .into_iter()
            .collect(),
        )
    }
}

struct LoadingInner {
    state: LocalProperty<LoadingState>,
    closed: LocalProperty<bool>,
    request: Cell<Option<LoadingRequest>>,
    generation: Cell<u64>,
}

/// Loader-scoped, UI-thread handle returned by `NodeContext::loading()`.
/// It does not retain the host or expose its writable state property.
#[derive(Clone)]
pub struct LoadingContext {
    inner: Weak<LoadingInner>,
}

impl LoadingContext {
    /// Read the current snapshot; `None` means the loading experience has closed.
    pub fn snapshot(&self) -> Option<LoadingState> {
        let inner = self.inner.upgrade()?;
        (!inner.closed.get()).then(|| inner.state.get())
    }

    /// Observe state through a one-way property suitable for `ctx.subscribe`.
    /// Writes to the returned property never mutate the host's loading state.
    /// The observation becomes `None` when the host closes the experience.
    pub fn observe(&self) -> LocalProperty<Option<LoadingState>> {
        let Some(inner) = self.inner.upgrade() else {
            return LocalProperty::new(None);
        };
        let dependencies = [inner.state.untyped(), inner.closed.untyped()];
        let context = self.clone();
        // Retain graph inputs, but never the host owner or its action queue.
        let inputs = (inner.state.clone(), inner.closed.clone());
        LocalProperty::computed(
            move || {
                let _ = &inputs;
                context.snapshot()
            },
            &dependencies,
        )
    }

    /// Request a supported retry. Returns false after close, when unsupported,
    /// or while another action is already queued. The host services it later.
    pub fn retry(&self) -> bool {
        self.request(LoadingRequest::Retry)
    }

    /// Request cancellation without re-entering the host from an event handler.
    pub fn cancel(&self) -> bool {
        self.request(LoadingRequest::Cancel)
    }

    fn request(&self, request: LoadingRequest) -> bool {
        let Some(inner) = self.inner.upgrade() else {
            return false;
        };
        if inner.closed.get() || inner.request.get().is_some() {
            return false;
        }
        let mut state = inner.state.get();
        if !match request {
            LoadingRequest::Retry => state.can_retry,
            LoadingRequest::Cancel => state.can_cancel,
        } {
            return false;
        }
        inner.request.set(Some(request));
        state.can_retry = false;
        state.can_cancel = false;
        inner.state.set(state);
        true
    }

    /// Expression-scope adapter. Each scope receives a one-way derived value.
    #[doc(hidden)]
    pub fn as_variable(&self) -> Variable {
        let observation = self.observe();
        let dependencies = [observation.untyped()];
        Variable::new_from_typed_property(LocalProperty::computed(
            move || {
                observation.get().unwrap_or_else(|| LoadingState {
                    phase: LoadingPhase::Cancelled,
                    ..Default::default()
                })
            },
            &dependencies,
        ))
    }
}

/// Command queued by a loader for its owning startup coordinator.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadingRequest {
    Retry,
    Cancel,
}

/// Identity of one attempt. A completion from a retired attempt cannot publish.
#[doc(hidden)]
#[derive(Clone)]
pub struct LoadingAttempt {
    generation: u64,
    owner: Weak<LoadingInner>,
}

/// Host-side owner of a loading experience. Create it in the engine's property
/// graph; keep it alive until the loader has actually unmounted.
#[doc(hidden)]
pub struct LoadingController {
    inner: Rc<LoadingInner>,
}

impl Default for LoadingController {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadingController {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(LoadingInner {
                state: LocalProperty::new(LoadingState::default()),
                closed: LocalProperty::new(false),
                request: Cell::new(None),
                generation: Cell::new(0),
            }),
        }
    }

    pub fn context(&self) -> LoadingContext {
        LoadingContext {
            inner: Rc::downgrade(&self.inner),
        }
    }

    /// Start an attempt, invalidating every earlier publication token.
    pub fn begin(&self, state: LoadingState) -> Option<LoadingAttempt> {
        if self.inner.closed.get() {
            return None;
        }
        let generation = self.inner.generation.get().checked_add(1)?;
        self.inner.generation.set(generation);
        self.inner.request.set(None);
        self.inner.state.set(state);
        Some(LoadingAttempt {
            generation,
            owner: Rc::downgrade(&self.inner),
        })
    }

    /// Publish one host snapshot. Pending actions fence off late async progress.
    pub fn publish(&self, attempt: &LoadingAttempt, state: LoadingState) -> bool {
        if self.inner.closed.get()
            || self.inner.request.get().is_some()
            || self.inner.generation.get() != attempt.generation
            || !Weak::ptr_eq(&attempt.owner, &Rc::downgrade(&self.inner))
        {
            return false;
        }
        if self.inner.state.get() != state {
            self.inner.state.set(state);
        }
        true
    }

    /// Retire the attempt before servicing an action, so stale completions stay
    /// invalid even after this single queued request has been taken.
    pub fn take_request(&self) -> Option<LoadingRequest> {
        let request = self.inner.request.take()?;
        if let Some(next) = self.inner.generation.get().checked_add(1) {
            self.inner.generation.set(next);
        } else {
            self.close();
        }
        Some(request)
    }

    pub fn close(&self) {
        self.inner.request.set(None);
        self.inner.closed.set(true);
    }
}

impl Drop for LoadingController {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests;
