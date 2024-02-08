//! # PaxManifestORM API
//!
//! `PaxManifestORM` provides an interface for managing `PaxManifest` objects, allowing for easy management of template nodes, selectors, and handlers.
//!
//! ## Main Functions
//!
//! - `build_new_node`: Create a new node builder instance. This method initializes a `NodeBuilder` for creating a new template node.
//! - `get_node`: Retrieve an existing node. This method returns a `NodeBuilder` initialized with an existing node's data.
//! - `remove_node`: Remove a specified node from the manifest.
//! - `build_new_selector`: Create a new selector builder instance. This method initializes a `SelectorBuilder` for creating a new selector.
//! - `get_selector`: Retrieve an existing selector. This method returns a `SelectorBuilder` initialized with an existing selector's data.
//! - `remove_selector`: Remove a specified selector from the manifest.
//! - `build_new_handler`: Create a new handler builder instance. This method initializes a `HandlerBuilder` for creating a new handler.
//! - `get_handler`: Retrieve an existing handler. This method returns a `HandlerBuilder` initialized with an existing handler's data.
//! - `remove_handler`: Remove a specified handler from the manifest.
//! - `execute_command`: Execute a command that implements the `Command` trait, allowing for actions like adding, updating, or removing nodes, selectors, and handlers.
//! - `undo`: Undo the last command. This method rolls back the last change made to the manifest.
//! - `redo`: Redo the last undone command. This method reapplies the last change that was undone.
//! - `undo_until`: Undo commands up to a specified command ID. This allows for targeted rollback of multiple changes.
//!
//! For usage examples see the tests in `pax-designtime/src/orm/tests.rs`.

use pax_manifest::{ComponentDefinition, LiteralBlockDefinition, PaxManifest};
use serde_derive::{Deserialize, Serialize};
#[allow(unused_imports)]
use serde_json;

use self::{
    handlers::{builder::HandlerBuilder, RemoveHandlerRequest},
    settings::{builder::SelectorBuilder, RemoveSelectorRequest},
    template::{builder::NodeBuilder, RemoveTemplateNodeRequest},
};

use anyhow::anyhow;
pub mod handlers;
pub mod settings;
pub mod template;
#[cfg(test)]
mod tests;

pub trait Request {
    type Response: Response;
}

pub trait Response {
    fn set_id(&mut self, id: usize);
}

pub trait Command<R: Request> {
    fn execute(&mut self, manifest: &mut PaxManifest) -> Result<R::Response, String>;
    fn as_undo_redo(&mut self) -> Option<UndoRedoCommand> {
        None
    }
}

#[derive(Serialize, Deserialize)]
pub struct PaxManifestORM {
    manifest: PaxManifest,
    undo_stack: Vec<(usize, UndoRedoCommand)>,
    redo_stack: Vec<(usize, UndoRedoCommand)>,
    next_command_id: usize,
    // This counter increase with each command execution/undo/redo (essentially tracks each unique change to the manifest)
    manifest_version: usize,
}

impl PaxManifestORM {
    pub fn new(manifest: PaxManifest) -> Self {
        PaxManifestORM {
            manifest,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            next_command_id: 0,
            manifest_version: 0,
        }
    }

    pub fn get_manifest(&self) -> &PaxManifest {
        &self.manifest
    }

    pub fn get_manifest_version(&self) -> usize {
        self.manifest_version
    }

    pub fn build_new_node(
        &mut self,
        component_type_id: String, //containing component
        type_id: String,           //the thing we want to create
        pascal_identifier: String, // split at last :: and take end
        parent_node_id: Option<usize>,
    ) -> NodeBuilder {
        NodeBuilder::new(
            self,
            component_type_id,
            type_id,
            pascal_identifier,
            parent_node_id,
        )
    }
    pub fn get_node(&mut self, component_type_id: &str, node_id: usize) -> NodeBuilder {
        NodeBuilder::retrieve_node(self, component_type_id, node_id)
    }

    pub fn get_main_component(&self) -> &str {
        &self.manifest.main_component_type_id
    }

    pub fn get_component(&self, type_id: &str) -> anyhow::Result<&ComponentDefinition> {
        self.manifest
            .components
            .get(type_id)
            .ok_or(anyhow!("couldn't find component"))
    }

    pub fn remove_node(&mut self, component_type_id: String, node_id: usize) -> Result<(), String> {
        let command = RemoveTemplateNodeRequest::new(component_type_id, node_id);
        self.execute_command(command)?;
        Ok(())
    }

    pub fn build_new_selector(
        &mut self,
        component_type_id: String,
        key: String,
        value: LiteralBlockDefinition,
    ) -> SelectorBuilder {
        SelectorBuilder::new(self, component_type_id, key, value)
    }

    pub fn get_selector(&mut self, component_type_id: String, key: String) -> SelectorBuilder {
        SelectorBuilder::retreive_selector(self, component_type_id, key)
    }

