use crate::design_server::{ActiveWebsocketClient, AppState, FileContent, WatcherFileChanged};
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
use pax_designtime::messages::{
    AgentMessage, ComponentSerializationRequest, DevClientInspectTreeRequest, DevClientLogsRequest,
    DevClientLookRequest, DevClientRayCastRequest, DevClientReplaceNodeRequest, DevClientResponse,
    DevClientSelectorQueryRequest, FileChangedNotification, LoadFileToStaticDirRequest,
    LoadManifestResponse, ManifestSerializationRequest, UpdateTemplateRequest,
};
use pax_manifest::{ComponentDefinition, ComponentTemplate, PaxManifest, TypeId};
use std::{collections::HashMap, fs, io::BufWriter, path::Path, time::Duration};

pub mod socket_message_accumulator;

pub use socket_message_accumulator::SocketMessageAccumulator;

pub struct PrivilegedAgentWebSocket {
    state: Data<AppState>,
    socket_msg_accum: SocketMessageAccumulator,
    connection_id: Option<usize>,
}

struct DisconnectSuperseded;

impl actix::Message for DisconnectSuperseded {
    type Result = ();
}

impl PrivilegedAgentWebSocket {
    pub fn new(state: Data<AppState>) -> Self {
        Self {
            state,
            socket_msg_accum: SocketMessageAccumulator::new(),
            connection_id: None,
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
                    let _ = fs::remove_file(&path);
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
        ctx.close(Some(ws::CloseReason {
            code: ws::CloseCode::Normal,
            description: Some("Superseded by a newer Pax dev browser client".to_string()),
        }));
        ctx.stop();
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
        println!("File changed: {:?}", msg.path);
        if self.is_active_client() {
            if let FileContent::Pax(content) = msg.contents {
                if let Some(manifest) = self.state.manifest.lock().unwrap().as_mut() {
                    let mut template_map: HashMap<String, TypeId> = HashMap::new();
                    let mut matched_component: Option<TypeId> = None;
                    let mut original_template: Option<ComponentTemplate> = None;

                    // Search for component that was changed, while building a template map for the parse context
                    for (type_id, component) in manifest.components.iter() {
                        template_map
                            .insert(type_id.get_pascal_identifier().unwrap(), type_id.clone());
                        if let Some(template) = &component.template {
                            if let Some(file_path) = template.get_file_path() {
                                if file_path == msg.path {
                                    matched_component = Some(type_id.clone());
                                    original_template = Some(template.clone());
                                }
                            }
                        }
                    }

                    if let Some(self_type_id) = matched_component {
                        let original_template = original_template.unwrap();
                        let mut tpc = TemplateNodeParseContext {
                            pascal_identifier_to_type_id_map: template_map,
                            template: ComponentTemplate::new(
                                self_type_id.clone(),
                                original_template.get_file_path(),
                            ),
                        };

                        let ast = pax_language::parse_pax_str(
                            pax_language::Rule::pax_component_definition,
                            &content,
                        )
                        .expect("Unsuccessful parse");
                        let settings =
                            pax_manifest::parsing::parse_settings_from_component_definition_string(
                                ast.clone(),
                            );
                        pax_manifest::parsing::parse_template_from_component_definition_string(
                            &mut tpc,
                            &content,
                            ast.clone(),
                        );

                        let new_template = tpc.template;

                        // update the manifest with this new template
                        let comp = manifest.components.get_mut(&self_type_id).unwrap();
                        comp.template = Some(new_template.clone());
                        let msg =
                            AgentMessage::UpdateTemplateRequest(Box::new(UpdateTemplateRequest {
                                type_id: self_type_id,
                                new_template,
                                settings_block: settings,
                            }));
                        let serialized_msg = rmp_serde::to_vec(&msg).unwrap();
                        ctx.binary(serialized_msg);
                    }
                }
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
            write_dev_json_response(
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
            write_dev_json_response(
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
            write_dev_json_response(
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
            write_dev_json_response(
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
            write_dev_json_response(
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
            write_dev_json_response(
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
        write_encoded_capture_file(
            &capture_path,
            &raw_capture.rgba_bytes,
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
