use crate::design_server::{resume_deferred_logic_reload, schedule_logic_reload, AppState};
use crate::dev_session::{
    self, session_request_dir, session_response_dir, write_registered_session, DevCapture,
    DevInspectTreeResponse, DevLogsRequest, DevLogsResponse, DevLookRequest, DevLookResponse,
    DevRayCastRequest, DevRayCastResponse, DevReplaceNodeRequest, DevReplaceNodeResponse,
    DevRequestEnvelope, DevSelectorQueryRequest, DevSelectorQueryResponse,
};
use pax_manifest::{
    code_serialization::serialize_component_to_file, parsing::TemplateNodeParseContext,
};

use actix::{Actor, ActorContext, AsyncContext, Handler, Running, StreamHandler};
use actix_web::web::Data;
use actix_web_actors::ws::{self};
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{ColorType, ImageBuffer, ImageEncoder, Rgba};
use miniz_oxide::inflate::decompress_to_vec_zlib;
use pax_designtime::messages::{
    ActivateAppRevision, AgentMessage, AppRevisionActivationRejected,
    ComponentSerializationRequest, DevClientInspectTreeRequest, DevClientLogsRequest,
    DevClientLookRequest, DevClientRayCastRequest, DevClientReplaceNodeRequest, DevClientResponse,
    DevClientSelectorQueryRequest, DisconnectNotification, LoadFileToStaticDirRequest,
    ManifestSerializationRequest, RevisionStamp, UpdateTemplateRequest,
    UserlandSourceUpdateRequest, UserlandSourceUpdateResponse,
};
use pax_manifest::{ComponentDefinition, ComponentTemplate, PaxManifest, TypeId};
use std::{
    any::Any,
    borrow::Cow,
    collections::HashMap,
    fs,
    io::BufWriter,
    panic::{catch_unwind, AssertUnwindSafe},
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};

pub mod socket_message_accumulator;

pub use socket_message_accumulator::SocketMessageAccumulator;

const WEBSOCKET_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const WEBSOCKET_CLIENT_TIMEOUT: Duration = Duration::from_secs(15);
const SUPERSEDED_CLIENT_CLOSE_DELAY: Duration = Duration::from_millis(50);

pub struct PrivilegedAgentWebSocket {
    state: Data<AppState>,
    socket_msg_accum: SocketMessageAccumulator,
    connection_id: Option<usize>,
    last_heartbeat: Instant,
    superseded: bool,
    /// Reservation established by this standby socket's successful preflight.
    /// If the socket disappears before final commit, release it so a failed
    /// Cancel frame cannot strand every newer build behind the reservation.
    reserved_activation: Option<String>,
}

struct DisconnectSuperseded;

impl actix::Message for DisconnectSuperseded {
    type Result = ();
}

pub(crate) struct SendAgentMessage {
    pub(crate) message: AgentMessage,
}

impl actix::Message for SendAgentMessage {
    type Result = ();
}

impl PrivilegedAgentWebSocket {
    pub fn new(state: Data<AppState>) -> Self {
        Self {
            state,
            socket_msg_accum: SocketMessageAccumulator::new(),
            connection_id: None,
            last_heartbeat: Instant::now(),
            superseded: false,
            reserved_activation: None,
        }
    }

    fn is_active_client(&self) -> bool {
        let Some(connection_id) = self.connection_id else {
            return false;
        };
        self.state.websocket_client_is_active(connection_id)
    }

    fn refresh_dev_session_registration(&self) {
        if !self.is_active_client() {
            return;
        }
        let mut dev_session = self.state.dev_session.lock().unwrap();
        let Some(dev_session) = dev_session.as_mut() else {
            return;
        };
        dev_session.last_seen_ms = dev_session::now_ms();
        if let Err(err) = write_registered_session(dev_session) {
            eprintln!(
                "failed to refresh web dev session {}: {err}",
                dev_session.session_id
            );
        }
    }

