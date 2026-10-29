#![cfg(feature = "parsing")]

use pax_language::{parse_pax_str, Rule};
use pax_manifest::{parsing::*, *};
use pax_runtime_api::PaxValue;
use std::collections::HashMap;

fn parse(source: &str) -> ComponentTemplate {
    let text = TypeId::build_singleton("test::Text", Some("Text"));
    let group = TypeId::build_singleton("test::Group", Some("Group"));
    let mut context = TemplateNodeParseContext {
        template: ComponentTemplate::new(group.clone(), None),
        pascal_identifier_to_type_id_map: HashMap::from([
            ("Text".into(), text.clone()),
            ("Group".into(), group),
        ]),
        route_branch_descriptors: HashMap::new(),
        content_properties: HashMap::from([(text, "text".into())]),
    };
    parse_template_from_component_definition_string(
        &mut context,
        source,
        parse_pax_str(Rule::pax_component_definition, source).unwrap(),
    );
    context.template
}

fn body_values(template: &ComponentTemplate) -> Vec<String> {
    template
        .get_nodes()
        .into_iter()
        .filter_map(|node| {
            node.settings
                .as_ref()?
                .iter()
                .find_map(|setting| match setting {
                    SettingElement::Setting(token, ValueDefinition::Expression(expression))
                        if token.token_value == "text" =>
                    {
                        if let PaxExpression::Primary(primary) = &expression.expression {
                            if let PaxPrimary::Literal(value) = primary.as_ref() {
                                return Some(value.to_string());
                            }
                        }
                        Some(format!("{{{}}}", expression.expression))
                    }
                    SettingElement::Setting(token, value) if token.token_value == "text" => {
                        Some(value.to_string())
                    }
                    _ => None,
                })
        })
        .collect()
}

#[test]
fn body_lowering_preserves_values_and_caller_expressions() {
    for (source, expected) in [
        ("<Text>Hello &amp; world</Text>", "\"Hello & world\""),
        (
            "<Text>\n    first\n        second\n</Text>",
            "r\"first\n    second\"",
        ),
        ("<Text>{self.title}</Text>", "{title}"),
        (
            "<Text>{r#\"\n  exact </Text>\"#}</Text>",
            "r\"\n  exact </Text>\"",
        ),
    ] {
        let template = parse(source);
        assert_eq!(body_values(&template), [expected], "{source}");
        assert_eq!(
            template.get_nodes()[0].body_property.as_deref(),
            Some("text")
        );
        assert_eq!(template.get_nodes().len(), 1);
    }
    for source in [
        "<Text/>",
        "<Text></Text>",
        "<Text>\n  \n</Text>",
        "<Group>\n</Group>",
    ] {
        assert!(body_values(&parse(source)).is_empty());
    }
    assert_eq!(parse("<Group>slot(0)</Group>").get_nodes().len(), 2);
    assert_eq!(parse("<Group>// a comment\n</Group>").get_nodes().len(), 2);
}

#[test]
fn undeclared_receivers_and_duplicate_assignments_fail() {
    for source in [
        "<Group>Hello</Group>",
        "<Text text=\"old\">new</Text>",
        "<Text text=bind:title>{self.title}</Text>",
        "<Text>&unknown;</Text>",
    ] {
        assert!(
            std::panic::catch_unwind(|| parse(source)).is_err(),
            "{source}"
        );
    }
}

#[test]
fn receiver_validation_is_shared_by_macro_and_static_analysis() {
    for (declaration, component, valid) in [
        ("struct Notice { #[pax(content)] message: Property<String> }", true, true),
        ("struct Notice { #[pax(content)] message: Property<String> }", false, false),
        ("struct Notice { #[pax(content)] message: String }", true, false),
        ("struct Notice { #[pax(content)] message: Property<bool> }", true, false),
        ("struct Notice { #[pax(content)] a: Property<String>, #[pax(content)] b: Property<String> }", true, false),
        ("struct Notice { #[pax(content)] #[pax(rust)] a: Property<String> }", true, false),
    ] {
        let item: syn::ItemStruct = syn::parse_str(declaration).unwrap();
        assert_eq!(validate_content_receiver(&item.fields, component).is_ok(), valid, "{declaration}");
    }
}

