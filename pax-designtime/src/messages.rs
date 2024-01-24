use pax_manifest::PaxManifest;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum AgentMessage {
    ProjectFileChangedNotification(FileChangedNotification),
    ManifestSerializationRequest(ManifestSerializationRequest),
    ManifestSerializationAcknowledgement(ManifestSerializationAcknowledgement),
    ManifestSerializationCompletedNotification(ManifestSerializationCompletedNotification),
    RecompilationRequest(RecompilationRequest),
    RecompilationAcknowledgement(RecompilationAcknowledgement),
    RecompilationCompletedNotification(RecompilationCompletedNotification),
}

/// A notification indicating that a project file has changed.
/// This message is sent from `pax-privileged-agent` to `pax-designtime`.
#[derive(Serialize, Deserialize, Default)]
pub struct FileChangedNotification {}

/// A request to serialize the provided manifest into code.
/// This is sent from `pax-designtime` to `pax-privileged-agent`.
#[derive(Serialize, Deserialize)]
pub struct ManifestSerializationRequest {
    pub manifest: PaxManifest,
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