    fn poll_dev_requests(&self, ctx: &mut ws::WebsocketContext<Self>) {
        if !self.is_active_client() {
            return;
        }
        let Some(dev_session) = self.state.dev_session.lock().unwrap().clone() else {
            return;
        };
        let request_dir = match session_request_dir(&dev_session) {
            Ok(request_dir) => request_dir,
            Err(err) => {
                eprintln!("failed to resolve web dev request directory: {err}");
                return;
            }
        };

        let request_files = match fs::read_dir(&request_dir) {
            Ok(request_files) => request_files,
            Err(err) => {
                eprintln!("failed to read web dev request directory {request_dir:?}: {err}");
                return;
            }
        };

        for request_file in request_files.flatten() {
            let path = request_file.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }

            let request_id = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("unknown-request")
                .to_string();
            if self
                .state
                .in_flight_dev_requests
                .lock()
                .unwrap()
                .contains(&request_id)
            {
                continue;
            }
            let request_bytes = match fs::read(&path) {
                Ok(request_bytes) => request_bytes,
                Err(err) => {
                    let _ = write_dev_error_response(
                        &dev_session,
                        &request_id,
                        format!("failed to read dev request: {err}"),
                    );
                    let _ = fs::remove_file(&path);
                    continue;
                }
            };
            let request_envelope: DevRequestEnvelope = match serde_json::from_slice(&request_bytes)
            {
                Ok(request_envelope) => request_envelope,
                Err(err) => {
                    let _ = write_dev_error_response(
                        &dev_session,
                        &request_id,
                        format!("failed to decode dev request envelope: {err}"),
                    );
                    let _ = fs::remove_file(&path);
                    continue;
                }
            };

            let forwarded_message = match request_envelope.kind.as_str() {
                "look" => {
                    let look_request: DevLookRequest = match serde_json::from_slice(&request_bytes)
                    {
                        Ok(look_request) => look_request,
                        Err(err) => {
                            let _ = write_dev_error_response(
                                &dev_session,
                                &request_envelope.request_id,
                                format!("failed to decode look request: {err}"),
                            );
                            let _ = fs::remove_file(&path);
                            continue;
                        }
                    };
                    self.state
                        .pending_dev_look_requests
                        .lock()
                        .unwrap()
                        .insert(look_request.request_id.clone(), look_request.clone());
                    AgentMessage::DevClientRequest(
                        pax_designtime::messages::DevClientRequest::Look(DevClientLookRequest {
                            request_id: look_request.request_id,
                            scale: look_request.scale,
                            period_ms: look_request.period_ms,
                            duration_ms: look_request.duration_ms,
                        }),
                    )
                }
                "inspect-tree" => {
                    let inspect_request = match serde_json::from_slice::<
                        crate::dev_session::DevInspectTreeRequest,
                    >(&request_bytes)
                    {
                        Ok(inspect_request) => inspect_request,
                        Err(err) => {
                            let _ = write_dev_error_response(
                                &dev_session,
                                &request_envelope.request_id,
                                format!("failed to decode inspect-tree request: {err}"),
                            );
                            let _ = fs::remove_file(&path);
                            continue;
                        }
                    };
                    AgentMessage::DevClientRequest(
                        pax_designtime::messages::DevClientRequest::InspectTree(
                            DevClientInspectTreeRequest {
                                request_id: inspect_request.request_id,
                                max_depth: inspect_request.max_depth,
                            },
                        ),
                    )
                }
                "ray-cast" => {
                    let ray_cast_request =
                        match serde_json::from_slice::<DevRayCastRequest>(&request_bytes) {
                            Ok(ray_cast_request) => ray_cast_request,
                            Err(err) => {
                                let _ = write_dev_error_response(
                                    &dev_session,
                                    &request_envelope.request_id,
                                    format!("failed to decode ray-cast request: {err}"),
                                );
                                let _ = fs::remove_file(&path);
                                continue;
                            }
                        };
                    AgentMessage::DevClientRequest(
                        pax_designtime::messages::DevClientRequest::RayCast(
                            DevClientRayCastRequest {
                                request_id: ray_cast_request.request_id,
                                x: ray_cast_request.x,
                                y: ray_cast_request.y,
                                hit_invisible: ray_cast_request.hit_invisible,
                            },
                        ),
                    )
                }
                "selector-query" => {
                    let selector_request =
                        match serde_json::from_slice::<DevSelectorQueryRequest>(&request_bytes) {
                            Ok(selector_request) => selector_request,
                            Err(err) => {
                                let _ = write_dev_error_response(
                                    &dev_session,
                                    &request_envelope.request_id,
                                    format!("failed to decode selector request: {err}"),
                                );
                                let _ = fs::remove_file(&path);
                                continue;
                            }
                        };
                    AgentMessage::DevClientRequest(
                        pax_designtime::messages::DevClientRequest::SelectorQuery(
                            DevClientSelectorQueryRequest {
                                request_id: selector_request.request_id,
                                selector: selector_request.selector,
                            },
                        ),
                    )
                }
                "replace-node" => {
                    if !self.state.pax_hot_reload_enabled() {
                        let _ = write_dev_error_response(
                            &dev_session,
                            &request_envelope.request_id,
                            "Pax hot reload is disabled; replace-node cannot mutate the running app"
                                .to_string(),
                        );
                        let _ = fs::remove_file(&path);
                        continue;
                    }
                    let replace_request =
                        match serde_json::from_slice::<DevReplaceNodeRequest>(&request_bytes) {
                            Ok(replace_request) => replace_request,
                            Err(err) => {
                                let _ = write_dev_error_response(
                                    &dev_session,
                                    &request_envelope.request_id,
                                    format!("failed to decode replace-node request: {err}"),
                                );
                                let _ = fs::remove_file(&path);
                                continue;
                            }
                        };
                    AgentMessage::DevClientRequest(
                        pax_designtime::messages::DevClientRequest::ReplaceNode(
                            DevClientReplaceNodeRequest {
                                request_id: replace_request.request_id,
                                component_type_id: replace_request.component_type_id,
                                template_node_id: replace_request.template_node_id,
                                subtemplate: replace_request.subtemplate,
                            },
                        ),
                    )
                }
                "logs" => {
                    let logs_request =
                        match serde_json::from_slice::<DevLogsRequest>(&request_bytes) {
                            Ok(logs_request) => logs_request,
                            Err(err) => {
                                let _ = write_dev_error_response(
                                    &dev_session,
                                    &request_envelope.request_id,
                                    format!("failed to decode logs request: {err}"),
                                );
                                let _ = fs::remove_file(&path);
                                continue;
                            }
                        };
                    AgentMessage::DevClientRequest(
                        pax_designtime::messages::DevClientRequest::Logs(DevClientLogsRequest {
                            request_id: logs_request.request_id,
                            since_seq: logs_request.since_seq,
                            limit: logs_request.limit,
                        }),
                    )
                }
                unsupported_kind => {
                    let _ = write_dev_error_response(
                        &dev_session,
                        &request_envelope.request_id,
                        format!("unsupported dev request kind: {unsupported_kind}"),
                    );
                    let _ = fs::remove_file(&path);
                    continue;
                }
            };

            match rmp_serde::to_vec(&forwarded_message) {
                Ok(serialized_message) => {
                    ctx.binary(serialized_message);
                    self.state
                        .in_flight_dev_requests
                        .lock()
                        .unwrap()
                        .insert(request_envelope.request_id.clone());
                }
                Err(err) => {
                    let _ = write_dev_error_response(
                        &dev_session,
                        &request_envelope.request_id,
                        format!("failed to serialize forwarded dev request: {err}"),
                    );
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }
}

impl Handler<DisconnectSuperseded> for PrivilegedAgentWebSocket {
    type Result = ();

    fn handle(&mut self, _msg: DisconnectSuperseded, ctx: &mut Self::Context) -> Self::Result {
        // Promotion and disconnect messages are delivered by different socket
        // actors. A late same-revision hello can therefore make this socket
        // active again before the queued supersession reaches it. In that case
        // the supersession is stale and must not disable reconnect or schedule
        // a close for the current owner.
        if self.is_active_client() {
            return;
        }
        // Once the notification is sent the client disables reconnect. Keep
        // this actor terminal so an inbound hello or activation cannot reclaim
        // ownership during the short notification-delivery grace period.
        self.superseded = true;
        let reason = "Superseded by a newer Pax dev browser client";
        let notification = AgentMessage::DisconnectNotification(DisconnectNotification {
            allow_reconnect: false,
            reason: reason.to_string(),
        });
        ctx.binary(rmp_serde::to_vec(&notification).unwrap());
        ctx.run_later(SUPERSEDED_CLIENT_CLOSE_DELAY, move |_actor, ctx| {
            ctx.close(Some(ws::CloseReason {
                code: ws::CloseCode::Normal,
                description: Some(reason.to_string()),
            }));
            ctx.stop();
        });
    }
}

impl Handler<SendAgentMessage> for PrivilegedAgentWebSocket {
    type Result = ();

    fn handle(&mut self, msg: SendAgentMessage, ctx: &mut Self::Context) -> Self::Result {
        if !self.is_active_client() {
            return;
        }
        match rmp_serde::to_vec(&msg.message) {
            Ok(serialized) => ctx.binary(serialized),
            Err(err) => eprintln!("failed to serialize outbound agent message: {err}"),
        }
    }
}

fn send_agent_message_in_context(
    ctx: &mut ws::WebsocketContext<PrivilegedAgentWebSocket>,
    message: AgentMessage,
) {
    match rmp_serde::to_vec(&message) {
        Ok(serialized) => ctx.binary(serialized),
        Err(err) => eprintln!("failed to serialize outbound agent message: {err}"),
    }
}

impl Actor for PrivilegedAgentWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let connection_id = self.state.generate_websocket_client_id();
        self.connection_id = Some(connection_id);
        // A freshly constructed cartridge stays on standby until its hello or
        // activation proves which logic revision it owns.  Replacing the active
        // socket here would strand the old, still-mounted host when preparation
        // of the candidate later fails.
        ctx.run_interval(WEBSOCKET_HEARTBEAT_INTERVAL, |actor, ctx| {
            if Instant::now().duration_since(actor.last_heartbeat) > WEBSOCKET_CLIENT_TIMEOUT {
                log::warn!("timed out waiting for a heartbeat from the Pax dev browser client");
                ctx.close(Some(ws::CloseReason {
                    code: ws::CloseCode::Policy,
                    description: Some("Timed out waiting for websocket heartbeat".to_string()),
                }));
                ctx.stop();
                return;
            }

            ctx.ping(b"pax-dev-heartbeat");
        });
        ctx.run_interval(Duration::from_millis(50), |actor, ctx| {
            actor.poll_dev_requests(ctx);
        });
        ctx.run_interval(Duration::from_secs(2), |actor, _ctx| {
            actor.refresh_dev_session_registration();
        });
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        if let Some((logic_revision_id, connection_id)) =
            self.reserved_activation.take().zip(self.connection_id)
        {
            let discarded = self
                .state
                .revisions
                .lock()
                .unwrap()
                .discard_logic_candidate_if_owner(&logic_revision_id, connection_id);
            if discarded {
                schedule_logic_reload(self.state.clone());
                resume_deferred_logic_reload(self.state.clone());
            }
        }
        let mut active_client = self.state.active_websocket_client.lock().unwrap();
        let was_active_client = active_client.as_ref().zip(self.connection_id).is_some_and(
            |(active_client, connection_id)| active_client.connection_id == connection_id,
        );
        if was_active_client {
            *active_client = None;
        }
        drop(active_client);

        if was_active_client {
            self.state.in_flight_dev_requests.lock().unwrap().clear();
            if let Some(dev_session) = self.state.dev_session.lock().unwrap().as_ref() {
                let _ = dev_session::remove_registered_session(&dev_session.session_id);
            }
        }
        Running::Stop
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for PrivilegedAgentWebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let Ok(msg) = msg else {
            eprintln!("failed to receive on socket");
            return;
        };

        match &msg {
            ws::Message::Ping(bytes) => {
                self.last_heartbeat = Instant::now();
                ctx.pong(bytes);
                return;
            }
            ws::Message::Pong(_) => {
                self.last_heartbeat = Instant::now();
                return;
            }
            ws::Message::Close(reason) => {
                self.last_heartbeat = Instant::now();
                ctx.close(reason.clone());
                ctx.stop();
                return;
            }
            _ => {}
        }

        let processed_message = self.socket_msg_accum.process(msg);
        if let Ok(Some(bin_data)) = processed_message {
            match rmp_serde::from_slice::<AgentMessage>(&bin_data) {
                Ok(AgentMessage::LoadManifestRequest(request)) => {
                    let client_stamp = request.active_revision.as_ref();
                    let client_revision =
                        client_stamp.map(|revision| revision.logic_revision_id.as_str());
                    if !self.superseded {
                        if let Some(connection_id) = self.connection_id {
                            let (promoted, previous) =
                                self.state.reconcile_and_promote_websocket_client(
                                    connection_id,
                                    ctx.address(),
                                    client_stamp,
                                );
                            if let Some(previous) = previous {
                                previous.addr.do_send(DisconnectSuperseded);
                            }
                            if promoted {
                                self.refresh_dev_session_registration();
                            }
                        }
                    }

                    if let Some(prepare) = self.state.prepare_for_web_client(client_revision) {
                        send_agent_message_in_context(
                            ctx,
                            AgentMessage::PrepareAppRevision(prepare),
                        );
                    }
                    match self.state.active_manifest_response() {
                        Ok(Some(response)) => send_agent_message_in_context(
                            ctx,
                            AgentMessage::LoadManifestResponse(response),
                        ),
                        Ok(None) => {}
                        Err(err) => eprintln!("failed to load active manifest: {err}"),
                    }
                }
                Ok(AgentMessage::ComponentSerializationRequest(request)) => {
                    let Some(connection_id) = self.connection_id else {
                        return;
                    };
                    let Some(_source_update) =
                        self.state.source_update_guard_for_client(connection_id)
                    else {
                        return;
                    };
                    let project_root = self.state.userland_project_root.lock().unwrap().clone();
                    let mut serialized_paths = vec![];
                    let result = self.state.apply_pax_source_mutation(
                        "component serialization",
                        |next_revisions| {
                            handle_component_serialization_request(
                                request,
                                next_revisions,
                                &project_root,
                                &mut serialized_paths,
                            )
                        },
                    );
                    register_serialized_watcher_echoes(
                        &self.state,
                        &project_root,
                        serialized_paths,
                    );
                    if let Err(err) = result {
                        eprintln!("rejected component serialization request: {err}");
                    }
                }
                Ok(AgentMessage::ManifestSerializationRequest(request)) => {
                    let Some(connection_id) = self.connection_id else {
                        return;
                    };
                    let Some(_source_update) =
                        self.state.source_update_guard_for_client(connection_id)
                    else {
                        return;
                    };
                    let project_root = self.state.userland_project_root.lock().unwrap().clone();
                    let mut serialized_paths = vec![];
                    let result = self.state.apply_pax_source_mutation(
                        "manifest serialization",
                        |next_revisions| {
                            handle_manifest_serialization_request(
                                request,
                                next_revisions,
                                &project_root,
                                &mut serialized_paths,
                            )
                        },
                    );
                    register_serialized_watcher_echoes(
                        &self.state,
                        &project_root,
                        serialized_paths,
                    );
                    if let Err(err) = result {
                        eprintln!("rejected manifest serialization request: {err}");
                    }
                }
                Ok(AgentMessage::LoadFileToStaticDirRequest(load_info)) => {
                    let Some(connection_id) = self.connection_id else {
                        return;
                    };
                    let Some(_source_update) =
                        self.state.source_update_guard_for_client(connection_id)
                    else {
                        return;
                    };
                    let LoadFileToStaticDirRequest { name, data } = load_info;
                    println!(
                        "received a file {} (size: {})! root dir to write to: {:?}",
                        name,
                        data.len(),
                        self.state.userland_project_root.lock().unwrap(),
                    );

                    let mut path = self.state.userland_project_root.lock().unwrap().clone();
                    path.push("assets");
                    path.push(&name);

                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)
                            .unwrap_or_else(|e| eprintln!("Failed to create directory: {}", e));
                    }
                    if std::fs::write(&path, data.clone()).is_err() {
                        eprintln!("server couldn't write to assets folder: {:?}", path);
                    };
                    let path = self
                        .state
                        .serve_dir
                        .lock()
                        .unwrap()
                        .clone()
                        .join("assets")
                        .join(name);
                    if std::fs::write(&path, data).is_err() {
                        eprintln!("server couldn't write to served folder: {:?}", path);
                    };
                }
                Ok(AgentMessage::UserlandSourceUpdateRequest(request)) => {
                    let Some(connection_id) = self.connection_id else {
                        return;
                    };
                    let Some(_source_update) =
                        self.state.source_update_guard_for_client(connection_id)
                    else {
                        return;
                    };
                    handle_userland_source_update_request(self.state.clone(), request, ctx);
                }
                Ok(AgentMessage::DevClientResponse(response)) => {
                    if self.is_active_client() {
                        if let Err(err) = handle_dev_client_response(&self.state, response) {
                            eprintln!("failed to handle web dev response: {err}");
                        }
                    }
                }
                Ok(AgentMessage::RequestAppRevisionActivation(ActivateAppRevision {
                    logic_revision_id,
                })) if !self.superseded => {
                    let Some(connection_id) = self.connection_id else {
                        return;
                    };
                    match self
                        .state
                        .prepare_logic_activation(&logic_revision_id, connection_id)
                    {
                        Ok(response) => {
                            self.reserved_activation = Some(logic_revision_id);
                            send_agent_message_in_context(
                                ctx,
                                AgentMessage::AppRevisionActivationPrepared(response),
                            );
                        }
                        Err(err) => {
                            eprintln!(
                                "rejected logic activation preflight {logic_revision_id}: {err}"
                            );
                            if update_requires_logic_reload(&err) {
                                schedule_logic_reload(self.state.clone());
                            }
                            send_agent_message_in_context(
                                ctx,
                                AgentMessage::AppRevisionActivationRejected(
                                    AppRevisionActivationRejected {
                                        logic_revision_id,
                                        reason: err,
                                    },
                                ),
                            );
                            resume_deferred_logic_reload(self.state.clone());
                        }
                    }
                }
                Ok(AgentMessage::CancelAppRevisionActivation(ActivateAppRevision {
                    logic_revision_id,
                })) if !self.superseded
                    && self.reserved_activation.as_deref() == Some(logic_revision_id.as_str()) =>
                {
                    let Some(connection_id) = self.connection_id else {
                        return;
                    };
                    let discarded = self
                        .state
                        .revisions
                        .lock()
                        .unwrap()
                        .discard_logic_candidate_if_owner(&logic_revision_id, connection_id);
                    if self.reserved_activation.as_deref() == Some(&logic_revision_id) {
                        self.reserved_activation = None;
                    }
                    if discarded {
                        schedule_logic_reload(self.state.clone());
                    }
                    resume_deferred_logic_reload(self.state.clone());
                }
                Ok(AgentMessage::CancelAppRevisionActivation(_)) => {}
                Ok(AgentMessage::ActivateAppRevision(ActivateAppRevision {
                    logic_revision_id,
                })) if !self.superseded => {
                    let Some(connection_id) = self.connection_id else {
                        return;
                    };
                    match self.state.activate_logic_revision_and_promote(
                        &logic_revision_id,
                        connection_id,
                        ctx.address(),
                    ) {
                        Ok((outcome, promoted, previous)) => {
                            if self.reserved_activation.as_deref() == Some(&logic_revision_id) {
                                self.reserved_activation = None;
                            }
                            if let Some(previous) = previous {
                                previous.addr.do_send(DisconnectSuperseded);
                            }
                            if promoted {
                                self.refresh_dev_session_registration();
                            }
                            send_agent_message_in_context(
                                ctx,
                                AgentMessage::LoadManifestResponse(outcome.response),
                            );
                            if outcome.requires_followup_rebuild {
                                schedule_logic_reload(self.state.clone());
                            }
                            resume_deferred_logic_reload(self.state.clone());
                        }
                        Err(err) => {
                            let discarded = self
                                .state
                                .revisions
                                .lock()
                                .unwrap()
                                .discard_logic_candidate_if_owner(
                                    &logic_revision_id,
                                    connection_id,
                                );
                            if self.reserved_activation.as_deref() == Some(&logic_revision_id) {
                                self.reserved_activation = None;
                            }
                            eprintln!("rejected logic activation {logic_revision_id}: {err}");
                            if update_requires_logic_reload(&err) {
                                schedule_logic_reload(self.state.clone());
                            }
                            if discarded {
                                schedule_logic_reload(self.state.clone());
                            }
                            send_agent_message_in_context(
                                ctx,
                                AgentMessage::AppRevisionActivationRejected(
                                    AppRevisionActivationRejected {
                                        logic_revision_id: logic_revision_id.clone(),
                                        reason: err,
                                    },
                                ),
                            );
                            if let Some(prepare) =
                                self.state.prepare_for_web_client(Some(&logic_revision_id))
                            {
                                send_agent_message_in_context(
                                    ctx,
                                    AgentMessage::PrepareAppRevision(prepare),
                                );
                            }
                        }
                    }
                }
                Ok(AgentMessage::ActivateAppRevision(_)) => {}
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Deserialization error: {:?}", e);
                }
            }
        } else if let Ok(None) = processed_message {
            // Do nothing, wait until entire message has been received
        } else {
            eprintln!("unhandled socket message");
        }
    }
}