#[test]
fn rich_and_baked_roundtrips_keep_only_required_body_data() {
    let template = parse("<Text>hello &#123;🚀&#125;</Text>");
    let id = TypeId::build_singleton("test::Group", Some("Group"));
    let mut manifest = PaxManifest {
        components: Default::default(),
        main_component_type_id: id.clone(),
        type_table: HashMap::new(),
        assets_dirs: vec![],
        engine_import_path: "pax_engine".into(),
    };
    manifest.components.insert(
        id.clone(),
        ComponentDefinition {
            type_id: id.clone(),
            is_main_component: true,
            is_primitive: false,
            is_struct_only_component: false,
            module_path: "test".into(),
            primitive_instance_import_path: None,
            template: Some(template),
            settings: None,
            timelines: vec![],
            route_branch: None,
        },
    );
    let expected = body_values(manifest.components[&id].template.as_ref().unwrap());
    let text = TypeId::build_singleton("test::Text", Some("Text"));
    manifest.type_table.insert(
        text.clone(),
        TypeDefinition {
            type_id: text.clone(),
            property_definitions: vec![PropertyDefinition {
                name: "text".into(),
                type_id: TypeId::build_primitive("String"),
                flags: PropertyDefinitionFlags {
                    is_content: true,
                    is_property_wrapped: true,
                    ..Default::default()
                },
            }],
            ..Default::default()
        },
    );
    let rich = binary::from_slice(&binary::to_vec(&manifest).unwrap()).unwrap();
    assert_eq!(
        content_properties(&rich.type_table)
            .get(&text)
            .map(String::as_str),
        Some("text")
    );
    assert_eq!(
        body_values(rich.components[&id].template.as_ref().unwrap()),
        expected
    );
    assert_eq!(
        rich.components[&id].template.as_ref().unwrap().get_nodes()[0]
            .body_property
            .as_deref(),
        Some("text")
    );
    let ir = program_ir::ProgramIR::from_manifest(&manifest);
    let baked = program_ir::binary::from_slice(&program_ir::binary::to_vec(&ir).unwrap()).unwrap();
    assert!(
        !baked.type_table[&text].property_definitions[0]
            .flags
            .is_content
    );
    let template = baked.components[&id].template.as_ref().unwrap();
    assert_eq!(body_values(template), expected);
    assert_eq!(template.get_nodes()[0].body_property, None);
}

#[cfg(feature = "code_serialization")]
#[test]
fn source_edits_serialize_current_values_not_stale_body_text() {
    for replacement in [
        "\n  new <Text> & \\\\n \"\n",
        "  inline  ",
        "slot(0)",
        "/* text */",
        "",
        "🚀 & {name}",
    ] {
        let mut template = parse("<Text>old</Text>");
        for node in template.get_nodes_mut() {
            node.settings = Some(vec![SettingElement::Setting(
                Token::new_without_location("text".into()),
                ValueDefinition::LiteralValue(PaxValue::String(replacement.into())),
            )]);
        }
        let expected = body_values(&template);
        let component = ComponentDefinition {
            type_id: TypeId::build_singleton("test::Group", Some("Group")),
            is_main_component: true,
            is_primitive: false,
            is_struct_only_component: false,
            module_path: "test".into(),
            primitive_instance_import_path: None,
            template: Some(template),
            settings: None,
            timelines: vec![],
            route_branch: None,
        };
        let source = code_serialization::press_code_serialization_template(component).unwrap();
        assert_eq!(body_values(&parse(&source)), expected, "{source}");
        assert!(!source.contains("text="), "{source}");
    }
}
