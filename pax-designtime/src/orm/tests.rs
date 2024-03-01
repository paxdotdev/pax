#[cfg(test)]
mod tests {
    use crate::orm::PaxManifestORM;
    use pax_manifest::{
        ComponentDefinition, LiteralBlockDefinition, PaxManifest, SettingsBlockElement, Token,
        TokenType, TypeId,
    };
    use std::collections::{HashMap, HashSet};

    fn create_basic_manifest() -> PaxManifest {
        let mut components = HashMap::new();
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
                    Token::new_from_raw_value("existing_selector".to_string(), TokenType::Selector),
                    LiteralBlockDefinition::new(vec![]),
                )]),
            },
        );

        PaxManifest {
            components,
            main_component_type_id: type_id,
            expression_specs: None,
            type_table: HashMap::new(),
            import_paths: HashSet::new(),
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
