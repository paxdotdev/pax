use super::*;

fn downloading() -> LoadingState {
    LoadingState {
        can_cancel: true,
        ..Default::default()
    }
}

#[test]
fn download_fraction_is_not_application_readiness() {
    assert_eq!(LoadingDownload::default().fraction(), None);
    assert_eq!(
        LoadingDownload {
            received_bytes: 0,
            total_bytes: Some(0)
        }
        .fraction(),
        None
    );
    let state = LoadingState {
        phase: LoadingPhase::Preparing,
        download: LoadingDownload {
            received_bytes: 120,
            total_bytes: Some(100),
        },
        ..Default::default()
    };
    assert_eq!(state.download.fraction(), Some(1.0));
    assert_eq!(state.phase, LoadingPhase::Preparing);
}

#[test]
fn observations_are_reactive_and_writes_do_not_change_host_state() {
    let controller = LoadingController::new();
    let attempt = controller.begin(downloading()).unwrap();
    let context = controller.context();
    let observation = context.observe();
    assert_eq!(observation.get().unwrap().download.received_bytes, 0);
    let state = LoadingState {
        download: LoadingDownload {
            received_bytes: 42,
            total_bytes: Some(100),
        },
        ..downloading()
    };
    assert!(controller.publish(&attempt, state.clone()));
    assert_eq!(observation.get(), Some(state.clone()));
    observation.set(None);
    assert_eq!(context.snapshot(), Some(state));
    let another = context.observe();
    controller.close();
    assert_eq!(another.get(), None);
    assert!(!context.cancel());
    assert!(!controller.publish(&attempt, downloading()));
}

#[test]
fn queued_actions_and_attempt_identity_reject_late_completions() {
    let controller = LoadingController::new();
    let attempt = controller.begin(downloading()).unwrap();
    let context = controller.context();
    assert!(!context.retry());
    assert!(context.cancel());
    assert!(!context.cancel());
    assert!(!controller.publish(&attempt, downloading()));
    assert_eq!(controller.take_request(), Some(LoadingRequest::Cancel));
    assert!(!controller.publish(&attempt, downloading()));
    let retry = controller
        .begin(LoadingState {
            phase: LoadingPhase::Cancelled,
            can_retry: true,
            ..Default::default()
        })
        .unwrap();
    assert!(context.retry());
    assert_eq!(controller.take_request(), Some(LoadingRequest::Retry));
    let next = controller.begin(downloading()).unwrap();
    assert!(!controller.publish(&retry, downloading()));
    assert!(controller.publish(&next, downloading()));
    let unrelated = LoadingController::new();
    unrelated.begin(downloading());
    assert!(!unrelated.publish(&next, downloading()));
}

#[test]
fn dropping_owner_invalidates_existing_observations_and_actions() {
    let controller = LoadingController::new();
    controller.begin(downloading());
    let context = controller.context();
    let observation = context.observe();
    assert!(observation.get().is_some());
    drop(controller);
    assert!(observation.get().is_none());
    assert!(context.snapshot().is_none());
    assert!(!context.cancel());
}

#[test]
fn expression_state_has_structured_optional_download_fields() {
    let state = LoadingState {
        download: LoadingDownload {
            received_bytes: 42,
            total_bytes: Some(100),
        },
        ..Default::default()
    };
    let controller = LoadingController::new();
    let attempt = controller.begin(state.clone()).unwrap();
    let variable = controller.context().as_variable();
    assert_eq!(variable.get_as_pax_value(), state.to_pax_value());
    assert!(controller.publish(&attempt, downloading()));
    assert_eq!(variable.get_as_pax_value(), downloading().to_pax_value());
}
