use std::path::PathBuf;

use pax_compiler::static_analysis::build_manifest;
use pax_manifest::program_ir::ProgramIR;
use pax_manifest::{ComponentTemplate, SettingElement, TypeId, ValueDefinition};

#[test]
fn increment_static_manifest_round_trips_through_program_ir_binary() {
    let workspace_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("pax-compiler should have a workspace parent")
        .to_path_buf();
    let project_path = workspace_dir.join("examples/increment");

    let manifest = build_manifest(&project_path).expect("increment manifest should build");
    let ir = ProgramIR::from_manifest(&manifest);
    let bytes = pax_manifest::program_ir::binary::to_vec(&ir).expect("program ir should serialize");
    let decoded = pax_manifest::program_ir::binary::from_slice(&bytes)
        .expect("program ir should deserialize");

    assert_eq!(decoded.main_component_type_id, ir.main_component_type_id);
    assert_eq!(decoded.components.len(), ir.components.len());
    assert_eq!(decoded.type_table.len(), ir.type_table.len());
    assert_eq!(decoded.assets_dirs, ir.assets_dirs);
    assert!(decoded.components.values().all(|component| component
        .template
        .as_ref()
        .map(|template| template.get_file_path().is_none())
        .unwrap_or(true)));
}

#[test]
fn standard_content_receivers_survive_rich_and_baked_programs() {
    let project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/fixtures/text-bodies");
    // Analyze the real component declarations and usages, including Toast's
    // caller-scoped expression, rather than supplying a synthetic receiver map.
    let manifest = build_manifest(&project).expect("text body fixture should build");
    let rich = pax_manifest::binary::from_slice(&pax_manifest::binary::to_vec(&manifest).unwrap())
        .unwrap();
    let baked = pax_manifest::program_ir::binary::from_slice(
        &pax_manifest::program_ir::binary::to_vec(&ProgramIR::from_manifest(&manifest)).unwrap(),
    )
    .unwrap();
    let root = &manifest.main_component_type_id;

    for (name, property, literal) in [
        ("Handwriter", "text", Some("Handwritten body\nSecond line")),
        ("Toast", "message", None),
        ("ConfirmationDialog", "text", Some("Continue?")),
    ] {
        let id = manifest
            .type_table
            .keys()
            .find(|id| id.get_pascal_identifier().as_deref() == Some(name))
            .unwrap();
        let receiver = manifest.type_table[id]
            .property_definitions
            .iter()
            .find(|definition| definition.name == property)
            .unwrap();
        assert!(receiver.flags.is_content, "{name}.{property}");
        let template = manifest.components[root].template.as_ref().unwrap();
        let value = content_setting(template, id, property);
        if let Some(expected) = literal {
            let ValueDefinition::LiteralValue(pax_runtime_api::PaxValue::String(actual)) = value
            else {
                panic!("{name} should receive a literal string")
            };
            assert_eq!(actual, expected, "{name}");
        } else {
            assert!(matches!(value, ValueDefinition::Expression(_)));
            assert!(value.to_string().contains("notifications"));
        }
        let rich_template = rich.components[root].template.as_ref().unwrap();
        let baked_template = baked.components[root].template.as_ref().unwrap();
        assert_eq!(
            content_setting(rich_template, id, property).to_string(),
            value.to_string(),
            "{name}"
        );
        assert_eq!(
            content_setting(baked_template, id, property).to_string(),
            value.to_string(),
            "{name}"
        );
        assert_eq!(
            rich_template
                .get_nodes()
                .into_iter()
                .find(|node| &node.type_id == id)
                .unwrap()
                .body_property
                .as_deref(),
            Some(property)
        );
        assert!(baked_template
            .get_nodes()
            .into_iter()
            .filter(|node| &node.type_id == id)
            .all(|node| node.body_property.is_none()));
        assert!(baked.type_table[id]
            .property_definitions
            .iter()
            .all(|definition| !definition.flags.is_content));
    }
}

fn content_setting<'a>(
    template: &'a ComponentTemplate,
    type_id: &TypeId,
    property: &str,
) -> &'a ValueDefinition {
    template
        .get_nodes()
        .into_iter()
        .find(|node| &node.type_id == type_id)
        .unwrap()
        .settings
        .as_ref()
        .unwrap()
        .iter()
        .find_map(|setting| match setting {
            SettingElement::Setting(key, value) if key.token_value == property => Some(value),
            _ => None,
        })
        .unwrap()
}
