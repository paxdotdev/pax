use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
pub struct Schema {
    pub abi: u32,
    pub components: Vec<Component>,
}
#[derive(Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub fields: Vec<Field>,
    pub handlers: Vec<Handler>,
}
#[derive(Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub kind: u32,
    pub default: pax_runtime_api::PaxValue,
}
#[derive(Serialize, Deserialize)]
pub struct Handler {
    pub name: String,
    pub event: String,
    pub index: usize,
}
#[derive(Serialize, Deserialize)]
pub struct Computation {
    pub callback: u32,
    pub dependencies: Vec<u32>,
}
#[derive(Serialize, Deserialize)]
pub struct Request {
    pub bounds: u32,
    pub clock: u32,
    pub event: serde_json::Value,
}
#[derive(Serialize, Deserialize)]
pub struct Ease {
    pub value: pax_runtime_api::PaxValue,
    pub duration: pax_runtime_api::Duration,
    pub curve: u32,
    pub append: bool,
}
pub const READ: u32 = 1;
pub const WRITE: u32 = 2;
pub const NEW: u32 = 3;
pub const COMPUTED: u32 = 4;
pub const REPLACE: u32 = 5;
pub const SUBSCRIBE: u32 = 6;
pub const CURSOR: u32 = 8;
pub const RETAIN: u32 = 9;
pub const RELEASE: u32 = 10;
pub const CANCEL: u32 = 11;
pub const EASE: u32 = 12;
pub const HANDLE: u32 = 13;
pub const WORD: u32 = 14;
pub const REQUEST: u32 = 15;
