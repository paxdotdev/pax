use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub mod orm;
pub mod privileged_agent;
mod revision;

pub mod messages;
pub mod serde_pax;

use messages::LLMRequest;
use messages::{
    AgentMessage, DevClientRequest, DevClientResponse, PrepareAppRevision,
    UserlandSourceUpdateRequest, UserlandSourceUpdateResponse,
};
use orm::{MessageType, ReloadType};
use pax_manifest::pax_runtime_api::Property;
use pax_message::ScreenshotData;
use privileged_agent::{ConnectionEvent, WebSocketConnection};
pub use revision::AppRevisionActivationStatus;
use revision::{ManifestAdmission, RevisionGate, TemplateAdmission};

use core::fmt::Debug;

pub use pax_manifest;
use pax_manifest::{
    server::*, ComponentDefinition, PaxManifest, TypeId, UniqueTemplateNodeIdentifier,
};
pub use serde_pax::error::{Error, Result};
pub use serde_pax::se::{to_pax, Serializer};

pub const INITIAL_MANIFEST_FILE_NAME: &str = "initial-manifest.json";

type Factories = HashMap<String, Box<fn(ComponentDefinition) -> Box<dyn Any>>>;
use crate::orm::PaxManifestORM;

pub struct DesigntimeManager {
    orm: PaxManifestORM,
    factories: Factories,
    privileged_agent_connection: Rc<RefCell<WebSocketConnection>>,
    pub_pax_connection: Option<Rc<RefCell<WebSocketConnection>>>,
    project_query: Option<String>,
    response_queue: Rc<RefCell<Vec<DesigntimeResponseMessage>>>,
    pending_dev_client_requests: Rc<RefCell<Vec<DevClientRequest>>>,
    pending_prepare_app_revisions: Rc<RefCell<Vec<PrepareAppRevision>>>,
    pending_userland_source_update_responses: Rc<RefCell<Vec<UserlandSourceUpdateResponse>>>,
    revision_gate: RevisionGate,
    last_rendered_manifest_version: Property<usize>,
    pub publish_state: Property<Option<PublishResponse>>,
    enqueued_llm_request: Option<LLMRequest>,
}

pub enum DesigntimeResponseMessage {
    LLMResponse(ComponentDefinition),
    PublishResponse(PublishResponse),
}

impl Debug for DesigntimeManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesigntimeManager").finish()
    }
}

impl Drop for DesigntimeManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

const VERSION_PREFIX: &str = "/v0";
const ENDPOINT_PUBLISH: &str = "/v0/publish";

const PROD_PUB_PAX_SERVER: &str = "https://pub.pax.dev";

fn get_server_base_url() -> String {
    // Fetch the environment variable or use the default value
    option_env!("PUB_PAX_SERVER")
        .unwrap_or(PROD_PUB_PAX_SERVER)
        .to_string()
}

impl DesigntimeManager {
    pub fn get_last_rendered_manifest_version(&self) -> Property<usize> {
        self.last_rendered_manifest_version.clone()
    }

    pub fn set_last_rendered_manifest_version(&self, version: usize) {
        self.last_rendered_manifest_version.set(version);
    }

    pub fn get_llm_messages(&mut self, request_id: u64) -> Vec<String> {
        self.orm.get_messages(request_id)
    }

    pub fn new_with_local_addr(manifest: PaxManifest, local_addr: &str) -> Self {
        let privileged_agent = WebSocketConnection::new(local_addr, None, "privileged-agent")
            .expect("couldn't connect to privileged agent");

        Self::new_with_connection(manifest, privileged_agent)
    }

    fn new_with_connection(manifest: PaxManifest, privileged_agent: WebSocketConnection) -> Self {
        let orm = PaxManifestORM::new(manifest);
        let factories = HashMap::new();
        DesigntimeManager {
            orm,
            factories,
            privileged_agent_connection: Rc::new(RefCell::new(privileged_agent)),
            pub_pax_connection: None,
            project_query: None,
            response_queue: Rc::new(RefCell::new(Vec::new())),
            pending_dev_client_requests: Rc::new(RefCell::new(Vec::new())),
            pending_prepare_app_revisions: Rc::new(RefCell::new(Vec::new())),
            pending_userland_source_update_responses: Rc::new(RefCell::new(Vec::new())),
            revision_gate: RevisionGate::default(),
            last_rendered_manifest_version: Property::new(0),
            publish_state: Default::default(),
            enqueued_llm_request: None,
        }
    }
    pub fn new(manifest: PaxManifest) -> Self {
        let local_addr = std::env::var("PAX_DESIGN_SERVER_ADDR")
            .ok()
            .or_else(resolve_default_local_addr);
        match local_addr {
            Some(local_addr) => Self::new_with_local_addr(manifest, &local_addr),
            None => Self::new_with_connection(
                manifest,
                WebSocketConnection::offline("privileged-agent"),
            ),
        }
    }

