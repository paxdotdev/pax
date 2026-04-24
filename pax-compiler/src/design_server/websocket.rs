use crate::design_server::{
    schedule_native_logic_reload, ActiveWebsocketClient, AppState, FileContent, WatcherFileChanged,
};
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
    AgentMessage, ComponentSerializationRequest, DevClientInspectTreeRequest, DevClientLogsRequest,
    DevClientLookRequest, DevClientRayCastRequest, DevClientReplaceNodeRequest, DevClientResponse,
    DevClientSelectorQueryRequest, DisconnectNotification, FileChangedNotification,
    LoadFileToStaticDirRequest, LoadManifestResponse, ManifestSerializationRequest,
    UpdateTemplateRequest, UserlandSourceUpdateRequest, UserlandSourceUpdateResponse,
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
}

struct DisconnectSuperseded;

impl actix::Message for DisconnectSuperseded {
    type Result = ();
}

struct SendAgentMessage {
    message: AgentMessage,
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
        }
    }

    fn is_active_client(&self) -> bool {
        let Some(connection_id) = self.connection_id else {
            return false;
        };

        self.state
            .active_websocket_client
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|active_client| active_client.connection_id == connection_id)
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

impl Actor for PrivilegedAgentWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let connection_id = self.state.generate_websocket_client_id();
        self.connection_id = Some(connection_id);

        let mut active_client = self.state.active_websocket_client.lock().unwrap();
        let previous_client = active_client.replace(ActiveWebsocketClient {
            connection_id,
            addr: ctx.address(),
        });
        drop(active_client);

        if let Some(previous_client) = previous_client {
            previous_client.addr.do_send(DisconnectSuperseded);
        }

        self.refresh_dev_session_registration();
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

impl Handler<WatcherFileChanged> for PrivilegedAgentWebSocket {
    type Result = ();

