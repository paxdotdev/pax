#![cfg(feature = "compiler")]

use pax_language::{parse_pax_expression, parse_pax_str, Computable, Rule};
use pax_manifest::{parsing::parse_value_definition, *};
use pax_runtime_api::{functions::Functions, ToPaxValue};
use std::{
    collections::{BTreeMap, HashMap},
    rc::Rc,
    sync::Arc,
};

fn gradient(source: &str) -> ValueDefinition {
    parse_value_definition(parse_pax_str(Rule::gradient_inline_value, source).unwrap())
}

const REACTIVE: &str =
    "@gradient { linear: {} 0%: {@gpu ? WHITE : BLACK} 100%: {@cpu ? RED : BLUE} }";

fn manifest_with_setting(name: &str, value: ValueDefinition) -> PaxManifest {
    let id = TypeId::build_singleton("crate::Example", Some("Example"));
    let mut template = ComponentTemplate::new(id.clone(), None);
    let mut node = TemplateNodeDefinition::default();
    node.type_id = id.clone();
    node.settings = Some(vec![SettingElement::Setting(
        Token::new_without_location(name.into()),
        value,
    )]);
    template.add(node);
    PaxManifest {
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
    }
}

#[test]
fn renderer_globals_survive_rich_binary_program_ir_and_rust_baking() {
    let manifest = manifest_with_setting("fill", gradient(REACTIVE));
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
    assert!(rust.contains("@gpu"));
    assert!(rust.contains("@cpu"));
    let decoded_ir = program_ir::binary::from_slice(&baked).unwrap();
    let ir_rust =
        serde_json::to_string(&decoded_ir.components.values().next().unwrap().template).unwrap();
    assert!(ir_rust.contains("@gpu"));
    assert!(ir_rust.contains("@cpu"));
}

#[test]
fn contextual_names_and_helper_calls_execute_after_debug_and_release_roundtrips() {
    let source = "@gpu ? Math::max(@base, @viewport.width) + @frames + SigilMigration::identity(@millis) : base";
    let manifest = manifest_with_setting(
        "value",
        ValueDefinition::Expression(ExpressionInfo::new(parse_pax_expression(source).unwrap())),
    );
    let debug: PaxManifest =
        serde_json::from_str(&serde_json::to_string(&manifest).unwrap()).unwrap();
    let binary = binary::from_slice(&binary::to_vec(&manifest).unwrap()).unwrap();
    let ir = program_ir::ProgramIR::from_manifest(&manifest);
    let baked = program_ir::binary::from_slice(&program_ir::binary::to_vec(&ir).unwrap()).unwrap();
    let templates = [
        debug
            .components
            .values()
            .next()
            .unwrap()
            .template
            .as_ref()
            .unwrap(),
        binary
            .components
            .values()
            .next()
            .unwrap()
            .template
            .as_ref()
            .unwrap(),
        baked
            .components
            .values()
            .next()
            .unwrap()
            .template
            .as_ref()
            .unwrap(),
    ];
    Functions::register_all_functions();
    pax_runtime_api::register_function(
        "SigilMigration".into(),
        "identity".into(),
        Arc::new(|args| match args.as_slice() {
            [value] => Ok(value.clone()),
            _ => Err("expected one argument".into()),
        }),
    );
    let graph = pax_runtime_api::PropertyGraph::new(|| {});
    let _scope = graph.enter();
    for template in templates {
        let nodes = template.get_nodes();
        let SettingElement::Setting(_, ValueDefinition::Expression(info)) =
            &nodes[0].settings.as_ref().unwrap()[0]
        else {
            panic!("lost expression")
        };
        assert_eq!(
            info.dependencies,
            ["@gpu", "@base", "@viewport", "@frames", "@millis", "base"]
        );
        for (gpu, expected) in [(true, 23), (false, 99)] {
            let values = HashMap::from([
                ("@gpu".into(), gpu.to_pax_value()),
                ("@base".into(), 10.to_pax_value()),
                (
                    "@viewport".into(),
                    pax_runtime_api::PaxValue::Object(vec![("width".into(), 20.to_pax_value())]),
                ),
                ("@frames".into(), 1.to_pax_value()),
                ("@millis".into(), 2.to_pax_value()),
                ("base".into(), 99.to_pax_value()),
            ]);
            assert_eq!(
                info.expression.compute(Rc::new(values)).unwrap(),
                expected.to_pax_value()
            );
        }
    }
    let rust = rust_manifest::to_rust_expression(&manifest);
    for name in [
        "@gpu",
        "@base",
        "@viewport",
        "@frames",
        "@millis",
        "Math",
        "max",
        "SigilMigration",
        "identity",
    ] {
        assert!(rust.contains(name), "Rust baking lost {name}");
    }
}
