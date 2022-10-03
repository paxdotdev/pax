
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::{fs, env};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use pest::iterators::{Pair, Pairs};

use uuid::Uuid;
use pest::Parser;

use serde_derive::{Serialize, Deserialize};
use serde_json;


//definition container for an entire Pax cartridge
#[derive(Serialize, Deserialize)]
pub struct PaxManifest {
    pub components: Vec<ComponentDefinition>,
    pub root_component_id: String,
}



// FOR BINCODE:
// these methods are exposed to encapsulate the serialization method/version (at time of writing: bincode 1.3.3)
// though that particular version isn't important, this prevents consuming libraries from having to
// coordinate versions/strategies for serialization
// impl PaxManifest {
//     pub fn serialize(&self) -> Vec<u8> {
//         serialize(&self).unwrap()
//     }
//
//     pub fn deserialize(bytes: &[u8]) -> Self {
//         deserialize(bytes).unwrap()
//     }
// }


#[derive(Serialize, Deserialize, Debug)]
pub struct ComponentDefinition {
    pub source_id: String,
    pub pascal_identifier: String,
    pub module_path: String,
    //optional not because it cannot exist, but because
    //there are times in this data structure's lifecycle when it
    //is not yet known
    pub root_template_node_id: Option<String>,
    pub template: Option<Vec<TemplateNodeDefinition>>,
    //can be hydrated as a tree via child_ids/parent_id
    pub settings: Option<Vec<SettingsSelectorBlockDefinition>>,
    pub property_definitions: Vec<PropertyDefinition>,
}

#[derive(Serialize, Deserialize, Debug)]
//Represents an entry within a component template, e.g. a <Rectangle> declaration inside a template
pub struct TemplateNodeDefinition {
    pub id: String,
    pub component_id: String,
    pub inline_attributes: Option<Vec<(String, AttributeValueDefinition)>>,
    pub children_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PropertyDefinition {
    /// String representation of the identifier of a declared Property
    pub name: String,
    /// Type as authored, literally.  May be partially namespace-qualified or aliased.
    pub original_type: String,
    /// Vec of constituent components of a type, for example `Rc<String>` would have the dependencies [`std::rc::Rc` and `std::string::String`]
    pub fully_qualified_dependencies: Vec<String>,
    /// Same type as `original_type`, but dynamically normalized to be fully qualified, suitable for reexporting
    pub fully_qualified_type: String,

    /// Same as fully qualified type, but Pascalized to make a suitable enum identifier
    pub pascalized_fully_qualified_type: String,
    //pub default_value ?
}

#[derive(Serialize, Deserialize, Debug)]
pub enum AttributeValueDefinition {
    LiteralValue(String),
    Expression(String),
    Identifier(String),
    EventBindingTarget(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SettingsSelectorBlockDefinition {
    pub selector: String,
    pub value_block: SettingsLiteralBlockDefinition,
    //TODO: think through this recursive data structure and de/serialization.
    //      might need to normalize it, keeping a tree of `SettingsLiteralBlockDefinition`s
    //      where nodes are flattened into a list.
    //     First: DO we need to normalize it?  Will something like Serde magically fix this?
    //     It's possible that it will.  Revisit only if we have trouble serializing this data.
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SettingsLiteralBlockDefinition {
    pub explicit_type_pascal_identifier: Option<String>,
    pub settings_key_value_pairs: Vec<(String, SettingsValueDefinition)>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum SettingsValueDefinition {
    Literal(SettingsLiteralValue),
    Expression(String),
    Enum(String),
    Block(SettingsLiteralBlockDefinition),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum SettingsLiteralValue {
    LiteralNumberWithUnit(Number, Unit),
    LiteralNumber(Number),
    LiteralArray(Vec<SettingsLiteralValue>),
    String(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Number {
    Float(f64),
    Int(isize)
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Unit {
    Pixels,
    Percent
}

//
//
// pub enum SettingsValue {
//     Literal(String),
//     Block(SettingsValueBlock),
// }
//
// #[allow(dead_code)]
// pub struct SettingsDefinition {
//     id: String,
//     selector: String,
//     value: SettingsValueBlock,
// }
//
// #[allow(dead_code)]
// pub struct SettingsValueBlock {
//     pairs: Option<Vec<(String, SettingsValue)>>,
// }

// use message::{AttributeValueDefinition, ComponentDefinition, Number, PaxManifest, SettingsLiteralBlockDefinition, SettingsSelectorBlockDefinition, SettingsValueDefinition, SettingsLiteralValue, TemplateNodeDefinition, Unit};
// use pest::prec_climber::PrecClimber;
