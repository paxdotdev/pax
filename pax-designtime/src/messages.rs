use std::fmt::Formatter;

use pax_manifest::{
    ComponentDefinition, ComponentTemplate, PaxManifest, SettingsBlockElement, TypeId,
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
    LoadManifestRequest,
    LoadManifestResponse(LoadManifestResponse),
    DisconnectNotification(DisconnectNotification),
    ComponentSerializationRequest(ComponentSerializationRequest),
    UpdateTemplateRequest(Box<UpdateTemplateRequest>),
    LoadFileToStaticDirRequest(LoadFileToStaticDirRequest),
    UserlandSourceUpdateRequest(UserlandSourceUpdateRequest),
    UserlandSourceUpdateResponse(UserlandSourceUpdateResponse),
    ReloadAppRequest(ReloadAppRequest),
    DevClientRequest(DevClientRequest),
    DevClientResponse(DevClientResponse),
    // LLM Requests to pub.pax.dev
    LLMRequest(LLMRequest),
    LLMPartialResponse(LLMPartialResponse),
    LLMFinalResponse(LLMFinalResponse),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReloadAppRequest {
    pub request_id: String,
    pub build_id: String,
    pub artifact_kind: String,
    pub artifact_location: String,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct LoadManifestRequest {}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoadManifestResponse {
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

/// A request to update the template of a component.
// Sent from `pax-priviliged-agent` to `pax-designtime`.
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateTemplateRequest {
    /// The type identifier of the component to update.
    pub type_id: TypeId,
    /// The new template for the component.
    pub new_template: ComponentTemplate,
    /// The settings block for the component.
    pub settings_block: Vec<SettingsBlockElement>,
}
