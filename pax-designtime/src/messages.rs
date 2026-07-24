use std::fmt::Formatter;

use pax_manifest::{
    ComponentDefinition, ComponentTemplate, PaxManifest, SettingsBlockElement, TimelineDefinition,
    TypeId,
};
use pax_message::ScreenshotData;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Serialize, Deserialize, Debug)]
pub enum AgentMessage {
    ProjectFileChangedNotification(FileChangedNotification),
    ManifestSerializationRequest(ManifestSerializationRequest),
    // Request to retrieve the manifest from the design server
    // sent from designtime to design-server
    LoadManifestRequest(LoadManifestRequest),
    LoadManifestResponse(LoadManifestResponse),
    DisconnectNotification(DisconnectNotification),
    ComponentSerializationRequest(ComponentSerializationRequest),
    UpdateTemplateRequest(Box<UpdateTemplateRequest>),
    LoadFileToStaticDirRequest(LoadFileToStaticDirRequest),
    UserlandSourceUpdateRequest(UserlandSourceUpdateRequest),
    UserlandSourceUpdateResponse(UserlandSourceUpdateResponse),
    PrepareAppRevision(PrepareAppRevision),
    RequestAppRevisionActivation(ActivateAppRevision),
    CancelAppRevisionActivation(ActivateAppRevision),
    AppRevisionActivationPrepared(LoadManifestResponse),
    ActivateAppRevision(ActivateAppRevision),
    AppRevisionActivationRejected(AppRevisionActivationRejected),
    DevClientRequest(DevClientRequest),
    DevClientResponse(DevClientResponse),
    // LLM Requests to pub.pax.dev
    LLMRequest(LLMRequest),
    LLMPartialResponse(LLMPartialResponse),
    LLMFinalResponse(LLMFinalResponse),
}

/// Identifies the executable logic generation and the mutable template snapshot
/// mounted on top of it. Template-only edits advance `template_version` without
/// requiring a new executable artifact.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RevisionStamp {
    pub logic_revision_id: String,
    pub template_version: u64,
}

/// Describes how a prepared logic revision will execute after host activation.
/// The protocol deliberately does not encode chassis-specific loading details.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DebugLogicExecutionMode {
    CompiledArtifact,
    InterpretedModule,
}

/// Host-resolvable debug artifact metadata. `kind` remains open-ended so a
/// future language runtime can introduce an artifact without changing the
/// revision state machine.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DebugArtifact {
    pub kind: String,
    pub location: String,
}

/// Announces a complete candidate revision which the host may prepare and
/// activate. The active manifest remains authoritative until activation.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PrepareAppRevision {
    pub logic_revision_id: String,
    pub execution_mode: DebugLogicExecutionMode,
    pub artifact: DebugArtifact,
}

/// Identifies a candidate whose prepared snapshot the host admitted. The final
/// activation message requests server commit before the host swaps runtimes.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ActivateAppRevision {
    pub logic_revision_id: String,
}

