use pax_manifest::pax_runtime_api::{Color, Duration, PaxValue};
use pax_manifest::*;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;

fn token(value: &str) -> Token {
    Token::new(
        value.into(),
        LocationInfo {
            start_line_col: (3, 5),
            end_line_col: (3, 17),
        },
    )
}

fn manifest() -> PaxManifest {
    let root = TypeId::build_singleton("example::left::Main", Some("Main"));
    let child = TypeId::build_singleton("example::right::Main", Some("Main"));
    let expression = ExpressionInfo::new(
        pax_language::parse_pax_expression("self.items[self.index] + 2").unwrap(),
    );
    let parent_id = TemplateNodeId::build(0);
    let child_id = TemplateNodeId::build(1);
    let comment_id = TemplateNodeId::build(2);
    let mut parent = TemplateNodeDefinition::default();
    parent.type_id = child.clone();
    parent.control_flow_settings = Some(ControlFlowSettingsDefinition {
        condition_expression: Some(expression.clone()),
        ..Default::default()
    });
    parent.settings = Some(vec![
        SettingElement::Comment("inline comment".into()),
        SettingElement::Setting(token("value"), ValueDefinition::Expression(expression)),
        SettingElement::Setting(
            token("fill"),
            ValueDefinition::LiteralValue(PaxValue::Color(Box::new(Color::RED))),
        ),
        SettingElement::Setting(
            token("nested"),
            ValueDefinition::Block(LiteralBlockDefinition {
                explicit_type_pascal_identifier: Some(token("Nested")),
                elements: vec![SettingElement::Setting(
                    token("title"),
                    ValueDefinition::LiteralValue(PaxValue::String("quoted \"text\"\nλ".into())),
                )],
            }),
        ),
    ]);
    let mut leaf = TemplateNodeDefinition::default();
    leaf.type_id = child.clone();
    let mut comment = TemplateNodeDefinition::default();
    comment.type_id = TypeId::build_comment();
    comment.raw_comment_string = Some("// source comment".into());
    let template = ComponentTemplate::from_parts(
        root.clone(),
        VecDeque::from([parent_id.clone()]),
        HashMap::from([(
            parent_id.clone(),
            VecDeque::from([comment_id.clone(), child_id.clone()]),
        )]),
        HashMap::from([(parent_id, parent), (child_id, leaf), (comment_id, comment)]),
        3,
        Some("src/quoted \"main\".pax".into()),
    );
    let mut main = ComponentDefinition {
        type_id: root.clone(),
        is_main_component: true,
        is_primitive: false,
        is_struct_only_component: false,
        module_path: "example::left".into(),
        primitive_instance_import_path: Some("example::MainInstance".into()),
        template: Some(template),
        settings: Some(vec![
            SettingsBlockElement::Comment("settings comment".into()),
            SettingsBlockElement::Handler(token("mount"), vec![token("mount_handler")]),
            SettingsBlockElement::SelectorBlock(
                token("#selected"),
                LiteralBlockDefinition {
                    explicit_type_pascal_identifier: None,
                    elements: vec![SettingElement::Setting(
                        token("value"),
                        ValueDefinition::Identifier(PaxIdentifier::new("self.value")),
                    )],
                },
            ),
        ]),
        timelines: vec![TimelineDefinition {
            name: Some(token("spin")),
            playhead: Some(ValueDefinition::Identifier(PaxIdentifier::new(
                "self.elapsed",
            ))),
            duration: Some(ValueDefinition::LiteralValue(PaxValue::Numeric(120.into()))),
            repeat: true,
            interruption: InOutInterruption::Restart,
            elements: vec![TimelineBlockElement::SelectorBlock(
                token("#selected"),
                TimelineSelectorBlockDefinition {
                    elements: vec![TimelineSelectorElement::Track(
                        token("rotate"),
                        TimelineTrackDefinition {
                            elements: vec![TimelineTrackElement::Keyframe(TimelineKeyframe {
                                marker: TimelineMarker::Frame(0),
                                value: ValueDefinition::LiteralValue(PaxValue::Numeric(5.into())),
                                easing: Some(token("Linear")),
                            })],
                            ..Default::default()
                        },
                    )],
                },
            )],
        }],
        route_branch: Some(RouteBranchDescriptor {
            path_property: "pattern".into(),
            default_property: "fallback".into(),
            modal: true,
        }),
    };
    // The compiled Rust representation must retain duration units used when
    // runtime cartridge instantiation derives lifecycle completion thresholds.
    let mut exit = main.timelines[0].clone();
    exit.name = Some(token("exit"));
    exit.duration = Some(ValueDefinition::LiteralValue(PaxValue::Duration(
        Duration::Frames(60.into()),
    )));
    let TimelineBlockElement::SelectorBlock(_, block) = &mut exit.elements[0] else {
        unreachable!()
    };
    let TimelineSelectorElement::Track(_, track) = &mut block.elements[0] else {
        unreachable!()
    };
    track.duration = Some(Box::new(ValueDefinition::LiteralValue(PaxValue::Duration(
        Duration::Milliseconds(1000.into()),
    ))));
    main.timelines.push(exit);
    main.settings.as_mut().unwrap().push(
        SettingsBlockElement::Transition(token("out"), token("exit")),
    );
    let mut other = main.clone();
    other.type_id = child.clone();
    other.is_main_component = false;
    other.is_primitive = true;
    other.module_path = "example::right".into();
    other.template = None;
    other.settings = None;
    other.timelines.clear();
    other.route_branch = None;
    let definition = |id: TypeId| TypeDefinition {
        type_id: id,
        inner_iterable_type_id: Some(TypeId::build_primitive("f64")),
        property_definitions: vec![PropertyDefinition {
            name: "value".into(),
            flags: PropertyDefinitionFlags {
                is_property_wrapped: true,
                ..Default::default()
            },
            type_id: TypeId::build_primitive("f64"),
        }],
    };
    PaxManifest {
        components: BTreeMap::from([(root.clone(), main), (child.clone(), other)]),
        main_component_type_id: root.clone(),
        type_table: HashMap::from([
            (root.clone(), definition(root)),
            (child.clone(), definition(child)),
        ]),
        assets_dirs: vec!["assets/fonts".into(), "other/assets".into()],
        engine_import_path: "crate::renamed_engine".into(),
    }
}

