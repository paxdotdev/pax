use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::rc::Rc;
use std::time::SystemTime;

pub mod cartridge_generation;
pub mod orm;
pub mod privileged_agent;

pub mod messages;
pub mod serde_pax;

mod setup;
use messages::LLMHelpRequest;
pub use setup::add_additional_dependencies_to_cargo_toml;

use core::fmt::Debug;
pub use pax_manifest;
use pax_manifest::{ComponentDefinition, PaxManifest, TypeId};
use privileged_agent::PrivilegedAgentConnection;
pub use serde_pax::error::{Error, Result};
pub use serde_pax::se::{to_pax, Serializer};

pub const INITIAL_MANIFEST_FILE_NAME: &str = "initial-manifest.json";

type Factories = HashMap<String, Box<fn(ComponentDefinition) -> Box<dyn Any>>>;
use crate::orm::PaxManifestORM;

pub struct DesigntimeManager {
    orm: PaxManifestORM,
    factories: Factories,
    priv_agent_connection: Rc<RefCell<PrivilegedAgentConnection>>,
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
        }
    }

    pub fn new(manifest: PaxManifest) -> Self {
        Self::new_with_addr(manifest, SocketAddr::from((Ipv4Addr::LOCALHOST, 8252)))
    }

    pub fn send_component_update(&mut self, type_id: &TypeId) -> anyhow::Result<()> {
        let component = self.orm.get_component(type_id)?;
        self.priv_agent_connection
            .borrow_mut()
            .send_component_update(component)?;
        Ok(())
    }

    pub fn llm_request(&mut self, request: &str) -> anyhow::Result<()> {
        // eventually I want viewport info an current selected component
        let manifest = self.orm.get_manifest();
        let userland_type_id = TypeId::build_singleton(
            "pax_designer::pax_reexports::designer_project::Example",
            None,
        );
        let userland_component = manifest.components.get(&userland_type_id).unwrap();
        let request = LLMHelpRequest {
            request: request.to_string(),
            component: userland_component.clone(),
        };
        self.priv_agent_connection
            .borrow_mut()
            .send_llm_request(request)?;
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

    pub fn get_manifest_version(&self) -> usize {
        self.orm.get_manifest_version()
    }

    pub fn get_orm(&self) -> &PaxManifestORM {
        &self.orm
    }

    pub fn get_orm_mut(&mut self) -> &mut PaxManifestORM {
        &mut self.orm
    }

    pub fn handle_recv(&mut self) -> anyhow::Result<()> {
        self.priv_agent_connection
            .borrow_mut()
            .handle_recv(&mut self.orm)
    }
}

pub enum Args {}
