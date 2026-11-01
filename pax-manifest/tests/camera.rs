#![cfg(feature = "compiler")]
use pax_language::{parse_pax_str, Rule};
use pax_manifest::{parsing::parse_camera_from_component_definition_string, *};
use std::collections::{BTreeMap, HashMap};

fn parse(source: &str) -> Option<LiteralBlockDefinition> {
    parse_camera_from_component_definition_string(
        parse_pax_str(Rule::pax_component_definition, source).unwrap(),
    )
}

#[test]
fn camera_survives_source_rich_binary_program_ir_and_rust_baking() {
    let source = "@camera { // retained in source only\n position: {[50% + (pan)px, 50%, 40px]} rotate_y: {angle} zoom: @timeline { 0: 1, 60: 2 } projection: CameraMode::Perspective vertical_fov: {fov} near: 1px far: 10000px }";
    let camera = parse(source).unwrap();
    let id = TypeId::build_singleton("Example", Some("crate::Example"));
    let component = ComponentDefinition {
        type_id: id.clone(),
        is_main_component: true,
        is_primitive: false,
        is_struct_only_component: false,
        module_path: "crate".into(),
        primitive_instance_import_path: None,
        template: None,
        settings: None,
        timelines: vec![],
        camera: Some(camera),
        route_branch: None,
    };
    let manifest = PaxManifest {
        components: BTreeMap::from([(id.clone(), component.clone())]),
        main_component_type_id: id.clone(),
        type_table: HashMap::new(),
        assets_dirs: vec![],
        engine_import_path: "pax_engine".into(),
    };
    let rich = serde_json::to_value(&manifest).unwrap();
    assert!(manifest.validate_runtime_capabilities(false).is_err());
    assert!(manifest.validate_runtime_capabilities(true).is_ok());
    let binary = binary::to_vec(&manifest).unwrap();
    assert_eq!(
        serde_json::to_value(binary::from_slice(&binary).unwrap()).unwrap(),
        rich
    );
    let ir = program_ir::ProgramIR::from_manifest(&manifest);
    assert_eq!(
        ir.components[&id].camera.as_ref().unwrap().elements.len(),
        7
    );
    let baked = program_ir::binary::to_vec(&ir).unwrap();
    assert_eq!(
        program_ir::binary::to_vec(&program_ir::binary::from_slice(&baked).unwrap()).unwrap(),
        baked
    );
    let rust = rust_manifest::to_rust_expression(&manifest);
    assert!(rust.contains("camera: Some("));
    assert!(rust.contains("pan"));
    assert!(rust.contains("TimelineTrackDefinition"));
    let serialized = code_serialization::press_code_serialization_template(component).unwrap();
    let reparsed = parse(&serialized).unwrap();
    assert_eq!(reparsed.elements.len(), 8);
    let formatted = pax_language::formatting::format_pax_template(source.to_owned()).unwrap();
    assert!(formatted.contains("@camera"));
    assert_eq!(parse(&formatted).unwrap().elements.len(), 8);
}

#[test]
fn camera_rejects_duplicate_blocks_keys_and_unknown_fields() {
    for source in [
        "@camera {} @camera {}",
        "@camera { zoom: 1 zoom: 2 }",
        "@camera { perspective: true }",
    ] {
        assert!(
            std::panic::catch_unwind(|| parse(source)).is_err(),
            "{source}"
        );
    }
    assert!(parse("").is_none());
}
