use actix::{spawn, Actor, AsyncContext, Handler, Running, StreamHandler};
use actix_web::web::Data;
use actix_web_actors::ws;
use pax_compiler::parsing::TemplateNodeParseContext;
use pax_designtime::{
    messages::{
        AgentMessage, ComponentSerializationRequest, FileChangedNotification, LLMHelpRequest,
        ManifestSerializationRequest, UpdateTemplateRequest,
    },
    orm::template::NodeAction,
};
use pax_manifest::{ComponentDefinition, ComponentTemplate, PaxManifest, TypeId};
use std::{collections::HashMap, fs, time::SystemTime};

use crate::{
    code_serialization::{press_code_serialization_template, serialize_component_to_file},
    llm::{
        constants::{
            TRAINING_DATA_AFTER_REQUEST, TRAINING_DATA_BEFORE_REQUEST, TRAINING_DATA_REQUEST,
        },
        query_open_ai,
        simple::{SimpleNodeAction, SimpleWorldInformation},
    },
    AppState, FileContent, LLMHelpResponseMessage, WatcherFileChanged,
};

use crate::llm::constants::TRAINING_DATA_PATH;

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
        let mut active_client = self.state.active_websocket_client.lock().unwrap();
        *active_client = Some(ctx.address());
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        let mut active_client = self.state.active_websocket_client.lock().unwrap();
        *active_client = None;
        Running::Stop
    }
}

impl Handler<LLMHelpResponseMessage> for PrivilegedAgentWebSocket {
    type Result = ();

    fn handle(&mut self, msg: LLMHelpResponseMessage, ctx: &mut Self::Context) -> Self::Result {
        let serialized_msg = rmp_serde::to_vec(&AgentMessage::LLMHelpResponse(msg.into())).unwrap();
        ctx.binary(serialized_msg);
    }
}

impl Handler<WatcherFileChanged> for PrivilegedAgentWebSocket {
    type Result = ();

    fn handle(&mut self, msg: WatcherFileChanged, ctx: &mut Self::Context) -> Self::Result {
        println!("File changed: {:?}", msg.path);
        if self.state.active_websocket_client.lock().unwrap().is_some() {
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

                        let settings = Some(
                            pax_compiler::parsing::parse_settings_from_component_definition_string(
                                &content,
                            ),
                        );

                        let mut new_template = tpc.template;

                        new_template.merge_with_settings(&settings);
                        new_template.populate_template_with_known_entities(&original_template);
                        println!("Rendering update!");
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
                Ok(AgentMessage::LLMHelpRequest(help_request)) => {
                    let request_id = format!(
                        "{}",
                        SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_millis()
                    );
                    //record_request_training_data(&help_request, &request_id);

                    let component_type_id = help_request.component.type_id.clone();
                    // let serialized_component =
                    //     press_code_serialization_template(help_request.component.clone());

                    let template = help_request.component.template.as_ref().unwrap();
                    let simple_template = template.clone().into();
                    let simple_world_info = SimpleWorldInformation {
                        template: simple_template,
                    };
                    let request = LLMRequestMessage {
                        request: help_request.request,
                        simple_world_info: serde_json::to_string(&simple_world_info).unwrap(),
                        file_content: String::new(),
                    };
                    let request = build_llm_request(request);
                    let state = self.state.clone();
                    spawn(async move {
                        match query_open_ai(&request).await {
                            Ok(response) => {
                                let mut node_actions: Vec<NodeAction> = vec![];
                                for simple_action in response {
                                    println!("LLM Action: {:?}", simple_action);
                                    node_actions.extend(SimpleNodeAction::build(
                                        component_type_id.clone(),
                                        simple_action,
                                    ));
                                }
                                state
                                    .active_websocket_client
                                    .lock()
                                    .unwrap()
                                    .as_ref()
                                    .unwrap()
                                    .do_send(LLMHelpResponseMessage {
                                        request_id,
                                        component: component_type_id,
                                        actions: node_actions,
                                    });
                            }
                            Err(e) => eprintln!("Error querying OpenAI API: {:?}", e),
                        }
                    });
                }
                Ok(AgentMessage::LLMUpdatedTemplateNotification(notification)) => {
                    // let folder_path = format!("{}{}", TRAINING_DATA_PATH, notification.request_id);
                    // let component = notification.component.clone();
                    // serialize_component_to_file(
                    //     &component,
                    //     format!("{}/{}", folder_path, TRAINING_DATA_AFTER_REQUEST),
                    // );
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
    _id: usize,
    _ctx: &mut ws::WebsocketContext<PrivilegedAgentWebSocket>,
) {
    let manifest: PaxManifest = rmp_serde::from_slice(&request.manifest).unwrap();

    for (_, component) in manifest.components {
        let file_path = component.template.as_ref().unwrap().get_file_path();
        if let Some(file_path) = &file_path {
            serialize_component_to_file(&component, file_path.clone());
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

fn record_request_training_data(help_request: &LLMHelpRequest, request_id: &str) {
    let folder_path = format!("{}{}", TRAINING_DATA_PATH, request_id);
    fs::create_dir_all(&folder_path).unwrap();
    // get a string for the date today using std
    serialize_component_to_file(
        &help_request.component.clone(),
        format!("{}/{}", &folder_path, TRAINING_DATA_BEFORE_REQUEST),
    );
    fs::write(
        format!("{}/{}", &folder_path, TRAINING_DATA_REQUEST),
        help_request.request.clone(),
    )
    .unwrap();
}
