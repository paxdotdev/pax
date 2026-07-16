#[cfg(test)]
mod tests {
    use crate::orm::{runtime_abi_identity, PaxManifestORM, ReloadType};
    use pax_manifest::{
        pax_runtime_api::Size, ComponentDefinition, ComponentTemplate, LiteralBlockDefinition,
        PaxManifest, SettingsBlockElement, TemplateNodeDefinition, Token, TypeDefinition, TypeId,
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
                timelines: vec![],
            },
        );

        PaxManifest {
            components,
            main_component_type_id: type_id,
            type_table: HashMap::new(),
            assets_dirs: vec![],
            engine_import_path: "".to_string(),
        }
    }

    fn handler(event: &str, target: &str) -> SettingsBlockElement {
        SettingsBlockElement::Handler(
            Token::new_without_location(event.to_string()),
            vec![Token::new_without_location(target.to_string())],
        )
    }

    #[test]
    fn test_add_and_undo_node() {
        let mut orm = PaxManifestORM::new(create_basic_manifest());
        let type_id: TypeId = TypeId::build_singleton("Component1", Some("Component1"));
        let rectangle_type_id: TypeId = TypeId::build_singleton("Rectangle", Some("Rectangle"));

        // Build and configure a new node
        let mut node_builder = orm.build_new_node(type_id.clone(), rectangle_type_id);

        node_builder
            .set_property_from_typed("x", Some(Size::Pixels(10.into())))
            .unwrap();
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

    #[test]
    fn initial_server_manifest_matching_pristine_manifest_does_not_reload() {
        let manifest = create_basic_manifest();
        let mut orm = PaxManifestORM::new(manifest.clone());

        orm.set_initial_server_manifest(manifest).unwrap();

        assert!(orm.manifest_loaded_from_server.get());
        assert_eq!(orm.get_manifest_version().get(), 0);
        assert!(orm.take_reload_queue().is_empty());
    }

    #[test]
    fn initial_server_manifest_with_newer_state_queues_tree_reload() {
        let mut server_manifest = create_basic_manifest();
        let type_id = server_manifest.main_component_type_id.clone();
        server_manifest
            .components
            .get_mut(&type_id)
            .unwrap()
            .module_path = "updated_module_path".to_string();

        let mut orm = PaxManifestORM::new(create_basic_manifest());
        orm.set_initial_server_manifest(server_manifest).unwrap();

        assert!(orm.manifest_loaded_from_server.get());
        assert_eq!(orm.get_manifest_version().get(), 1);
        let reload_queue = orm.take_reload_queue();
        assert!(reload_queue.contains(&ReloadType::Tree));
    }

    #[test]
    fn incompatible_server_manifest_does_not_replace_live_cartridge_manifest() {
        let mut server_manifest = create_basic_manifest();
        let added_type_id = TypeId::build_singleton("Component2", Some("Component2"));
        server_manifest.components.insert(
            added_type_id.clone(),
            ComponentDefinition {
                type_id: added_type_id,
                is_main_component: false,
                is_primitive: false,
                is_struct_only_component: false,
                module_path: "module_path".to_string(),
                primitive_instance_import_path: None,
                template: None,
                settings: None,
                timelines: vec![],
            },
        );

        let mut orm = PaxManifestORM::new(create_basic_manifest());
        let err = orm
            .set_initial_server_manifest(server_manifest)
            .unwrap_err();

        assert!(err.to_string().contains("different cartridge generation"));
        assert!(!orm.manifest_loaded_from_server.get());
        assert_eq!(orm.get_manifest_version().get(), 0);
        assert!(orm.take_reload_queue().is_empty());
        assert_eq!(orm.get_manifest().components.len(), 1);
    }

    #[test]
    fn changed_type_table_does_not_replace_live_cartridge_manifest() {
        let mut server_manifest = create_basic_manifest();
        let component_type_id = server_manifest.main_component_type_id.clone();
        server_manifest.type_table.insert(
            component_type_id.clone(),
            TypeDefinition {
                type_id: component_type_id,
                ..Default::default()
            },
        );

        let mut orm = PaxManifestORM::new(create_basic_manifest());
        let err = orm
            .set_initial_server_manifest(server_manifest)
            .unwrap_err();

        assert!(err.to_string().contains("different cartridge generation"));
        assert!(!orm.manifest_loaded_from_server.get());
        assert!(orm.take_reload_queue().is_empty());
    }

    #[test]
    fn internally_inconsistent_server_manifest_is_rejected() {
        let mut server_manifest = create_basic_manifest();
        let main_type_id = server_manifest.main_component_type_id.clone();
        let missing_type_id = TypeId::build_singleton("MissingRow", Some("MissingRow"));
        let mut template = ComponentTemplate::new(main_type_id.clone(), None);
        template.add(TemplateNodeDefinition {
            type_id: missing_type_id,
            control_flow_settings: None,
            settings: Some(vec![]),
            selector_info: Default::default(),
            raw_comment_string: None,
        });
        server_manifest
            .components
            .get_mut(&main_type_id)
            .unwrap()
            .template = Some(template);

        let mut orm = PaxManifestORM::new(create_basic_manifest());
        let err = orm
            .set_initial_server_manifest(server_manifest)
            .unwrap_err();

        assert!(err.to_string().contains("references missing component"));
        assert!(!orm.manifest_loaded_from_server.get());
        assert!(orm.take_reload_queue().is_empty());
    }

    #[test]
    fn template_update_cannot_reference_component_missing_from_live_cartridge() {
        let manifest = create_basic_manifest();
        let main_type_id = manifest.main_component_type_id.clone();
        let missing_type_id = TypeId::build_singleton("MissingRow", Some("MissingRow"));
        let mut template = ComponentTemplate::new(main_type_id.clone(), None);
        template.add(TemplateNodeDefinition {
            type_id: missing_type_id,
            control_flow_settings: None,
            settings: Some(vec![]),
            selector_info: Default::default(),
            raw_comment_string: None,
        });

        let mut orm = PaxManifestORM::new(manifest);
        let err = orm
            .replace_template(main_type_id, template, vec![])
            .unwrap_err();

        assert!(err.contains("references missing component"));
        assert_eq!(orm.get_manifest_version().get(), 0);
        assert!(orm.take_reload_queue().is_empty());
    }

    #[test]
    fn template_update_targeting_missing_component_is_rejected() {
        let manifest = create_basic_manifest();
        let missing_type_id = TypeId::build_singleton("MissingComponent", Some("MissingComponent"));
        let template = ComponentTemplate::new(missing_type_id.clone(), None);
        let mut orm = PaxManifestORM::new(manifest);

        let err = orm
            .replace_template(missing_type_id, template, vec![])
            .unwrap_err();

        assert!(err.contains("targets missing component"));
        assert_eq!(orm.get_manifest_version().get(), 0);
        assert!(orm.take_reload_queue().is_empty());
    }

    #[test]
    fn generated_handler_argument_type_is_part_of_runtime_abi_identity() {
        let mut click_manifest = create_basic_manifest();
        let main_type_id = click_manifest.main_component_type_id.clone();
        click_manifest
            .components
            .get_mut(&main_type_id)
            .unwrap()
            .settings = Some(vec![handler("click", "self.handle_event")]);
        let mut tick_manifest = click_manifest.clone();
        tick_manifest
            .components
            .get_mut(&main_type_id)
            .unwrap()
            .settings = Some(vec![handler("tick", "self.handle_event")]);

        assert_ne!(
            runtime_abi_identity(&click_manifest).unwrap(),
            runtime_abi_identity(&tick_manifest).unwrap()
        );
    }

    #[test]
    fn template_update_cannot_introduce_uncompiled_handler_descriptor() {
        let manifest = create_basic_manifest();
        let main_type_id = manifest.main_component_type_id.clone();
        let template = ComponentTemplate::new(main_type_id.clone(), None);
        let mut orm = PaxManifestORM::new(manifest);

        let err = orm
            .replace_template(
                main_type_id,
                template,
                vec![handler("click", "self.handle_click")],
            )
            .unwrap_err();

        assert!(err.contains("requires different runtime descriptors"));
        assert_eq!(orm.get_manifest_version().get(), 0);
        assert!(orm.take_reload_queue().is_empty());
    }

    #[test]
    fn order_between_distinct_handler_names_is_not_part_of_runtime_abi_identity() {
        let mut compiled_manifest = create_basic_manifest();
        let main_type_id = compiled_manifest.main_component_type_id.clone();
        let compiled_settings = vec![
            handler("click", "self.handle_click"),
            handler("tick", "self.handle_tick"),
        ];
        compiled_manifest
            .components
            .get_mut(&main_type_id)
            .unwrap()
            .settings = Some(compiled_settings);

        let reordered_settings = vec![
            handler("tick", "self.handle_tick"),
            handler("click", "self.handle_click"),
        ];
        let mut reordered_manifest = compiled_manifest.clone();
        reordered_manifest
            .components
            .get_mut(&main_type_id)
            .unwrap()
            .settings = Some(reordered_settings.clone());

        assert_eq!(
            runtime_abi_identity(&compiled_manifest).unwrap(),
            runtime_abi_identity(&reordered_manifest).unwrap()
        );
    }

    #[test]
    fn duplicate_after_first_handler_match_is_not_part_of_runtime_abi_identity() {
        let mut compiled_manifest = create_basic_manifest();
        let main_type_id = compiled_manifest.main_component_type_id.clone();
        compiled_manifest
            .components
            .get_mut(&main_type_id)
            .unwrap()
            .settings = Some(vec![handler("click", "self.handle_click")]);

        let duplicate_settings = vec![
            handler("click", "self.handle_click"),
            handler("click", "self.handle_click"),
        ];
        let mut duplicate_manifest = compiled_manifest.clone();
        duplicate_manifest
            .components
            .get_mut(&main_type_id)
            .unwrap()
            .settings = Some(duplicate_settings.clone());

        assert_eq!(
            runtime_abi_identity(&compiled_manifest).unwrap(),
            runtime_abi_identity(&duplicate_manifest).unwrap()
        );
    }

    #[test]
    fn conflicting_duplicate_order_is_part_of_runtime_abi_identity() {
        let mut compiled_manifest = create_basic_manifest();
        let main_type_id = compiled_manifest.main_component_type_id.clone();
        let compiled_settings = vec![
            handler("click", "self.handle_event"),
            handler("tick", "self.handle_event"),
        ];
        compiled_manifest
            .components
            .get_mut(&main_type_id)
            .unwrap()
            .settings = Some(compiled_settings);

        let reordered_settings = vec![
            handler("tick", "self.handle_event"),
            handler("click", "self.handle_event"),
        ];
        let mut reordered_manifest = compiled_manifest.clone();
        reordered_manifest
            .components
            .get_mut(&main_type_id)
            .unwrap()
            .settings = Some(reordered_settings.clone());

        assert_ne!(
            runtime_abi_identity(&compiled_manifest).unwrap(),
            runtime_abi_identity(&reordered_manifest).unwrap()
        );

        let mut orm = PaxManifestORM::new(compiled_manifest);
        let err = orm
            .replace_template(
                main_type_id.clone(),
                ComponentTemplate::new(main_type_id, None),
                reordered_settings,
            )
            .unwrap_err();

        assert!(err.contains("requires different runtime descriptors"));
        assert_eq!(orm.get_manifest_version().get(), 0);
        assert!(orm.take_reload_queue().is_empty());
    }

    #[test]
    fn later_conflicting_duplicate_is_part_of_runtime_abi_identity() {
        let mut compiled_manifest = create_basic_manifest();
        let main_type_id = compiled_manifest.main_component_type_id.clone();
        compiled_manifest
            .components
            .get_mut(&main_type_id)
            .unwrap()
            .settings = Some(vec![handler("click", "self.handle_event")]);

        let conflicting_settings = vec![
            handler("click", "self.handle_event"),
            handler("tick", "self.handle_event"),
        ];
        let mut conflicting_manifest = compiled_manifest.clone();
        conflicting_manifest
            .components
            .get_mut(&main_type_id)
            .unwrap()
            .settings = Some(conflicting_settings.clone());

        assert_ne!(
            runtime_abi_identity(&compiled_manifest).unwrap(),
            runtime_abi_identity(&conflicting_manifest).unwrap()
        );

        let mut orm = PaxManifestORM::new(compiled_manifest);
        let err = orm
            .replace_template(
                main_type_id.clone(),
                ComponentTemplate::new(main_type_id, None),
                conflicting_settings,
            )
            .unwrap_err();

        assert!(err.contains("requires different runtime descriptors"));
        assert_eq!(orm.get_manifest_version().get(), 0);
        assert!(orm.take_reload_queue().is_empty());
    }
}