fn handle_component_serialization_request(
    request: ComponentSerializationRequest,
    revisions: &mut crate::design_server::revision::DebugRevisionCoordinator,
    project_root: &Path,
    serialized_paths: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let component: ComponentDefinition = rmp_serde::from_slice(&request.component_bytes)
        .map_err(|err| format!("failed to decode component: {err}"))?;
    let file_path = component
        .template
        .as_ref()
        .ok_or_else(|| "serialized component is missing a template".to_string())?
        .get_file_path()
        .ok_or_else(|| "serialized component is missing a source path".to_string())?
        .to_owned();
    revisions.replace_active_component(component.clone())?;
    serialize_component_to_file(&component, file_path.clone());
    serialized_paths.push(PathBuf::from(&file_path));
    revisions.record_serialized_component(
        normalized_pax_mutation_path(&file_path, project_root),
        component,
    );
    Ok(())
}

fn handle_manifest_serialization_request(
    request: ManifestSerializationRequest,
    revisions: &mut crate::design_server::revision::DebugRevisionCoordinator,
    project_root: &Path,
    serialized_paths: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let manifest: PaxManifest = rmp_serde::from_slice(&request.manifest)
        .map_err(|err| format!("failed to decode manifest: {err}"))?;
    revisions.replace_active_authoring_manifest(&manifest)?;
    for component in manifest.components.values() {
        let file_path = component
            .template
            .as_ref()
            .and_then(|template| template.get_file_path());
        if let Some(file_path) = &file_path {
            serialize_component_to_file(component, file_path.clone());
            serialized_paths.push(PathBuf::from(file_path));
            revisions.record_serialized_component(
                normalized_pax_mutation_path(file_path, project_root),
                component.clone(),
            );
        }
    }
    Ok(())
}

fn register_serialized_watcher_echoes(
    state: &AppState,
    project_root: &Path,
    serialized_paths: Vec<PathBuf>,
) {
    for path in serialized_paths {
        let path = if path.is_absolute() {
            path
        } else {
            project_root.join(path)
        };
        match fs::read_to_string(&path) {
            Ok(contents) => state.expect_watcher_write(&path, &contents),
            Err(err) => eprintln!(
                "failed to register serialized source write {}: {err}",
                path.display()
            ),
        }
    }
}

