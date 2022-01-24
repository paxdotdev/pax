use std::path::Component;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}



//definition container for an entire Pax cartridge
pub struct PaxManifest {
    pub components: Vec<ComponentDefinition>,
    pub root_component_id: String,
}

pub enum Action {
    Create,
    Read,
    Update,
    Delete,
    Command,
}

#[allow(dead_code)]
pub struct PaxMessage {
    pub action: Action,
    pub payload: Entity,
}

pub enum Entity {
    ComponentDefinition(ComponentDefinition),
    TemplateNodeDefinition(TemplateNodeDefinition),
    CommandDefinitionTODO,
}

#[derive(Debug)]
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
//Represents an entry within a component template, e.g. a <Rectangle> declaration inside a template
pub struct TemplateNodeDefinition {
    pub id: String,
    pub component_id: String,
    pub inline_attributes: Option<Vec<(String, AttributeValueDefinition)>>,
    pub children_ids: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum AttributeValueDefinition {
    String,
    Expression(String),
}

#[derive(Debug)]
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
pub struct SettingsLiteralBlockDefinition {
    pub explicit_type_pascal_identifier: Option<String>,
    pub settings_key_value_pairs: Vec<(String, SettingsValueDefinition)>,
}

#[derive(Debug)]
pub enum SettingsValueDefinition {
    Literal(String),
    Expression(String),
    Enum(String),
    Block(SettingsLiteralBlockDefinition),
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






