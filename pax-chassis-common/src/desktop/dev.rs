//! Debug inspection uses the existing design-server protocol on the owner thread.
use pax_designtime::{messages::*, DesigntimeManager};
use pax_message::{NativeMessage, ScreenshotPatch};
use pax_runtime::{cartridge::DefinitionToInstanceTraverser, designtime_support::*, PaxEngine};
use std::{
    cell::RefCell,
    collections::HashSet,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

const CAPTURE_BUDGET: usize = 64 * 1024 * 1024;
#[derive(Default)]
pub(super) struct NativeDev {
    look: Option<PendingLook>,
    next_id: u32,
    abandoned: HashSet<u32>,
}
struct PendingLook {
    request: DevClientLookRequest,
    captures: Vec<DevClientRawCapture>,
    bytes: usize,
    next_at: u128,
    deadline: u128,
    in_flight: Option<(u32, u128)>,
}
fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
fn look_error(request_id: String, error: &str) -> DevClientResponse {
    DevClientResponse::Look(DevClientLookResponse {
        request_id,
        status: "error".into(),
        captures: vec![],
        error: Some(error.into()),
    })
}
impl NativeDev {
    pub(super) fn discard_abandoned_capture(&mut self, id: u32) -> bool {
        self.abandoned.remove(&id)
    }
    pub(super) fn poll(
        &mut self,
        engine: &PaxEngine,
        traverser: &dyn DefinitionToInstanceTraverser,
        manager: &Rc<RefCell<DesigntimeManager>>,
    ) {
        let requests = manager.borrow_mut().take_dev_client_requests();
        let mut responses = vec![];
        for request in requests {
            responses.push(match request {
                DevClientRequest::Look(request) => {
                    if self.abandoned.len() >= 8 {
                        look_error(request.request_id, "The native host has eight unreturned captures; restart the app before capturing again")
                    } else if self.look.is_some() {
                        look_error(request.request_id, "A native screenshot request is already active")
                    } else if !request.scale.is_finite() || request.scale <= 0.0 || request.scale > 4.0 || request.duration_ms > 60_000 ||
                        (request.duration_ms > 0 && request.period_ms < 50) {
                        look_error(request.request_id, "Native capture requires 0 < scale <= 4, duration <= 60000 ms and period >= 50 ms for sequences")
                    } else {
                        let time = now();
                        self.look = Some(PendingLook { deadline: time + request.duration_ms as u128,
                            request, captures: vec![], bytes: 0, next_at: time, in_flight: None });
                        continue;
                    }
                }
                DevClientRequest::InspectTree(request) => {
                    let payload = build_designtime_inspect_tree_payload(engine, traverser,
                        request.max_depth.map(|n| n as i64).unwrap_or(-1));
                    DevClientResponse::InspectTree(DevClientInspectTreeResponse { request_id: request.request_id,
                        status: payload.status, node_count: payload.node_count, tree_json: payload.tree_json, error: payload.error })
                }
                DevClientRequest::RayCast(request) => {
                    let payload = build_designtime_ray_cast_payload(engine, traverser, request.x, request.y, request.hit_invisible);
                    DevClientResponse::RayCast(DevClientRayCastResponse { request_id: request.request_id,
                        status: payload.status, x: request.x, y: request.y, hit_invisible: request.hit_invisible,
                        node_count: payload.node_count, nodes_json: payload.nodes_json, error: payload.error })
                }
                DevClientRequest::SelectorQuery(request) => {
                    let payload = build_designtime_selector_query_payload(engine, traverser, &request.selector);
                    DevClientResponse::SelectorQuery(DevClientSelectorQueryResponse { request_id: request.request_id,
                        status: payload.status, selector: request.selector, node_count: payload.node_count,
                        nodes_json: payload.nodes_json, error: payload.error })
                }
                DevClientRequest::ReplaceNode(request) => {
                    let payload = apply_designtime_replace_node_subtemplate(traverser, manager, &request.component_type_id,
                        request.template_node_id, &request.subtemplate);
                    DevClientResponse::ReplaceNode(DevClientReplaceNodeResponse { request_id: request.request_id,
                        status: payload.status, component_type_id: payload.component_type_id, template_node_id: payload.template_node_id,
                        reload_scope: payload.reload_scope, reloaded_template_node_id: payload.reloaded_template_node_id,
                        source_path: payload.source_path, error: payload.error })
                }
                DevClientRequest::Logs(request) => DevClientResponse::Logs(DevClientLogsResponse {
                    request_id: request.request_id, status: "error".into(), entries: vec![], next_seq: 1, oldest_seq: None,
                    error: Some("Use the platform log stream (adb logcat on Android) for native host and application logs".into()),
                }),
            });
        }
        if let Some(mut pending) = self.look.take() {
            let time = now();
            let mut error = None;
            let mut done = false;
            if let Some((id, started)) = pending.in_flight {
                let capture = engine
                    .runtime_context
                    .get_screenshot_map()
                    .borrow_mut()
                    .remove(&id);
                if let Some(capture) = capture {
                    pending.in_flight = None;
                    if capture.width == 0 || capture.height == 0 {
                        error = Some("The native host could not capture its current window");
                    } else if pending.bytes.saturating_add(capture.data.len()) > CAPTURE_BUDGET
                        || pending.captures.len() >= 120
                    {
                        error = Some("Native capture exceeds the 64 MiB / 120-frame sequence budget; reduce scale or duration");
                    } else {
                        pending.bytes += capture.data.len();
                        // The wire protocol already supports zlib; keep full native pixels.
                        let compressed =
                            miniz_oxide::deflate::compress_to_vec_zlib(&capture.data, 1);
                        pending.captures.push(DevClientRawCapture {
                            rgba_bytes: compressed,
                            compression: Some("zlib".into()),
                            width: capture.width,
                            height: capture.height,
                            captured_at_ms: time,
                        });
                        done = pending.request.period_ms == 0 || time >= pending.deadline;
                        pending.next_at = time + pending.request.period_ms as u128;
                    }
                } else if time.saturating_sub(started) > 5_000 {
                    self.abandoned.insert(id);
                    error = Some("Timed out waiting for a native window capture");
                }
            }
            if let Some(error) = error {
                responses.push(look_error(pending.request.request_id, error));
            } else if done {
                responses.push(DevClientResponse::Look(DevClientLookResponse {
                    request_id: pending.request.request_id,
                    status: "ok".into(),
                    captures: pending.captures,
                    error: None,
                }));
            } else {
                if pending.in_flight.is_none() && time >= pending.next_at {
                    self.next_id = self.next_id.wrapping_add(1).max(1_000_000);
                    pending.in_flight = Some((self.next_id, time));
                    engine
                        .runtime_context
                        .enqueue_native_message(NativeMessage::Screenshot(ScreenshotPatch {
                            id: self.next_id,
                            scale: Some(pending.request.scale),
                        }));
                }
                self.look = Some(pending);
            }
        }
        for response in responses {
            if let Err(error) = manager.borrow_mut().send_dev_client_response(response) {
                log::warn!("Cannot send native development response: {error}");
            }
        }
    }
}
