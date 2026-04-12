use js_sys::Date;
use pax_designtime::messages::{
    DevClientInspectTreeResponse, DevClientLogEntry, DevClientLogsResponse, DevClientLookRequest,
    DevClientLookResponse, DevClientRawCapture, DevClientRayCastResponse,
    DevClientReplaceNodeResponse, DevClientRequest, DevClientResponse,
    DevClientSelectorQueryResponse,
};
use pax_message::{NativeMessage, ScreenshotPatch};
use pax_runtime::designtime_support::{
    apply_designtime_replace_node_subtemplate, apply_designtime_userland_reload,
    build_designtime_inspect_tree_payload, build_designtime_ray_cast_payload,
    build_designtime_selector_query_payload,
};

use crate::read_dev_console_entries_json;
use crate::PaxChassisWeb;

pub(super) struct PendingWebDevLookRequest {
    pub request: DevClientLookRequest,
    pub captures: Vec<DevClientRawCapture>,
    pub next_capture_at_ms: u128,
    pub deadline_ms: u128,
    pub capture_in_flight: Option<u32>,
}

pub(super) fn next_web_dev_capture_id(next_capture_id: &mut u32) -> u32 {
    let capture_id = *next_capture_id;
    *next_capture_id = (*next_capture_id).wrapping_add(1);
    if *next_capture_id < 1_000_000 {
        *next_capture_id = 1_000_000;
    }
    capture_id
}

impl PaxChassisWeb {
    pub(super) fn update_userland_component(&mut self) {
        let mut engine = self.engine.borrow_mut();
        apply_designtime_userland_reload(
            &mut engine,
            self.userland_definition_to_instance_traverser.as_ref(),
            &self.designtime_manager,
        );
    }

    pub(super) fn process_pending_dev_client_requests(&mut self) {
        let requests = self
            .designtime_manager
            .borrow_mut()
            .take_dev_client_requests();
        for request in requests {
            match request {
                DevClientRequest::Look(request) => {
                    let now_ms = web_dev_now_ms();
                    self.pending_dev_look_requests.insert(
                        request.request_id.clone(),
                        PendingWebDevLookRequest {
                            next_capture_at_ms: now_ms,
                            deadline_ms: now_ms.saturating_add(request.duration_ms as u128),
                            request,
                            captures: vec![],
                            capture_in_flight: None,
                        },
                    );
                }
                DevClientRequest::InspectTree(request) => {
                    let payload = {
                        let engine = self.engine.borrow();
                        build_designtime_inspect_tree_payload(
                            &engine,
                            self.userland_definition_to_instance_traverser.as_ref(),
                            request.max_depth.map(|value| value as i64).unwrap_or(-1),
                        )
                    };
                    let response = DevClientResponse::InspectTree(DevClientInspectTreeResponse {
                        request_id: request.request_id,
                        status: payload.status,
                        node_count: payload.node_count,
                        tree_json: payload.tree_json,
                        error: payload.error,
                    });
                    if let Err(err) = self
                        .designtime_manager
                        .borrow_mut()
                        .send_dev_client_response(response)
                    {
                        log::warn!("failed to send web inspect-tree response: {err}");
                    }
                }
                DevClientRequest::RayCast(request) => {
                    let payload = {
                        let engine = self.engine.borrow();
                        build_designtime_ray_cast_payload(
                            &engine,
                            self.userland_definition_to_instance_traverser.as_ref(),
                            request.x,
                            request.y,
                            request.hit_invisible,
                        )
                    };
                    let response = DevClientResponse::RayCast(DevClientRayCastResponse {
                        request_id: request.request_id,
                        status: payload.status,
                        x: request.x,
                        y: request.y,
                        hit_invisible: request.hit_invisible,
                        node_count: payload.node_count,
                        nodes_json: payload.nodes_json,
                        error: payload.error,
                    });
                    if let Err(err) = self
                        .designtime_manager
                        .borrow_mut()
                        .send_dev_client_response(response)
                    {
                        log::warn!("failed to send web ray-cast response: {err}");
                    }
                }
                DevClientRequest::SelectorQuery(request) => {
                    let payload = {
                        let engine = self.engine.borrow();
                        build_designtime_selector_query_payload(
                            &engine,
                            self.userland_definition_to_instance_traverser.as_ref(),
                            &request.selector,
                        )
                    };
                    let response =
                        DevClientResponse::SelectorQuery(DevClientSelectorQueryResponse {
                            request_id: request.request_id,
                            status: payload.status,
                            selector: request.selector,
                            node_count: payload.node_count,
                            nodes_json: payload.nodes_json,
                            error: payload.error,
                        });
                    if let Err(err) = self
                        .designtime_manager
                        .borrow_mut()
                        .send_dev_client_response(response)
                    {
                        log::warn!("failed to send web selector response: {err}");
                    }
                }
                DevClientRequest::ReplaceNode(request) => {
                    let payload = apply_designtime_replace_node_subtemplate(
                        self.userland_definition_to_instance_traverser.as_ref(),
                        &self.designtime_manager,
                        &request.component_type_id,
                        request.template_node_id,
                        &request.subtemplate,
                    );
                    let response = DevClientResponse::ReplaceNode(DevClientReplaceNodeResponse {
                        request_id: request.request_id,
                        status: payload.status,
                        component_type_id: payload.component_type_id,
                        template_node_id: payload.template_node_id,
                        reload_scope: payload.reload_scope,
                        reloaded_template_node_id: payload.reloaded_template_node_id,
                        source_path: payload.source_path,
                        error: payload.error,
                    });
                    if let Err(err) = self
                        .designtime_manager
                        .borrow_mut()
                        .send_dev_client_response(response)
                    {
                        log::warn!("failed to send web replace-node response: {err}");
                    }
                }
                DevClientRequest::Logs(request) => {
                    let request_id = request.request_id;
                    let since_seq = request.since_seq;
                    let limit = request.limit.unwrap_or(200);
                    let response = match serde_json::from_str::<serde_json::Value>(
                        &read_dev_console_entries_json(since_seq, limit),
                    ) {
                        Ok(payload) => {
                            let entries = payload
                                .get("entries")
                                .cloned()
                                .and_then(|value| {
                                    serde_json::from_value::<Vec<DevClientLogEntry>>(value).ok()
                                })
                                .unwrap_or_default();
                            let next_seq = payload
                                .get("next_seq")
                                .and_then(|value| value.as_u64())
                                .unwrap_or(1);
                            let oldest_seq =
                                payload.get("oldest_seq").and_then(|value| value.as_u64());
                            DevClientResponse::Logs(DevClientLogsResponse {
                                request_id,
                                status: "ok".to_string(),
                                entries,
                                next_seq,
                                oldest_seq,
                                error: None,
                            })
                        }
                        Err(err) => DevClientResponse::Logs(DevClientLogsResponse {
                            request_id,
                            status: "error".to_string(),
                            entries: vec![],
                            next_seq: 1,
                            oldest_seq: None,
                            error: Some(format!("failed to read console log buffer: {err}")),
                        }),
                    };
                    if let Err(err) = self
                        .designtime_manager
                        .borrow_mut()
                        .send_dev_client_response(response)
                    {
                        log::warn!("failed to send web logs response: {err}");
                    }
                }
            }
        }
    }

