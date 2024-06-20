use pax_manifest::{ComponentDefinition, ComponentTemplate, TypeId};
use serde::{Deserialize, Serialize};

use crate::orm::template::NodeAction;

#[derive(Serialize, Deserialize)]
pub enum AgentMessage {
    ProjectFileChangedNotification(FileChangedNotification),
    ManifestSerializationRequest(ManifestSerializationRequest),
    ComponentSerializationRequest(ComponentSerializationRequest),
    UpdateTemplateRequest(Box<UpdateTemplateRequest>),
    LLMHelpRequest(LLMHelpRequest),
    LLMHelpResponse(LLMHelpResponse),
    LLMUpdatedTemplateNotification(LLMUpdatedTemplateNotification),
    LoadFileToStaticDirRequest(LoadFileToStaticDirRequest),
}

#[derive(Serialize, Deserialize)]
pub struct LoadFileToStaticDirRequest {
    pub name: String,
    pub data: Vec<u8>,
}

/// A notification indicating that a project file has changed.
/// This message is sent from `pax-design-server` to `pax-designtime`.
#[derive(Serialize, Deserialize, Default)]
pub struct FileChangedNotification {}

/// A request to serialize the provided manifest into code.
/// This is sent from `pax-designtime` to `pax-design-server`.
#[derive(Serialize, Deserialize)]
pub struct ManifestSerializationRequest {
    pub manifest: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct ComponentSerializationRequest {
    pub component_bytes: Vec<u8>,
}

/// A request to update the template of a component.
// Sent from `pax-priviliged-agent` to `pax-designtime`.
#[derive(Serialize, Deserialize)]
pub struct UpdateTemplateRequest {
    /// The type identifier of the component to update.
    pub type_id: TypeId,
    /// The new template for the component.
    pub new_template: ComponentTemplate,
}

/// A request for help from the LLM.
/// Sent from `pax-designtime` to `pax-design-server`.
#[derive(Serialize, Deserialize, Clone)]
pub struct LLMHelpRequest {
    pub request: String,
    pub component: ComponentDefinition,
}

/// A response from the LLM.
/// Sent from `pax-design-server` to `pax-designtime`.
#[derive(Serialize, Deserialize)]
pub struct LLMHelpResponse {
    pub request_id: String,
    pub component_type_id: TypeId,
    pub response: Vec<NodeAction>,
}

/// A notification that the updated template after an llm request
/// has been applied to the manifest.
/// This message is sent from `pax-designtime` to `pax-design-server`.
#[derive(Serialize, Deserialize)]
pub struct LLMUpdatedTemplateNotification {
    pub request_id: String,
    pub component: ComponentDefinition,
}
