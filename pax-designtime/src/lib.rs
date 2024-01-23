use std::collections::HashMap;

use crate::orm::PaxManifestORM;
use crate::selection::PaxSelectionManager;
use crate::undo::PaxUndoManager;
use pax_core::ComponentInstance;

pub mod cartridge_generation;
pub mod orm;
pub mod selection;
pub mod undo;

mod serde_pax;

use core::fmt::Debug;
pub use pax_manifest;
use pax_manifest::{ComponentDefinition, PaxManifest};
pub use serde_pax::de::{from_pax, Deserializer};
pub use serde_pax::error::{Error, Result};
pub use serde_pax::se::{to_pax, Serializer};

pub struct DesigntimeManager {
    orm: PaxManifestORM,
    selection: PaxSelectionManager,
    undo_stack: PaxUndoManager,
    factories: HashMap<String, Box<fn(ComponentDefinition) -> ComponentInstance>>,
}

#[cfg(debug_assertions)]
impl Debug for DesigntimeManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesigntimeManager").finish()
    }
}

impl DesigntimeManager {
    pub fn new(manifest: PaxManifest) -> Self {
        let orm = PaxManifestORM::new(manifest);
        let selection = PaxSelectionManager::new();
        let undo_stack = PaxUndoManager::new();
        let factories = HashMap::new();
        DesigntimeManager {
            orm,
            selection,
            undo_stack,
            factories,
        }
    }

    pub fn add_factory(
        &mut self,
        type_id: String,
        factory: Box<fn(ComponentDefinition) -> ComponentInstance>,
    ) {
        self.factories.insert(type_id, factory);
    }

    pub fn get_manifest(&self) -> &PaxManifest {
        self.orm.get_manifest()
    }

    pub fn get_orm(&self) -> &PaxManifestORM {
        &self.orm
    }
}