    pub(super) fn collect_completed_dev_look_captures(&mut self) {
        let screenshot_map = self.engine.borrow().runtime_context.get_screenshot_map();
        let mut screenshot_map = screenshot_map.borrow_mut();
        let mut completed_request_ids = vec![];
        let mut responses = vec![];

        for (request_id, pending_request) in self.pending_dev_look_requests.iter_mut() {
            let Some(capture_id) = pending_request.capture_in_flight else {
                continue;
            };
            let Some(screenshot) = screenshot_map.remove(&capture_id) else {
                continue;
            };

            pending_request.capture_in_flight = None;
            let captured_at_ms = web_dev_now_ms();
            pending_request.captures.push(DevClientRawCapture {
                rgba_bytes: screenshot.data,
                width: screenshot.width,
                height: screenshot.height,
                captured_at_ms,
            });

            if pending_request.request.period_ms == 0
                || captured_at_ms >= pending_request.deadline_ms
            {
                completed_request_ids.push(request_id.clone());
                responses.push(DevClientResponse::Look(DevClientLookResponse {
                    request_id: request_id.clone(),
                    status: "ok".to_string(),
                    captures: pending_request.captures.clone(),
                    error: None,
                }));
            } else {
                pending_request.next_capture_at_ms =
                    captured_at_ms.saturating_add(pending_request.request.period_ms as u128);
            }
        }

        drop(screenshot_map);

        for request_id in completed_request_ids {
            self.pending_dev_look_requests.remove(&request_id);
        }
        for response in responses {
            if let Err(err) = self
                .designtime_manager
                .borrow_mut()
                .send_dev_client_response(response)
            {
                log::warn!("failed to send web look response: {err}");
            }
        }
    }

    pub(super) fn schedule_due_dev_look_captures(&mut self) {
        let now_ms = web_dev_now_ms();
        let mut capture_requests = vec![];
        for pending_request in self.pending_dev_look_requests.values_mut() {
            if pending_request.capture_in_flight.is_some()
                || pending_request.next_capture_at_ms > now_ms
            {
                continue;
            }

            let capture_id = next_web_dev_capture_id(&mut self.next_dev_capture_id);
            pending_request.capture_in_flight = Some(capture_id);
            capture_requests.push((capture_id, pending_request.request.scale));
        }

        if capture_requests.is_empty() {
            return;
        }

        let engine = self.engine.borrow();
        for (capture_id, scale) in capture_requests {
            engine
                .runtime_context
                .enqueue_native_message(NativeMessage::Screenshot(ScreenshotPatch {
                    id: capture_id,
                    scale: Some(scale),
                }));
        }
    }
}

fn web_dev_now_ms() -> u128 {
    Date::now() as u128
}