    pub fn set_project(&mut self, project_query: String) {
        self.project_query = Some(project_query);
    }

    pub fn send_file_to_static_dir(&self, name: &str, data: Vec<u8>) -> anyhow::Result<()> {
        self.privileged_agent_connection
            .borrow_mut()
            .send_file_to_static_dir(name, data)?;
        Ok(())
    }

    pub fn get_new_message_listener(&self) -> Property<Vec<MessageType>> {
        self.orm.new_message.clone()
    }

    pub fn get_manifest_loaded_from_server_prop(&self) -> Property<bool> {
        self.orm.manifest_loaded_from_server.clone()
    }

    pub fn send_component_update(&mut self, type_id: &TypeId) -> anyhow::Result<()> {
        self.orm.send_component_update(type_id);
        let component = self.orm.get_component(type_id)?;
        self.privileged_agent_connection
            .borrow_mut()
            .send_component_update(component)?;

        Ok(())
    }

    pub fn llm_request(&mut self, prompt: &str, request_id: u64) -> anyhow::Result<()> {
        let manifest = self.orm.get_manifest().clone();

        let llm_request = LLMRequest {
            manifest: manifest.clone(),
            prompt: prompt.to_string(),
            request_id,
            screenshot: None,
        };

        self.enqueued_llm_request = Some(llm_request);

        Ok(())
    }

    pub fn publish_project(&mut self) {
        let manifest = self.orm.get_manifest().clone();

        let publish_request = PublishRequest {
            manifest: manifest.clone(),
        };

        let queue_cloned = self.response_queue.clone();

        wasm_bindgen_futures::spawn_local(async move {
            let url = get_server_base_url() + ENDPOINT_PUBLISH;

            let response = reqwasm::http::Request::post(&url)
                .header("Content-Type", "application/json")
                .body(serde_json::to_string(&publish_request).unwrap())
                .send()
                .await;

            log::info!("response_text: {:?}", response);

            let pub_response = match response {
                Ok(resp) => {
                    let pub_response: PublishResponse = resp.json().await.unwrap();

                    let pub_response = if let PublishResponse::Success(prs) = pub_response {
                        prs
                    } else {
                        unimplemented!()
                    };
                    log::info!("publish success: {:?}", &pub_response.pull_request_url);
                    PublishResponse::Success(pub_response)
                }
                Err(msg) => PublishResponse::Error(ResponseError {
                    message: msg.to_string(),
                }),
            };

            queue_cloned
                .borrow_mut()
                .push(DesigntimeResponseMessage::PublishResponse(pub_response));
        });
    }

    pub fn add_factory(
        &mut self,
        type_id: String,
        factory: Box<fn(ComponentDefinition) -> Box<dyn Any>>,
    ) {
        self.factories.insert(type_id, factory);
    }

    pub fn get_manifest(&self) -> &PaxManifest {
        self.orm.get_manifest()
    }

    pub fn take_reload_queue(&mut self) -> HashSet<ReloadType> {
        self.orm.take_reload_queue()
    }

    pub fn reload(&mut self) {
        self.orm.insert_reload(ReloadType::Tree);
        self.orm.increment_manifest_version();
    }

    pub fn set_userland_root_component_type_id(&mut self, type_id: &TypeId) {
        self.orm.set_userland_root_component_type_id(type_id);
        self.orm.increment_manifest_version();
        self.orm.insert_reload(ReloadType::Tree);
    }

    pub fn get_last_written_manifest_version(&self) -> Property<usize> {
        self.orm.get_manifest_version()
    }

