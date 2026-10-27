#![cfg(feature = "compiler")]

use pax_language::{parse_pax_str, Rule};
use pax_manifest::{parsing::parse_value_definition, *};
use std::collections::{BTreeMap, HashMap};

fn gradient(source: &str) -> ValueDefinition {
    parse_value_definition(parse_pax_str(Rule::gradient_inline_value, source).unwrap())
}

const REACTIVE: &str =
    "@gradient { linear: {} 0%: {$gpu ? WHITE : BLACK} 100%: {$cpu ? RED : BLUE} }";

#[test]
fn renderer_globals_survive_rich_binary_program_ir_and_rust_baking() {
    let id = TypeId::build_singleton("crate::Example", Some("Example"));
    let mut template = ComponentTemplate::new(id.clone(), None);
    let mut node = TemplateNodeDefinition::default();
    node.type_id = id.clone();
    node.settings = Some(vec![SettingElement::Setting(
        Token::new_without_location("fill".into()),
        gradient(REACTIVE),
    )]);
    template.add(node);
    let manifest = PaxManifest {
        main_component_type_id: id.clone(),
        components: BTreeMap::from([(
            id.clone(),
            ComponentDefinition {
                type_id: id,
                is_main_component: true,
                is_primitive: false,
                is_struct_only_component: false,
                module_path: "crate".into(),
                primitive_instance_import_path: None,
                template: Some(template),
                settings: None,
                timelines: vec![],
                route_branch: None,
            },
        )]),
        type_table: HashMap::new(),
        assets_dirs: vec![],
        engine_import_path: "pax_engine".into(),
    };
    let rich = serde_json::to_value(&manifest).unwrap();
    let binary = binary::to_vec(&manifest).unwrap();
    assert_eq!(
        serde_json::to_value(binary::from_slice(&binary).unwrap()).unwrap(),
        rich
    );
    let ir = program_ir::ProgramIR::from_manifest(&manifest);
    let baked = program_ir::binary::to_vec(&ir).unwrap();
    assert_eq!(
        program_ir::binary::to_vec(&program_ir::binary::from_slice(&baked).unwrap()).unwrap(),
        baked
    );
    let rust = rust_manifest::to_rust_expression(&manifest);
    assert!(rust.contains("$gpu"));
    assert!(rust.contains("$cpu"));
    let decoded_ir = program_ir::binary::from_slice(&baked).unwrap();
    let ir_rust =
        serde_json::to_string(&decoded_ir.components.values().next().unwrap().template).unwrap();
    assert!(ir_rust.contains("$gpu"));
    assert!(ir_rust.contains("$cpu"));
}
