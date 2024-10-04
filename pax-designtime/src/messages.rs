use std::fmt::Formatter;

use pax_manifest::{ComponentDefinition, ComponentTemplate, PaxManifest, SettingsBlockElement, TypeId};
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
    ComponentSerializationRequest(ComponentSerializationRequest),
    UpdateTemplateRequest(Box<UpdateTemplateRequest>),
    LoadFileToStaticDirRequest(LoadFileToStaticDirRequest),
    // LLM Requests to pub.pax.dev
    LLMRequest(LLMRequest),
    LLMPartialResponse(LLMPartialResponse),
    LLMFinalResponse(LLMFinalResponse),
}


#[derive(Deserialize, Serialize)]
pub struct LLMRequest {
    pub manifest: PaxManifest,
    pub prompt: String,
    pub request_id: u64,
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
    pub fn new(manifest: PaxManifest, prompt: String, request_id: u64 ) -> Self {
        Self { manifest, prompt, request_id }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LLMPartialResponse {
    pub request_id: u64,
    pub message: String,
}

impl LLMPartialResponse {
    pub fn new(request_id: u64, message: String) -> Self {
        Self { request_id, message }
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
    pub fn new(request_id: u64, message: String, component_definition: ComponentDefinition) -> Self {
        Self { request_id, message, component_definition }
    }
}


#[derive(Serialize, Deserialize, Debug)]
pub struct LoadFileToStaticDirRequest {
    pub name: String,
    pub data: Vec<u8>,
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