    pub fn take_dev_client_requests(&mut self) -> Vec<DevClientRequest> {
        let mut pending_requests = self.pending_dev_client_requests.borrow_mut();
        pending_requests.drain(..).collect()
    }

    pub fn send_dev_client_response(&mut self, response: DevClientResponse) -> anyhow::Result<()> {
        self.privileged_agent_connection
            .borrow_mut()
            .send_dev_client_response(response)
    }

    pub fn take_prepare_app_revisions(&mut self) -> Vec<PrepareAppRevision> {
        let mut pending_requests = self.pending_prepare_app_revisions.borrow_mut();
        pending_requests.drain(..).collect()
    }

    /// Gives a freshly initialized host-managed artifact the logic identity
    /// from its out-of-band prepare request. Native hosts call this immediately
    /// before activation because native prepare envelopes do not traverse the
    /// artifact's websocket connection.
    pub fn prime_app_revision(&mut self, logic_revision_id: &str) -> bool {
        self.revision_gate.prime(logic_revision_id)
    }

    /// Begins the server-validation phase without changing the locally active
    /// revision. Hosts keep their last-known-good runtime mounted while polling
    /// `app_revision_activation_status` on the candidate.
    pub fn request_app_revision_activation(
        &mut self,
        logic_revision_id: &str,
    ) -> anyhow::Result<bool> {
        let Some(request) = self.revision_gate.request_activation(logic_revision_id) else {
            return Ok(false);
        };
        self.send_revision_messages(vec![AgentMessage::RequestAppRevisionActivation(request)]);
        Ok(true)
    }

    pub fn app_revision_activation_status(
        &self,
        logic_revision_id: &str,
    ) -> AppRevisionActivationStatus {
        self.revision_gate.activation_status(logic_revision_id)
    }

    pub fn cancel_app_revision_activation(&mut self, logic_revision_id: &str) -> bool {
        if !self.revision_gate.cancel_activation(logic_revision_id) {
            return false;
        }
        let _ = self
            .privileged_agent_connection
            .borrow_mut()
            .send_agent_message(&AgentMessage::CancelAppRevisionActivation(
                messages::ActivateAppRevision {
                    logic_revision_id: logic_revision_id.to_string(),
                },
            ));
        true
    }

    /// Confirms that the host installed a prepared executable or interpreted
    /// logic revision. The acknowledgement remains queued across a disconnect
    /// until the server returns the matching manifest snapshot.
    pub fn activate_app_revision(&mut self, logic_revision_id: &str) -> anyhow::Result<bool> {
        let Some(activation) = self.revision_gate.activate(logic_revision_id) else {
            return Ok(false);
        };

        self.send_revision_messages(vec![AgentMessage::ActivateAppRevision(activation)]);
        Ok(true)
    }

    pub fn send_userland_source_update(
        &mut self,
        request: UserlandSourceUpdateRequest,
    ) -> anyhow::Result<()> {
        self.privileged_agent_connection
            .borrow_mut()
            .send_userland_source_update_request(request)
    }

    pub fn take_userland_source_update_responses(&mut self) -> Vec<UserlandSourceUpdateResponse> {
        let mut responses = self.pending_userland_source_update_responses.borrow_mut();
        responses.drain(..).collect()
    }

    pub fn get_orm(&self) -> &PaxManifestORM {
        &self.orm
    }

    pub fn get_orm_mut(&mut self) -> &mut PaxManifestORM {
        &mut self.orm
    }

