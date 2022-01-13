

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}







pub enum Action {
    Create,
    Read,
    Update,
    Delete
}


#[allow(dead_code)]
pub struct PaxMessage {
    action: Option<Action>,
    payload: Entity 
}

pub enum Entity {
    ComponentDefinition(ComponentDefinition),
    TemplateNodeDefinition(TemplateNodeDefinition),
}

#[allow(dead_code)]
pub struct ComponentDefinition {
    id: String,
    name: String,
    template: TemplateNodeDefinition,
    settings: Option<Vec<SettingsDefinition>>,
}
#[allow(dead_code)]
pub struct TemplateNodeDefinition {
    id: String,
    component_type: String,
    inline_attributes: Option<Vec<(String, AttributeValue)>>,
}
#[allow(dead_code)]
pub enum AttributeValue {
    String,
    Expression(String),
}

pub enum SettingsValue {
    Literal(String),
    Block(SettingsValueBlock),
}

#[allow(dead_code)]
pub struct SettingsDefinition {
    id: String,
    selector: String,
    value: SettingsValueBlock,
}

#[allow(dead_code)]
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