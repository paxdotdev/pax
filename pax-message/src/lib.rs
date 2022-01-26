use std::path::Component;
use serde::{Serialize, Deserialize};
use bincode::{serialize, deserialize};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}

//definition container for an entire Pax cartridge
#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct PaxManifest {
    pub components: Vec<ComponentDefinition>,
    pub root_component_id: String,
}

//this method is exposed to encapsulate the serialization method/version (at time of writing: bincode 1.3.3)
//though that particular version isn't important, this prevents consuming libraries from having to
//coordinate versions/strategies for serialization
impl PaxManifest {
    pub fn serialize(&self) -> Vec<u8> {
        serialize(&self).unwrap()
    }
}
//
// pub enum Action {
//     Create,
//     Read,
//     Update,
//     Delete,
//     Command,
// }
//
// #[allow(dead_code)]
// pub struct PaxMessage {
//     pub action: Action,
//     pub payload: Entity,
// }
//
// pub enum Entity {
//     ComponentDefinition(ComponentDefinition),
//     TemplateNodeDefinition(TemplateNodeDefinition),
//     CommandDefinitionTODO,
// }

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct ComponentDefinition {
    pub id: String,
    pub pascal_identifier: String,
    pub module_path: String,
    //optional not because it cannot exist, but because
    //there are times in this data structure's lifecycle when it
    //is not yet known
    pub root_template_node_id: Option<String>,
    pub template: Option<Vec<TemplateNodeDefinition>>, //can be hydrated as a tree via child_ids/parent_id
    pub settings: Option<Vec<SettingsSelectorBlockDefinition>>,
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
//Represents an entry within a component template, e.g. a <Rectangle> declaration inside a template
pub struct TemplateNodeDefinition {
    pub id: String,
    pub component_id: String,
    pub inline_attributes: Option<Vec<(String, AttributeValueDefinition)>>,
    pub children_ids: Vec<String>,
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub enum AttributeValueDefinition {
    String(String),
    Expression(String),
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct SettingsSelectorBlockDefinition {
    pub selector: String,
    pub value_block: SettingsLiteralBlockDefinition,
    //TODO: think through this recursive data structure and de/serialization.
    //      might need to normalize it, keeping a tree of `SettingsLiteralBlockDefinition`s
    //      where nodes are flattened into a list.
    //     First: DO we need to normalize it?  Will something like Serde magically fix this?
    //     It's possible that it will.  Revisit only if we have trouble serializing this data.
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct SettingsLiteralBlockDefinition {
    pub explicit_type_pascal_identifier: Option<String>,
    pub settings_key_value_pairs: Vec<(String, SettingsValueDefinition)>,
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub enum SettingsValueDefinition {
    Literal(SettingsLiteralValue),
    Expression(String),
    Enum(String),
    Block(SettingsLiteralBlockDefinition),
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub enum SettingsLiteralValue {
    LiteralNumberWithUnit(Number, Unit),
    LiteralNumber(Number),
    LiteralArray(Vec<SettingsLiteralValue>),
    String(String),
}

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub enum Number {
    Float(f64),
    Int(i64)
}

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub enum Unit {
    Pixels,
    Percent
}

//
//
// #[derive(Debug)]
// pub enum SettingsValue {
//     Literal(String),
//     Block(SettingsValueBlock),
// }
//
// #[allow(dead_code)]
// #[derive(Debug)]
// pub struct SettingsDefinition {
//     id: String,
//     selector: String,
//     value: SettingsValueBlock,
// }
//
// #[allow(dead_code)]
// #[derive(Debug)]
// pub struct SettingsValueBlock {
//     pairs: Option<Vec<(String, SettingsValue)>>,
// }






