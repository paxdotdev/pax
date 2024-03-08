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
#[derive(Serialize, Deserialize, Debug)]
pub struct LLMHelpRequest {
    pub request_id: usize,
    pub request: String,
    pub component: ComponentDefinition,
}

/// A response from the LLM.
/// Sent from `pax-design-server` to `pax-designtime`.
#[derive(Serialize, Deserialize)]
pub struct LLMHelpResponse {
    pub request_id: usize,
    pub response: Vec<NodeAction>,
}