/// Rejects an activation acknowledgement which is no longer authoritative.
/// The mounted runtime remains intact and may receive a newer prepare envelope
/// or a full snapshot for its existing logic identity.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AppRevisionActivationRejected {
    pub logic_revision_id: String,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DevClientRequest {
    Look(DevClientLookRequest),
    InspectTree(DevClientInspectTreeRequest),
    RayCast(DevClientRayCastRequest),
    SelectorQuery(DevClientSelectorQueryRequest),
    ReplaceNode(DevClientReplaceNodeRequest),
    Logs(DevClientLogsRequest),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DevClientResponse {
    Look(DevClientLookResponse),
    InspectTree(DevClientInspectTreeResponse),
    RayCast(DevClientRayCastResponse),
    SelectorQuery(DevClientSelectorQueryResponse),
    ReplaceNode(DevClientReplaceNodeResponse),
    Logs(DevClientLogsResponse),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevClientLookRequest {
    pub request_id: String,
    pub scale: f64,
    pub period_ms: u64,
    pub duration_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevClientRawCapture {
    pub rgba_bytes: Vec<u8>,
    pub compression: Option<String>,
    pub width: usize,
    pub height: usize,
    pub captured_at_ms: u128,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevClientLookResponse {
    pub request_id: String,
    pub status: String,
    pub captures: Vec<DevClientRawCapture>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevClientInspectTreeRequest {
    pub request_id: String,
    pub max_depth: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevClientInspectTreeResponse {
    pub request_id: String,
    pub status: String,
    pub node_count: Option<usize>,
    pub tree_json: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevClientRayCastRequest {
    pub request_id: String,
    pub x: f64,
    pub y: f64,
    pub hit_invisible: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevClientRayCastResponse {
    pub request_id: String,
    pub status: String,
    pub x: f64,
    pub y: f64,
    pub hit_invisible: bool,
    pub node_count: Option<usize>,
    pub nodes_json: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevClientSelectorQueryRequest {
    pub request_id: String,
    pub selector: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevClientSelectorQueryResponse {
    pub request_id: String,
    pub status: String,
    pub selector: String,
    pub node_count: Option<usize>,
    pub nodes_json: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevClientReplaceNodeRequest {
    pub request_id: String,
    pub component_type_id: String,
    pub template_node_id: usize,
    pub subtemplate: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevClientReplaceNodeResponse {
    pub request_id: String,
    pub status: String,
    pub component_type_id: String,
    pub template_node_id: usize,
    pub reload_scope: String,
    pub reloaded_template_node_id: Option<usize>,
    pub source_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevClientLogsRequest {
    pub request_id: String,
    pub since_seq: Option<u64>,
    pub limit: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevClientLogEntry {
    pub seq: u64,
    pub level: String,
    pub message: String,
    pub timestamp_ms: u128,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevClientLogsResponse {
    pub request_id: String,
    pub status: String,
    pub entries: Vec<DevClientLogEntry>,
    pub next_seq: u64,
    pub oldest_seq: Option<u64>,
    pub error: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct LLMRequest {
    pub manifest: PaxManifest,
    pub prompt: String,
    pub request_id: u64,
    pub screenshot: Option<ScreenshotData>,
}

impl Debug for LLMRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LLMRequest")
            .field("prompt", &self.prompt)
            .field("request_id", &self.request_id)
            .finish()
    }
}

impl LLMRequest {
    pub fn new(
        manifest: PaxManifest,
        prompt: String,
        request_id: u64,
        screenshot: Option<ScreenshotData>,
    ) -> Self {
        Self {
            manifest,
            prompt,
            request_id,
            screenshot,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LLMPartialResponse {
    pub request_id: u64,
    pub message: String,
}

impl LLMPartialResponse {
    pub fn new(request_id: u64, message: String) -> Self {
        Self {
            request_id,
            message,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct LLMFinalResponse {
    pub request_id: u64,
    pub message: String,
    pub component_definition: ComponentDefinition,
}

impl Debug for LLMFinalResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LLMFinalResponse")
            .field("request_id", &self.request_id)
            .field("message", &self.message)
            .finish()
    }
}

impl LLMFinalResponse {
    pub fn new(
        request_id: u64,
        message: String,
        component_definition: ComponentDefinition,
    ) -> Self {
        Self {
            request_id,
            message,
            component_definition,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoadFileToStaticDirRequest {
    pub name: String,
    pub data: Vec<u8>,
}

/// A request from userland to replace a project source file during designtime.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserlandSourceUpdateRequest {
    pub request_id: String,
    pub path: String,
    pub contents: String,
}

/// A response to a userland source replacement request.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserlandSourceUpdateResponse {
    pub request_id: String,
    pub path: String,
    pub status: String,
    pub error: Option<String>,
}

/// A notification indicating that a project file has changed.
/// This message is sent from `pax-design-server` to `pax-designtime`.
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct FileChangedNotification {}

/// A request to serialize the provided manifest into code.
/// This is sent from `pax-designtime` to `pax-design-server`.
#[derive(Serialize, Deserialize, Debug)]
pub struct ManifestSerializationRequest {
    pub manifest: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct LoadManifestRequest {
    pub active_revision: Option<RevisionStamp>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoadManifestResponse {
    pub revision: RevisionStamp,
    pub manifest: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DisconnectNotification {
    pub allow_reconnect: bool,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ComponentSerializationRequest {
    pub component_bytes: Vec<u8>,
}

/// A request to update the Pax-authored definition of a component.
// Sent from `pax-priviliged-agent` to `pax-designtime`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateTemplateRequest {
    /// Revision against which this template was parsed and committed.
    pub revision: RevisionStamp,
    /// The type identifier of the component to update.
    pub type_id: TypeId,
    /// The new template for the component.
    pub new_template: ComponentTemplate,
    /// The settings block for the component.
    pub settings_block: Vec<SettingsBlockElement>,
    /// The named timeline blocks for the component.
    #[serde(default)]
    pub timelines: Vec<TimelineDefinition>,
}
