#[cfg(test)]
mod tests {
    use crate::orm::{
        handlers::{
            builder::HandlerBuilder, AddHandlerRequest, GetAllHandlersRequest, GetHandlerRequest,
            RemoveHandlerRequest, UpdateHandlerRequest,
        },
        Command, PaxManifestORM, UndoRedo,
    };
    use pax_manifest::{
        get_primitive_type_table, ComponentDefinition, HandlersBlockElement, PaxManifest, Token,
        TokenType,
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
                settings: None,
                handlers: Some(vec![HandlersBlockElement::Handler(
                    Token::new_from_raw_value("existing_handler".to_string(), TokenType::EventId),
                    vec![Token::new_from_raw_value(
                        "handler_action".to_string(),
                        TokenType::Handler,
                    )],
                )]),
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
    fn test_add_handler_command() {
        let manifest = create_basic_manifest();
        let mut orm = PaxManifestORM::new(manifest);

        let mut command = AddHandlerRequest {
            component_type_id: "component1".to_string(),
            handler_index: None,
            key: "new_handler".to_string(),
            value: vec!["new_action".to_string()],
        };

        command.execute(&mut orm.manifest).unwrap();

        // Check creation
        assert!(orm.manifest.components["component1"]
            .handlers
            .as_ref()
            .unwrap()
            .iter()
            .any(|h| match h {
                HandlersBlockElement::Handler(key, actions) =>
                    key.raw_value == "new_handler"
                        && actions.iter().any(|a| a.raw_value == "new_action"),
                _ => false,
            }));

        // Undo
        command.undo(&mut orm.manifest).unwrap();
        assert!(!orm.manifest.components["component1"]
            .handlers
            .as_ref()
            .unwrap()
            .iter()
            .any(|h| match h {
                HandlersBlockElement::Handler(key, _) => key.raw_value == "new_handler",
                _ => false,
            }));

        // Redo
        command.redo(&mut orm.manifest).unwrap();
        assert!(orm.manifest.components["component1"]
            .handlers
            .as_ref()
            .unwrap()
            .iter()
            .any(|h| match h {
                HandlersBlockElement::Handler(key, actions) =>
                    key.raw_value == "new_handler"
                        && actions.iter().any(|a| a.raw_value == "new_action"),
                _ => false,
            }));
    }

    #[test]
    fn test_update_handler_command() {
        let mut orm = PaxManifestORM::new(create_basic_manifest());

        let mut command = UpdateHandlerRequest {
            component_type_id: "component1".to_string(),
            new_index: None,
            key: "existing_handler".to_string(),
            value: vec!["updated_action".to_string()],
            cached_prev_state: None,
            cached_prev_position: None,
        };

        command.execute(&mut orm.manifest).unwrap();

        // Check update
        assert!(orm.manifest.components["component1"]
            .handlers
            .as_ref()
            .unwrap()
            .iter()
            .any(|h| match h {
                HandlersBlockElement::Handler(key, actions) =>
                    key.raw_value == "existing_handler"
                        && actions.iter().any(|a| a.raw_value == "updated_action"),
                _ => false,
            }));

        // Undo
        command.undo(&mut orm.manifest).unwrap();
        assert!(orm.manifest.components["component1"]
            .handlers
            .as_ref()
            .unwrap()
            .iter()
            .any(|h| match h {
                HandlersBlockElement::Handler(key, actions) =>
                    key.raw_value == "existing_handler"
                        && actions.iter().any(|a| a.raw_value == "handler_action"),
                _ => false,
            }));

        // Redo
        command.redo(&mut orm.manifest).unwrap();
        assert!(orm.manifest.components["component1"]
            .handlers
            .as_ref()
            .unwrap()
            .iter()
            .any(|h| match h {
                HandlersBlockElement::Handler(key, actions) =>
                    key.raw_value == "existing_handler"
                        && actions.iter().any(|a| a.raw_value == "updated_action"),
                _ => false,
            }));
    }

    #[test]
    fn test_remove_handler_command() {
        let mut orm = PaxManifestORM::new(create_basic_manifest());

        let mut command = RemoveHandlerRequest {
            component_type_id: "component1".to_string(),
            key: "existing_handler".to_string(),
            cached_prev_state: None,
            cached_prev_position: None,
        };
        command.execute(&mut orm.manifest).unwrap();

        // Check removal
        assert!(orm.manifest.components["component1"].handlers.is_none());

        // Undo
        command.undo(&mut orm.manifest).unwrap();
        assert!(orm.manifest.components["component1"]
            .handlers
            .as_ref()
            .unwrap()
            .iter()
            .any(|h| match h {
                HandlersBlockElement::Handler(key, _) => key.raw_value == "existing_handler",
                _ => false,
            }));

        // Redo
        command.redo(&mut orm.manifest).unwrap();
        assert!(orm.manifest.components["component1"].handlers.is_none());
    }

    #[test]
    fn test_get_handler_command() {
        let mut orm = PaxManifestORM::new(create_basic_manifest());

        // Prepare and execute the GetHandlerRequest
        let mut command = GetHandlerRequest {
            component_type_id: "component1".to_string(),
            key: "existing_handler".to_string(),
        };
        let response = command.execute(&mut orm.manifest).unwrap();

        // Check retrieval
        if let Some(HandlersBlockElement::Handler(key, actions)) = response.handler {
            assert_eq!(key.raw_value, "existing_handler");
            assert!(actions.iter().any(|a| a.raw_value == "handler_action"));
        } else {
            panic!("Handler 'existing_handler' not found");
        }
    }

    #[test]
    fn test_get_all_handlers_command() {
        let mut orm = PaxManifestORM::new(create_basic_manifest());

        // Add a second handler to the manifest
        let add_request = AddHandlerRequest {
            component_type_id: "component1".to_string(),
            handler_index: None,
            key: "second_handler".to_string(),
            value: vec!["second_action".to_string()],
        };
        orm.execute_command(add_request).unwrap();

        // Prepare and execute the GetAllHandlersRequest
        let mut command = GetAllHandlersRequest {
            component_type_id: "component1".to_string(),
        };
        let response = command.execute(&mut orm.manifest).unwrap();

        // Check retrieval
        let handlers = response.handlers.unwrap();
        assert_eq!(handlers.len(), 2);
        if let HandlersBlockElement::Handler(key, actions) = &handlers[0] {
            assert_eq!(key.raw_value, "existing_handler");
            assert!(actions.iter().any(|a| a.raw_value == "handler_action"));
        } else {
            panic!("First handler not in expected format");
        }
        if let HandlersBlockElement::Handler(key, actions) = &handlers[1] {
            assert_eq!(key.raw_value, "second_handler");
            assert!(actions.iter().any(|a| a.raw_value == "second_action"));
        } else {
            panic!("Second handler not in expected format");
        }
    }

    #[test]
    fn test_handler_builder_add() {
        let mut orm = PaxManifestORM::new(create_basic_manifest());

        let builder = HandlerBuilder::new(
            &mut orm,
            "component1".to_string(),
            "new_handler".to_string(),
        )
        .set_handler_value(vec!["new_action".to_string()]);

        builder.save().unwrap();

        // Verify that the new handler has been added
        assert!(orm.manifest.components["component1"]
            .handlers
            .as_ref()
            .unwrap()
            .iter()
            .any(|h| match h {
                HandlersBlockElement::Handler(key, actions) =>
                    key.raw_value == "new_handler"
                        && actions.iter().any(|a| a.raw_value == "new_action"),
                _ => false,
            }));
    }

    #[test]
    fn test_handler_builder_update() {
        let mut orm = PaxManifestORM::new(create_basic_manifest());

        let builder = HandlerBuilder::retrieve_handler(
            &mut orm,
            "component1".to_string(),
            "existing_handler".to_string(),
        )
        .set_handler_value(vec!["updated_action".to_string()]);

        builder.save().unwrap();

        // Verify that the handler has been updated
        assert!(orm.manifest.components["component1"]
            .handlers
            .as_ref()
            .unwrap()
            .iter()
            .any(|h| match h {
                HandlersBlockElement::Handler(key, actions) =>
                    key.raw_value == "existing_handler"
                        && actions.iter().any(|a| a.raw_value == "updated_action"),
                _ => false,
            }));
    }

    #[test]
    fn test_handler_builder_remove() {
        let mut orm = PaxManifestORM::new(create_basic_manifest());

        let builder = HandlerBuilder::retrieve_handler(
            &mut orm,
            "component1".to_string(),
            "existing_handler".to_string(),
        );

        builder.remove().unwrap();

        // Verify that the handler has been removed
        assert!(orm.manifest.components["component1"].handlers.is_none());
    }
}
