use std::any::Any;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;

use crate::orm::PaxManifestORM;
use crate::selection::PaxSelectionManager;
use crate::undo::PaxUndoManager;

pub mod cartridge_generation;
pub mod orm;
pub mod selection;
pub mod undo;

pub mod messages;
mod serde_pax;
mod setup;
pub use setup::add_additional_dependencies_to_cargo_toml;

use core::fmt::Debug;
pub use pax_manifest;
use pax_manifest::{
    ComponentDefinition, PaxManifest, PropertyDefinition, SettingElement, SettingsBlockElement,
    TemplateNodeDefinition, Token, ValueDefinition,
};
use priveleged_agent::PrivilegedAgentConnection;
pub use serde_pax::de::{from_pax, Deserializer};
pub use serde_pax::error::{Error, Result};
pub use serde_pax::se::{to_pax, Serializer};
pub struct DesigntimeManager {
    orm: PaxManifestORM,
    selection: PaxSelectionManager,
    undo_stack: PaxUndoManager,
    factories: HashMap<String, Box<fn(ComponentDefinition) -> Box<dyn Any>>>,
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
            selection,
            undo_stack,
            factories,
            priv_agent_connection: PrivilegedAgentConnection::new(priv_addr)
                .expect("couldn't connect to privaleged agent"),
        }
    }

    pub fn new(manifest: PaxManifest) -> Self {
        Self::new_with_addr(manifest, SocketAddr::from((Ipv4Addr::LOCALHOST, 8252)))
    }

    pub fn send_manifest_update(&mut self) -> anyhow::Result<()> {
        self.priv_agent_connection
            .send_manifest_update(self.orm.get_manifest())?;
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

    pub fn get_orm(&self) -> &PaxManifestORM {
        &self.orm
    }

    pub fn get_orm_mut(&mut self) -> &mut PaxManifestORM {
        &mut self.orm
    }
}

pub enum Args {}