fn handle_userland_source_update_request(
    state: Data<AppState>,
    request: UserlandSourceUpdateRequest,
    ctx: &mut ws::WebsocketContext<PrivilegedAgentWebSocket>,
) {
    let resolved_path = match resolve_userland_source_path(&state, &request.path) {
        Ok(resolved_path) => resolved_path,
        Err(err) => {
            send_userland_source_update_response_in_context(
                ctx,
                UserlandSourceUpdateResponse {
                    request_id: request.request_id,
                    path: request.path,
                    status: "error".to_string(),
                    error: Some(err),
                },
            );
            return;
        }
    };

    match resolved_path
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("pax") => {
            let project_root = state.userland_project_root.lock().unwrap().clone();
            let authoring_manifest = {
                let revisions = state.revisions.lock().unwrap();
                revisions
                    .candidate_manifest_clone()
                    .or_else(|| revisions.active_manifest_clone())
            };
            let Some(authoring_manifest) = authoring_manifest else {
                send_userland_source_update_response_in_context(
                    ctx,
                    UserlandSourceUpdateResponse {
                        request_id: request.request_id,
                        path: request.path,
                        status: "error".to_string(),
                        error: Some("design server manifest is unavailable".to_string()),
                    },
                );
                return;
            };
            if let Err(err) = parse_pax_source_update(
                &authoring_manifest,
                &resolved_path.to_string_lossy(),
                &request.contents,
                &project_root,
            ) {
                send_userland_source_update_response_in_context(
                    ctx,
                    UserlandSourceUpdateResponse {
                        request_id: request.request_id,
                        path: request.path,
                        status: "error".to_string(),
                        error: Some(err),
                    },
                );
                return;
            }

            state.expect_watcher_write(&resolved_path, &request.contents);
            if let Err(err) = fs::write(&resolved_path, &request.contents) {
                state.cancel_expected_watcher_write(&resolved_path);
                send_userland_source_update_response_in_context(
                    ctx,
                    UserlandSourceUpdateResponse {
                        request_id: request.request_id,
                        path: request.path,
                        status: "error".to_string(),
                        error: Some(format!("failed to write Pax source: {err}")),
                    },
                );
                return;
            }
            if !state.pax_hot_reload_enabled() {
                state.note_pax_restart_required();
                send_userland_source_update_response_in_context(
                    ctx,
                    UserlandSourceUpdateResponse {
                        request_id: request.request_id,
                        path: request.path,
                        status: "ok".to_string(),
                        error: None,
                    },
                );
                return;
            }
            let mutation_path =
                normalized_pax_mutation_path(&resolved_path.to_string_lossy(), &project_root);
            let mutation_generation = state
                .revisions
                .lock()
                .unwrap()
                .record_pax_source(mutation_path.clone(), request.contents.clone());

            let mut rebuild_required = false;
            let mut update_handled = false;
            let mut terminal_error = None;
            for _ in 0..3 {
                let (active_snapshot, has_candidate, mutation_is_current) = {
                    let revisions = state.revisions.lock().unwrap();
                    (
                        revisions.active_manifest_snapshot(),
                        revisions.has_candidate(),
                        revisions.pax_mutation_is_current(&mutation_path, mutation_generation),
                    )
                };
                if !mutation_is_current {
                    // A newer edit to this source path owns the journal entry;
                    // it will stream or rebuild the authoritative contents.
                    update_handled = true;
                    break;
                }
                let Some((expected_revision, active_manifest)) = active_snapshot else {
                    terminal_error = Some("design server manifest is unavailable".to_string());
                    break;
                };
                let update_request = match parse_pax_source_update(
                    &active_manifest,
                    &resolved_path.to_string_lossy(),
                    &request.contents,
                    &project_root,
                ) {
                    Ok(update) => update,
                    Err(err) if has_candidate => {
                        eprintln!("deferring Pax source update until logic activation: {err}");
                        update_handled = true;
                        break;
                    }
                    Err(err) => {
                        terminal_error = Some(err);
                        break;
                    }
                };

                match state.commit_template_update_and_route(
                    &expected_revision,
                    Some((&mutation_path, mutation_generation)),
                    update_request,
                ) {
                    Ok(_) => {
                        update_handled = true;
                        break;
                    }
                    Err(err)
                        if err.starts_with("active revision changed while parsing Pax source") =>
                    {
                        continue;
                    }
                    Err(err)
                        if err.starts_with(
                            "Pax source mutation was superseded by a newer update",
                        ) =>
                    {
                        update_handled = true;
                        break;
                    }
                    Err(err) if update_requires_logic_reload(&err) => {
                        rebuild_required = true;
                        update_handled = true;
                        break;
                    }
                    Err(err) => {
                        let has_candidate = state.revisions.lock().unwrap().has_candidate();
                        if has_candidate {
                            eprintln!("deferring Pax source update until logic activation: {err}");
                            update_handled = true;
                        } else {
                            terminal_error = Some(err);
                        }
                        break;
                    }
                }
            }

            if !update_handled && terminal_error.is_none() {
                // Repeated concurrent mutations kept invalidating the parse.
                // The source journal is durable, so rebuild instead of risking
                // a stale template commit or silently dropping the edit.
                rebuild_required = true;
            }

            if let Some(err) = terminal_error {
                send_userland_source_update_response_in_context(
                    ctx,
                    UserlandSourceUpdateResponse {
                        request_id: request.request_id,
                        path: request.path,
                        status: "error".to_string(),
                        error: Some(err),
                    },
                );
                return;
            }

            if rebuild_required {
                schedule_logic_reload(state.clone());
            }

            send_userland_source_update_response_in_context(
                ctx,
                UserlandSourceUpdateResponse {
                    request_id: request.request_id,
                    path: request.path,
                    status: "ok".to_string(),
                    error: None,
                },
            );
        }
        Some("rs") => {
            let request_id = request.request_id.clone();
            let request_path = request.path.clone();
            state.expect_watcher_write(&resolved_path, &request.contents);
            if let Err(err) = fs::write(&resolved_path, &request.contents) {
                state.cancel_expected_watcher_write(&resolved_path);
                send_userland_source_update_response_in_context(
                    ctx,
                    UserlandSourceUpdateResponse {
                        request_id,
                        path: request_path,
                        status: "error".to_string(),
                        error: Some(format!("failed to write Rust source: {err}")),
                    },
                );
                return;
            }

            if !state.logic_hot_reload_available() {
                state.note_logic_restart_required();
                send_userland_source_update_response_in_context(
                    ctx,
                    UserlandSourceUpdateResponse {
                        request_id,
                        path: request_path,
                        status: "ok".to_string(),
                        error: None,
                    },
                );
                return;
            }

            if let Err(err) =
                spawn_userland_rust_source_update(state, request_id.clone(), request_path.clone())
            {
                send_userland_source_update_response_in_context(
                    ctx,
                    UserlandSourceUpdateResponse {
                        request_id,
                        path: request_path,
                        status: "error".to_string(),
                        error: Some(err),
                    },
                );
            }
        }
        _ => {
            send_userland_source_update_response_in_context(
                ctx,
                UserlandSourceUpdateResponse {
                    request_id: request.request_id,
                    path: request.path,
                    status: "error".to_string(),
                    error: Some("only .rs and .pax source updates are supported".to_string()),
                },
            );
        }
    }
}

fn send_userland_source_update_response_in_context(
    ctx: &mut ws::WebsocketContext<PrivilegedAgentWebSocket>,
    response: UserlandSourceUpdateResponse,
) {
    let message = AgentMessage::UserlandSourceUpdateResponse(response);
    match rmp_serde::to_vec(&message) {
        Ok(serialized) => ctx.binary(serialized),
        Err(err) => eprintln!("failed to serialize userland source update response: {err}"),
    }
}

fn send_agent_message_to_active_client(state: &Data<AppState>, message: AgentMessage) {
    super::send_agent_message_to_active_client(state, message);
}

fn send_userland_source_update_response(
    state: &Data<AppState>,
    request_id: String,
    path: String,
    status: &str,
    error: Option<String>,
) {
    send_agent_message_to_active_client(
        state,
        AgentMessage::UserlandSourceUpdateResponse(UserlandSourceUpdateResponse {
            request_id,
            path,
            status: status.to_string(),
            error,
        }),
    );
}

fn resolve_userland_source_path(
    state: &Data<AppState>,
    requested_path: &str,
) -> Result<PathBuf, String> {
    let requested_path = Path::new(requested_path);
    if requested_path.is_absolute() {
        return Err("absolute source paths are not allowed".to_string());
    }
    if requested_path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err("source path must stay inside the project root".to_string());
    }

    let resolved_path = state
        .userland_project_root
        .lock()
        .unwrap()
        .join(requested_path);
    match resolved_path
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("rs") | Some("pax") => Ok(resolved_path),
        _ => Err("only .rs and .pax source updates are supported".to_string()),
    }
}

pub(super) fn apply_pax_source_update(
    state: &Data<AppState>,
    source_path: &str,
    content: &str,
) -> Result<(), String> {
    if !state.pax_hot_reload_enabled() {
        return Err("Pax hot reload is disabled for this session".to_string());
    }
    let _source_update = state.source_update_lock.lock().unwrap();
    let project_root = state.userland_project_root.lock().unwrap().clone();
    let disk_path = if Path::new(source_path).is_absolute() {
        PathBuf::from(source_path)
    } else {
        project_root.join(source_path)
    };
    if disk_path.is_file() {
        let current = fs::read_to_string(&disk_path)
            .map_err(|err| format!("failed to verify Pax watcher source: {err}"))?;
        if current != content {
            return Err(
                "stale Pax watcher update was superseded by newer file contents".to_string(),
            );
        }
    }
    let mutation_path = normalized_pax_mutation_path(source_path, &project_root);
    let mutation_generation = state
        .revisions
        .lock()
        .unwrap()
        .record_pax_source(mutation_path.clone(), content.to_string());

    for _ in 0..3 {
        let (active_snapshot, has_candidate, mutation_is_current) = {
            let revisions = state.revisions.lock().unwrap();
            (
                revisions.active_manifest_snapshot(),
                revisions.has_candidate(),
                revisions.pax_mutation_is_current(&mutation_path, mutation_generation),
            )
        };
        if !mutation_is_current {
            return Ok(());
        }
        let Some((expected_revision, active_manifest)) = active_snapshot else {
            return Err("design server manifest is unavailable".to_string());
        };
        let update_request =
            match parse_pax_source_update(&active_manifest, source_path, content, &project_root) {
                Ok(update) => update,
                Err(err) if has_candidate => {
                    eprintln!("deferring Pax watcher update until logic activation: {err}");
                    return Ok(());
                }
                Err(err) => return Err(err),
            };

        match state.commit_template_update_and_route(
            &expected_revision,
            Some((&mutation_path, mutation_generation)),
            update_request,
        ) {
            Ok(_) => return Ok(()),
            Err(err) if err.starts_with("active revision changed while parsing Pax source") => {
                continue;
            }
            Err(err) if err.starts_with("Pax source mutation was superseded by a newer update") => {
                return Ok(());
            }
            Err(err) if has_candidate => {
                eprintln!("deferring Pax watcher update until logic activation: {err}");
                return Ok(());
            }
            Err(err) => return Err(err),
        }
    }

    Err(
        "logic artifact changed repeatedly while applying Pax watcher update; rebuilding"
            .to_string(),
    )
}

pub(super) fn normalized_pax_mutation_path(source_path: &str, project_root: &Path) -> String {
    let source_path = Path::new(source_path);
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::new());
    let absolute_project_root = if project_root.is_absolute() {
        project_root.to_path_buf()
    } else {
        current_dir.join(project_root)
    };
    let absolute = if source_path.is_absolute() {
        source_path.to_path_buf()
    } else {
        let cwd_candidate = current_dir.join(source_path);
        if cwd_candidate.starts_with(&absolute_project_root) && cwd_candidate.exists() {
            cwd_candidate
        } else if let Ok(relative) = source_path.strip_prefix(project_root) {
            absolute_project_root.join(relative)
        } else {
            absolute_project_root.join(source_path)
        }
    };
    absolute
        .canonicalize()
        .unwrap_or(absolute)
        .to_string_lossy()
        .into_owned()
}

