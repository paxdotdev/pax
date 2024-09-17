use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::rc::Rc;

pub mod orm;
pub mod privileged_agent;

pub mod messages;
pub mod serde_pax;

use orm::ReloadType;
use pax_manifest::pax_runtime_api::Property;
use privileged_agent::PrivilegedAgentConnection;

use core::fmt::Debug;
pub use pax_manifest;
use pax_manifest::{
    server::*, ComponentDefinition, PaxManifest, TypeId, UniqueTemplateNodeIdentifier,
};
use reqwasm::http::Response;
pub use serde_pax::error::{Error, Result};
pub use serde_pax::se::{to_pax, Serializer};

pub const INITIAL_MANIFEST_FILE_NAME: &str = "initial-manifest.json";

type Factories = HashMap<String, Box<fn(ComponentDefinition) -> Box<dyn Any>>>;
use crate::orm::PaxManifestORM;

pub struct DesigntimeManager {
    orm: PaxManifestORM,
    factories: Factories,
    priv_agent_connection: Rc<RefCell<PrivilegedAgentConnection>>,
    #[allow(unused)]
    last_written_manifest_version: usize,
    project_query: Option<String>,
    response_queue: Rc<RefCell<Vec<DesigntimeResponseMessage>>>,
    pub publish_state: Property<Option<PublishResponse>>,
}

pub enum DesigntimeResponseMessage {
    LLMResponse(ComponentDefinition),
    PublishResponse(PublishResponse),
}

#[cfg(debug_assertions)]
impl Debug for DesigntimeManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesigntimeManager").finish()
    }
}

const ENDPOINT_LLM: &str = "/v0/llm_request";
const ENDPOINT_PUBLISH: &str = "/v0/publish";
const PROD_PUB_PAX_SERVER: &str = "https://pub.pax.dev";

fn get_server_base_url() -> String {
    // Fetch the environment variable or use the default value
    option_env!("PUB_PAX_SERVER")
        .unwrap_or(PROD_PUB_PAX_SERVER)
        .to_string()
}

impl DesigntimeManager {
    pub fn new_with_addr(manifest: PaxManifest, priv_addr: SocketAddr) -> Self {
        let priv_agent = Rc::new(RefCell::new(
            PrivilegedAgentConnection::new(priv_addr)
                .expect("couldn't connect to privileged agent"),
        ));

        let orm = PaxManifestORM::new(manifest);
        let factories = HashMap::new();
        DesigntimeManager {
            orm,
            factories,
            priv_agent_connection: priv_agent,
            last_written_manifest_version: 0,
            project_query: None,
            response_queue: Rc::new(RefCell::new(Vec::new())),
            publish_state: Default::default(),
        }
    }

    pub fn new(manifest: PaxManifest) -> Self {
        Self::new_with_addr(manifest, SocketAddr::from((Ipv4Addr::LOCALHOST, 8080)))
    }

    pub fn set_project(&mut self, project_query: String) {
        self.project_query = Some(project_query);
    }

    pub fn send_file_to_static_dir(&self, name: &str, data: Vec<u8>) -> anyhow::Result<()> {
        self.priv_agent_connection
            .borrow_mut()
            .send_file_to_static_dir(name, data)?;
        Ok(())
    }

    pub fn get_manifest_loaded_from_server_prop(&self) -> Property<bool> {
        self.orm.manifest_loaded_from_server.clone()
    }

    pub fn send_component_update(&mut self, type_id: &TypeId) -> anyhow::Result<()> {
        let component = self.orm.get_component(type_id)?;
        self.priv_agent_connection
            .borrow_mut()
            .send_component_update(component)?;

        for c in self.orm.get_new_components() {
            self.priv_agent_connection
                .borrow_mut()
                .send_component_update(&c)?;
        }

        Ok(())
    }

    pub fn llm_request(&mut self, prompt: &str) -> anyhow::Result<()> {
        let manifest = self.orm.get_manifest().clone();

        let llm_request = LLMRequest {
            manifest: manifest.clone(),
            prompt: prompt.to_string(),
        };

        let queue_cloned = self.response_queue.clone();

        wasm_bindgen_futures::spawn_local(async move {
            let url = get_server_base_url() + ENDPOINT_LLM;

            let response = reqwasm::http::Request::post(&url)
                .header("Content-Type", "application/json")
                .body(serde_json::to_string(&llm_request).unwrap())
                .send()
                .await;

            log::info!("response_text: {:?}", response);
            let updated_main_component: ComponentDefinition =
                response.unwrap().json().await.unwrap();
            log::info!(
                "updated_main_component: {:?}",
                updated_main_component.template
            );
            queue_cloned
                .borrow_mut()
                .push(DesigntimeResponseMessage::LLMResponse(
                    updated_main_component,
                ));
        });

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

    pub fn take_reload_queue(&mut self) -> Vec<ReloadType> {
        self.orm.take_reload_queue()
    }

    pub fn reload_play(&mut self) {
        self.orm.set_reload(ReloadType::FullPlay);
        self.orm.increment_manifest_version();
    }

    pub fn reload_edit(&mut self) {
        self.orm.set_reload(ReloadType::FullEdit);
        self.orm.increment_manifest_version();
    }

    pub fn set_userland_root_component_type_id(&mut self, type_id: &TypeId) {
        self.orm.set_userland_root_component_type_id(type_id);
        self.orm.increment_manifest_version();
        self.orm.set_reload(ReloadType::FullEdit);
    }

    pub fn get_manifest_version(&self) -> Property<usize> {
        self.orm.get_manifest_version()
    }

    pub fn get_orm(&self) -> &PaxManifestORM {
        &self.orm
    }

    pub fn get_orm_mut(&mut self) -> &mut PaxManifestORM {
        &mut self.orm
    }

    pub fn handle_recv(&mut self) -> anyhow::Result<()> {
        let current_manifest_version = self.orm.get_manifest_version().get();
        if current_manifest_version != self.last_written_manifest_version {
            self.last_written_manifest_version = current_manifest_version;
        }
        self.priv_agent_connection
            .borrow_mut()
            .handle_recv(&mut self.orm)?;

        let response_queue = {
            let mut queue = self.response_queue.borrow_mut();
            queue.drain(..).collect::<Vec<DesigntimeResponseMessage>>()
        };
        for response in response_queue {
            self.handle_response(response);
        }
        Ok(())
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
}

pub enum Args {}

pub struct NodeWithBounds {
    pub uni: UniqueTemplateNodeIdentifier,
    pub x: f64,
    pub y: f64,
}
