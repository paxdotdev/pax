#[cfg(test)]
mod tests {
    use crate::orm::{
        settings::{
            builder::SelectorBuilder, AddSelectorRequest, GetAllSelectorsRequest,
            GetSelectorRequest, RemoveSelectorRequest, UpdateSelectorRequest,
        },
        Command, PaxManifestORM, UndoRedo,
    };
    use pax_manifest::{
        get_primitive_type_table, ComponentDefinition, LiteralBlockDefinition, PaxManifest,
        SettingElement, SettingsBlockElement, Token, TokenType,
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
                handlers: None,
                next_template_id: None,
            },
        );

        PaxManifest {
            components,
            main_component_type_id: "component1".to_string(),
            expression_specs: None,
            type_table: get_primitive_type_table(),
            import_paths: HashSet::new(),
        }
    }
    #[test]
    fn test_add_selector_command() {
        let mut manifest = create_basic_manifest();
        let mut orm = PaxManifestORM::new(manifest);

        // Creating a new selector with a specific value
        let new_selector_value = LiteralBlockDefinition {
            elements: vec![SettingElement::Comment("New Selector".to_string())],
            explicit_type_pascal_identifier: None,
        };

        let request = AddSelectorRequest {
            component_type_id: "component1".to_string(),
            selector_index: None,
            key: "new_selector".to_string(),
            value: new_selector_value.clone(),
            cached_selector: None,
        };

        // Execute
        let mut command = request;
        command.execute(&mut orm.manifest).unwrap();

        // Check creation
        assert_eq!(
            *orm.manifest.components["component1"]
                .settings
                .as_ref()
                .unwrap(),
            vec![
                SettingsBlockElement::SelectorBlock(
                    Token::new_from_raw_value("existing_selector".to_string(), TokenType::Selector),
                    LiteralBlockDefinition::new(vec![]),
                ),
                SettingsBlockElement::SelectorBlock(
                    Token::new_from_raw_value("new_selector".to_string(), TokenType::Selector),
                    new_selector_value,
                )
            ]
        );

        // Undo
        command.undo(&mut orm.manifest).unwrap();
        assert_eq!(
            *orm.manifest.components["component1"]
                .settings
                .as_ref()
                .unwrap(),
            vec![SettingsBlockElement::SelectorBlock(
                Token::new_from_raw_value("existing_selector".to_string(), TokenType::Selector),
                LiteralBlockDefinition::new(vec![]),
            )]
        );

        // Redo
        command.redo(&mut orm.manifest).unwrap();
        assert_eq!(
            orm.manifest.components["component1"]
                .settings
                .as_ref()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn test_update_selector_command() {
        let manifest = create_basic_manifest();
        let mut orm = PaxManifestORM::new(manifest);

        let updated_selector_value = LiteralBlockDefinition {
            elements: vec![SettingElement::Comment("Updated Selector".to_string())],
            explicit_type_pascal_identifier: None,
        };

        let request = UpdateSelectorRequest {
            component_type_id: "component1".to_string(),
            new_index: None,
            key: "existing_selector".to_string(),
            value: updated_selector_value.clone(),
            cached_prev_state: None,
            cached_prev_position: None,
        };

        // Execute
        let mut command = request;
        command.execute(&mut orm.manifest).unwrap();

        // Check update
        assert_eq!(
            *orm.manifest.components["component1"]
                .settings
                .as_ref()
                .unwrap(),
            vec![SettingsBlockElement::SelectorBlock(
                Token::new_from_raw_value("existing_selector".to_string(), TokenType::Selector),
                updated_selector_value.clone(),
            )]
        );

        // Undo
        command.undo(&mut orm.manifest).unwrap();

        assert_eq!(
            *orm.manifest.components["component1"]
                .settings
                .as_ref()
                .unwrap(),
            vec![SettingsBlockElement::SelectorBlock(
                Token::new_from_raw_value("existing_selector".to_string(), TokenType::Selector),
                LiteralBlockDefinition {
                    elements: vec![],
                    explicit_type_pascal_identifier: None
                },
            )]
        );

        // Redo
        command.redo(&mut orm.manifest).unwrap();

        assert_eq!(
            *orm.manifest.components["component1"]
                .settings
                .as_ref()
                .unwrap(),
            vec![SettingsBlockElement::SelectorBlock(
                Token::new_from_raw_value("existing_selector".to_string(), TokenType::Selector),
                updated_selector_value,
            )]
        );
    }

    #[test]
    fn test_remove_selector_command() {
        let manifest = create_basic_manifest();
        let mut orm = PaxManifestORM::new(manifest);

        let request = RemoveSelectorRequest {
            component_type_id: "component1".to_string(),
            key: "existing_selector".to_string(),
            cached_prev_state: None,
            cached_prev_position: None,
        };

        // Execute
        let mut command = request;
        command.execute(&mut orm.manifest).unwrap();

        // Check removal
        assert!(orm.manifest.components["component1"].settings.is_none());

        // Undo
        command.undo(&mut orm.manifest).unwrap();

        assert_eq!(
            *orm.manifest.components["component1"]
                .settings
                .as_ref()
                .unwrap(),
            vec![SettingsBlockElement::SelectorBlock(
                Token::new_from_raw_value("existing_selector".to_string(), TokenType::Selector),
                LiteralBlockDefinition {
                    elements: vec![],
                    explicit_type_pascal_identifier: None
                },
            )]
        );

        // Redo
        command.redo(&mut orm.manifest).unwrap();

        assert!(orm.manifest.components["component1"].settings.is_none());
    }

    #[test]
    fn test_get_selector_command() {
        let manifest = create_basic_manifest();
        let mut orm = PaxManifestORM::new(manifest);

        // Prepare and execute the GetSelectorRequest
        let request = GetSelectorRequest {
            component_type_id: "component1".to_string(),
            key: "existing_selector".to_string(),
        };
        let mut command = request;
        let response = command.execute(&mut orm.manifest).unwrap();

        // Check retrieval
        assert_eq!(
            response.selector,
            Some(SettingsBlockElement::SelectorBlock(
                Token::new_from_raw_value("existing_selector".to_string(), TokenType::Selector),
                LiteralBlockDefinition {
                    elements: vec![],
                    explicit_type_pascal_identifier: None
                }
            ))
        );
    }

    #[test]
    fn test_get_all_selectors_command() {
        let mut manifest = create_basic_manifest();
        let mut orm = PaxManifestORM::new(manifest);

        // Add a second selector to the manifest
        let second_selector_value = LiteralBlockDefinition::new(vec![SettingElement::Comment(
            "Second Selector".to_string(),
        )]);

        orm.manifest
            .components
            .get_mut("component1")
            .unwrap()
            .settings
            .as_mut()
            .unwrap()
            .push(SettingsBlockElement::SelectorBlock(
                Token::new_from_raw_value("second_selector".to_string(), TokenType::Selector),
                second_selector_value,
            ));

        // Prepare and execute the GetAllSelectorsRequest
        let request = GetAllSelectorsRequest {
            component_type_id: "component1".to_string(),
        };
        let mut command = request;
        let response = command.execute(&mut orm.manifest).unwrap();

        // Check retrieval
        assert_eq!(response.selectors.as_ref().unwrap().len(), 2);
        assert_eq!(
            response.selectors.as_ref().unwrap()[0],
            SettingsBlockElement::SelectorBlock(
                Token::new_from_raw_value("existing_selector".to_string(), TokenType::Selector),
                LiteralBlockDefinition {
                    elements: vec![],
                    explicit_type_pascal_identifier: None
                }
            )
        );
        assert_eq!(
            response.selectors.as_ref().unwrap()[1],
            SettingsBlockElement::SelectorBlock(
                Token::new_from_raw_value("second_selector".to_string(), TokenType::Selector),
                LiteralBlockDefinition::new(vec![SettingElement::Comment(
                    "Second Selector".to_string()
                )])
            )
        );
    }

    #[test]
    fn test_selector_builder_add() {
        let mut orm = PaxManifestORM::new(create_basic_manifest());

        // Create a new selector using SelectorBuilder
        let builder = orm.build_new_selector(
            "component1".to_string(),
            "new_selector".to_string(),
            LiteralBlockDefinition::new(vec![]),
        );
        builder.save().unwrap();

        // Verify that the new selector has been added
        assert!(orm
            .get_manifest()
            .components
            .get("component1")
            .unwrap()
            .settings
            .as_ref()
            .unwrap()
            .iter()
            .any(|s| {
                if let SettingsBlockElement::SelectorBlock(key, _) = s {
                    key.raw_value == "new_selector"
                } else {
                    false
                }
            }));
    }

    #[test]
    fn test_selector_builder_update() {
        let mut orm = PaxManifestORM::new(create_basic_manifest());

        // Retrieve an existing selector and update it
        let builder = orm.get_selector("component1".to_string(), "existing_selector".to_string());
        builder
            .set_value(LiteralBlockDefinition::new(vec![SettingElement::Comment(
                "Updated Selector".to_string(),
            )]))
            .save()
            .unwrap();

        // Verify that the selector has been updated
        let updated_selector = orm
            .get_manifest()
            .components
            .get("component1")
            .unwrap()
            .settings
            .as_ref()
            .unwrap()
            .iter()
            .find(|s| {
                if let SettingsBlockElement::SelectorBlock(key, _) = s {
                    key.raw_value == "existing_selector"
                } else {
                    false
                }
            })
            .unwrap();

        if let SettingsBlockElement::SelectorBlock(_, value) = updated_selector {
            assert!(value.elements.iter().any(|elem| match elem {
                SettingElement::Comment(comment) => comment == "Updated Selector",
                _ => false,
            }));
        }
    }

    #[test]
    fn test_selector_builder_remove() {
        let mut orm = PaxManifestORM::new(create_basic_manifest());

        // Remove an existing selector
        orm.remove_selector("component1".to_string(), "existing_selector".to_string())
            .unwrap();

        // Verify that the selector has been removed
        assert!(orm
            .get_manifest()
            .components
            .get("component1")
            .unwrap()
            .settings
            .is_none());
    }
}