    fn handle(&mut self, msg: WatcherFileChanged, ctx: &mut Self::Context) -> Self::Result {
        let WatcherFileChanged { contents, path } = msg;
        println!("File changed: {:?}", path);
        if self.is_active_client() {
            match contents {
                FileContent::Pax(content) => match apply_pax_source_update(&self.state, &path, &content)
                {
                    Ok(update_request) => {
                        let msg = AgentMessage::UpdateTemplateRequest(Box::new(update_request));
                        match rmp_serde::to_vec(&msg) {
                            Ok(serialized_msg) => ctx.binary(serialized_msg),
                            Err(err) => eprintln!(
                                "failed to serialize pax template update for watcher change: {err}"
                            ),
                        }
                    }
                    Err(err) => {
                        eprintln!("ignoring invalid Pax watcher update for {path}: {err}");
                    }
                },
                FileContent::Rust(_) => schedule_native_logic_reload(self.state.clone()),
                FileContent::Unknown => {}
            }
        }
        let serialized_notification = rmp_serde::to_vec(
            &AgentMessage::ProjectFileChangedNotification(FileChangedNotification {}),
        )
        .unwrap();
        ctx.binary(serialized_notification);
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
                Ok(AgentMessage::LoadManifestRequest) => {
                    let manifest =
                        rmp_serde::to_vec(&*self.state.manifest.lock().unwrap()).unwrap();

                    let message =
                        AgentMessage::LoadManifestResponse(LoadManifestResponse { manifest });
                    ctx.binary(rmp_serde::to_vec(&message).unwrap());
                }
                Ok(AgentMessage::ComponentSerializationRequest(request)) => {
                    handle_component_serialization_request(
                        request,
                        self.state.manifest.lock().unwrap().as_mut(),
                    );
                    self.state.update_last_written_timestamp();
                }
                Ok(AgentMessage::ManifestSerializationRequest(request)) => {
                    handle_manifest_serialization_request(
                        request,
                        &mut self.state.manifest.lock().unwrap(),
                        self.state.generate_request_id(),
                        ctx,
                    );
                    self.state.update_last_written_timestamp();
                }
                Ok(AgentMessage::LoadFileToStaticDirRequest(load_info)) => {
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
                    handle_userland_source_update_request(self.state.clone(), request, ctx);
                }
                Ok(AgentMessage::DevClientResponse(response)) => {
                    if let Err(err) = handle_dev_client_response(&self.state, response) {
                        eprintln!("failed to handle web dev response: {err}");
                    }
                }
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
    manifest: Option<&mut PaxManifest>,
) {
    let component: ComponentDefinition = rmp_serde::from_slice(&request.component_bytes).unwrap();
    let file_path = component
        .template
        .as_ref()
        .unwrap()
        .get_file_path()
        .unwrap()
        .to_owned();
    serialize_component_to_file(&component, file_path.clone());
    // update in memory manifest
    if let Some(manifest) = manifest {
        for comp in manifest.components.values_mut() {
            if comp
                .template
                .as_ref()
                .is_some_and(|t| t.get_file_path().is_some_and(|p| p == file_path))
            {
                *comp = component;
                break;
            }
        }
    }
}

fn handle_manifest_serialization_request(
    request: ManifestSerializationRequest,
    manifest: &mut Option<PaxManifest>,
    _id: usize,
    _ctx: &mut ws::WebsocketContext<PrivilegedAgentWebSocket>,
) {
    *manifest = Some(rmp_serde::from_slice(&request.manifest).unwrap());
    if let Some(manifest) = manifest {
        for component in manifest.components.values() {
            let file_path = component.template.as_ref().unwrap().get_file_path();
            if let Some(file_path) = &file_path {
                serialize_component_to_file(component, file_path.clone());
            }
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

    match resolved_path.extension().and_then(|extension| extension.to_str()) {
        Some("pax") => {
            let update_request = {
                let manifest = state.manifest.lock().unwrap();
                let Some(manifest) = manifest.as_ref() else {
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
                let project_root = state.userland_project_root.lock().unwrap().clone();
                match parse_pax_source_update(
                    manifest,
                    &resolved_path.to_string_lossy(),
                    &request.contents,
                    &project_root,
                ) {
                    Ok(update_request) => update_request,
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
                }
            };

            state.update_last_written_timestamp();
            if let Err(err) = fs::write(&resolved_path, &request.contents) {
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

            if let Err(err) = commit_pax_source_update(&state, &update_request) {
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

            if let Ok(serialized_update) = rmp_serde::to_vec(&AgentMessage::UpdateTemplateRequest(
                Box::new(update_request),
            )) {
                ctx.binary(serialized_update);
            } else {
                send_userland_source_update_response_in_context(
                    ctx,
                    UserlandSourceUpdateResponse {
                        request_id: request.request_id,
                        path: request.path,
                        status: "error".to_string(),
                        error: Some("failed to serialize template update".to_string()),
                    },
                );
                return;
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
            state.update_last_written_timestamp();
            if let Err(err) = fs::write(&resolved_path, &request.contents) {
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

            if let Err(err) = spawn_userland_rust_source_update(state, request_id.clone(), request_path.clone()) {
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
    let active_client = state.active_websocket_client.lock().unwrap().clone();
    if let Some(active_client) = active_client {
        active_client.addr.do_send(SendAgentMessage { message });
    }
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

    let resolved_path = state.userland_project_root.lock().unwrap().join(requested_path);
    match resolved_path.extension().and_then(|extension| extension.to_str()) {
        Some("rs") | Some("pax") => Ok(resolved_path),
        _ => Err("only .rs and .pax source updates are supported".to_string()),
    }
}

fn apply_pax_source_update(
    state: &Data<AppState>,
    source_path: &str,
    content: &str,
) -> Result<UpdateTemplateRequest, String> {
    let update_request = {
        let manifest = state.manifest.lock().unwrap();
        let manifest = manifest
            .as_ref()
            .ok_or_else(|| "design server manifest is unavailable".to_string())?;
        let project_root = state.userland_project_root.lock().unwrap().clone();
        parse_pax_source_update(manifest, source_path, content, &project_root)?
    };
    commit_pax_source_update(state, &update_request)?;
    Ok(update_request)
}

fn parse_pax_source_update(
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
            template: ComponentTemplate::new(self_type_id.clone(), original_template.get_file_path()),
        };

        let ast = pax_language::parse_pax_str(
            pax_language::Rule::pax_component_definition,
            content,
        )
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
            &mut tpc,
            content,
            ast,
        );

        Ok(UpdateTemplateRequest {
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

    match (manifest_absolute.canonicalize(), source_absolute.canonicalize()) {
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
        vec![src_dir.join("lib.rs"), src_dir.join("main.rs"), src_dir.join("mod.rs")]
    } else {
        let mut path = src_dir;
        for segment in &module_segments {
            path.push(segment);
        }
        vec![path.with_extension("rs"), path.join("mod.rs")]
    };

    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn commit_pax_source_update(
    state: &Data<AppState>,
    update_request: &UpdateTemplateRequest,
) -> Result<(), String> {
    let mut manifest = state.manifest.lock().unwrap();
    let manifest = manifest
        .as_mut()
        .ok_or_else(|| "design server manifest is unavailable".to_string())?;
    let component = manifest
        .components
        .get_mut(&update_request.type_id)
        .ok_or_else(|| format!("missing component {}", update_request.type_id))?;
    component.template = Some(update_request.new_template.clone());
    component.settings = Some(update_request.settings_block.clone());
    Ok(())
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
    use super::{parse_pax_source_update, resolve_component_rust_source_path, source_paths_match};
    use pax_manifest::{ComponentDefinition, ComponentTemplate, PaxManifest, SettingsBlockElement, TypeId};
    use std::{collections::{BTreeMap, HashMap}, fs};
    use std::path::Path;
    use tempfile::tempdir;

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
    let config = {
        let mut native_logic_reload = state.native_logic_reload.lock().unwrap();
        let reload_state = native_logic_reload
            .as_mut()
            .ok_or_else(|| "native logic reload is unavailable for this session".to_string())?;
        if reload_state.build_in_progress {
            return Err("a logic rebuild is already in progress".to_string());
        }
        reload_state.build_in_progress = true;
        reload_state.rebuild_pending = false;
        reload_state.config.clone()
    };

    std::thread::spawn(move || {
        let project_root = state.userland_project_root.lock().unwrap().clone();
        let build_result = crate::building::apple::rebuild_staged_macos_logic_dylib(
            &project_root,
            &config.session_dir,
            config.should_run_designer,
        );

        match build_result {
            Ok(build) => {
                let crate::building::apple::MacosLogicReloadBuild {
                    manifest,
                    dylib_path,
                } = build;
                *state.manifest.lock().unwrap() = Some(manifest);
                if let Err(err) =
                    super::enqueue_native_logic_reload_request(&state, &config, dylib_path.as_path())
                {
                    send_userland_source_update_response(
                        &state,
                        request_id.clone(),
                        path.clone(),
                        "error",
                        Some(format!("failed to queue logic reload: {err}")),
                    );
                } else {
                    send_userland_source_update_response(
                        &state,
                        request_id.clone(),
                        path.clone(),
                        "ok",
                        None,
                    );
                }
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
            let mut native_logic_reload = state.native_logic_reload.lock().unwrap();
            let Some(reload_state) = native_logic_reload.as_mut() else {
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
            std::thread::spawn(move || super::run_native_logic_reload_loop(state_for_repeat));
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
