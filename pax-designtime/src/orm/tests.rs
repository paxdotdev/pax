#[cfg(test)]
mod tests {
    use crate::orm::PaxManifestORM;
    use pax_manifest::{
        ComponentDefinition, LiteralBlockDefinition, PaxManifest, SettingsBlockElement, Token, TypeId,
    };
    use std::collections::{BTreeMap, HashMap};

    fn create_basic_manifest() -> PaxManifest {
        let mut components = BTreeMap::new();
        let type_id: TypeId = TypeId::build_singleton("Component1", Some("Component1"));
        components.insert(
            type_id.clone(),
            ComponentDefinition {
                type_id: type_id.clone(),
                is_main_component: false,
                is_primitive: false,
                is_struct_only_component: false,
                module_path: "module_path1".to_string(),
                primitive_instance_import_path: None,
                template: None,
                settings: Some(vec![SettingsBlockElement::SelectorBlock(
                    Token::new_without_location("existing_selector".to_string()),
                    LiteralBlockDefinition::new(vec![]),
                )]),
            },
        );

        PaxManifest {
            components,
            main_component_type_id: type_id,
            type_table: HashMap::new(),
        }
    }

    #[test]
    fn test_add_and_undo_node() {
        let mut orm = PaxManifestORM::new(create_basic_manifest());
        let type_id: TypeId = TypeId::build_singleton("Component1", Some("Component1"));
        let rectangle_type_id: TypeId = TypeId::build_singleton("Rectangle", Some("Rectangle"));

        // Build and configure a new node
        let mut node_builder = orm.build_new_node(type_id.clone(), rectangle_type_id);

        node_builder.set_property("x", "10px").unwrap();
        node_builder.save().unwrap();

        assert!(orm
            .get_manifest()
            .components
            .get(&type_id)
            .unwrap()
            .template
            .is_some());

        // Undo the creation
        orm.undo().unwrap();

        assert!(orm
            .get_manifest()
            .components
            .get(&type_id)
            .unwrap()
            .template
            .is_some());
    }
}
