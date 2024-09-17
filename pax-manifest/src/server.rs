use serde::{Deserialize, Serialize};
use pax_runtime_api::{Interpolatable, PaxValue, ToPaxValue};
use crate::PaxManifest;

#[derive(Deserialize, Serialize)]
pub struct LLMRequest {
    pub manifest: PaxManifest,
    pub prompt: String,
}

impl LLMRequest {
    pub fn new(manifest: PaxManifest, prompt: String) -> Self {
        Self { manifest, prompt }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PublishRequest {
    pub manifest: PaxManifest,
    //github_username: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PublishResponseSuccess {
    pub pull_request_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PublishResponse {
    Success(PublishResponseSuccess),
    Error(ResponseError),
}

impl Interpolatable for PublishResponse {}
impl Interpolatable for PublishResponseSuccess {}
impl Interpolatable for ResponseError {}


impl ToPaxValue for PublishResponse {
    fn to_pax_value(&self) -> PaxValue {
        match self {
            PublishResponse::Success(success) => {
                PaxValue::Enum("PublishResponse".to_string(), "Success".to_string(), vec![ success.to_pax_value()])
            }
            PublishResponse::Error(error) => {
                PaxValue::Struct(vec![("Error".to_string(), error.to_pax_value())])
            }
        }
    }
}

impl ToPaxValue for PublishResponseSuccess {
    fn to_pax_value(&self) -> PaxValue {
        PaxValue::Struct(vec![("pull_request_url".to_string(), self.pull_request_url.to_pax_value())])
    }
}

impl ToPaxValue for ResponseError {
    fn to_pax_value(&self) -> PaxValue {
        PaxValue::Struct(vec![("message".to_string(), self.message.to_pax_value())])
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResponseError {
    pub message: String,
}

