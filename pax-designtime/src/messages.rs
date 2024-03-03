use pax_manifest::{ComponentTemplate, PaxManifest, TypeId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum AgentMessage {
    ProjectFileChangedNotification(FileChangedNotification),
    ManifestSerializationRequest(ManifestSerializationRequest),
    ComponentSerializationRequest(ComponentSerializationRequest),
    ManifestSerializationAcknowledgement(ManifestSerializationAcknowledgement),
    ManifestSerializationCompletedNotification(ManifestSerializationCompletedNotification),
    RecompilationRequest(Box<RecompilationRequest>),
    RecompilationAcknowledgement(RecompilationAcknowledgement),
    RecompilationCompletedNotification(RecompilationCompletedNotification),
    UpdateTemplateRequest(UpdateTemplateRequest),
}

/// A notification indicating that a project file has changed.
/// This message is sent from `pax-privileged-agent` to `pax-designtime`.
#[derive(Serialize, Deserialize, Default)]
pub struct FileChangedNotification {}

/// A request to serialize the provided manifest into code.
/// This is sent from `pax-designtime` to `pax-privileged-agent`.
#[derive(Serialize, Deserialize)]
pub struct ManifestSerializationRequest {
    pub manifest: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct ComponentSerializationRequest {
    pub component_bytes: Vec<u8>,
}
/// A response acknowledging the receipt of a manifest serialization request.
/// This message is sent from `pax-privileged-agent` back to `pax-designtime`.
/// It includes a unique identifier for the request.
#[derive(Serialize, Deserialize)]
pub struct ManifestSerializationAcknowledgement {
    /// The unique identifier for the serialization request.
    pub id: usize,
}

/// A notification indicating the completion of manifest serialization.
/// Sent from `pax-privileged-agent` to `pax-designtime` once the manifest is serialized into code.
/// Contains the identifier of the completed request.
#[derive(Serialize, Deserialize)]
pub struct ManifestSerializationCompletedNotification {
    /// The identifier of the request that has been completed.
    pub id: usize,
}

/// A request to recompile the project based on the provided manifest.
/// If no manifest is provided, it will trigger a recompilation based on file system.
/// Sent from `pax-designtime` to `pax-privileged-agent`.
#[derive(Serialize, Deserialize)]
pub struct RecompilationRequest {
    pub manifest: Option<PaxManifest>,
}

/// An acknowledgement response to a recompilation request.
/// Sent from `pax-privileged-agent` back to `pax-designtime`.
/// Contains a unique identifier for the recompilation request.
#[derive(Serialize, Deserialize)]
pub struct RecompilationAcknowledgement {
    /// The unique identifier for the recompilation request.
    pub id: usize,
}

/// A notification that the recompilation process has been completed.
/// Sent from `pax-privileged-agent` to `pax-designtime`.
/// Contains the identifier of the recompilation request that was completed.
#[derive(Serialize, Deserialize)]
pub struct RecompilationCompletedNotification {
    /// The identifier of the recompilation request that has been completed.
    pub id: usize,
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
