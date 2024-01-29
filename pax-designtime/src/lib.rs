use std::any::Any;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};

use crate::orm::PaxManifestORM;
use crate::selection::PaxSelectionManager;
use crate::undo::PaxUndoManager;

pub mod cartridge_generation;
pub mod orm;
pub mod selection;
pub mod undo;

pub mod messages;
pub mod serde_pax;

mod setup;
pub use setup::add_additional_dependencies_to_cargo_toml;

use core::fmt::Debug;
pub use pax_manifest;
use pax_manifest::{ComponentDefinition, PaxManifest};
use priveleged_agent::PrivilegedAgentConnection;
pub use serde_pax::de::{from_pax, Deserializer};
pub use serde_pax::error::{Error, Result};
pub use serde_pax::se::{to_pax, Serializer};

pub const INITIAL_MANIFEST_FILE_NAME: &str = "initial-manifest.json";

type Factories = HashMap<String, Box<fn(ComponentDefinition) -> Box<dyn Any>>>;

pub struct DesigntimeManager {
    orm: PaxManifestORM,
    _selection: PaxSelectionManager,
    active_component_id: String,
    _undo_stack: PaxUndoManager,
    factories: Factories,
    priv_agent_connection: PrivilegedAgentConnection,
}

#[cfg(debug_assertions)]
impl Debug for DesigntimeManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesigntimeManager").finish()
    }
}
pub mod priveleged_agent;

impl DesigntimeManager {
    pub fn new_with_addr(manifest: PaxManifest, priv_addr: SocketAddr) -> Self {
        let orm = PaxManifestORM::new(manifest);
        let selection = PaxSelectionManager::new();
        let undo_stack = PaxUndoManager::new();
        let factories = HashMap::new();
        DesigntimeManager {
            orm,
            _selection: selection,
            _undo_stack: undo_stack,
            factories,
            priv_agent_connection: PrivilegedAgentConnection::new(priv_addr)
                .expect("couldn't connect to privaleged agent"),
        }
    }

    pub fn new(manifest: PaxManifest) -> Self {
        Self::new_with_addr(manifest, SocketAddr::from((Ipv4Addr::LOCALHOST, 8252)))
    }

    pub fn send_component_update(&mut self, type_id: &str) -> anyhow::Result<()> {
        let component = self.orm.get_component(type_id)?;
        self.priv_agent_connection
            .send_component_update(component)?;
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
}

pub enum Args {}
