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
    pub name: String,
    pub template: Option<Vec<TemplateNodeDefinition>>, //can be hydrated as a tree via child_ids/parent_id
    pub settings: Option<Vec<SettingsDefinition>>,
}
#[allow(dead_code)]
#[derive(Debug)]
pub struct TemplateNodeDefinition {
    id: String,
    component_type: String,
    inline_attributes: Option<Vec<(String, AttributeValue)>>,
    parent_id: String, //maybe only one of parent/children id is necessary.
    children_ids: Vec<String>,
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









pub struct DefinitionOfProperty {
    property_name: String,
    type_name: String,
}

pub struct DefinitionOfComponent {
    component_id: String,
    component_name: String,
    component_tree: DefinitionOfComponentTemplateInstance,
    component_properties: Vec<DefinitionOfProperty>,
    descendant_settings: Vec<DefinitionOfSettings>,
}

pub struct DefinitionOfComponentTemplateInstance {
    component_id: String,
    instance_id: String,
    instance_class: String,
    children: Vec<DefinitionOfComponentTemplateInstance>,
}

pub struct DefinitionOfSettings {
    selector: String,
    property_pairs: PropertyCoproduct,
}

pub struct PropertyCoproduct {} //TODO: patch this with codegen