use crate::parsing::Reflectable;
use crate::{PaxManifest, TypeId};
use pax_runtime_api::{CoercionRules, HelperFunctions, Interpolatable, PaxValue, ToPaxValue};
use serde::{Deserialize, Serialize};
use std::cell::Ref;

#[derive(Serialize, Deserialize)]
pub struct PublishRequest {
    pub manifest: PaxManifest,
    //github_username: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PublishResponseSuccess {
    pub pull_request_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum PublishResponse {
    #[default]
    Undefined,
    Success(PublishResponseSuccess),
    Error(ResponseError),
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ResponseError {
    pub message: String,
}

impl Interpolatable for PublishResponse {}
impl Interpolatable for PublishResponseSuccess {}
impl Interpolatable for ResponseError {}

impl ToPaxValue for PublishResponse {
    fn to_pax_value(self) -> PaxValue {
        match self {
            PublishResponse::Success(success) => PaxValue::Enum(Box::new((
                "PublishResponse".to_string(),
                "Success".to_string(),
                vec![success.clone().to_pax_value()],
            ))),
            PublishResponse::Error(error) => PaxValue::Enum(Box::new((
                "PublishResponse".to_string(),
                "Error".to_string(),
                vec![error.clone().to_pax_value()],
            ))),
            PublishResponse::Undefined => PaxValue::Enum(Box::new((
                "PublishResponse".to_string(),
                "Undefined".to_string(),
                vec![],
            ))),
        }
    }
}

impl ToPaxValue for PublishResponseSuccess {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Enum(Box::new((
            "".to_string(),
            "PublishResponseSuccess".to_string(),
            vec![self.pull_request_url.clone().to_pax_value()],
        )))
    }
}

impl ToPaxValue for ResponseError {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Enum(Box::new((
            "".to_string(),
            "ResponseError".to_string(),
            vec![self.message.clone().to_pax_value()],
        )))
    }
}

impl CoercionRules for PublishResponse {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        match value {
            PaxValue::Enum(contents) => {
                let (_, variant, values) = *contents;
                match variant.as_str() {
                    "Success" => {
                        let success = PublishResponseSuccess::try_coerce(values[0].clone())?;
                        Ok(PublishResponse::Success(success))
                    }
                    "Error" => {
                        let error = ResponseError::try_coerce(values[0].clone())?;
                        Ok(PublishResponse::Error(error))
                    }
                    _ => Err(format!("Invalid variant: {}", variant)),
                }
            }
            _ => Err("Invalid PaxValue".to_string()),
        }
    }
}

impl CoercionRules for PublishResponseSuccess {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        match value {
            PaxValue::Enum(contents) => {
                let (_, variant, values) = *contents;
                match variant.as_str() {
                    "PublishResponseSuccess" => {
                        let pull_request_url = String::try_coerce(values[0].clone())?;
                        Ok(PublishResponseSuccess { pull_request_url })
                    }
                    _ => Err(format!("Invalid variant: {}", variant)),
                }
            }
            _ => Err("Invalid PaxValue".to_string()),
        }
    }
}

impl CoercionRules for ResponseError {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        match value {
            PaxValue::Enum(contents) => {
                let (_, variant, values) = *contents;
                match variant.as_str() {
                    "ResponseError" => {
                        let message = String::try_coerce(values[0].clone())?;
                        Ok(ResponseError { message })
                    }
                    _ => Err(format!("Invalid variant: {}", variant)),
                }
            }
            _ => Err("Invalid PaxValue".to_string()),
        }
    }
}

impl Reflectable for PublishResponse {
    fn get_self_pascal_identifier() -> String {
        "PublishResponse".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            "pax_manifest::server::PublishResponse",
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for PublishResponseSuccess {
    fn get_self_pascal_identifier() -> String {
        "PublishResponseSuccess".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            "pax_manifest::server::PublishResponseSuccess",
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for ResponseError {
    fn get_self_pascal_identifier() -> String {
        "ResponseError".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            "pax_manifest::server::ResponseError",
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl HelperFunctions for PublishResponseSuccess {}
impl HelperFunctions for PublishResponse {}
impl HelperFunctions for ResponseError {}
