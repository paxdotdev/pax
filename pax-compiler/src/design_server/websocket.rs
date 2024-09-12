use crate::design_server::{
    code_serialization::serialize_component_to_file, AppState, FileContent, WatcherFileChanged,
};

use pax_manifest::parsing::TemplateNodeParseContext;

use actix::{Actor, AsyncContext, Handler, Running, StreamHandler};
use actix_web::web::Data;
use actix_web_actors::ws::{self};
use pax_designtime::messages::{
    AgentMessage, ComponentSerializationRequest, FileChangedNotification,
    LoadFileToStaticDirRequest, LoadManifestResponse, ManifestSerializationRequest,
    UpdateTemplateRequest,
};
use pax_manifest::{ComponentDefinition, ComponentTemplate, PaxManifest, TypeId};
use std::collections::HashMap;

use self::socket_message_accumulator::SocketMessageAccumulator;

mod socket_message_accumulator;

pub struct PrivilegedAgentWebSocket {
    state: Data<AppState>,
    socket_msg_accum: SocketMessageAccumulator,
}

impl PrivilegedAgentWebSocket {
    pub fn new(state: Data<AppState>) -> Self {
        Self {
            state,
            socket_msg_accum: SocketMessageAccumulator::new(),
        }
    }
}

impl Actor for PrivilegedAgentWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let mut active_client = self.state.active_websocket_client.lock().unwrap();
        *active_client = Some(ctx.address());
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        let mut active_client = self.state.active_websocket_client.lock().unwrap();
        *active_client = None;
        Running::Stop
    }
}

impl Handler<WatcherFileChanged> for PrivilegedAgentWebSocket {
    type Result = ();

    fn handle(&mut self, msg: WatcherFileChanged, ctx: &mut Self::Context) -> Self::Result {
        println!("File changed: {:?}", msg.path);
        if self.state.active_websocket_client.lock().unwrap().is_some() {
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

                        let ast = pax_lang::parse_pax_str(
                            pax_lang::Rule::pax_component_definition,
                            &content,
                        )
                        .expect("Unsuccessful parse");
                        let _settings =
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
            eprintln!("failed to recieve on socket");
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
                        "recieved a file {} (size: {})! root dir to write to: {:?}",
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
                Ok(
                    AgentMessage::UpdateTemplateRequest(_)
                    | AgentMessage::ProjectFileChangedNotification(_)
                    | AgentMessage::LoadManifestResponse(_),
                ) => {}
                Err(e) => {
                    eprintln!("Deserialization error: {:?}", e);
                }
            }
        } else if let Ok(None) = processed_message {
            // Do nothing, wait until entire message has been recieved
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
