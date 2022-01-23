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

#[allow(dead_code)]
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
    pub settings: Option<Vec<SettingsDefinition>>,
}
#[allow(dead_code)]
#[derive(Debug)]
//Represents an entry within a component template, e.g. a <Rectangle> declaration inside a template
pub struct TemplateNodeDefinition {
    pub id: String,
    pub component_id: String,
    pub inline_attributes: Option<Vec<(String, AttributeValue)>>,
    pub children_ids: Vec<String>,
}
#[allow(dead_code)]
#[derive(Debug)]
pub enum AttributeValue {
    String,
    Expression(String),
}

#[derive(Debug)]
pub enum SettingsValue {
    Literal(String),
    Block(SettingsValueBlock),
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SettingsDefinition {
    id: String,
    selector: String,
    value: SettingsValueBlock,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SettingsValueBlock {
    pairs: Option<Vec<(String, SettingsValue)>>,
}






