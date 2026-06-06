#[cfg(test)]
mod tests {
    use crate::orm::{PaxManifestORM, ReloadType};
    use pax_manifest::{
        pax_runtime_api::Size, ComponentDefinition, LiteralBlockDefinition, PaxManifest,
        SettingsBlockElement, Token, TypeId,
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

        orm.set_initial_server_manifest(manifest);

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
        orm.set_initial_server_manifest(server_manifest);

        assert!(orm.manifest_loaded_from_server.get());
        assert_eq!(orm.get_manifest_version().get(), 1);
        let reload_queue = orm.take_reload_queue();
        assert!(reload_queue.contains(&ReloadType::Tree));
    }
}
