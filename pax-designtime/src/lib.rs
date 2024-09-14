use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::rc::Rc;

pub mod orm;
pub mod privileged_agent;

pub mod messages;
pub mod serde_pax;

use messages::LLMHelpRequest;
use orm::ReloadType;
use pax_manifest::pax_runtime_api::Property;
use privileged_agent::PrivilegedAgentConnection;
use serde_json::json;

use core::fmt::Debug;
pub use pax_manifest;
use pax_manifest::{
    ComponentDefinition, LLMRequest, PaxManifest, TypeId, UniqueTemplateNodeIdentifier,
};
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
}

pub enum DesigntimeResponseMessage {
    LLMResponse(ComponentDefinition),
}

#[cfg(debug_assertions)]
impl Debug for DesigntimeManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesigntimeManager").finish()
    }
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
            let response = reqwasm::http::Request::post("http://127.0.0.1:80/llm_request")
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
        log::warn!("project publishing not implemented");
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
                log::warn!("handling LLM response");
                let _ = self.orm.swap_main_component(component).map_err(|e| {
                    log::error!("Error swapping main component for LLM response: {:?}", e);
                });
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
