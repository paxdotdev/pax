#![cfg(feature = "compiler")]

use pax_language::{parse_pax_str, Rule};
use pax_manifest::{parsing::parse_value_definition, *};
use std::collections::{BTreeMap, HashMap};

fn gradient(source: &str) -> ValueDefinition {
    parse_value_definition(parse_pax_str(Rule::gradient_inline_value, source).unwrap())
}

const REACTIVE: &str = "@gradient { mesh: { rows: [
    [{position: [0%,0%], color: CYAN}, {position: [100%,0%], color: FUCHSIA}],
    [{position: [0%,100%], color: YELLOW}, {position: [{self.x},100%], color: {self.accent}}]
] } }";

#[test]
fn mesh_rows_survive_rich_binary_program_ir_and_rust_baking() {
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
    assert!(rust.contains("GradientShapeDefinition::Mesh"));
    assert!(rust.contains("accent"));
    let ValueDefinition::Gradient(value) = gradient(REACTIVE) else {
        panic!("mesh gradient")
    };
    let formatted = format!("@gradient {value}");
    let ValueDefinition::Gradient(roundtrip) = gradient(&formatted) else {
        panic!("mesh gradient")
    };
    assert_eq!(value.to_string(), roundtrip.to_string());
    let ValueDefinition::Gradient(bound) = gradient("@gradient {mesh: {rows: {self.rows}}}") else {
        panic!("mesh gradient")
    };
    assert!(matches!(bound.shape, GradientShapeDefinition::Mesh { .. }));
}

#[test]
fn malformed_static_meshes_are_rejected_before_mount() {
    for source in [
        "@gradient {mesh: {rows: []}}",
        "@gradient {mesh: {rows: [[{position:[0%,0%],color:CYAN}]]}}",
        "@gradient {mesh: {rows: [[],[]]}}",
        "@gradient {mesh: {rows: [[{position:[0%],color:CYAN},{position:[0%,0%],color:CYAN}],[{position:[0%,0%],color:CYAN},{position:[0%,0%],color:CYAN}]]}}",
        "@gradient {mesh: {rows: {self.rows}, radius: 1}}",
        "@gradient {mesh: {rows: {self.rows}}, 0%: CYAN, 100%: FUCHSIA}",
        "@gradient {linear: {rows: {self.rows}}, 0%: CYAN, 100%: FUCHSIA}",
    ] {
        assert!(std::panic::catch_unwind(|| gradient(source)).is_err(), "accepted {source}");
    }
}
