use pax_manifest::{parsing::*, *};
use std::collections::{BTreeMap, HashMap};
#[path = "../../schema.rs"]
mod schema;
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let source = std::fs::read_to_string(&args[1]).unwrap();
    let schema: schema::Schema = serde_json::from_slice(&std::fs::read(&args[2]).unwrap()).unwrap();
    let root = TypeId::build_singleton(&schema.component, Some("Counter"));
    let primitives = [
        ("Group", "pax_std::core::group::Group"),
        ("Text", "pax_std::core::text::Text"),
        ("Button", "pax_std::forms::button::Button"),
        ("Rectangle", "pax_std::drawing::rectangle::Rectangle"),
    ];
    let mut ctx = TemplateNodeParseContext {
        template: ComponentTemplate::new(root.clone(), None),
        pascal_identifier_to_type_id_map: primitives
            .into_iter()
            .map(|(name, path)| (name.into(), TypeId::build_singleton(path, Some(name))))
            .collect(),
        route_branch_descriptors: HashMap::new(),
    };
    let ast =
        pax_language::parse_pax_str(pax_language::Rule::pax_component_definition, &source).unwrap();
    parse_template_from_component_definition_string(&mut ctx, &source, ast.clone());
    let component = ComponentDefinition {
        type_id: root.clone(),
        is_main_component: true,
        is_primitive: false,
        is_struct_only_component: false,
        module_path: "probe_bundle".into(),
        primitive_instance_import_path: None,
        template: Some(ctx.template),
        settings: Some(parse_settings_from_component_definition_string(ast.clone())),
        timelines: parse_timeline_from_component_definition_string(ast),
        route_branch: None,
    };
    let mut type_table = HashMap::new();
    type_table.insert(
        root.clone(),
        TypeDefinition {
            type_id: root.clone(),
            inner_iterable_type_id: None,
            property_definitions: schema
                .fields
                .iter()
                .map(|f| PropertyDefinition::primitive_with_name(&f.kind, &f.name))
                .collect(),
        },
    );
    let manifest = PaxManifest {
        components: BTreeMap::from([(root.clone(), component)]),
        main_component_type_id: root,
        type_table,
        assets_dirs: vec![],
        engine_import_path: "pax_kit::pax_engine".into(),
    };
    let bytes = binary::to_vec(&manifest).unwrap();
    // Exercise the existing release binary format, including expressions,
    // selector settings, and event routes, before embedding it in the module.
    assert_eq!(
        binary::to_vec(&binary::from_slice(&bytes).unwrap()).unwrap(),
        bytes
    );
    std::fs::write(&args[3], bytes).unwrap();
}
