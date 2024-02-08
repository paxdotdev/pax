use std::any::Any;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};

use std::cell::RefCell;
use std::rc::Rc;

pub mod action;
pub mod cartridge_generation;
pub mod input;
pub mod orm;
pub mod selection;

pub mod messages;
pub mod serde_pax;

mod setup;
use input::FSMEvent;
pub use setup::add_additional_dependencies_to_cargo_toml;

use core::fmt::Debug;
pub use pax_manifest;
use pax_manifest::{ComponentDefinition, PaxManifest};
use privileged_agent::PrivilegedAgentConnection;
pub use serde_pax::de::{from_pax, Deserializer};
pub use serde_pax::error::{Error, Result};
pub use serde_pax::se::{to_pax, Serializer};

pub const INITIAL_MANIFEST_FILE_NAME: &str = "initial-manifest.json";

type Factories = HashMap<String, Box<fn(ComponentDefinition) -> Box<dyn Any>>>;
use crate::action::ActionManager;
use crate::input::InputManager;
use crate::orm::PaxManifestORM;
use crate::selection::SelectionManager;

pub struct DesigntimeManager {
    orm: PaxManifestORM,
    selection_manager: SelectionManager,
    // active_component_id: String,
    factories: Factories,
    input_manager: InputManager,
    action_manager: ActionManager,
    priv_agent_connection: PrivilegedAgentConnection,
}

#[cfg(debug_assertions)]
impl Debug for DesigntimeManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesigntimeManager").finish()
    }
}
pub mod privileged_agent;

impl DesigntimeManager {
    pub fn new_with_addr(manifest: PaxManifest, priv_addr: SocketAddr) -> Self {
        let orm = PaxManifestORM::new(manifest);
        let selection_manager = SelectionManager::new();
        let action_manager = ActionManager::new();
        let input_manager = InputManager::new();
        let factories = HashMap::new();
        DesigntimeManager {
            orm,
            selection_manager,
            action_manager,
            input_manager,
            factories,
            priv_agent_connection: PrivilegedAgentConnection::new(priv_addr)
                .expect("couldn't connect to privileged agent"),
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

    pub fn input_transition(&mut self, event: FSMEvent) -> anyhow::Result<()> {
        self.input_manager
            .transition(event, &mut self.action_manager, &mut self.orm)
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
