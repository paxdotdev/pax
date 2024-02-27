#[cfg(test)]
mod tests {
    use crate::orm::PaxManifestORM;
    use pax_manifest::{
        ComponentDefinition, HandlerBindingElement, LiteralBlockDefinition, PaxManifest,
        SettingElement, SettingsBlockElement, Token, TokenType, ValueDefinition,
    };
    use std::collections::{HashMap, HashSet};

    fn create_basic_manifest() -> PaxManifest {
        let mut components = HashMap::new();
        components.insert(
            "component1".to_string(),
            ComponentDefinition {
                type_id: "component1".to_string(),
                type_id_escaped: "Component1".to_string(),
                is_main_component: false,
                is_primitive: false,
                is_struct_only_component: false,
                pascal_identifier: "Component1".to_string(),
                module_path: "module_path1".to_string(),
                primitive_instance_import_path: None,
                template: None,
                settings: Some(vec![SettingsBlockElement::SelectorBlock(
                    Token::new_from_raw_value("existing_selector".to_string(), TokenType::Selector),
                    LiteralBlockDefinition::new(vec![]),
                )]),
                handlers: Some(vec![HandlerBindingElement::Handler(
                    Token::new_from_raw_value("existing_handler".to_string(), TokenType::EventId),
                    vec![Token::new_from_raw_value(
                        "handler_action".to_string(),
                        TokenType::Handler,
                    )],
                )]),
                next_template_id: None,
                template_source_file_path: None,
            },
        );

        PaxManifest {
            components,
            main_component_type_id: "component1".to_string(),
            expression_specs: None,
            type_table: HashMap::new(),
            import_paths: HashSet::new(),
        }
    }

    #[test]
    fn test_add_and_undo_node() {
        let mut orm = PaxManifestORM::new(create_basic_manifest());

        // Build and configure a new node
        let mut node_builder = orm.build_new_node(
            "component1".to_string(),
            "type1".to_string(),
            "Node1".to_string(),
            None,
        );
        node_builder.set_property("key", "new_Value").unwrap();
        node_builder.save().unwrap();

        assert!(orm.get_manifest().components["component1"]
            .template
            .is_some());

        // Undo the creation
        orm.undo().unwrap();

        assert!(orm.get_manifest().components["component1"]
            .template
            .is_none());
    }

    #[test]
    fn test_update_and_redo_selector() {
        let mut orm = PaxManifestORM::new(create_basic_manifest());

        // Update an existing selector
        let mut selector_builder =
            orm.get_selector("component1".to_string(), "existing_selector".to_string());
        selector_builder.set_value(LiteralBlockDefinition::new(vec![SettingElement::Setting(
            Token::new_from_raw_value("key".to_string(), TokenType::SettingKey),
            ValueDefinition::LiteralValue(Token::new_from_raw_value(
                "value".to_string(),
                TokenType::LiteralValue,
            )),
        )]));
        selector_builder.save().unwrap();

        let value = if let SettingsBlockElement::SelectorBlock(_, v) = orm.get_manifest().components
            ["component1"]
            .settings
            .as_ref()
            .unwrap()[0]
            .clone()
        {
            v
        } else {
            panic!("Invalid selector type")
        };

        assert_eq!(value.elements.len(), 1);

        // Undo the update
        orm.undo().unwrap();

        let value = if let SettingsBlockElement::SelectorBlock(_, v) = orm.get_manifest().components
            ["component1"]
            .settings
            .as_ref()
            .unwrap()[0]
            .clone()
        {
            v
        } else {
            panic!("Invalid selector type")
        };
        assert_eq!(value.elements.len(), 0);

        // Redo the update
        orm.redo().unwrap();

        let value = if let SettingsBlockElement::SelectorBlock(_, v) = orm.get_manifest().components
            ["component1"]
            .settings
            .as_ref()
            .unwrap()[0]
            .clone()
        {
            v
        } else {
            panic!("Invalid selector type")
        };
        assert_eq!(value.elements.len(), 1);
    }

    #[test]
    fn test_remove_and_redo_handler() {
        let mut orm = PaxManifestORM::new(create_basic_manifest());

        // Remove an existing handler
        orm.remove_handler("component1".to_string(), "existing_handler".to_string())
            .unwrap();

        assert!(orm.get_manifest().components["component1"]
            .handlers
            .is_none());

        // Undo the removal
        orm.undo().unwrap();

        assert!(orm.get_manifest().components["component1"]
            .handlers
            .is_some());

        // Redo the removal
        orm.redo().unwrap();

        assert!(orm.get_manifest().components["component1"]
            .handlers
            .is_none());
    }

    #[test]
    fn test_undo_until_specific_command() {
        let mut orm = PaxManifestORM::new(create_basic_manifest());

        // Perform multiple actions
        orm.build_new_node(
            "component1".to_string(),
            "type2".to_string(),
            "Node2".to_string(),
            None,
        )
        .save()
        .unwrap();
        orm.build_new_selector(
            "component1".to_string(),
            "new_selector".to_string(),
            LiteralBlockDefinition::new(vec![]),
        )
        .save()
        .unwrap();
        orm.build_new_handler("component1".to_string(), "new_handler".to_string())
            .save()
            .unwrap();

        assert_eq!(
            orm.get_manifest().components["component1"]
                .template
                .as_ref()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            orm.get_manifest().components["component1"]
                .settings
                .as_ref()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            orm.get_manifest().components["component1"]
                .handlers
                .as_ref()
                .unwrap()
                .len(),
            2
        );

        // Undo until the first command (node creation)
        orm.undo_until(0).unwrap();

        assert_eq!(
            orm.get_manifest().components["component1"]
                .template
                .as_ref()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            orm.get_manifest().components["component1"]
                .settings
                .as_ref()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            orm.get_manifest().components["component1"]
                .handlers
                .as_ref()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn test_complex_workflow_with_builders_and_undo_redo() {
        let mut orm = PaxManifestORM::new(create_basic_manifest());

        // Perform complex workflow with builders
        let mut node = orm.build_new_node(
            "component1".to_string(),
            "type3".to_string(),
            "Node3".to_string(),
            None,
        );
        node.set_property("key", "newValue").unwrap();
        node.save().unwrap();

        orm.build_new_selector(
            "component1".to_string(),
            "another_selector".to_string(),
            LiteralBlockDefinition::new(vec![]),
        )
        .save()
        .unwrap();
        orm.build_new_handler("component1".to_string(), "another_handler".to_string())
            .save()
            .unwrap();

        // Perform multiple undo and redo operations
        orm.undo().unwrap();
        orm.undo().unwrap();
        orm.redo().unwrap();
        orm.redo().unwrap();

        // Assert final state
        assert!(orm.get_manifest().components["component1"]
            .template
            .is_some());
        assert_eq!(
            orm.get_manifest().components["component1"]
                .settings
                .as_ref()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            orm.get_manifest().components["component1"]
                .handlers
                .as_ref()
                .unwrap()
                .len(),
            2
        );
    }
}