pub(super) fn parse_pax_source_update(
    manifest: &PaxManifest,
    source_path: &str,
    content: &str,
    project_root: &Path,
) -> Result<UpdateTemplateRequest, String> {
    let mut template_map = HashMap::new();
    let mut matched_component: Option<TypeId> = None;
    let mut matched_component_module_path: Option<String> = None;
    let mut original_template: Option<ComponentTemplate> = None;

    for (type_id, component) in manifest.components.iter() {
        if let Some(identifier) = type_id.get_pascal_identifier() {
            template_map.insert(identifier, type_id.clone());
        }
        if let Some(template) = &component.template {
            if template
                .get_file_path()
                .as_ref()
                .is_some_and(|file_path| source_paths_match(file_path, source_path, project_root))
            {
                matched_component = Some(type_id.clone());
                matched_component_module_path = Some(component.module_path.clone());
                original_template = Some(template.clone());
            }
        }
    }

    let self_type_id = matched_component.ok_or_else(|| {
        format!("no component in the manifest is backed by source path {source_path}")
    })?;
    let component_module_path = matched_component_module_path
        .ok_or_else(|| "component definition is missing a module path".to_string())?;
    let original_template =
        original_template.ok_or_else(|| "component template is missing a file path".to_string())?;

    catch_unwind(AssertUnwindSafe(|| {
        let mut tpc = TemplateNodeParseContext {
            pascal_identifier_to_type_id_map: template_map,
            route_branch_descriptors: manifest
                .components
                .iter()
                .filter_map(|(type_id, definition)| {
                    definition
                        .route_branch
                        .clone()
                        .map(|descriptor| (type_id.clone(), descriptor))
                })
                .collect(),
            template: ComponentTemplate::new(
                self_type_id.clone(),
                original_template.get_file_path(),
            ),
        };

        let ast =
            pax_language::parse_pax_str(pax_language::Rule::pax_component_definition, content)
                .map_err(|err| format!("failed to parse Pax source: {err}"))?;
        let mut settings =
            pax_manifest::parsing::parse_settings_from_component_definition_string(ast.clone());
        if let Some(rust_source_path) =
            resolve_component_rust_source_path(project_root, &component_module_path)
        {
            pax_manifest::parsing::augment_settings_with_implicit_lifecycle_handlers(
                &mut settings,
                &component_module_path,
                &self_type_id,
                &rust_source_path.to_string_lossy(),
            );
        }
        pax_manifest::parsing::parse_template_from_component_definition_string(
            &mut tpc, content, ast,
        );

        Ok(UpdateTemplateRequest {
            revision: RevisionStamp {
                logic_revision_id: String::new(),
                template_version: 0,
            },
            type_id: self_type_id,
            new_template: tpc.template,
            settings_block: settings,
        })
    }))
    .map_err(panic_payload_to_string)?
}

fn source_paths_match(manifest_path: &str, source_path: &str, project_root: &Path) -> bool {
    let manifest_path = Path::new(manifest_path);
    let source_path = Path::new(source_path);

    if manifest_path == source_path {
        return true;
    }

    let manifest_relative = project_relative_path(manifest_path, project_root);
    let source_relative = project_relative_path(source_path, project_root);
    if manifest_relative == source_relative {
        return true;
    }

    let manifest_absolute = project_absolute_path(manifest_path, project_root);
    let source_absolute = project_absolute_path(source_path, project_root);
    if manifest_absolute == source_absolute {
        return true;
    }

    match (
        manifest_absolute.canonicalize(),
        source_absolute.canonicalize(),
    ) {
        (Ok(manifest_canonical), Ok(source_canonical)) => manifest_canonical == source_canonical,
        _ => false,
    }
}

fn project_relative_path<'a>(path: &'a Path, project_root: &Path) -> Cow<'a, Path> {
    if let Ok(relative_path) = path.strip_prefix(project_root) {
        return Cow::Borrowed(relative_path);
    }

    if let Ok(current_dir) = std::env::current_dir() {
        let absolute_project_root = if project_root.is_absolute() {
            project_root.to_path_buf()
        } else {
            current_dir.join(project_root)
        };

        if path.is_absolute() {
            if let Ok(relative_path) = path.strip_prefix(&absolute_project_root) {
                return Cow::Owned(relative_path.to_path_buf());
            }
        } else {
            let absolute_path = current_dir.join(path);
            if let Ok(relative_path) = absolute_path.strip_prefix(&absolute_project_root) {
                return Cow::Owned(relative_path.to_path_buf());
            }
        }
    }

    if path.is_absolute() {
        Cow::Borrowed(path)
    } else {
        Cow::Borrowed(path)
    }
}

fn project_absolute_path(path: &Path, project_root: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    }
}

fn resolve_component_rust_source_path(project_root: &Path, module_path: &str) -> Option<PathBuf> {
    let cleaned_module_path = pax_manifest::parsing::clean_module_path(module_path);
    let mut module_segments: Vec<&str> = cleaned_module_path.split("::").collect();
    if module_segments.first().copied() == Some("crate") {
        module_segments.remove(0);
    }

    let src_dir = project_root.join("src");
    let candidates = if module_segments.is_empty() {
        vec![
            src_dir.join("lib.rs"),
            src_dir.join("main.rs"),
            src_dir.join("mod.rs"),
        ]
    } else {
        let mut path = src_dir;
        for segment in &module_segments {
            path.push(segment);
        }
        vec![path.with_extension("rs"), path.join("mod.rs")]
    };

    candidates.into_iter().find(|candidate| candidate.is_file())
}

pub(super) fn update_requires_logic_reload(error: &str) -> bool {
    error.contains("compiled cartridge capability")
        || error.contains("logic artifact")
        || error.contains("could not replay the latest Pax source")
}

