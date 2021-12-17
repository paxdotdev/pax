#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
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