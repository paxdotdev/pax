#[cfg(test)]
#[cfg(feature = "parsing")]
mod tests {
    use std::collections::{BTreeMap, HashMap};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use pax_language::{parse_pax_str, Rule};
    use pax_manifest::pax_runtime_api::PaxValue;
    use pax_manifest::{
        parsing::{
            assemble_component_definition, parse_settings_from_component_definition_string,
            parse_timeline_from_component_definition_string, ParsingContext,
        },
        utils, ComponentDefinition, ControlFlowConditionalBranchKind,
        ControlFlowRepeatPredicateDefinition, PaxIdentifier, PaxManifest, SettingElement,
        SettingsBlockElement, TemplateNodeDefinition, TimelineBlockElement, Token, TypeId,
        ValueDefinition,
    };

    #[cfg(feature = "code_serialization")]
    use pax_manifest::code_serialization::press_code_serialization_template;
    #[cfg(feature = "code_serialization")]
    use pax_manifest::{
        ComponentTemplate, TimelineDefinition, TimelineKeyframe, TimelineMarker,
        TimelineSelectorBlockDefinition, TimelineSelectorElement, TimelineTrackDefinition,
        TimelineTrackElement,
    };

    fn write_temp_rust_source(contents: &str) -> std::path::PathBuf {
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "pax-manifest-implicit-handlers-{}-{}.rs",
            std::process::id(),
            unique_suffix
        ));
        fs::write(&path, contents).expect("temporary Rust source should be writable");
        path
    }

    fn lifecycle_bindings(component: &ComponentDefinition) -> Vec<(String, String)> {
        component
            .settings
            .as_ref()
            .into_iter()
            .flatten()
            .filter_map(|setting| match setting {
                pax_manifest::SettingsBlockElement::Handler(event, handlers)
                    if handlers.len() == 1 =>
                {
                    Some((event.token_value.clone(), handlers[0].token_value.clone()))
                }
                _ => None,
            })
            .collect()
    }

    fn assemble_component_definition_with_inferred_rust_source(
        ctx: ParsingContext,
        pax: &str,
        is_main_component: bool,
        template_map: HashMap<String, TypeId>,
        module_path: &str,
        self_type_id: TypeId,
        template_source_file_path: &str,
    ) -> (ParsingContext, ComponentDefinition) {
        let rust_source_file_path = template_source_file_path
            .strip_suffix(".pax")
            .map(|path| format!("{path}.rs"))
            .unwrap_or_else(|| "example.rs".to_string());
        assemble_component_definition(
            ctx,
            pax,
            is_main_component,
            template_map,
            module_path,
            self_type_id,
            template_source_file_path,
            &rust_source_file_path,
        )
    }

    #[test]
    fn test_parse_empty() {
        assert!(matches!(utils::parse_value(""), Ok(None)));
    }

    #[test]
    fn test_parse_identifier() {
        let res = utils::parse_value("identifier");
        if let Ok(Some(ValueDefinition::Identifier(ident))) = res {
            assert_eq!(&ident.name, "identifier");
        } else {
            panic!("unexpected result: {:?}", res);
        }
    }

    #[test]
    fn test_parse_literal_number() {
        let res = utils::parse_value("5");
        if let Ok(Some(ValueDefinition::LiteralValue(pv))) = res {
            assert_eq!(&pv.to_string(), "5");
        } else {
            panic!("unexpected result: {:?}", res);
        }
    }

    #[test]
    fn test_parse_expression() {
        let res = utils::parse_value("{5 + 3}");
        if let Ok(Some(ValueDefinition::Expression(info))) = res {
            assert_eq!(&info.expression.to_string(), "5 + 3");
        } else {
            panic!("unexpected result: {:?}", res);
        }
    }

    #[test]
    fn test_parse_ternary_expression() {
        let res = utils::parse_value("{is_selected ? 1 : 0}");
        if let Ok(Some(ValueDefinition::Expression(info))) = res {
            assert_eq!(&info.expression.to_string(), "is_selected ? 1 : 0");
        } else {
            panic!("unexpected result: {:?}", res);
        }
    }

    #[test]
    fn test_parse_null_coalesce_expression() {
        let res = utils::parse_value("{maybe_title ?? \"Untitled\"}");
        if let Ok(Some(ValueDefinition::Expression(info))) = res {
            assert_eq!(&info.expression.to_string(), "maybe_title ?? \"Untitled\"");
        } else {
            panic!("unexpected result: {:?}", res);
        }
    }

    #[test]
    fn test_parse_inline_timeline() {
        let res = utils::parse_value("@timeline { 0: 0, Linear, 100%: 1 }");
        if let Ok(Some(ValueDefinition::Timeline(track))) = res {
            let keyframes: Vec<_> = track.keyframes().collect();
            assert_eq!(keyframes.len(), 2);
        } else {
            panic!("unexpected result: {:?}", res);
        }
    }

    #[test]
    fn test_parse_inline_timeline_with_local_timing() {
        let res =
            utils::parse_value("@timeline { duration: 90, loop: false, 0: 0, Linear, 100%: 1 }");
        if let Ok(Some(ValueDefinition::Timeline(track))) = res {
            let keyframes: Vec<_> = track.keyframes().collect();
            assert!(matches!(
                track.duration.as_deref(),
                Some(ValueDefinition::LiteralValue(PaxValue::Numeric(value))) if value.to_int() == 90
            ));
            assert_eq!(track.repeat, Some(false));
            assert_eq!(keyframes.len(), 2);
        } else {
            panic!("unexpected result: {:?}", res);
        }
    }

    #[test]
    #[should_panic(expected = "timeline setting `frames` has been removed")]
    fn test_parse_inline_timeline_rejects_frames_setting() {
        let _ = utils::parse_value("@timeline { frames: 90, 0: 0, 100%: 1 }");
    }

    #[test]
    fn test_parse_inline_timeline_with_duration_units() {
        let res = utils::parse_value("@timeline { duration: 250ms, 0: 0, Linear, 250ms: 1 }");
        if let Ok(Some(ValueDefinition::Timeline(track))) = res {
            assert!(matches!(
                track.duration.as_deref(),
                Some(ValueDefinition::LiteralValue(PaxValue::Duration(
                    pax_manifest::pax_runtime_api::Duration::Milliseconds(value)
                ))) if value.to_int() == 250
            ));
            let keyframes: Vec<_> = track.keyframes().collect();
            assert!(matches!(
                &keyframes[1].marker,
                TimelineMarker::Duration(
                    pax_manifest::pax_runtime_api::Duration::Milliseconds(value)
                ) if value.to_int() == 250
            ));
        } else {
            panic!("unexpected result: {:?}", res);
        }
    }

    #[test]
    fn test_parse_inline_timeline_with_duration_expression() {
        let res = utils::parse_value("@timeline { duration: {(100 + offset)ms}, 0: 0, 100%: 1 }");
        if let Ok(Some(ValueDefinition::Timeline(track))) = res {
            assert!(matches!(
                track.duration.as_deref(),
                Some(ValueDefinition::Expression(info)) if info.expression.to_string() == "(100 + offset)ms"
            ));
        } else {
            panic!("unexpected result: {:?}", res);
        }
    }

    #[test]
    fn test_parse_inline_timeline_with_playhead_binding() {
        let res = utils::parse_value("@timeline { playhead: self.phase, 0: 0, 100%: 1 }");
        if let Ok(Some(ValueDefinition::Timeline(track))) = res {
            assert!(matches!(
                track.playhead.as_deref(),
                Some(ValueDefinition::Identifier(identifier)) if identifier.name == "self.phase"
            ));
        } else {
            panic!("unexpected result: {:?}", res);
        }
    }

    #[test]
    fn test_parse_multiple_named_timeline_blocks() {
        let component = parse_pax_str(
            Rule::pax_component_definition,
            r#"
                <Group />

                @timeline scene {
                    playhead: self.scene_t,
                    self {
                        breeze_t: {
                            0: 0,
                            100: 24,
                        },
                    }
                }

                @timeline breeze {
                    playhead: self.breeze_t,
                    .leaf {
                        rotate: {
                            0%: -6deg,
                            100%: 6deg,
                        },
                    }
                }
            "#,
        )
        .expect("component should parse");

        let timelines = parse_timeline_from_component_definition_string(component);
        assert_eq!(timelines.len(), 2);
        assert_eq!(
            timelines[0]
                .name
                .as_ref()
                .map(|token| token.token_value.as_str()),
            Some("scene")
        );
        assert!(matches!(
            timelines[0].playhead.as_ref(),
            Some(ValueDefinition::Identifier(identifier)) if identifier.name == "self.scene_t"
        ));
        assert!(matches!(
            timelines[0].elements.first(),
            Some(TimelineBlockElement::SelectorBlock(token, _)) if token.token_value == "self"
        ));
        assert_eq!(
            timelines[1]
                .name
                .as_ref()
                .map(|token| token.token_value.as_str()),
            Some("breeze")
        );
    }

    #[test]
    fn test_parse_transition_bindings_from_settings_block() {
        let component = parse_pax_str(
            Rule::pax_component_definition,
            r#"
                <Group />

                @settings {
                    @in: enter,
                    @out: exit,
                }
            "#,
        )
        .expect("component should parse");

        let settings = parse_settings_from_component_definition_string(component);
        assert!(matches!(
            &settings[0],
            SettingsBlockElement::Transition(key, value)
                if key.token_value == "in" && value.token_value == "enter"
        ));
        assert!(matches!(
            &settings[1],
            SettingsBlockElement::Transition(key, value)
                if key.token_value == "out" && value.token_value == "exit"
        ));
    }

    #[test]
    fn test_transition_timeline_blocks_lower_to_transition_properties() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let text_type_id = TypeId::build_singleton("Text", Some("Text"));
        let mut template_map = HashMap::new();
        template_map.insert("Text".to_string(), text_type_id);

        let pax = r#"
            <Text id=hero opacity=0.5 />

            @settings {
                @in: enter,
                @out: exit,
            }

            @timeline enter {
                #hero {
                    opacity: {
                        0: 0,
                        10: 1,
                    },
                }
            }

            @timeline exit {
                #hero {
                    opacity: {
                        0: 1,
                        10: 0,
                    },
                }
            }
        "#;

        let (_, component) = assemble_component_definition(
            ParsingContext::default(),
            pax,
            false,
            template_map,
            "crate",
            component_type_id.clone(),
            "example.pax",
            file!(),
        );
        let mut components = BTreeMap::new();
        components.insert(component_type_id.clone(), component);
        let manifest = PaxManifest {
            components,
            main_component_type_id: component_type_id.clone(),
            type_table: Default::default(),
            assets_dirs: vec![],
            engine_import_path: "pax_kit::pax_engine".to_string(),
        };

        let component = manifest.components.get(&component_type_id).unwrap();
        let template = component.template.as_ref().unwrap();
        let root_id = template.get_root().remove(0);
        let node = template.get_node(&root_id).unwrap();
        let common = manifest.get_inline_common_properties(&component_type_id, node);

        match common.get("opacity") {
            Some(ValueDefinition::Transition(transition)) => {
                assert!(transition.enter.is_some());
                assert!(transition.exit.is_some());
                assert!(matches!(
                    transition.starting_value.as_deref(),
                    Some(ValueDefinition::LiteralValue(PaxValue::Numeric(_)))
                ));
            }
            other => panic!("expected transition opacity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_with_extra() {
        let res = utils::parse_value("{5 + 3}this_shouldn't succeed");
        assert!(matches!(res, Err(_)));
    }

    #[test]
    fn test_parse_if_else_if_else_template_branches() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let text_type_id = TypeId::build_singleton("Text", Some("Text"));
        let rectangle_type_id = TypeId::build_singleton("Rectangle", Some("Rectangle"));
        let group_type_id = TypeId::build_singleton("Group", Some("Group"));
        let mut template_map = HashMap::new();
        template_map.insert("Text".to_string(), text_type_id.clone());
        template_map.insert("Rectangle".to_string(), rectangle_type_id);
        template_map.insert("Group".to_string(), group_type_id);

        let pax = r#"
            if self.mode == 0 {
                <Text id=playing />
            } else if self.mode == 1 {
                <Rectangle id=game_over_backdrop />
                <Text id=game_over />
            } else {
                <Group id=error />
            }
        "#;

        let (_, component) = assemble_component_definition(
            ParsingContext::default(),
            pax,
            false,
            template_map,
            "crate",
            component_type_id,
            "example.pax",
            file!(),
        );

        let template = component.template.unwrap();
        let if_id = template.get_root().remove(0);
        let if_node = template.get_node(&if_id).unwrap();
        let control_flow_settings = if_node.control_flow_settings.as_ref().unwrap();

        assert_eq!(control_flow_settings.conditional_branches.len(), 3);
        assert!(matches!(
            control_flow_settings.conditional_branches[0].branch_kind,
            ControlFlowConditionalBranchKind::If
        ));
        assert!(matches!(
            control_flow_settings.conditional_branches[1].branch_kind,
            ControlFlowConditionalBranchKind::ElseIf
        ));
        assert!(matches!(
            control_flow_settings.conditional_branches[2].branch_kind,
            ControlFlowConditionalBranchKind::Else
        ));
        assert_eq!(
            control_flow_settings.conditional_branches[0]
                .condition_expression
                .as_ref()
                .unwrap()
                .expression
                .to_string(),
            "mode == 0"
        );
        assert_eq!(
            control_flow_settings.conditional_branches[1]
                .condition_expression
                .as_ref()
                .unwrap()
                .expression
                .to_string(),
            "mode == 1"
        );
        assert!(control_flow_settings.conditional_branches[2]
            .condition_expression
            .is_none());
        assert_eq!(
            control_flow_settings.conditional_branches[0]
                .child_ids
                .len(),
            1
        );
        assert_eq!(
            control_flow_settings.conditional_branches[1]
                .child_ids
                .len(),
            2
        );
        assert_eq!(
            control_flow_settings.conditional_branches[2]
                .child_ids
                .len(),
            1
        );
        assert_eq!(template.get_children(&if_id).unwrap().len(), 4);
    }

    #[test]
    fn test_parse_router_template() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let text_type_id = TypeId::build_singleton("Text", Some("Text"));
        let group_type_id = TypeId::build_singleton("Group", Some("Group"));
        let mut template_map = HashMap::new();
        template_map.insert("Text".to_string(), text_type_id);
        template_map.insert("Group".to_string(), group_type_id);

        let pax = r#"
            <Router>
                <Route path="/">
                    <Text id=home />
                </Route>
                <Route path="/settings/*">
                    <Group id=settings />
                    <Text id=settings_title />
                </Route>
                <Route default=true>
                    <Text id=missing />
                </Route>
            </Router>
        "#;

        let (_, component) = assemble_component_definition(
            ParsingContext::default(),
            pax,
            false,
            template_map,
            "crate",
            component_type_id,
            "example.pax",
            file!(),
        );

        let template = component.template.unwrap();
        let router_id = template.get_root().remove(0);
        let router_node = template.get_node(&router_id).unwrap();
        let control_flow_settings = router_node.control_flow_settings.as_ref().unwrap();

        assert!(matches!(
            router_node.type_id.get_pax_type(),
            pax_manifest::PaxType::Router
        ));
        assert_eq!(control_flow_settings.route_branches.len(), 3);
        assert_eq!(
            control_flow_settings.route_branches[0].path.as_deref(),
            Some("/")
        );
        assert_eq!(
            control_flow_settings.route_branches[1].path.as_deref(),
            Some("/settings/*")
        );
        assert!(control_flow_settings.route_branches[2].default);
        assert_eq!(control_flow_settings.route_branches[0].child_ids.len(), 1);
        assert_eq!(control_flow_settings.route_branches[1].child_ids.len(), 2);
        assert_eq!(control_flow_settings.route_branches[2].child_ids.len(), 1);
        assert_eq!(template.get_children(&router_id).unwrap().len(), 4);
    }

    #[test]
    fn test_parse_relative_nested_router_template() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let text_type_id = TypeId::build_singleton("Text", Some("Text"));
        let mut template_map = HashMap::new();
        template_map.insert("Text".to_string(), text_type_id);

        let pax = r#"
            <Router>
                <Route path="/teams/:team_id/*">
                    <Router>
                        <Route path="members/:member_id">
                            <Text id=member />
                        </Route>
                        <Route path="settings/*">
                            <Text id=settings />
                        </Route>
                    </Router>
                </Route>
            </Router>
        "#;

        let (_, component) = assemble_component_definition(
            ParsingContext::default(),
            pax,
            false,
            template_map,
            "crate",
            component_type_id,
            "example.pax",
            file!(),
        );

        let template = component.template.unwrap();
        let root_router_id = template.get_root().remove(0);
        let root_branch = template
            .get_node(&root_router_id)
            .unwrap()
            .control_flow_settings
            .as_ref()
            .unwrap()
            .route_branches[0]
            .child_ids[0]
            .clone();
        let nested_router_id = root_branch;
        assert!(matches!(
            template
                .get_node(&nested_router_id)
                .unwrap()
                .type_id
                .get_pax_type(),
            pax_manifest::PaxType::Router
        ));
        let nested_branches = &template
            .get_node(&nested_router_id)
            .unwrap()
            .control_flow_settings
            .as_ref()
            .unwrap()
            .route_branches;

        assert_eq!(
            nested_branches[0].path.as_deref(),
            Some("members/:member_id")
        );
        assert_eq!(nested_branches[1].path.as_deref(), Some("settings/*"));
    }

    #[test]
    fn test_parse_keyed_for_template() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let text_type_id = TypeId::build_singleton("Text", Some("Text"));
        let mut template_map = HashMap::new();
        template_map.insert("Text".to_string(), text_type_id);

        let pax = r#"
            for (item, i) in self.items key item.id {
                <Text text={item.label} />
            }
        "#;

        let (_, component) = assemble_component_definition_with_inferred_rust_source(
            ParsingContext::default(),
            pax,
            false,
            template_map,
            "crate",
            component_type_id,
            "example.pax",
        );

        let template = component.template.unwrap();
        let repeat_id = template.get_root().remove(0);
        let repeat_node = template.get_node(&repeat_id).unwrap();
        let control_flow_settings = repeat_node.control_flow_settings.as_ref().unwrap();

        assert!(matches!(
            control_flow_settings.repeat_predicate_definition.as_ref(),
            Some(ControlFlowRepeatPredicateDefinition::ElemIdIndexId(elem, i))
                if elem == "item" && i == "i"
        ));
        assert_eq!(
            control_flow_settings
                .repeat_key_expression
                .as_ref()
                .unwrap()
                .expression
                .to_string(),
            "item.id"
        );
    }

    #[test]
    #[cfg(feature = "code_serialization")]
    fn test_serialize_keyed_for_template() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let text_type_id = TypeId::build_singleton("Text", Some("Text"));
        let mut template_map = HashMap::new();
        template_map.insert("Text".to_string(), text_type_id);

        let pax = r#"
            for (item, i) in self.items key item.id {
                <Text text={item.label} />
            }
        "#;

        let (_, component) = assemble_component_definition_with_inferred_rust_source(
            ParsingContext::default(),
            pax,
            false,
            template_map.clone(),
            "crate",
            component_type_id.clone(),
            "example.pax",
        );

        let rendered = press_code_serialization_template(component).unwrap();
        assert!(rendered.contains("key item.id"));

        let (_, parsed_component) = assemble_component_definition_with_inferred_rust_source(
            ParsingContext::default(),
            &rendered,
            false,
            template_map,
            "crate",
            component_type_id,
            "example.pax",
        );
        let template = parsed_component.template.unwrap();
        let repeat_id = template.get_root().remove(0);
        let repeat_node = template.get_node(&repeat_id).unwrap();
        let control_flow_settings = repeat_node.control_flow_settings.as_ref().unwrap();

        assert!(control_flow_settings.repeat_key_expression.is_some());
    }

    #[test]
    #[cfg(feature = "code_serialization")]
    fn test_serialize_if_else_if_else_template_branches() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let text_type_id = TypeId::build_singleton("Text", Some("Text"));
        let rectangle_type_id = TypeId::build_singleton("Rectangle", Some("Rectangle"));
        let group_type_id = TypeId::build_singleton("Group", Some("Group"));
        let mut template_map = HashMap::new();
        template_map.insert("Text".to_string(), text_type_id.clone());
        template_map.insert("Rectangle".to_string(), rectangle_type_id);
        template_map.insert("Group".to_string(), group_type_id);

        let pax = r#"
            if self.mode == 0 {
                <Text id=playing />
            } else if self.mode == 1 {
                <Rectangle id=game_over_backdrop />
                <Text id=game_over />
            } else {
                <Group id=error />
            }
        "#;

        let (_, component) = assemble_component_definition(
            ParsingContext::default(),
            pax,
            false,
            template_map.clone(),
            "crate",
            component_type_id.clone(),
            "example.pax",
            file!(),
        );

        let rendered = press_code_serialization_template(component).unwrap();
        assert!(rendered.contains("else if mode == 1"));
        assert!(rendered.contains("else"));

        let (_, parsed_component) = assemble_component_definition(
            ParsingContext::default(),
            &rendered,
            false,
            template_map,
            "crate",
            component_type_id,
            "example.pax",
            file!(),
        );
        let template = parsed_component.template.unwrap();
        let if_id = template.get_root().remove(0);
        let branches = &template
            .get_node(&if_id)
            .unwrap()
            .control_flow_settings
            .as_ref()
            .unwrap()
            .conditional_branches;

        assert_eq!(branches.len(), 3);
        assert_eq!(branches[0].child_ids.len(), 1);
        assert_eq!(branches[1].child_ids.len(), 2);
        assert_eq!(branches[2].child_ids.len(), 1);
    }

    #[test]
    #[cfg(feature = "code_serialization")]
    fn test_serialize_router_template() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let text_type_id = TypeId::build_singleton("Text", Some("Text"));
        let group_type_id = TypeId::build_singleton("Group", Some("Group"));
        let mut template_map = HashMap::new();
        template_map.insert("Text".to_string(), text_type_id);
        template_map.insert("Group".to_string(), group_type_id);

        let pax = r#"
            <Router>
                <Route path="/">
                    <Text id=home />
                </Route>
                <Route path="/settings/*">
                    <Group id=settings />
                    <Text id=settings_title />
                </Route>
                <Route default=true>
                    <Text id=missing />
                </Route>
            </Router>
        "#;

        let (_, component) = assemble_component_definition(
            ParsingContext::default(),
            pax,
            false,
            template_map.clone(),
            "crate",
            component_type_id.clone(),
            "example.pax",
            file!(),
        );

        let rendered = press_code_serialization_template(component).unwrap();
        assert!(rendered.contains("<Router>"));
        assert!(rendered.contains(r#"<Route path="/settings/*">"#));
        assert!(rendered.contains("<Route default=true>"));

        let (_, parsed_component) = assemble_component_definition(
            ParsingContext::default(),
            &rendered,
            false,
            template_map,
            "crate",
            component_type_id,
            "example.pax",
            file!(),
        );
        let template = parsed_component.template.unwrap();
        let router_id = template.get_root().remove(0);
        let branches = &template
            .get_node(&router_id)
            .unwrap()
            .control_flow_settings
            .as_ref()
            .unwrap()
            .route_branches;

        assert_eq!(branches.len(), 3);
        assert_eq!(branches[0].child_ids.len(), 1);
        assert_eq!(branches[1].child_ids.len(), 2);
        assert_eq!(branches[2].child_ids.len(), 1);
        assert!(branches[2].default);
    }

    #[test]
    #[cfg(feature = "code_serialization")]
    fn test_serialize_ternary_expression() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let rectangle_type_id = TypeId::build_singleton("Rectangle", Some("Rectangle"));
        let mut template_map = HashMap::new();
        template_map.insert("Rectangle".to_string(), rectangle_type_id);

        let pax = r#"
            <Rectangle fill={is_selected ? WHITE : GRAY} />
        "#;

        let (_, component) = assemble_component_definition(
            ParsingContext::default(),
            pax,
            false,
            template_map.clone(),
            "crate",
            component_type_id.clone(),
            "example.pax",
            file!(),
        );

        let rendered = press_code_serialization_template(component).unwrap();
        assert!(rendered.contains("fill={is_selected ? WHITE : GRAY}"));

        let (_, parsed_component) = assemble_component_definition(
            ParsingContext::default(),
            &rendered,
            false,
            template_map,
            "crate",
            component_type_id,
            "example.pax",
            file!(),
        );
        let template = parsed_component.template.unwrap();
        let rectangle_id = template.get_root().remove(0);
        let rectangle_node = template.get_node(&rectangle_id).unwrap();
        let settings = rectangle_node.settings.as_ref().unwrap();
        let fill = settings.iter().find_map(|setting| match setting {
            SettingElement::Setting(token, value) if token.token_value == "fill" => Some(value),
            _ => None,
        });

        assert!(matches!(
            fill,
            Some(ValueDefinition::Expression(info))
                if info.expression.to_string() == "is_selected ? WHITE : GRAY"
        ));
    }

    #[test]
    #[cfg(feature = "code_serialization")]
    fn test_serialize_null_coalesce_expression() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let text_type_id = TypeId::build_singleton("Text", Some("Text"));
        let mut template_map = HashMap::new();
        template_map.insert("Text".to_string(), text_type_id);

        let pax = r#"
            <Text text={maybe_title ?? "Untitled"} />
        "#;

        let (_, component) = assemble_component_definition(
            ParsingContext::default(),
            pax,
            false,
            template_map.clone(),
            "crate",
            component_type_id.clone(),
            "example.pax",
            file!(),
        );

        let rendered = press_code_serialization_template(component).unwrap();
        assert!(rendered.contains("text={maybe_title ?? \"Untitled\"}"));

        let (_, parsed_component) = assemble_component_definition(
            ParsingContext::default(),
            &rendered,
            false,
            template_map,
            "crate",
            component_type_id,
            "example.pax",
            file!(),
        );
        let template = parsed_component.template.unwrap();
        let text_id = template.get_root().remove(0);
        let text_node = template.get_node(&text_id).unwrap();
        let settings = text_node.settings.as_ref().unwrap();
        let text = settings.iter().find_map(|setting| match setting {
            SettingElement::Setting(token, value) if token.token_value == "text" => Some(value),
            _ => None,
        });

        assert!(matches!(
            text,
            Some(ValueDefinition::Expression(info))
                if info.expression.to_string() == "maybe_title ?? \"Untitled\""
        ));
    }

    #[test]
    #[cfg(feature = "code_serialization")]
    fn test_serialize_timeline_block() {
        let component = ComponentDefinition {
            type_id: TypeId::build_singleton("Example", Some("Example")),
            is_main_component: false,
            is_primitive: false,
            is_struct_only_component: false,
            module_path: "example".to_string(),
            primitive_instance_import_path: None,
            template: None,
            settings: None,
            timelines: vec![TimelineDefinition {
                name: Some(Token::new_without_location("orbital".to_string())),
                playhead: Some(ValueDefinition::Identifier(
                    pax_manifest::PaxIdentifier::new("self.phase"),
                )),
                duration: Some(ValueDefinition::LiteralValue(PaxValue::Numeric(120.into()))),
                repeat: true,
                elements: vec![TimelineBlockElement::SelectorBlock(
                    Token::new_without_location("#orb".to_string()),
                    TimelineSelectorBlockDefinition {
                        elements: vec![TimelineSelectorElement::Track(
                            Token::new_without_location("opacity".to_string()),
                            TimelineTrackDefinition {
                                elements: vec![
                                    TimelineTrackElement::Keyframe(TimelineKeyframe {
                                        marker: TimelineMarker::Frame(0),
                                        value: ValueDefinition::LiteralValue(PaxValue::Numeric(
                                            0.4.into(),
                                        )),
                                        easing: Some(Token::new_without_location(
                                            "Linear".to_string(),
                                        )),
                                    }),
                                    TimelineTrackElement::Keyframe(TimelineKeyframe {
                                        marker: TimelineMarker::Percent(100.0),
                                        value: ValueDefinition::LiteralValue(PaxValue::Numeric(
                                            1.0.into(),
                                        )),
                                        easing: None,
                                    }),
                                ],
                                playhead: None,
                                duration: None,
                                repeat: None,
                                starting_value: None,
                                use_local_property_scope: false,
                            },
                        )],
                    },
                )],
            }],
        };

        let rendered = press_code_serialization_template(component).unwrap();
        assert!(rendered.contains("@timeline orbital"));
        assert!(rendered.contains("playhead: self.phase"));
        assert!(rendered.contains("duration: 120"));
        assert!(rendered.contains("0: 0.40, Linear"));
        assert!(rendered.contains("100%: 1.00"));
    }

    #[test]
    #[cfg(feature = "code_serialization")]
    fn test_round_trip_latest_timeline_syntax() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let text_type_id = TypeId::build_singleton("Text", Some("Text"));
        let mut template =
            ComponentTemplate::new(component_type_id.clone(), Some("example.pax".to_string()));
        template.add(TemplateNodeDefinition {
            type_id: text_type_id.clone(),
            control_flow_settings: None,
            settings: Some(vec![
                SettingElement::Setting(
                    Token::new_without_location("id".to_string()),
                    ValueDefinition::Identifier(PaxIdentifier::new("caption")),
                ),
                SettingElement::Setting(
                    Token::new_without_location("opacity".to_string()),
                    ValueDefinition::Timeline(TimelineTrackDefinition {
                        elements: vec![
                            TimelineTrackElement::Keyframe(TimelineKeyframe {
                                marker: TimelineMarker::Duration(
                                    pax_manifest::pax_runtime_api::Duration::Milliseconds(0.into()),
                                ),
                                value: ValueDefinition::LiteralValue(PaxValue::Numeric(
                                    0.35.into(),
                                )),
                                easing: Some(Token::new_without_location("Linear".to_string())),
                            }),
                            TimelineTrackElement::Keyframe(TimelineKeyframe {
                                marker: TimelineMarker::Duration(
                                    pax_manifest::pax_runtime_api::Duration::Milliseconds(
                                        125.into(),
                                    ),
                                ),
                                value: ValueDefinition::LiteralValue(PaxValue::Numeric(1.0.into())),
                                easing: Some(Token::new_without_location("OutQuad".to_string())),
                            }),
                            TimelineTrackElement::Keyframe(TimelineKeyframe {
                                marker: TimelineMarker::Duration(
                                    pax_manifest::pax_runtime_api::Duration::Milliseconds(
                                        250.into(),
                                    ),
                                ),
                                value: ValueDefinition::LiteralValue(PaxValue::Numeric(
                                    0.35.into(),
                                )),
                                easing: None,
                            }),
                        ],
                        playhead: Some(Box::new(ValueDefinition::Identifier(PaxIdentifier::new(
                            "self.phase",
                        )))),
                        duration: Some(Box::new(ValueDefinition::LiteralValue(
                            PaxValue::Duration(
                                pax_manifest::pax_runtime_api::Duration::Milliseconds(250.into()),
                            ),
                        ))),
                        repeat: Some(false),
                        starting_value: None,
                        use_local_property_scope: false,
                    }),
                ),
            ]),
            selector_info: Default::default(),
            raw_comment_string: None,
        });

        let component = ComponentDefinition {
            type_id: component_type_id.clone(),
            is_main_component: false,
            is_primitive: false,
            is_struct_only_component: false,
            module_path: "example".to_string(),
            primitive_instance_import_path: None,
            template: Some(template),
            settings: None,
            timelines: vec![TimelineDefinition {
                name: None,
                playhead: None,
                duration: Some(ValueDefinition::LiteralValue(PaxValue::Numeric(120.into()))),
                repeat: true,
                elements: vec![TimelineBlockElement::SelectorBlock(
                    Token::new_without_location(".glow".to_string()),
                    TimelineSelectorBlockDefinition {
                        elements: vec![TimelineSelectorElement::Track(
                            Token::new_without_location("rotate".to_string()),
                            TimelineTrackDefinition {
                                elements: vec![
                                    TimelineTrackElement::Keyframe(TimelineKeyframe {
                                        marker: TimelineMarker::Percent(0.0),
                                        value: ValueDefinition::LiteralValue(PaxValue::Rotation(
                                            pax_manifest::pax_runtime_api::Rotation::Degrees(
                                                0.0.into(),
                                            ),
                                        )),
                                        easing: Some(Token::new_without_location(
                                            "Linear".to_string(),
                                        )),
                                    }),
                                    TimelineTrackElement::Keyframe(TimelineKeyframe {
                                        marker: TimelineMarker::Percent(50.0),
                                        value: ValueDefinition::LiteralValue(PaxValue::Rotation(
                                            pax_manifest::pax_runtime_api::Rotation::Degrees(
                                                8.0.into(),
                                            ),
                                        )),
                                        easing: Some(Token::new_without_location(
                                            "OutQuad".to_string(),
                                        )),
                                    }),
                                    TimelineTrackElement::Keyframe(TimelineKeyframe {
                                        marker: TimelineMarker::Percent(100.0),
                                        value: ValueDefinition::LiteralValue(PaxValue::Rotation(
                                            pax_manifest::pax_runtime_api::Rotation::Degrees(
                                                0.0.into(),
                                            ),
                                        )),
                                        easing: None,
                                    }),
                                ],
                                playhead: None,
                                duration: None,
                                repeat: None,
                                starting_value: None,
                                use_local_property_scope: false,
                            },
                        )],
                    },
                )],
            }],
        };

        let rendered = press_code_serialization_template(component).unwrap();
        assert!(rendered.contains("opacity=@timeline {"));
        assert!(rendered.contains("playhead: self.phase"));
        assert!(rendered.contains("duration: 250ms"));
        assert!(rendered.contains("loop: false"));
        assert!(rendered.contains("125ms: 1.00, OutQuad"));
        assert!(rendered.contains("@timeline {"));
        assert!(rendered.contains(".glow {"));

        let mut template_map = HashMap::new();
        template_map.insert("Text".to_string(), text_type_id);
        let (_, parsed_component) = assemble_component_definition(
            ParsingContext::default(),
            &rendered,
            false,
            template_map,
            "crate",
            component_type_id,
            "example.pax",
            file!(),
        );

        assert_eq!(parsed_component.timelines.len(), 1);
        assert!(matches!(
            &parsed_component.timelines[0].duration,
            Some(ValueDefinition::LiteralValue(PaxValue::Numeric(value))) if value.to_int() == 120
        ));
        assert!(parsed_component.timelines[0].repeat);

        let template = parsed_component.template.unwrap();
        let root_id = template.get_root().remove(0);
        let node = template.get_node(&root_id).unwrap();
        let opacity_track = node
            .settings
            .as_ref()
            .unwrap()
            .iter()
            .find_map(|setting| match setting {
                SettingElement::Setting(token, ValueDefinition::Timeline(track))
                    if token.token_value == "opacity" =>
                {
                    Some(track)
                }
                _ => None,
            })
            .expect("opacity inline timeline should round-trip");

        assert!(matches!(
            opacity_track.duration.as_deref(),
            Some(ValueDefinition::LiteralValue(PaxValue::Duration(
                pax_manifest::pax_runtime_api::Duration::Milliseconds(value)
            ))) if value.to_int() == 250
        ));
        assert_eq!(opacity_track.repeat, Some(false));
        assert!(matches!(
            opacity_track.playhead.as_deref(),
            Some(ValueDefinition::Identifier(identifier)) if identifier.name == "self.phase"
        ));
        assert_eq!(opacity_track.keyframes().count(), 3);
    }

    #[test]
    fn test_assemble_component_definition_adds_implicit_lifecycle_handlers() {
        let rust_source = write_temp_rust_source(
            r#"
            pub struct Example;

            impl Example {
                pub fn on_mount(&mut self, _ctx: &NodeContext) {}
                pub fn on_tick(&mut self, _ctx: &NodeContext) {}
                pub fn on_pre_render(&mut self, _ctx: &NodeContext) {}
                pub fn on_unmount(&mut self, _ctx: &NodeContext) {}
            }
            "#,
        );

        let group_type_id = TypeId::build_singleton("Group", Some("Group"));
        let mut template_map = HashMap::new();
        template_map.insert("Group".to_string(), group_type_id);

        let (_, component) = assemble_component_definition(
            ParsingContext::default(),
            "<Group />",
            false,
            template_map,
            "crate",
            TypeId::build_singleton("crate::Example", Some("Example")),
            "example.pax",
            rust_source.to_str().unwrap(),
        );

        let bindings = lifecycle_bindings(&component);
        assert!(bindings.contains(&("mount".to_string(), "on_mount".to_string())));
        assert!(bindings.contains(&("tick".to_string(), "on_tick".to_string())));
        assert!(bindings.contains(&("pre_render".to_string(), "on_pre_render".to_string())));
        assert!(bindings.contains(&("unmount".to_string(), "on_unmount".to_string())));

        fs::remove_file(rust_source).expect("temporary Rust source should be removable");
    }

    #[test]
    fn test_assemble_component_definition_prefers_explicit_lifecycle_bindings() {
        let rust_source = write_temp_rust_source(
            r#"
            pub struct Example;

            impl Example {
                pub fn on_mount(&mut self, _ctx: &NodeContext) {}
                pub fn tick(&mut self, _ctx: &NodeContext) {}
            }
            "#,
        );

        let group_type_id = TypeId::build_singleton("Group", Some("Group"));
        let mut template_map = HashMap::new();
        template_map.insert("Group".to_string(), group_type_id);

        let (_, component) = assemble_component_definition(
            ParsingContext::default(),
            r#"
                <Group />

                @settings {
                    @mount: custom_mount
                }
            "#,
            false,
            template_map,
            "crate",
            TypeId::build_singleton("crate::Example", Some("Example")),
            "example.pax",
            rust_source.to_str().unwrap(),
        );

        let bindings = lifecycle_bindings(&component);
        assert!(bindings.contains(&("mount".to_string(), "custom_mount".to_string())));
        assert!(!bindings.contains(&("mount".to_string(), "on_mount".to_string())));
        assert!(bindings.contains(&("tick".to_string(), "tick".to_string())));

        fs::remove_file(rust_source).expect("temporary Rust source should be removable");
    }

    #[test]
    fn test_runtime_settings_merge_uses_selector_metadata_precedence() {
        let mut node = TemplateNodeDefinition {
            type_id: TypeId::build_singleton("example::Text", Some("Text")),
            control_flow_settings: None,
            settings: Some(vec![
                SettingElement::Setting(
                    Token::new_without_location("class".to_string()),
                    ValueDefinition::Identifier(PaxIdentifier::new("headline")),
                ),
                SettingElement::Setting(
                    Token::new_without_location("id".to_string()),
                    ValueDefinition::Identifier(PaxIdentifier::new("hero")),
                ),
                SettingElement::Setting(
                    Token::new_without_location("fill".to_string()),
                    ValueDefinition::LiteralValue(PaxValue::Numeric(4.into())),
                ),
            ]),
            selector_info: Default::default(),
            raw_comment_string: None,
        };
        node.normalize_selector_info();

        let settings = Some(vec![
            SettingsBlockElement::SelectorBlock(
                Token::new_without_location("Text".to_string()),
                pax_manifest::LiteralBlockDefinition::new(vec![
                    SettingElement::Setting(
                        Token::new_without_location("width".to_string()),
                        ValueDefinition::LiteralValue(PaxValue::Numeric(1.into())),
                    ),
                    SettingElement::Setting(
                        Token::new_without_location("fill".to_string()),
                        ValueDefinition::LiteralValue(PaxValue::Numeric(1.into())),
                    ),
                ]),
            ),
            SettingsBlockElement::SelectorBlock(
                Token::new_without_location(".headline".to_string()),
                pax_manifest::LiteralBlockDefinition::new(vec![
                    SettingElement::Setting(
                        Token::new_without_location("height".to_string()),
                        ValueDefinition::LiteralValue(PaxValue::Numeric(2.into())),
                    ),
                    SettingElement::Setting(
                        Token::new_without_location("fill".to_string()),
                        ValueDefinition::LiteralValue(PaxValue::Numeric(2.into())),
                    ),
                ]),
            ),
            SettingsBlockElement::SelectorBlock(
                Token::new_without_location("#hero".to_string()),
                pax_manifest::LiteralBlockDefinition::new(vec![
                    SettingElement::Setting(
                        Token::new_without_location("opacity".to_string()),
                        ValueDefinition::LiteralValue(PaxValue::Numeric(3.into())),
                    ),
                    SettingElement::Setting(
                        Token::new_without_location("fill".to_string()),
                        ValueDefinition::LiteralValue(PaxValue::Numeric(3.into())),
                    ),
                ]),
            ),
        ]);

        let merged =
            PaxManifest::merge_inline_settings_with_settings_block(&node, &settings).unwrap();

        let merged_map = merged
            .into_iter()
            .filter_map(|setting| match setting {
                SettingElement::Setting(token, value) => Some((token.token_value, value)),
                SettingElement::Comment(_) => None,
            })
            .collect::<HashMap<_, _>>();

        assert!(matches!(
            merged_map.get("width"),
            Some(ValueDefinition::LiteralValue(PaxValue::Numeric(value))) if *value == 1.into()
        ));
        assert!(matches!(
            merged_map.get("height"),
            Some(ValueDefinition::LiteralValue(PaxValue::Numeric(value))) if *value == 2.into()
        ));
        assert!(matches!(
            merged_map.get("opacity"),
            Some(ValueDefinition::LiteralValue(PaxValue::Numeric(value))) if *value == 3.into()
        ));
        assert!(matches!(
            merged_map.get("fill"),
            Some(ValueDefinition::LiteralValue(PaxValue::Numeric(value))) if *value == 4.into()
        ));
    }
}