fn panic_payload_to_string(payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        format!("failed to parse Pax source: {message}")
    } else if let Some(message) = payload.downcast_ref::<&'static str>() {
        format!("failed to parse Pax source: {message}")
    } else {
        "failed to parse Pax source".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        normalized_pax_mutation_path, parse_pax_source_update, resolve_component_rust_source_path,
        source_paths_match,
    };
    use crate::design_server::{revision::ActivationChannel, web_socket, AppState};
    use crate::HotReloadMode;
    use actix_web::{web::Data, App};
    use futures_util::{SinkExt, StreamExt};
    use pax_designtime::messages::{
        ActivateAppRevision, AgentMessage, DebugArtifact, DebugLogicExecutionMode,
        LoadManifestRequest, PrepareAppRevision,
    };
    use pax_manifest::{
        ComponentDefinition, ComponentTemplate, PaxManifest, SettingsBlockElement, TypeId,
    };
    use rmp_serde::from_slice;
    use std::path::{Path, PathBuf};
    use std::{
        collections::{BTreeMap, HashMap},
        fs,
    };
    use tempfile::tempdir;

    #[actix_web::test]
    async fn sends_pending_web_revision_after_client_hello() {
        let manifest = basic_manifest("Example");
        let state = Data::new(
            AppState::new(
                PathBuf::new(),
                PathBuf::new(),
                manifest.clone(),
                None,
                None,
                HotReloadMode::All,
                None,
                None,
            )
            .unwrap(),
        );
        state
            .revisions
            .lock()
            .unwrap()
            .stage_logic_candidate(
                manifest,
                PrepareAppRevision {
                    logic_revision_id: "build-1".to_string(),
                    execution_mode: DebugLogicExecutionMode::CompiledArtifact,
                    artifact: DebugArtifact {
                        kind: "web-cartridge".to_string(),
                        location: "/__reloads__/build-1/pax-cartridge".to_string(),
                    },
                },
                ActivationChannel::WebSocket,
            )
            .unwrap();
        let server_state = state.clone();
        let srv = actix_test::start(move || {
            App::new()
                .app_data(server_state.clone())
                .service(web_socket)
        });

        let client = awc::Client::new();
        let (_resp, mut connection) = client.ws(srv.url("/ws")).connect().await.unwrap();
        let hello = AgentMessage::LoadManifestRequest(LoadManifestRequest {
            active_revision: None,
        });
        connection
            .send(awc::ws::Message::Binary(
                rmp_serde::to_vec(&hello).unwrap().into(),
            ))
            .await
            .unwrap();

        let Some(Ok(awc::ws::Frame::Binary(bin_data))) = connection.next().await else {
            panic!("expected prepare-app-revision websocket message");
        };
        let message: AgentMessage = from_slice(&bin_data).unwrap();
        let AgentMessage::PrepareAppRevision(request) = message else {
            panic!("expected PrepareAppRevision");
        };

        assert_eq!(request.logic_revision_id, "build-1");
        assert_eq!(request.artifact.kind, "web-cartridge");
        assert_eq!(
            request.artifact.location,
            "/__reloads__/build-1/pax-cartridge"
        );

        connection.close().await.unwrap();
    }

    #[actix_web::test]
    async fn stale_activation_is_rejected_without_closing_the_socket() {
        let state = Data::new(
            AppState::new(
                PathBuf::new(),
                PathBuf::new(),
                basic_manifest("Example"),
                None,
                None,
                HotReloadMode::All,
                None,
                None,
            )
            .unwrap(),
        );
        let server_state = state.clone();
        let srv = actix_test::start(move || {
            App::new()
                .app_data(server_state.clone())
                .service(web_socket)
        });

        let client = awc::Client::new();
        let (_resp, mut connection) = client.ws(srv.url("/ws")).connect().await.unwrap();
        connection
            .send(awc::ws::Message::Binary(
                rmp_serde::to_vec(&AgentMessage::ActivateAppRevision(ActivateAppRevision {
                    logic_revision_id: "surviving-artifact".to_string(),
                }))
                .unwrap()
                .into(),
            ))
            .await
            .unwrap();

        let Some(Ok(awc::ws::Frame::Binary(bin_data))) = connection.next().await else {
            panic!("expected activation rejection");
        };
        let message: AgentMessage = from_slice(&bin_data).unwrap();
        assert!(matches!(
            message,
            AgentMessage::AppRevisionActivationRejected(rejection)
                if rejection.logic_revision_id == "surviving-artifact"
        ));

        connection
            .send(awc::ws::Message::Binary(
                rmp_serde::to_vec(&AgentMessage::LoadManifestRequest(LoadManifestRequest {
                    active_revision: Some(pax_designtime::messages::RevisionStamp {
                        logic_revision_id: "surviving-artifact".to_string(),
                        template_version: 6,
                    }),
                }))
                .unwrap()
                .into(),
            ))
            .await
            .unwrap();

        let Some(Ok(awc::ws::Frame::Binary(bin_data))) = connection.next().await else {
            panic!("socket closed instead of returning the reconciled manifest");
        };
        let message: AgentMessage = from_slice(&bin_data).unwrap();
        let AgentMessage::LoadManifestResponse(response) = message else {
            panic!("expected LoadManifestResponse");
        };
        assert_eq!(response.revision.logic_revision_id, "surviving-artifact");
        assert_eq!(response.revision.template_version, 6);

        connection.close().await.unwrap();
    }

    #[actix_web::test]
    async fn reconnect_owner_survives_old_cancel_and_cleanup_then_commits() {
        let state = Data::new(
            AppState::new(
                PathBuf::new(),
                PathBuf::new(),
                basic_manifest("A"),
                None,
                None,
                HotReloadMode::All,
                None,
                None,
            )
            .unwrap(),
        );
        state
            .revisions
            .lock()
            .unwrap()
            .stage_logic_candidate(
                basic_manifest("B"),
                PrepareAppRevision {
                    logic_revision_id: "candidate-b".to_string(),
                    execution_mode: DebugLogicExecutionMode::CompiledArtifact,
                    artifact: DebugArtifact {
                        kind: "web-cartridge".to_string(),
                        location: "/candidate-b/pax-cartridge".to_string(),
                    },
                },
                ActivationChannel::WebSocket,
            )
            .unwrap();

        let server_state = state.clone();
        let srv = actix_test::start(move || {
            App::new()
                .app_data(server_state.clone())
                .service(web_socket)
        });
        let client = awc::Client::new();
        let (_resp, mut connection_a) = client.ws(srv.url("/ws")).connect().await.unwrap();
        let (_resp, mut connection_b) = client.ws(srv.url("/ws")).connect().await.unwrap();
        let preflight = AgentMessage::RequestAppRevisionActivation(ActivateAppRevision {
            logic_revision_id: "candidate-b".to_string(),
        });

        connection_a
            .send(awc::ws::Message::Binary(
                rmp_serde::to_vec(&preflight).unwrap().into(),
            ))
            .await
            .unwrap();
        let Some(Ok(awc::ws::Frame::Binary(response))) = connection_a.next().await else {
            panic!("first owner did not receive preflight response");
        };
        assert!(matches!(
            from_slice::<AgentMessage>(&response).unwrap(),
            AgentMessage::AppRevisionActivationPrepared(_)
        ));

        connection_b
            .send(awc::ws::Message::Binary(
                rmp_serde::to_vec(&preflight).unwrap().into(),
            ))
            .await
            .unwrap();
        let Some(Ok(awc::ws::Frame::Binary(response))) = connection_b.next().await else {
            panic!("reconnect owner did not receive preflight response");
        };
        assert!(matches!(
            from_slice::<AgentMessage>(&response).unwrap(),
            AgentMessage::AppRevisionActivationPrepared(_)
        ));

        connection_a
            .send(awc::ws::Message::Binary(
                rmp_serde::to_vec(&AgentMessage::CancelAppRevisionActivation(
                    ActivateAppRevision {
                        logic_revision_id: "candidate-b".to_string(),
                    },
                ))
                .unwrap()
                .into(),
            ))
            .await
            .unwrap();
        connection_a.close().await.unwrap();
        actix_web::rt::time::sleep(std::time::Duration::from_millis(25)).await;

        connection_b
            .send(awc::ws::Message::Binary(
                rmp_serde::to_vec(&AgentMessage::ActivateAppRevision(ActivateAppRevision {
                    logic_revision_id: "candidate-b".to_string(),
                }))
                .unwrap()
                .into(),
            ))
            .await
            .unwrap();
        let Some(Ok(awc::ws::Frame::Binary(response))) = connection_b.next().await else {
            panic!("reconnect owner did not receive final commit response");
        };
        assert!(matches!(
            from_slice::<AgentMessage>(&response).unwrap(),
            AgentMessage::LoadManifestResponse(response)
                if response.revision.logic_revision_id == "candidate-b"
        ));
        assert!(state
            .revisions
            .lock()
            .unwrap()
            .is_active_logic_revision("candidate-b"));

        connection_b.close().await.unwrap();
    }

    #[actix_web::test]
    async fn superseded_socket_cannot_reclaim_ownership_during_close_grace_period() {
        let state = Data::new(
            AppState::new(
                PathBuf::new(),
                PathBuf::new(),
                basic_manifest("Example"),
                None,
                None,
                HotReloadMode::All,
                None,
                None,
            )
            .unwrap(),
        );
        let server_state = state.clone();
        let srv = actix_test::start(move || {
            App::new()
                .app_data(server_state.clone())
                .service(web_socket)
        });

        let client = awc::Client::new();
        let (_resp, mut connection_a) = client.ws(srv.url("/ws")).connect().await.unwrap();
        connection_a
            .send(awc::ws::Message::Binary(
                rmp_serde::to_vec(&AgentMessage::LoadManifestRequest(LoadManifestRequest {
                    active_revision: None,
                }))
                .unwrap()
                .into(),
            ))
            .await
            .unwrap();
        let Some(Ok(awc::ws::Frame::Binary(bin_data))) = connection_a.next().await else {
            panic!("expected the first socket's manifest");
        };
        let AgentMessage::LoadManifestResponse(response) =
            from_slice::<AgentMessage>(&bin_data).unwrap()
        else {
            panic!("expected LoadManifestResponse");
        };
        let active_revision = response.revision;

        let (_resp, mut connection_b) = client.ws(srv.url("/ws")).connect().await.unwrap();
        connection_b
            .send(awc::ws::Message::Binary(
                rmp_serde::to_vec(&AgentMessage::LoadManifestRequest(LoadManifestRequest {
                    active_revision: Some(active_revision.clone()),
                }))
                .unwrap()
                .into(),
            ))
            .await
            .unwrap();
        let Some(Ok(awc::ws::Frame::Binary(bin_data))) = connection_b.next().await else {
            panic!("expected the replacement socket's manifest");
        };
        assert!(matches!(
            from_slice::<AgentMessage>(&bin_data).unwrap(),
            AgentMessage::LoadManifestResponse(_)
        ));

        let Some(Ok(awc::ws::Frame::Binary(bin_data))) = connection_a.next().await else {
            panic!("expected the first socket's supersession notification");
        };
        assert!(matches!(
            from_slice::<AgentMessage>(&bin_data).unwrap(),
            AgentMessage::DisconnectNotification(_)
        ));

        // The server leaves a short grace period after notifying A so the
        // notification can reach the client. A late same-revision hello in
        // that interval must not promote A and enqueue a reciprocal close for B.
        connection_a
            .send(awc::ws::Message::Binary(
                rmp_serde::to_vec(&AgentMessage::LoadManifestRequest(LoadManifestRequest {
                    active_revision: Some(active_revision.clone()),
                }))
                .unwrap()
                .into(),
            ))
            .await
            .unwrap();
        actix_web::rt::time::sleep(std::time::Duration::from_millis(75)).await;

        connection_b
            .send(awc::ws::Message::Binary(
                rmp_serde::to_vec(&AgentMessage::LoadManifestRequest(LoadManifestRequest {
                    active_revision: Some(active_revision),
                }))
                .unwrap()
                .into(),
            ))
            .await
            .expect("replacement socket should remain connected");
        let Some(Ok(awc::ws::Frame::Binary(bin_data))) = connection_b.next().await else {
            panic!("replacement socket was closed by a stale reciprocal supersession");
        };
        assert!(matches!(
            from_slice::<AgentMessage>(&bin_data).unwrap(),
            AgentMessage::LoadManifestResponse(_)
        ));

        connection_b.close().await.unwrap();
    }

    #[actix_web::test]
    async fn promotion_barrier_orders_admitted_mutation_and_rejects_stale_socket_write() {
        let dir = tempdir().unwrap();
        let rust_path = dir.path().join("src/stale.rs");
        fs::create_dir_all(rust_path.parent().unwrap()).unwrap();
        fs::write(&rust_path, "original").unwrap();
        let state = Data::new(
            AppState::new(
                PathBuf::new(),
                dir.path().to_path_buf(),
                basic_manifest("Example"),
                None,
                None,
                HotReloadMode::All,
                None,
                None,
            )
            .unwrap(),
        );
        let server_state = state.clone();
        let srv = actix_test::start(move || {
            App::new()
                .app_data(server_state.clone())
                .service(web_socket)
        });
        let client = awc::Client::new();
        let (_resp, mut connection_a) = client.ws(srv.url("/ws")).connect().await.unwrap();
        connection_a
            .send(awc::ws::Message::Binary(
                rmp_serde::to_vec(&AgentMessage::LoadManifestRequest(LoadManifestRequest {
                    active_revision: None,
                }))
                .unwrap()
                .into(),
            ))
            .await
            .unwrap();
        let Some(Ok(awc::ws::Frame::Binary(_))) = connection_a.next().await else {
            panic!("first socket did not receive its manifest");
        };
        let connection_a_id = state
            .active_websocket_client
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .connection_id;

        let (_resp, mut connection_b) = client.ws(srv.url("/ws")).connect().await.unwrap();
        let active_revision = state.revisions.lock().unwrap().active_stamp().unwrap();
        let source_barrier = state.source_update_lock.lock().unwrap();
        connection_b
            .send(awc::ws::Message::Binary(
                rmp_serde::to_vec(&AgentMessage::LoadManifestRequest(LoadManifestRequest {
                    active_revision: Some(active_revision),
                }))
                .unwrap()
                .into(),
            ))
            .await
            .unwrap();
        actix_web::rt::time::sleep(std::time::Duration::from_millis(25)).await;
        assert_eq!(
            state
                .active_websocket_client
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .connection_id,
            connection_a_id,
            "promotion must wait for the admitted mutation lane"
        );
        {
            let mut revisions = state.revisions.lock().unwrap();
            let manifest = revisions.active_manifest_clone().unwrap();
            revisions
                .replace_active_authoring_manifest(&manifest)
                .unwrap();
        }
        drop(source_barrier);
        let Some(Ok(awc::ws::Frame::Binary(_))) = connection_b.next().await else {
            panic!("replacement socket did not finish promotion");
        };
        let connection_b_id = state
            .active_websocket_client
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .connection_id;
        assert_ne!(connection_b_id, connection_a_id);
        assert_eq!(
            state
                .revisions
                .lock()
                .unwrap()
                .active_stamp()
                .unwrap()
                .template_version,
            1
        );

        let stamp_before_stale_attempt = state.revisions.lock().unwrap().active_stamp().unwrap();
        let generation_before_stale_attempt = state.revisions.lock().unwrap().mutation_generation();
        let (passed_initial_check_tx, passed_initial_check_rx) = std::sync::mpsc::channel();
        let (continue_stale_attempt_tx, continue_stale_attempt_rx) = std::sync::mpsc::channel();
        let stale_state = state.clone();
        let stale_path = rust_path.clone();
        let stale_attempt = std::thread::spawn(move || {
            let source_update =
                stale_state.source_update_guard_for_client_after(connection_b_id, || {
                    passed_initial_check_tx.send(()).unwrap();
                    continue_stale_attempt_rx.recv().unwrap();
                });
            let Some(_source_update) = source_update else {
                return false;
            };

            fs::write(&stale_path, "stale").unwrap();
            let mut revisions = stale_state.revisions.lock().unwrap();
            let manifest = revisions.active_manifest_clone().unwrap();
            revisions
                .replace_active_authoring_manifest(&manifest)
                .unwrap();
            revisions.record_pax_source("src/stale.rs".to_string(), "stale".to_string());
            true
        });
        passed_initial_check_rx
            .recv_timeout(std::time::Duration::from_secs(1))
            .expect("stale socket did not pass its initial ownership check");

        let (_resp, mut connection_c) = client.ws(srv.url("/ws")).connect().await.unwrap();
        let active_revision = state.revisions.lock().unwrap().active_stamp().unwrap();
        connection_c
            .send(awc::ws::Message::Binary(
                rmp_serde::to_vec(&AgentMessage::LoadManifestRequest(LoadManifestRequest {
                    active_revision: Some(active_revision),
                }))
                .unwrap()
                .into(),
            ))
            .await
            .unwrap();
        let Some(Ok(awc::ws::Frame::Binary(_))) = connection_c.next().await else {
            panic!("third socket did not finish promotion");
        };
        let connection_c_id = state
            .active_websocket_client
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .connection_id;
        assert_ne!(connection_c_id, connection_b_id);

        continue_stale_attempt_tx.send(()).unwrap();
        assert!(
            !stale_attempt.join().unwrap(),
            "socket superseded between ownership checks mutated source state"
        );
        assert_eq!(fs::read_to_string(&rust_path).unwrap(), "original");
        let revisions = state.revisions.lock().unwrap();
        assert_eq!(
            revisions.active_stamp().unwrap(),
            stamp_before_stale_attempt
        );
        assert_eq!(
            revisions.mutation_generation(),
            generation_before_stale_attempt
        );
        drop(revisions);

        connection_c.close().await.unwrap();
    }

    fn basic_manifest(component_name: &str) -> PaxManifest {
        let type_id = TypeId::build_singleton(component_name, Some(component_name));
        let mut components = BTreeMap::new();
        components.insert(
            type_id.clone(),
            ComponentDefinition {
                type_id: type_id.clone(),
                is_main_component: true,
                is_primitive: false,
                is_struct_only_component: false,
                module_path: component_name.to_string(),
                primitive_instance_import_path: None,
                template: None,
                settings: None,
                timelines: vec![],
                route_branch: None,
            },
        );
        PaxManifest {
            components,
            main_component_type_id: type_id,
            type_table: HashMap::new(),
            assets_dirs: vec![],
            engine_import_path: String::new(),
        }
    }

    #[test]
    fn source_paths_match_relative_manifest_to_absolute_request() {
        let temp_dir = tempdir().unwrap();
        let project_root = temp_dir.path();
        let source_path = project_root.join("src/lib.pax");
        fs::create_dir_all(source_path.parent().unwrap()).unwrap();
        fs::write(&source_path, "<Group/>").unwrap();

        assert!(source_paths_match(
            "src/lib.pax",
            &source_path.to_string_lossy(),
            project_root,
        ));
    }

    #[test]
    fn source_paths_match_absolute_manifest_to_relative_request() {
        let temp_dir = tempdir().unwrap();
        let project_root = temp_dir.path();
        let source_path = project_root.join("src/lib.pax");
        fs::create_dir_all(source_path.parent().unwrap()).unwrap();
        fs::write(&source_path, "<Group/>").unwrap();

        assert!(source_paths_match(
            &source_path.to_string_lossy(),
            "src/lib.pax",
            project_root,
        ));
    }

    #[test]
    fn replay_journal_uses_one_key_for_relative_and_absolute_source_paths() {
        let temp_dir = tempdir().unwrap();
        let project_root = temp_dir.path();
        let source_path = project_root.join("src/lib.pax");
        fs::create_dir_all(source_path.parent().unwrap()).unwrap();
        fs::write(&source_path, "<Group/>").unwrap();

        assert_eq!(
            normalized_pax_mutation_path("src/lib.pax", project_root),
            normalized_pax_mutation_path(&source_path.to_string_lossy(), project_root)
        );
    }

    #[test]
    fn replay_journal_normalizes_project_prefixed_relative_source_paths() {
        let current_dir = std::env::current_dir().unwrap();
        let temp_dir = tempfile::tempdir_in(&current_dir).unwrap();
        let project_root = temp_dir.path().join("examples/src/example");
        let source_path = project_root.join("src/lib.pax");
        fs::create_dir_all(source_path.parent().unwrap()).unwrap();
        fs::write(&source_path, "<Group/>").unwrap();
        let project_prefixed = source_path.strip_prefix(&current_dir).unwrap();

        assert_eq!(
            normalized_pax_mutation_path(&project_prefixed.to_string_lossy(), &project_root),
            normalized_pax_mutation_path(&source_path.to_string_lossy(), &project_root)
        );
    }

    #[test]
    fn source_paths_match_relative_project_prefixed_request_path() {
        let temp_dir = tempdir().unwrap();
        let cwd = temp_dir.path();
        let project_root = cwd.join("examples/src/dynamic-linking-lab");
        let source_path = project_root.join("src/lib.pax");
        fs::create_dir_all(source_path.parent().unwrap()).unwrap();
        fs::write(&source_path, "<Group/>").unwrap();

        let previous_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(cwd).unwrap();

        let matches = source_paths_match(
            "src/lib.pax",
            "examples/src/dynamic-linking-lab/src/lib.pax",
            Path::new("examples/src/dynamic-linking-lab"),
        );

        std::env::set_current_dir(previous_dir).unwrap();

        assert!(matches);
    }

    #[test]
    fn resolve_component_rust_source_path_finds_root_lib() {
        let temp_dir = tempdir().unwrap();
        let project_root = temp_dir.path();
        let rust_source = project_root.join("src/lib.rs");
        fs::create_dir_all(rust_source.parent().unwrap()).unwrap();
        fs::write(&rust_source, "pub struct Example;").unwrap();

        assert_eq!(
            resolve_component_rust_source_path(project_root, "crate").as_deref(),
            Some(rust_source.as_path())
        );
    }

    #[test]
    fn parse_pax_source_update_preserves_implicit_lifecycle_handlers() {
        let temp_dir = tempdir().unwrap();
        let project_root = temp_dir.path();
        let rust_source = project_root.join("src/lib.rs");
        let pax_source = project_root.join("src/lib.pax");
        fs::create_dir_all(pax_source.parent().unwrap()).unwrap();
        fs::write(
            &rust_source,
            r#"
            pub struct Example;

            impl Example {
                pub fn on_mount(&mut self, _ctx: &NodeContext) {}
                pub fn on_pre_render(&mut self, _ctx: &NodeContext) {}
            }
            "#,
        )
        .unwrap();
        fs::write(&pax_source, "<Group />").unwrap();

        let example_type_id = TypeId::build_singleton("crate::Example", Some("Example"));
        let group_type_id = TypeId::build_singleton("Group", Some("Group"));
        let mut components = BTreeMap::new();
        components.insert(
            example_type_id.clone(),
            ComponentDefinition {
                type_id: example_type_id.clone(),
                is_main_component: true,
                is_primitive: false,
                is_struct_only_component: false,
                module_path: "crate".to_string(),
                primitive_instance_import_path: None,
                template: Some(ComponentTemplate::new(
                    example_type_id.clone(),
                    Some("src/lib.pax".to_string()),
                )),
                settings: Some(vec![]),
                timelines: vec![],
                route_branch: None,
            },
        );
        components.insert(
            group_type_id.clone(),
            ComponentDefinition {
                type_id: group_type_id.clone(),
                is_main_component: false,
                is_primitive: true,
                is_struct_only_component: false,
                module_path: "crate".to_string(),
                primitive_instance_import_path: None,
                template: None,
                settings: None,
                timelines: vec![],
                route_branch: None,
            },
        );
        let manifest = PaxManifest {
            components,
            main_component_type_id: example_type_id.clone(),
            type_table: HashMap::new(),
            assets_dirs: vec![],
            engine_import_path: "pax_kit".to_string(),
        };

        let update = parse_pax_source_update(
            &manifest,
            &pax_source.to_string_lossy(),
            "<Group />",
            project_root,
        )
        .unwrap();

        let bindings = update
            .settings_block
            .iter()
            .filter_map(|setting| match setting {
                SettingsBlockElement::Handler(key, values) => values
                    .first()
                    .map(|value| (key.token_value.clone(), value.token_value.clone())),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(bindings.contains(&("mount".to_string(), "on_mount".to_string())));
        assert!(bindings.contains(&("pre_render".to_string(), "on_pre_render".to_string())));
    }
}

fn spawn_userland_rust_source_update(
    state: Data<AppState>,
    request_id: String,
    path: String,
) -> Result<(), String> {
    if !state.logic_hot_reload_available() {
        state.note_logic_restart_required();
        return Err("logic hot reload is unavailable for this session".to_string());
    }
    let config = {
        let mut logic_reload = state.logic_reload.lock().unwrap();
        let reload_state = logic_reload
            .as_mut()
            .ok_or_else(|| "logic reload is unavailable for this session".to_string())?;
        if reload_state.build_in_progress {
            return Err("a logic rebuild is already in progress".to_string());
        }
        reload_state.build_in_progress = true;
        reload_state.rebuild_pending = false;
        reload_state.config.clone()
    };

    std::thread::spawn(move || {
        let project_root = state.userland_project_root.lock().unwrap().clone();
        match super::perform_logic_reload(&state, &project_root, &config) {
            Ok(()) => {
                send_userland_source_update_response(
                    &state,
                    request_id.clone(),
                    path.clone(),
                    "ok",
                    None,
                );
            }
            Err(err) => {
                send_userland_source_update_response(
                    &state,
                    request_id.clone(),
                    path.clone(),
                    "error",
                    Some(format!("{err}")),
                );
            }
        }

        let should_repeat = {
            let mut logic_reload = state.logic_reload.lock().unwrap();
            let Some(reload_state) = logic_reload.as_mut() else {
                return;
            };
            if reload_state.rebuild_pending {
                reload_state.rebuild_pending = false;
                true
            } else {
                reload_state.build_in_progress = false;
                false
            }
        };

        if should_repeat {
            let state_for_repeat = state.clone();
            std::thread::spawn(move || super::run_logic_reload_loop(state_for_repeat));
        }
    });

    Ok(())
}

fn handle_dev_client_response(
    state: &Data<AppState>,
    response: DevClientResponse,
) -> std::io::Result<()> {
    let dev_session = state.dev_session.lock().unwrap().clone().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "missing web dev session")
    })?;

    match response {
        DevClientResponse::Look(response) => {
            let request_id = response.request_id.clone();
            let pending_request = state
                .pending_dev_look_requests
                .lock()
                .unwrap()
                .remove(&request_id);
            let pending_request = pending_request.ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("no pending web look request exists for {}", request_id),
                )
            })?;

            let captures = write_dev_look_capture_files(&pending_request, &response.captures)?;
            write_and_finalize_dev_response(
                state,
                &dev_session,
                &request_id,
                &DevLookResponse {
                    request_id: request_id.clone(),
                    status: response.status,
                    captures,
                    error: response.error,
                },
            )
        }
        DevClientResponse::InspectTree(response) => {
            let request_id = response.request_id.clone();
            write_and_finalize_dev_response(
                state,
                &dev_session,
                &request_id,
                &DevInspectTreeResponse {
                    request_id: request_id.clone(),
                    status: response.status,
                    node_count: response.node_count,
                    tree_json: response.tree_json,
                    error: response.error,
                },
            )
        }
        DevClientResponse::RayCast(response) => {
            let request_id = response.request_id.clone();
            write_and_finalize_dev_response(
                state,
                &dev_session,
                &request_id,
                &DevRayCastResponse {
                    request_id: request_id.clone(),
                    status: response.status,
                    x: response.x,
                    y: response.y,
                    hit_invisible: response.hit_invisible,
                    node_count: response.node_count,
                    nodes_json: response.nodes_json,
                    error: response.error,
                },
            )
        }
        DevClientResponse::SelectorQuery(response) => {
            let request_id = response.request_id.clone();
            write_and_finalize_dev_response(
                state,
                &dev_session,
                &request_id,
                &DevSelectorQueryResponse {
                    request_id: request_id.clone(),
                    status: response.status,
                    selector: response.selector,
                    node_count: response.node_count,
                    nodes_json: response.nodes_json,
                    error: response.error,
                },
            )
        }
        DevClientResponse::ReplaceNode(response) => {
            let request_id = response.request_id.clone();
            write_and_finalize_dev_response(
                state,
                &dev_session,
                &request_id,
                &DevReplaceNodeResponse {
                    request_id: request_id.clone(),
                    status: response.status,
                    component_type_id: response.component_type_id,
                    template_node_id: response.template_node_id,
                    reload_scope: response.reload_scope,
                    reloaded_template_node_id: response.reloaded_template_node_id,
                    source_path: response.source_path,
                    error: response.error,
                },
            )
        }
        DevClientResponse::Logs(response) => {
            let request_id = response.request_id.clone();
            write_and_finalize_dev_response(
                state,
                &dev_session,
                &request_id,
                &DevLogsResponse {
                    request_id: request_id.clone(),
                    status: response.status,
                    entries: response
                        .entries
                        .into_iter()
                        .map(|entry| crate::dev_session::DevLogEntry {
                            seq: entry.seq,
                            level: entry.level,
                            message: entry.message,
                            timestamp_ms: entry.timestamp_ms,
                        })
                        .collect(),
                    next_seq: response.next_seq,
                    oldest_seq: response.oldest_seq,
                    error: response.error,
                },
            )
        }
    }
}

