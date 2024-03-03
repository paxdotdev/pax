use actix::{Actor, AsyncContext, Handler, Running, StreamHandler};
use actix_web::web::Data;
use actix_web_actors::ws;
use pax_compiler::parsing::TemplateNodeParseContext;
use pax_designtime::messages::{
    AgentMessage, ComponentSerializationRequest, FileChangedNotification,
    ManifestSerializationAcknowledgement, ManifestSerializationCompletedNotification,
    ManifestSerializationRequest, RecompilationAcknowledgement, RecompilationRequest,
    UpdateTemplateRequest,
};
use pax_manifest::{ComponentDefinition, ComponentTemplate, PaxManifest, TypeId};
use std::collections::HashMap;

use crate::{
    code_serialization::serialize_component_to_file, AppState, FileContent, WatcherFileChanged,
};

pub struct PrivilegedAgentWebSocket {
    state: Data<AppState>,
}

impl PrivilegedAgentWebSocket {
    pub fn new(state: Data<AppState>) -> Self {
        Self { state }
    }
}

impl Actor for PrivilegedAgentWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let mut active_client = self.state.active_client.lock().unwrap();
        *active_client = Some(ctx.address());
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        let mut active_client = self.state.active_client.lock().unwrap();
        *active_client = None;
        Running::Stop
    }
}

impl Handler<WatcherFileChanged> for PrivilegedAgentWebSocket {
    type Result = ();

    fn handle(&mut self, msg: WatcherFileChanged, ctx: &mut Self::Context) -> Self::Result {
        if let Some(_) = &*self.state.active_client.lock().unwrap() {
            if let FileContent::Pax(content) = msg.contents {
                if let Some(manifest) = &self.state.manifest {
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

                        pax_compiler::parsing::parse_template_from_component_definition_string(
                            &mut tpc, &content,
                        );

                        let mut new_template = tpc.template;
                        new_template.populate_template_with_known_entities(&original_template);

                        let msg = AgentMessage::UpdateTemplateRequest(UpdateTemplateRequest {
                            type_id: self_type_id,
                            new_template,
                        });
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
        if let Ok(ws::Message::Binary(bin_data)) = msg {
            match rmp_serde::from_slice::<AgentMessage>(&bin_data) {
                Ok(AgentMessage::ComponentSerializationRequest(request)) => {
                    handle_component_serialization_request(request);
                    self.state.update_last_written_timestamp();
                }
                Ok(AgentMessage::ManifestSerializationRequest(request)) => {
                    handle_manifest_serialization_request(
                        request,
                        self.state.generate_request_id(),
                        ctx,
                    );
                    self.state.update_last_written_timestamp();
                }
                Ok(AgentMessage::RecompilationRequest(request)) => {
                    handle_recompilation_request(*request, self.state.generate_request_id(), ctx);
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Deserialization error: {:?}", e);
                }
            }
        }
    }
}

fn handle_component_serialization_request(request: ComponentSerializationRequest) {
    let component: ComponentDefinition = rmp_serde::from_slice(&request.component_bytes).unwrap();
    let file_path = component
        .template
        .as_ref()
        .unwrap()
        .get_file_path()
        .unwrap()
        .to_owned();
    serialize_component_to_file(&component, file_path);
}

fn handle_manifest_serialization_request(
    request: ManifestSerializationRequest,
    id: usize,
    ctx: &mut ws::WebsocketContext<PrivilegedAgentWebSocket>,
) {
    let response =
        AgentMessage::ManifestSerializationAcknowledgement(ManifestSerializationAcknowledgement {
            id,
        });
    let serialized_response = rmp_serde::to_vec(&response).unwrap();
    ctx.binary(serialized_response);

    let manifest: PaxManifest = rmp_serde::from_slice(&request.manifest).unwrap();

    for (_, component) in manifest.components {
        let file_path = component.template.as_ref().unwrap().get_file_path();
        if let Some(file_path) = &file_path {
            serialize_component_to_file(&component, file_path.clone());
        }
    }
    let completion_notification = AgentMessage::ManifestSerializationCompletedNotification(
        ManifestSerializationCompletedNotification { id },
    );
    let serialized_completion_notification = rmp_serde::to_vec(&completion_notification).unwrap();
    ctx.binary(serialized_completion_notification);
}

fn handle_recompilation_request(
    _request: RecompilationRequest,
    id: usize,
    ctx: &mut ws::WebsocketContext<PrivilegedAgentWebSocket>,
) {
    let response = RecompilationAcknowledgement { id };
    let serialized_response = rmp_serde::to_vec(&response).unwrap();
    ctx.binary(serialized_response);

    // process recompilation request
}
