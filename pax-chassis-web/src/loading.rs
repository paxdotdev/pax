//! Host-owned presentation and loading state, independent of application lifetime.
use crate::PaxChassisWeb;
use pax_runtime::api::loading::{LoadingAttempt, LoadingController, LoadingRequest};
use pax_runtime::api::{LoadingDownload, LoadingPhase, LoadingState};
use std::cell::Cell;
use wasm_bindgen::prelude::*;

pub(crate) struct PresentationClock {
    source: Box<dyn Fn() -> u128>,
    origin: Cell<Option<u128>>,
}
impl PresentationClock {
    pub fn new(source: Box<dyn Fn() -> u128>, held: bool) -> Self {
        Self {
            source,
            origin: Cell::new((!held).then_some(0)),
        }
    }
    pub fn now(&self) -> u128 {
        self.origin
            .get()
            .map_or(0, |origin| (self.source)().saturating_sub(origin))
    }
    pub fn publish(&self) {
        if self.origin.get().is_none() {
            self.origin.set(Some((self.source)()));
        }
    }
}

pub(crate) struct LoadingOwner {
    pub controller: LoadingController,
    attempt: Option<LoadingAttempt>,
}
impl LoadingOwner {
    pub fn new(controller: LoadingController) -> Self {
        let attempt = controller.begin(LoadingState::default());
        Self {
            controller,
            attempt,
        }
    }
}

#[wasm_bindgen]
impl PaxChassisWeb {
    /// Observe this application's cleanup, independently of other hosted scenes.
    pub fn application_closed(&self) -> js_sys::Promise {
        let closed = self
            .engine
            .borrow()
            .runtime_context
            .application
            .context()
            .closed();
        wasm_bindgen_futures::future_to_promise(async move {
            closed.await;
            Ok(JsValue::UNDEFINED)
        })
    }

    /// Start presentation after native/canvas readiness; the host gates input separately.
    pub fn publish_application(&mut self) {
        self.presentation_clock.publish();
        self.engine.borrow_mut().publish_presentation();
    }

    /// Begin the loader's authored root/subtree exit timelines.
    pub fn begin_presentation_exit(&mut self) {
        self.engine.borrow_mut().begin_presentation_exit();
    }

    pub fn presentation_exit_complete(&self) -> bool {
        self.engine.borrow().presentation_exit_complete()
    }

    /// Update the read-only context of this loading application only.
    pub fn loading_update(
        &mut self,
        phase: &str,
        received: f64,
        total: Option<f64>,
        error: Option<String>,
        retry: bool,
        cancel: bool,
    ) -> Result<(), JsValue> {
        let phase = match phase {
            "downloading" => LoadingPhase::Downloading,
            "preparing" => LoadingPhase::Preparing,
            "ready" => LoadingPhase::Ready,
            "revealing" => LoadingPhase::Revealing,
            "active" => LoadingPhase::Active,
            "failed" => LoadingPhase::Failed,
            "cancelled" => LoadingPhase::Cancelled,
            _ => return Err(JsValue::from_str("unknown loading phase")),
        };
        if !received.is_finite()
            || received < 0.0
            || total.is_some_and(|v| !v.is_finite() || v < 0.0)
        {
            return Err(JsValue::from_str("invalid loading byte counters"));
        }
        let owner = self
            .loading
            .as_mut()
            .ok_or_else(|| JsValue::from_str("not a loading application"))?;
        let _graph = self.engine.borrow().runtime_context.property_graph.enter();
        let state = LoadingState {
            phase,
            download: LoadingDownload {
                received_bytes: received as u64,
                total_bytes: total.map(|v| v as u64),
            },
            error,
            can_retry: retry,
            can_cancel: cancel,
        };
        if owner.attempt.is_none() {
            owner.attempt = owner.controller.begin(state.clone());
        }
        if let Some(attempt) = &owner.attempt {
            owner.controller.publish(attempt, state);
        }
        Ok(())
    }

    /// Drain after returning from the user's event handler, never reentrantly.
    pub fn take_loading_request(&mut self) -> Option<String> {
        let owner = self.loading.as_mut()?;
        let request = owner.controller.take_request()?;
        owner.attempt = None;
        Some(
            match request {
                LoadingRequest::Retry => "retry",
                LoadingRequest::Cancel => "cancel",
            }
            .into(),
        )
    }
}

/// Check cancellation after the split boundary, before invoking user initialization.
#[doc(hidden)]
pub fn web_check_open(host: &JsValue) -> Result<(), JsValue> {
    let callback = js_sys::Reflect::get(host, &JsValue::from_str("checkOpen"))?;
    if let Some(callback) = callback.dyn_ref::<js_sys::Function>() {
        callback.call0(&JsValue::NULL)?;
    }
    Ok(())
}