fn finalize_in_flight_dev_request(
    state: &Data<AppState>,
    dev_session: &crate::dev_session::DevSession,
    request_id: &str,
) -> std::io::Result<()> {
    let request_path = session_request_dir(dev_session)
        .map_err(report_to_io)?
        .join(format!("{request_id}.json"));
    let _ = fs::remove_file(request_path);
    state
        .in_flight_dev_requests
        .lock()
        .unwrap()
        .remove(request_id);
    Ok(())
}

fn write_and_finalize_dev_response<T: serde::Serialize>(
    state: &Data<AppState>,
    dev_session: &crate::dev_session::DevSession,
    request_id: &str,
    response: &T,
) -> std::io::Result<()> {
    match write_dev_json_response(dev_session, request_id, response) {
        Ok(()) => finalize_in_flight_dev_request(state, dev_session, request_id),
        Err(err) => {
            state
                .in_flight_dev_requests
                .lock()
                .unwrap()
                .remove(request_id);
            Err(err)
        }
    }
}

fn write_dev_look_capture_files(
    request: &DevLookRequest,
    raw_captures: &[pax_designtime::messages::DevClientRawCapture],
) -> std::io::Result<Vec<DevCapture>> {
    fs::create_dir_all(&request.output_dir)?;
    let extension = if request.format.eq_ignore_ascii_case("jpeg") {
        "jpg"
    } else {
        "png"
    };

    let mut captures = Vec::with_capacity(raw_captures.len());
    for (capture_index, raw_capture) in raw_captures.iter().enumerate() {
        let capture_path = request
            .output_dir
            .join(format!("{capture_index:04}.{extension}"));
        let rgba_bytes = decode_dev_capture_rgba(raw_capture)?;
        write_encoded_capture_file(
            &capture_path,
            rgba_bytes.as_ref(),
            raw_capture.width,
            raw_capture.height,
            &request.format,
            request.quality,
        )?;
        captures.push(DevCapture {
            path: capture_path,
            width: raw_capture.width,
            height: raw_capture.height,
            captured_at_ms: raw_capture.captured_at_ms,
        });
    }

    Ok(captures)
}

