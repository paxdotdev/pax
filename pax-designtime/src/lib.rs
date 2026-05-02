use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub mod orm;
pub mod privileged_agent;

pub mod messages;
pub mod serde_pax;

use messages::LLMRequest;
use messages::{
    DevClientRequest, DevClientResponse, ReloadAppRequest, UserlandSourceUpdateRequest,
    UserlandSourceUpdateResponse,
};
use orm::{MessageType, ReloadType};
use pax_manifest::pax_runtime_api::Property;
use pax_message::ScreenshotData;
use privileged_agent::WebSocketConnection;

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
    pending_reload_app_requests: Rc<RefCell<Vec<ReloadAppRequest>>>,
    pending_userland_source_update_responses: Rc<RefCell<Vec<UserlandSourceUpdateResponse>>>,
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
        let privileged_agent = Rc::new(RefCell::new(
            WebSocketConnection::new(local_addr, None, "privileged-agent")
                .expect("couldn't connect to privileged agent"),
        ));

        let orm = PaxManifestORM::new(manifest);
        let factories = HashMap::new();
        DesigntimeManager {
            orm,
            factories,
            privileged_agent_connection: privileged_agent,
            pub_pax_connection: None,
            project_query: None,
            response_queue: Rc::new(RefCell::new(Vec::new())),
            pending_dev_client_requests: Rc::new(RefCell::new(Vec::new())),
            pending_reload_app_requests: Rc::new(RefCell::new(Vec::new())),
            pending_userland_source_update_responses: Rc::new(RefCell::new(Vec::new())),
            last_rendered_manifest_version: Property::new(0),
            publish_state: Default::default(),
            enqueued_llm_request: None,
        }
    }
    pub fn new(manifest: PaxManifest) -> Self {
        let local_addr = std::env::var("PAX_DESIGN_SERVER_ADDR")
            .ok()
            .or_else(resolve_default_local_addr)
            .unwrap_or_else(|| "ws://localhost:8080".to_string());
        Self::new_with_local_addr(manifest, &local_addr)
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

    pub fn take_reload_app_requests(&mut self) -> Vec<ReloadAppRequest> {
        let mut pending_requests = self.pending_reload_app_requests.borrow_mut();
        pending_requests.drain(..).collect()
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

        let privileged_agent_messages = match self
            .privileged_agent_connection
            .borrow_mut()
            .handle_recv(&mut self.orm)
        {
            Ok(messages) => messages,
            Err(err) => {
                log::warn!("privileged-agent receive failed: {err:?}");
                Vec::new()
            }
        };
        for message in privileged_agent_messages {
            match message {
                crate::messages::AgentMessage::DevClientRequest(request) => {
                    self.pending_dev_client_requests.borrow_mut().push(request);
                }
                crate::messages::AgentMessage::ReloadAppRequest(request) => {
                    self.pending_reload_app_requests.borrow_mut().push(request);
                }
                crate::messages::AgentMessage::UserlandSourceUpdateResponse(response) => {
                    self.pending_userland_source_update_responses
                        .borrow_mut()
                        .push(response);
                }
                _ => {}
            }
        }

        if let Some(pub_pax_connection) = &self.pub_pax_connection {
            if let Err(err) = pub_pax_connection.borrow_mut().handle_recv(&mut self.orm) {
                log::warn!("pub-pax receive failed: {err:?}");
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