    pub fn remove_selector(
        &mut self,
        component_type_id: String,
        key: String,
    ) -> Result<(), String> {
        let command = RemoveSelectorRequest::new(component_type_id, key);
        self.execute_command(command)?;
        Ok(())
    }

    pub fn build_new_handler(&mut self, component_type_id: String, key: String) -> HandlerBuilder {
        HandlerBuilder::new(self, component_type_id, key)
    }

    pub fn get_handler(&mut self, component_type_id: String, key: String) -> HandlerBuilder {
        HandlerBuilder::retrieve_handler(self, component_type_id, key)
    }

    pub fn remove_handler(&mut self, component_type_id: String, key: String) -> Result<(), String> {
        let command = RemoveHandlerRequest::new(component_type_id, key);
        self.execute_command(command)?;
        Ok(())
    }

    pub fn execute_command<R: Request, C>(&mut self, mut command: C) -> Result<R::Response, String>
    where
        C: Command<R>,
    {
        let mut response = command.execute(&mut self.manifest)?;
        let command_id = self.next_command_id;
        if let Some(command) = command.as_undo_redo() {
            self.undo_stack.push((command_id, command));
            self.redo_stack.clear();
        }

        response.set_id(command_id);
        self.next_command_id += 1;
        self.manifest_version += 1;
        Ok(response)
    }

    pub fn undo(&mut self) -> Result<(), String> {
        if let Some((id, mut command)) = self.undo_stack.pop() {
            command.undo(&mut self.manifest)?;
            self.redo_stack.push((id, command));
            self.manifest_version += 1;
        }
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), String> {
        if let Some((id, mut command)) = self.redo_stack.pop() {
            command.redo(&mut self.manifest)?;
            self.undo_stack.push((id, command));
            self.manifest_version += 1;
        }
        Ok(())
    }

    pub fn undo_until(&mut self, command_id: usize) -> Result<(), String> {
        while let Some((id, _)) = self.undo_stack.last() {
            if *id == command_id {
                break;
            }
            self.undo()?;
        }
        Ok(())
    }
}

pub trait UndoRedo {
    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String>;
    fn redo(&mut self, manifest: &mut PaxManifest) -> Result<(), String>;
}

#[derive(Serialize, Deserialize)]
pub enum UndoRedoCommand {
    AddTemplateNodeRequest(template::AddTemplateNodeRequest),
    RemoveTemplateNodeRequest(template::RemoveTemplateNodeRequest),
    UpdateTemplateNodeRequest(template::UpdateTemplateNodeRequest),
    AddSelectorRequest(settings::AddSelectorRequest),
    UpdateSelectorRequest(settings::UpdateSelectorRequest),
    RemoveSelectorRequest(settings::RemoveSelectorRequest),
    AddHandlerRequest(handlers::AddHandlerRequest),
    UpdateHandlerRequest(handlers::UpdateHandlerRequest),
    RemoveHandlerRequest(handlers::RemoveHandlerRequest),
}

impl UndoRedo for UndoRedoCommand {
    fn undo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        match self {
            UndoRedoCommand::AddTemplateNodeRequest(command) => command.undo(manifest),
            UndoRedoCommand::RemoveTemplateNodeRequest(command) => command.undo(manifest),
            UndoRedoCommand::UpdateTemplateNodeRequest(command) => command.undo(manifest),
            UndoRedoCommand::AddSelectorRequest(command) => command.undo(manifest),
            UndoRedoCommand::UpdateSelectorRequest(command) => command.undo(manifest),
            UndoRedoCommand::RemoveSelectorRequest(command) => command.undo(manifest),
            UndoRedoCommand::AddHandlerRequest(command) => command.undo(manifest),
            UndoRedoCommand::UpdateHandlerRequest(command) => command.undo(manifest),
            UndoRedoCommand::RemoveHandlerRequest(command) => command.undo(manifest),
        }
    }

    fn redo(&mut self, manifest: &mut PaxManifest) -> Result<(), String> {
        match self {
            UndoRedoCommand::AddTemplateNodeRequest(command) => command.redo(manifest),
            UndoRedoCommand::RemoveTemplateNodeRequest(command) => command.redo(manifest),
            UndoRedoCommand::UpdateTemplateNodeRequest(command) => command.redo(manifest),
            UndoRedoCommand::AddSelectorRequest(command) => command.redo(manifest),
            UndoRedoCommand::UpdateSelectorRequest(command) => command.redo(manifest),
            UndoRedoCommand::RemoveSelectorRequest(command) => command.redo(manifest),
            UndoRedoCommand::AddHandlerRequest(command) => command.redo(manifest),
            UndoRedoCommand::UpdateHandlerRequest(command) => command.redo(manifest),
            UndoRedoCommand::RemoveHandlerRequest(command) => command.redo(manifest),
        }
    }
}