fn decode_dev_capture_rgba(
    raw_capture: &pax_designtime::messages::DevClientRawCapture,
) -> std::io::Result<Cow<'_, [u8]>> {
    match raw_capture.compression.as_deref() {
        None => Ok(Cow::Borrowed(&raw_capture.rgba_bytes)),
        Some("zlib") => decompress_to_vec_zlib(&raw_capture.rgba_bytes)
            .map(Cow::Owned)
            .map_err(|err| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("failed to inflate zlib-compressed capture: {err}"),
                )
            }),
        Some(other) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unsupported dev capture compression: {other}"),
        )),
    }
}

fn write_encoded_capture_file(
    path: &Path,
    rgba_bytes: &[u8],
    width: usize,
    height: usize,
    format: &str,
    quality: Option<f64>,
) -> std::io::Result<()> {
    let width_u32 = u32::try_from(width).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "capture width overflowed u32",
        )
    })?;
    let height_u32 = u32::try_from(height).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "capture height overflowed u32",
        )
    })?;
    let file = fs::File::create(path)?;
    let mut writer = BufWriter::new(file);

    if format.eq_ignore_ascii_case("jpeg") {
        let rgba_image =
            ImageBuffer::<Rgba<u8>, _>::from_raw(width_u32, height_u32, rgba_bytes.to_vec())
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "invalid RGBA buffer length for JPEG capture",
                    )
                })?;
        let rgb_image = image::DynamicImage::ImageRgba8(rgba_image).into_rgb8();
        let jpeg_quality = (quality.unwrap_or(0.9).clamp(0.0, 1.0) * 100.0).round() as u8;
        let mut encoder = JpegEncoder::new_with_quality(&mut writer, jpeg_quality.max(1));
        encoder
            .encode(rgb_image.as_raw(), width_u32, height_u32, ColorType::Rgb8)
            .map_err(image_error_to_io)
    } else {
        let encoder = PngEncoder::new(&mut writer);
        encoder
            .write_image(rgba_bytes, width_u32, height_u32, ColorType::Rgba8)
            .map_err(image_error_to_io)
    }
}

fn write_dev_json_response<T: serde::Serialize>(
    dev_session: &crate::dev_session::DevSession,
    request_id: &str,
    response: &T,
) -> std::io::Result<()> {
    let response_dir = session_response_dir(dev_session).map_err(report_to_io)?;
    fs::create_dir_all(&response_dir)?;
    let response_path = response_dir.join(format!("{request_id}.json"));
    let response_bytes = serde_json::to_vec_pretty(response).map_err(json_error_to_io)?;
    fs::write(response_path, response_bytes)
}

fn write_dev_error_response(
    dev_session: &crate::dev_session::DevSession,
    request_id: &str,
    error: String,
) -> std::io::Result<()> {
    write_dev_json_response(
        dev_session,
        request_id,
        &serde_json::json!({
            "request_id": request_id,
            "status": "error",
            "error": error,
        }),
    )
}

fn image_error_to_io(err: image::ImageError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, err)
}

fn json_error_to_io(err: serde_json::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, err)
}

fn report_to_io(err: color_eyre::eyre::Report) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, err.to_string())
}

struct LLMRequestMessage {
    pub request: String,
    pub simple_world_info: String,
    pub file_content: String,
}

impl actix::Message for LLMRequestMessage {
    type Result = ();
}

#[allow(unused)]
fn build_llm_request(request: LLMRequestMessage) -> String {
    let mut req = format!("User Request:\n {}\n\n", request.request);
    req.push_str(&format!(
        "Simple World Information:\n {} \n\n",
        request.simple_world_info
    ));
    req.push_str(&format!(
        "Full Pax Template:\n {} \n\n",
        request.file_content
    ));
    req
}
