#[cfg(test)]
mod renamed_engine {
    pub use pax_language;
    pub use pax_manifest;
    pub use pax_manifest::pax_runtime_api as api;
}

#[cfg(test)]
mod tests {
    use pax_manifest::binary;

    include!(concat!(env!("OUT_DIR"), "/generated.rs"));

    const EXPECTED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/expected.bin"));

    #[test]
    fn compiled_manifest_matches_every_serialized_field() {
        let (first, second) = construct_pair();
        assert_eq!(binary::to_vec(&first).unwrap(), EXPECTED);
        assert_eq!(binary::to_vec(&second).unwrap(), EXPECTED);
        assert_eq!(
            binary::to_vec(&construct_empty()).unwrap(),
            include_bytes!(concat!(env!("OUT_DIR"), "/empty.bin")),
        );
    }

    #[test]
    fn compiled_wall_time_exit_does_not_add_a_frame_requirement() {
        let manifest = construct_pair().0;
        let config = manifest.get_component_transition_config(&manifest.main_component_type_id);
        assert!(config.has_exit);
        assert_eq!(
            (config.exit_frame_count, config.exit_millis_count),
            (0, Some(1000))
        );
    }

    #[test]
    fn compiled_manifest_instances_are_independent() {
        let (mut changed, untouched) = construct_pair();
        let root = changed.main_component_type_id.clone();
        let component = changed.components.get_mut(&root).unwrap();
        component.module_path.push_str("::changed");
        component.settings.as_mut().unwrap().clear();
        component.timelines.clear();
        component.route_branch.as_mut().unwrap().modal = false;
        let template = component.template.as_mut().unwrap();
        let node_id = template.get_root()[0].clone();
        let mut node = template.get_node(&node_id).unwrap().clone();
        node.settings.as_mut().unwrap().clear();
        template.set_node(node_id, node);
        changed
            .type_table
            .get_mut(&root)
            .unwrap()
            .property_definitions
            .clear();
        changed.assets_dirs.clear();
        assert_ne!(binary::to_vec(&changed).unwrap(), EXPECTED);
        assert_eq!(binary::to_vec(&untouched).unwrap(), EXPECTED);
        assert_eq!(binary::to_vec(&construct_pair().0).unwrap(), EXPECTED);
    }
}