fn check_helpers(expression: &str, expected: usize) {
    let syn::Expr::Block(block) = syn::parse_str::<syn::Expr>(expression).unwrap() else {
        panic!("manifest constructors must scope their helpers inside an expression block");
    };
    let helpers: Vec<_> = block
        .block
        .stmts
        .iter()
        .filter_map(|statement| match statement {
            syn::Stmt::Item(syn::Item::Fn(function)) => Some(function),
            _ => None,
        })
        .collect();
    assert_eq!(helpers.len(), expected);
    for helper in helpers {
        assert!(matches!(helper.vis, syn::Visibility::Inherited));
        assert!(helper.attrs.iter().any(|attribute| {
            attribute.path.is_ident("inline")
                && attribute.parse_args::<syn::Ident>().unwrap() == "never"
        }));
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let output = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let mut manifest = manifest();
    let expression = rust_manifest::to_rust_expression(&manifest);
    check_helpers(
        &expression,
        manifest.components.len() + manifest.type_table.len(),
    );
    let mut types: Vec<_> = manifest.type_table.drain().collect();
    types.sort_by(|(left, _), (right, _)| right.cmp(left));
    manifest.type_table = types.into_iter().collect();
    assert_eq!(expression, rust_manifest::to_rust_expression(&manifest));
    std::fs::write(
        output.join("expected.bin"),
        binary::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    manifest.components.clear();
    manifest.type_table.clear();
    manifest.assets_dirs.clear();
    let empty = rust_manifest::to_rust_expression(&manifest);
    check_helpers(&empty, 0);
    std::fs::write(output.join("empty.bin"), binary::to_vec(&manifest).unwrap()).unwrap();
    std::fs::write(output.join("generated.rs"), format!(
        "fn construct_pair() -> (pax_manifest::PaxManifest, pax_manifest::PaxManifest) {{ ({expression}, {expression}) }}\nfn construct_empty() -> pax_manifest::PaxManifest {{ {empty} }}\n"
    )).unwrap();
}
