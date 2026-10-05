use pax_manifest::{
    program_ir::{self, ProgramIR},
    ComponentDefinition, ComponentTemplate, PaxIdentifier, PaxManifest, SettingElement,
    SettingsBlockElement, TemplateNodeDefinition, Token, TypeId, ValueDefinition,
};

const PHASES: [&str; 4] = ["mount", "tick", "pre_render", "unmount"];

fn component(type_id: TypeId, owner: &str) -> ComponentDefinition {
    ComponentDefinition {
        type_id,
        is_main_component: owner == "owner",
        is_primitive: false,
        is_struct_only_component: false,
        module_path: "test".to_string(),
        primitive_instance_import_path: None,
        template: None,
        settings: Some(
            PHASES
                .iter()
                .map(|phase| {
                    SettingsBlockElement::Handler(
                        Token::new_without_location(phase.to_string()),
                        vec![Token::new_without_location(format!("{owner}_{phase}"))],
                    )
                })
                .collect(),
        ),
        timelines: vec![],
        route_branch: None,
    }
}

#[test]
fn lifecycle_bindings_preserve_authored_location_in_rich_binary_and_program_ir() {
    let owner_id = TypeId::build_singleton("test::Owner", Some("Owner"));
    let child_id = TypeId::build_singleton("test::Child", Some("Child"));
    let mut template = ComponentTemplate::new(owner_id.clone(), None);
    let child_node_id = template
        .add(TemplateNodeDefinition {
            type_id: child_id.clone(),
            settings: Some(
                PHASES
                    .iter()
                    .map(|phase| {
                        SettingElement::Setting(
                            Token::new_without_location(phase.to_string()),
                            ValueDefinition::EventBindingTarget(PaxIdentifier::new(&format!(
                                "self.inline_{phase}"
                            ))),
                        )
                    })
                    .collect(),
            ),
            ..Default::default()
        })
        .get_template_node_id();
    let mut owner = component(owner_id.clone(), "owner");
    owner.template = Some(template);
    let manifest = PaxManifest {
        components: [
            (owner_id.clone(), owner),
            (child_id.clone(), component(child_id.clone(), "child")),
        ]
        .into_iter()
        .collect(),
        main_component_type_id: owner_id.clone(),
        type_table: Default::default(),
        assets_dirs: vec![],
        engine_import_path: "pax_kit::pax_engine".to_string(),
    };
    let decoded =
        pax_manifest::binary::from_slice(&pax_manifest::binary::to_vec(&manifest).unwrap())
            .unwrap();
    let inline = PHASES.map(|phase| (phase.to_string(), format!("inline_{phase}")));
    for manifest in [&manifest, &decoded] {
        for (type_id, prefix) in [(&owner_id, "owner"), (&child_id, "child")] {
            assert_eq!(
                manifest.get_component_handlers(type_id),
                PHASES.map(|phase| { (phase.to_string(), vec![format!("{prefix}_{phase}")]) })
            );
        }
        let node = manifest.components[&owner_id]
            .template
            .as_ref()
            .unwrap()
            .get_node(&child_node_id)
            .unwrap();
        assert_eq!(manifest.get_inline_event_handlers(node), inline);
        let ir = ProgramIR::from_manifest(manifest);
        let ir = program_ir::binary::from_slice(&program_ir::binary::to_vec(&ir).unwrap()).unwrap();
        let node = ir.components[&owner_id]
            .template
            .as_ref()
            .unwrap()
            .get_node(&child_node_id)
            .unwrap();
        assert_eq!(manifest.get_inline_event_handlers(node), inline);
        for (type_id, prefix) in [(&owner_id, "owner"), (&child_id, "child")] {
            let settings = ir.components[type_id].settings.as_ref().unwrap();
            assert_eq!(settings.len(), PHASES.len());
            for (setting, phase) in settings.iter().zip(PHASES) {
                let SettingsBlockElement::Handler(event, methods) = setting else {
                    panic!("lifecycle setting must retain its component binding");
                };
                assert_eq!(event.token_value, phase);
                assert_eq!(methods[0].token_value, format!("{prefix}_{phase}"));
            }
        }
    }
}