    pub fn handle_recv(
        &mut self,
        screenshot_map: Rc<RefCell<HashMap<u32, ScreenshotData>>>,
    ) -> anyhow::Result<()> {
        if let Some(mut llm_request) = self.enqueued_llm_request.take() {
            let mut screenshot_map = screenshot_map.borrow_mut();
            if let Some(screenshot) = screenshot_map.remove(&(llm_request.request_id as u32)) {
                llm_request.screenshot = Some(screenshot);
                match self.pub_pax_connection() {
                    Ok(connection) => {
                        if let Err(err) = connection.borrow_mut().send_llm_request(llm_request) {
                            log::warn!("failed to send LLM request over pub-pax websocket: {err}");
                        }
                    }
                    Err(err) => {
                        log::warn!("failed to connect pub-pax websocket for LLM request: {err}");
                    }
                }
            } else {
                self.enqueued_llm_request = Some(llm_request);
            }
        }

        let privileged_agent_events =
            match self.privileged_agent_connection.borrow_mut().handle_recv() {
                Ok(events) => events,
                Err(err) => {
                    log::warn!("privileged-agent receive failed: {err:?}");
                    Vec::new()
                }
            };
        let mut revision_messages = Vec::new();
        for event in privileged_agent_events {
            match event {
                ConnectionEvent::Opened => {
                    revision_messages.extend(self.revision_gate.connection_messages());
                }
                ConnectionEvent::Disconnected => {
                    self.revision_gate.disconnected();
                    self.pending_prepare_app_revisions.borrow_mut().clear();
                }
                ConnectionEvent::Message(message) => {
                    self.handle_privileged_agent_message(message, &mut revision_messages)
                }
            }
        }
        self.send_revision_messages(revision_messages);

        if let Some(pub_pax_connection) = self.pub_pax_connection.clone() {
            match pub_pax_connection.borrow_mut().handle_recv() {
                Ok(events) => {
                    for event in events {
                        if let ConnectionEvent::Message(message) = event {
                            self.handle_pub_pax_message(message);
                        }
                    }
                }
                Err(err) => {
                    log::warn!("pub-pax receive failed: {err:?}");
                }
            }
        }

        let response_queue = {
            let mut queue = self.response_queue.borrow_mut();
            queue.drain(..).collect::<Vec<DesigntimeResponseMessage>>()
        };
        for response in response_queue {
            self.handle_response(response);
        }
        Ok(())
    }

    fn handle_privileged_agent_message(
        &mut self,
        message: AgentMessage,
        revision_messages: &mut Vec<AgentMessage>,
    ) {
        match message {
            AgentMessage::AppRevisionActivationPrepared(response) => {
                let logic_revision_id = response.revision.logic_revision_id.clone();
                if !self
                    .revision_gate
                    .admit_prepared_activation_manifest(&response.revision)
                {
                    log::debug!(
                        "ignoring unrequested activation snapshot for {}",
                        logic_revision_id
                    );
                    return;
                }
                let manifest = match rmp_serde::from_slice::<PaxManifest>(&response.manifest) {
                    Ok(manifest) => manifest,
                    Err(err) => {
                        if self.revision_gate.reject_activation(&logic_revision_id) {
                            revision_messages.push(AgentMessage::CancelAppRevisionActivation(
                                messages::ActivateAppRevision {
                                    logic_revision_id: logic_revision_id.clone(),
                                },
                            ));
                        }
                        log::warn!("received invalid activation manifest payload: {err}");
                        return;
                    }
                };
                match self.orm.set_initial_server_manifest(manifest) {
                    Ok(()) => self
                        .revision_gate
                        .commit_prepared_activation_manifest(response.revision),
                    Err(err) => {
                        if self.revision_gate.reject_activation(&logic_revision_id) {
                            revision_messages.push(AgentMessage::CancelAppRevisionActivation(
                                messages::ActivateAppRevision {
                                    logic_revision_id: logic_revision_id.clone(),
                                },
                            ));
                        }
                        log::warn!("rejected prepared activation manifest: {err}");
                    }
                }
            }
            AgentMessage::LoadManifestResponse(response) => {
                if self.revision_gate.admit_manifest(&response.revision)
                    == ManifestAdmission::Ignore
                {
                    log::debug!(
                        "ignoring manifest for inactive logic revision {}",
                        response.revision.logic_revision_id
                    );
                    return;
                }

                let manifest = match rmp_serde::from_slice::<PaxManifest>(&response.manifest) {
                    Ok(manifest) => manifest,
                    Err(err) => {
                        self.revision_gate.reject_manifest();
                        log::warn!("received invalid design-server manifest payload: {err}");
                        return;
                    }
                };
                match self.orm.set_initial_server_manifest(manifest) {
                    Ok(()) => self.revision_gate.commit_manifest(response.revision),
                    Err(err) => {
                        self.revision_gate.reject_manifest();
                        log::warn!("rejected design-server manifest: {err}");
                    }
                }
            }
            AgentMessage::UpdateTemplateRequest(update) => {
                let admission = self.revision_gate.admit_template(&update.revision);
                match admission {
                    TemplateAdmission::Apply => {
                        let revision = update.revision;
                        if let Err(err) = self.orm.replace_template(
                            update.type_id,
                            update.new_template,
                            update.settings_block,
                        ) {
                            self.revision_gate.reject_manifest();
                            log::warn!("failed to apply template update from design-server: {err}");
                        } else {
                            self.revision_gate.commit_template(revision);
                        }
                    }
                    TemplateAdmission::RequestSnapshot => {
                        revision_messages.push(self.revision_gate.manifest_request());
                    }
                    TemplateAdmission::Ignore => {}
                }
            }
            AgentMessage::AppRevisionActivationRejected(rejection) => {
                if self
                    .revision_gate
                    .reject_activation(&rejection.logic_revision_id)
                {
                    log::warn!(
                        "design server rejected application revision {}: {}",
                        rejection.logic_revision_id,
                        rejection.reason
                    );
                } else {
                    log::debug!(
                        "ignoring stale activation rejection for {}",
                        rejection.logic_revision_id
                    );
                }
            }
            AgentMessage::PrepareAppRevision(prepared) => {
                if self.revision_gate.prepare(prepared.clone()) {
                    let mut pending = self.pending_prepare_app_revisions.borrow_mut();
                    pending.clear();
                    pending.push(prepared);
                }
            }
            AgentMessage::DevClientRequest(request) => {
                self.pending_dev_client_requests.borrow_mut().push(request);
            }
            AgentMessage::UserlandSourceUpdateResponse(response) => {
                self.pending_userland_source_update_responses
                    .borrow_mut()
                    .push(response);
            }
            AgentMessage::LLMPartialResponse(partial) => {
                self.orm
                    .add_new_message(partial.request_id, partial.message, None);
            }
            AgentMessage::LLMFinalResponse(final_response) => {
                self.orm.add_new_message(
                    final_response.request_id,
                    final_response.message,
                    Some(final_response.component_definition),
                );
            }
            _ => {}
        }
    }

