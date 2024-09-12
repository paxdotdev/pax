use pax_manifest::{ComponentTemplate, TypeId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum AgentMessage {
    ProjectFileChangedNotification(FileChangedNotification),
    ManifestSerializationRequest(ManifestSerializationRequest),
    // Request to retrieve the manifest from the design server
    // sent from designtime to design-server
    LoadManifestRequest,
    LoadManifestResponse(LoadManifestResponse),
    ComponentSerializationRequest(ComponentSerializationRequest),
    UpdateTemplateRequest(Box<UpdateTemplateRequest>),
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
pub struct LoadManifestRequest {}

#[derive(Serialize, Deserialize)]
pub struct LoadManifestResponse {
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
