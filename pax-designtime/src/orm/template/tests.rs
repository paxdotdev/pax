#[cfg(test)]
mod tests {
    use pax_manifest::{ComponentDefinition, PaxManifest, TemplateNodeDefinition};

    use crate::orm::{
        template::{
            AddTemplateNodeRequest, NodeType, RemoveTemplateNodeRequest, UpdateTemplateNodeRequest, GetTemplateNodeRequest, GetAllTemplateNodeRequest,
        },
        Command,
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
            parent_node_id: 1,
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
            parent_node_id: 1,
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

}
