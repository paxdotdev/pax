#[cfg(test)]
mod tests {
    use pax_manifest::{
        ComponentDefinition, PaxManifest, SettingElement, TemplateNodeDefinition, Token, TokenType,
        ValueDefinition,
    };

    use crate::orm::{
        template::{
            builder::NodeBuilder, AddTemplateNodeRequest, GetAllTemplateNodeRequest,
            GetTemplateNodeRequest, NodeType, RemoveTemplateNodeRequest, UpdateTemplateNodeRequest,
        },
        Command, PaxManifestORM, UndoRedo,
    };

    use std::collections::{HashMap, HashSet};

    fn create_basic_manifest() -> PaxManifest {
        let mut components = HashMap::new();
        let mut template = HashMap::new();

        template.insert(
            0,
            TemplateNodeDefinition {
                id: 0,
                child_ids: vec![1],
                type_id: "root".to_string(),
                control_flow_settings: None,
                settings: None,
                pascal_identifier: "Root".to_string(),
                raw_comment_string: None,
            },
        );

        template.insert(
            1,
            TemplateNodeDefinition {
                id: 1,
                child_ids: vec![],
                type_id: "type1".to_string(),
                control_flow_settings: None,
                settings: None,
                pascal_identifier: "Node1".to_string(),
                raw_comment_string: None,
            },
        );

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
                template: Some(template),
                settings: None,
                handlers: None,
                next_template_id: Some(2),
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
    fn test_add_template_node_command() {
        let mut manifest = create_basic_manifest();

        let request = AddTemplateNodeRequest {
            component_type_id: "component1".to_string(),
            parent_node_id: Some(1),
            node_id: None,
            child_ids: vec![],
            type_id: "type2".to_string(),
            node_type: NodeType::Template(vec![]),
            pascal_identifier: "Node2".to_string(),
            cached_node: None,
        };

        // Execute
        let mut command = request;
        let response = command.execute(&mut manifest).unwrap();
        assert_eq!(response.template_node.id, 2);
        assert!(manifest.components["component1"]
            .template
            .as_ref()
            .unwrap()
            .get(&1)
            .unwrap()
            .child_ids
            .contains(&2));

        // Undo
        command.undo(&mut manifest).unwrap();
        assert!(manifest.components["component1"]
            .template
            .as_ref()
            .unwrap()
            .get(&2)
            .is_none());

        // Redo
        command.redo(&mut manifest).unwrap();
        assert!(manifest.components["component1"]
            .template
            .as_ref()
            .unwrap()
            .get(&2)
            .is_some());
    }

    #[test]
    fn test_update_template_node_command() {
        let mut manifest = create_basic_manifest();

        let request = UpdateTemplateNodeRequest {
            component_type_id: "component1".to_string(),
            new_parent: None,
            updated_node: TemplateNodeDefinition {
                id: 1,
                child_ids: vec![],
                type_id: "updated_type".to_string(),
                control_flow_settings: None,
                settings: None,
                pascal_identifier: "UpdatedNode".to_string(),
                raw_comment_string: None,
            },
            cached_prev_state: None,
            cached_prev_parent: None,
            cached_prev_position: None,
        };

        // Execute
        let mut command = request;
        let response = command.execute(&mut manifest).unwrap();
        assert_eq!(response.template_node.pascal_identifier, "UpdatedNode");

        // Undo
        command.undo(&mut manifest).unwrap();
        assert_eq!(
            manifest.components["component1"]
                .template
                .as_ref()
                .unwrap()
                .get(&1)
                .unwrap()
                .pascal_identifier,
            "Node1"
        );

        // Redo
        command.redo(&mut manifest).unwrap();
        assert_eq!(
            manifest.components["component1"]
                .template
                .as_ref()
                .unwrap()
                .get(&1)
                .unwrap()
                .pascal_identifier,
            "UpdatedNode"
        );
    }

    #[test]
    fn test_remove_template_node_command() {
        let mut manifest = create_basic_manifest();

        let request = RemoveTemplateNodeRequest {
            component_type_id: "component1".to_string(),
            node_id: 1,
            cached_prev_state: None,
            cached_prev_parent: None,
            cached_prev_position: None,
        };

        // Execute
        let mut command = request;
        command.execute(&mut manifest).unwrap();
        assert!(manifest.components["component1"]
            .template
            .as_ref()
            .unwrap()
            .get(&1)
            .is_none());

        // Undo
        command.undo(&mut manifest).unwrap();
        assert!(manifest.components["component1"]
            .template
            .as_ref()
            .unwrap()
            .get(&1)
            .is_some());

        // Redo
        command.redo(&mut manifest).unwrap();
        assert!(manifest.components["component1"]
            .template
            .as_ref()
            .unwrap()
            .get(&1)
            .is_none());
    }

    #[test]
    fn test_get_template_node_command() {
        let mut manifest = create_basic_manifest();

        let request = GetTemplateNodeRequest {
            component_type_id: "component1".to_string(),
            node_id: 1,
        };

        // Execute
        let mut command = request;
        let response = command.execute(&mut manifest).unwrap();
        let node = response.node.unwrap();
        assert_eq!(node.pascal_identifier, "Node1".to_string());
        assert_eq!(node.id, 1);
    }

    #[test]
    fn test_get_all_template_node_command() {
        let mut manifest = create_basic_manifest();

        let request = GetAllTemplateNodeRequest {
            component_type_id: "component1".to_string(),
        };

        let mut command = request;
        let response = command.execute(&mut manifest).unwrap();
        assert_eq!(response.nodes.unwrap().len(), 2);

        let request = AddTemplateNodeRequest {
            component_type_id: "component1".to_string(),
            parent_node_id: Some(1),
            node_id: None,
            child_ids: vec![],
            type_id: "type2".to_string(),
            node_type: NodeType::Template(vec![]),
            pascal_identifier: "Node2".to_string(),
            cached_node: None,
        };

        // Add a new node
        let mut command = request;
        command.execute(&mut manifest).unwrap();

        let request = GetAllTemplateNodeRequest {
            component_type_id: "component1".to_string(),
        };

        let mut command = request;
        let response = command.execute(&mut manifest).unwrap();
        assert_eq!(response.nodes.unwrap().len(), 3);
    }

    #[test]
    fn test_node_builder_create_new_node() {
        let manifest = create_basic_manifest();
        let mut orm = PaxManifestORM::new(manifest);

        let node_builder = NodeBuilder::new(
            &mut orm,
            "component1".to_string(),
            "type2".to_string(),
            "NewNode".to_string(),
            Some(1),
        );

        node_builder.save().unwrap();

        let manifest = orm.get_manifest();
        let component = manifest.components.get("component1").unwrap();
        let template = component.template.as_ref().unwrap();
        let new_node = template.get(&2).unwrap();

        assert_eq!(new_node.pascal_identifier, "NewNode");
        assert_eq!(new_node.type_id, "type2");
        assert!(new_node.child_ids.is_empty());
    }

    #[test]
    fn test_update_existing_node() {
        let manifest = create_basic_manifest();
        let mut orm = PaxManifestORM::new(manifest);
        let mut node_builder = orm.get_node("component1".to_string(), 1);

        let new_value = ValueDefinition::LiteralValue(Token::new_from_raw_value(
            "newValue".to_string(),
            TokenType::LiteralValue,
        ));

        node_builder = node_builder.set_property("newProperty".to_string(), new_value);
        assert!(node_builder.save().is_ok());

        let updated_manifest = orm.get_manifest();
        let component = updated_manifest.components.get("component1").unwrap();
        let node = component.template.as_ref().unwrap().get(&1).unwrap();
        assert!(node
            .settings
            .clone()
            .unwrap()
            .iter()
            .any(|setting| match setting {
                SettingElement::Setting(key, ValueDefinition::LiteralValue(val)) =>
                    key.raw_value == "newProperty" && val.raw_value == "newValue",
                _ => false,
            }));
    }

    #[test]
    fn test_remove_node() {
        let manifest = create_basic_manifest();
        let component = manifest.components.get("component1").unwrap();
        assert!(component.template.as_ref().unwrap().contains_key(&1));

        let mut orm = PaxManifestORM::new(manifest);
        let removal = orm.remove_node("component1".to_string(), 1);

        assert!(removal.is_ok());

        let updated_manifest = orm.get_manifest();
        let component = updated_manifest.components.get("component1").unwrap();
        assert!(!component.template.as_ref().unwrap().contains_key(&1));
    }

    #[test]
    #[should_panic]
    fn test_error_handling_non_existent_node() {
        let manifest = create_basic_manifest();
        let mut orm = PaxManifestORM::new(manifest);
        orm.get_node("component1".to_string(), 999);
    }
}
