// Fixture package metadata. This is not a stable public cartridge format.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Schema {
    pub abi: u32,
    pub component: String,
    pub fields: Vec<Field>,
    pub handlers: Vec<Handler>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub name: String,
    pub kind: String,
    pub import: Option<String>,
    pub default: u32,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Handler {
    pub name: String,
    pub kind: String,
    pub opcode: u32,
}
