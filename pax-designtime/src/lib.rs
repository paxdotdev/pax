use std::any::Any;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};

pub mod orm;
pub mod privileged_agent;

pub mod messages;
pub mod serde_pax;

mod setup;
use orm::ReloadType;
use pax_manifest::pax_runtime_api::Property;
pub use setup::add_additional_dependencies_to_cargo_toml;

use core::fmt::Debug;
pub use pax_manifest;
use pax_manifest::{ComponentDefinition, PaxManifest, TypeId, UniqueTemplateNodeIdentifier};
pub use serde_pax::error::{Error, Result};
pub use serde_pax::se::{to_pax, Serializer};
use wasm_bindgen::prelude::*;
use web_sys::window;

pub const INITIAL_MANIFEST_FILE_NAME: &str = "initial-manifest.json";

type Factories = HashMap<String, Box<fn(ComponentDefinition) -> Box<dyn Any>>>;
use crate::orm::PaxManifestORM;

pub struct DesigntimeManager {
    orm: PaxManifestORM,
    factories: Factories,
    // priv_agent_connection: Rc<RefCell<PrivilegedAgentConnection>>,
    #[allow(unused)]
    last_written_manifest_version: usize,
    project_query: Option<String>,
}

#[cfg(debug_assertions)]
impl Debug for DesigntimeManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesigntimeManager").finish()
    }
}

impl DesigntimeManager {
    pub fn new_with_addr(manifest: PaxManifest, _priv_addr: SocketAddr) -> Self {
        // let priv_agent = Rc::new(RefCell::new(
        //     PrivilegedAgentConnection::new(priv_addr)
        //         .expect("couldn't connect to privileged agent"),
        // ));

        let orm = PaxManifestORM::new(manifest);
        let factories = HashMap::new();
        DesigntimeManager {
            orm,
            factories,
            // priv_agent_connection: priv_agent,
            last_written_manifest_version: 0,
            project_query: None,
        }
    }

    pub fn new(manifest: PaxManifest) -> Self {
        Self::new_with_addr(manifest, SocketAddr::from((Ipv4Addr::LOCALHOST, 8252)))
    }

    pub fn set_project(&mut self, project_query: String) {
        self.project_query = Some(project_query);
    }

    pub fn send_file_to_static_dir(&self, _name: &str, _data: Vec<u8>) -> anyhow::Result<()> {
        // self.priv_agent_connection
        //     .borrow_mut()
        //     .send_file_to_static_dir(name, data)?;
        Ok(())
    }

    pub fn send_manifest_update(&mut self) -> anyhow::Result<()> {
        // Serialize the manifest to JSON
        let json = serde_json::to_string(self.orm.get_manifest()).unwrap();
        let proj_str = self.project_query.clone();
        let url = format!("http://localhost:9000/create/save{}", proj_str.unwrap());
        wasm_bindgen_futures::spawn_local(async move {
            // Create a RequestInit object with the method and body
            let mut opts = web_sys::RequestInit::new();
            opts.method("POST");
            opts.body(Some(&JsValue::from_str(&json)));

            // Create the request
            let request = web_sys::Request::new_with_str_and_init(&url, &opts).unwrap();

            // Send the request
            Into::<wasm_bindgen_futures::JsFuture>::into(
                window().unwrap().fetch_with_request(&request),
            )
            .await
            .unwrap();
            log::info!("sucessfully saved")
        });
        Ok(())
    }

    pub fn send_component_update(&mut self, _type_id: &TypeId) -> anyhow::Result<()> {
        // Send the JSON response back to JS.
        // let component = self.orm.get_component(type_id)?;
        // self.priv_agent_connection
        //     .borrow_mut()
        //     .send_component_update(component)?;

        // for c in self.orm.get_new_components() {
        //     self.priv_agent_connection
        //         .borrow_mut()
        //         .send_component_update(&c)?;
        // }

        Ok(())
    }

    pub fn llm_request(&mut self, _request: &str) -> anyhow::Result<()> {
        // let manifest = self.orm.get_manifest();
        // let userland_type_id = TypeId::build_singleton(
        //     "pax_designer::pax_reexports::designer_project::Example",
        //     None,
        // );
        // let userland_component = manifest.components.get(&userland_type_id).unwrap();
        // let request = LLMHelpRequest {
        //     request: request.to_string(),
        //     component: userland_component.clone(),
        // };
        // self.priv_agent_connection
        //     .borrow_mut()
        //     .send_llm_request(request)?;
        Ok(())
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
        Ok(())
        // let current_manifest_version = self.orm.get_manifest_version().get();
        // if current_manifest_version != self.last_written_manifest_version
        //     && current_manifest_version % 5 == 0
        // {
        //     let queue = self.get_orm_mut().take_reload_queue();
        //     for item in queue {
        //         match item {
        //             ReloadType::FullEdit => {
        //                 self.send_component_update(&TypeId::build_singleton(
        //                     "pax_designer::pax_reexports::designer_project::Example",
        //                     None,
        //                 ))?;
        //             }
        //             ReloadType::FullPlay => {
        //                 self.send_component_update(&TypeId::build_singleton(
        //                     "pax_designer::pax_reexports::designer_project::Example",
        //                     None,
        //                 ))?;
        //             }
        //             ReloadType::Partial(uni) => {
        //                 self.send_component_update(&uni.get_containing_component_type_id())?;
        //             }
        //         }
        //     }
        //     self.last_written_manifest_version = current_manifest_version;
        // }
        // self.priv_agent_connection
        //     .borrow_mut()
        //     .handle_recv(&mut self.orm)
    }
}

pub enum Args {}

pub struct NodeWithBounds {
    pub uni: UniqueTemplateNodeIdentifier,
    pub x: f64,
    pub y: f64,
}