    fn handle_pub_pax_message(&mut self, message: AgentMessage) {
        match message {
            AgentMessage::LLMPartialResponse(partial) => {
                self.orm
                    .add_new_message(partial.request_id, partial.message, None);
            }
            AgentMessage::LLMFinalResponse(final_response) => {
                self.orm.add_new_message(
                    final_response.request_id,
                    final_response.message,
                    Some(final_response.component_definition),
                );
            }
            _ => {}
        }
    }

    fn send_revision_messages(&self, messages: Vec<AgentMessage>) {
        if messages.is_empty() {
            return;
        }
        let mut connection = self.privileged_agent_connection.borrow_mut();
        for message in messages {
            if let Err(err) = connection.send_agent_message(&message) {
                log::debug!("revision message queued for websocket reconnect: {err}");
                break;
            }
        }
    }

    fn pub_pax_connection(&mut self) -> anyhow::Result<Rc<RefCell<WebSocketConnection>>> {
        if let Some(connection) = &self.pub_pax_connection {
            return Ok(connection.clone());
        }

        let address = get_server_base_url();
        let connection = Rc::new(RefCell::new(WebSocketConnection::new(
            &address,
            Some(VERSION_PREFIX),
            "pub-pax",
        )?));
        self.pub_pax_connection = Some(connection.clone());
        Ok(connection)
    }

    pub fn handle_response(&mut self, response: DesigntimeResponseMessage) {
        match response {
            DesigntimeResponseMessage::LLMResponse(component) => {
                log::info!("handling LLM response");
                let _ = self.orm.swap_main_component(component).map_err(|e| {
                    log::error!("Error swapping main component for LLM response: {:?}", e);
                });
            }
            DesigntimeResponseMessage::PublishResponse(response) => {
                log::info!("received publish response");
                self.publish_state.set(Some(response));
            }
        }
    }

    pub fn shutdown(&mut self) {
        self.privileged_agent_connection
            .borrow_mut()
            .shutdown_permanently();
        if let Some(pub_pax_connection) = &self.pub_pax_connection {
            pub_pax_connection.borrow_mut().shutdown_permanently();
        }
    }
}

fn resolve_default_local_addr() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()?.location().origin().ok()
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

pub enum Args {}

pub struct NodeWithBounds {
    pub uni: UniqueTemplateNodeIdentifier,
    pub x: f64,
    pub y: f64,
}
