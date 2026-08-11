#[cfg(test)]
#[cfg(feature = "parsing")]
mod tests {
    use std::collections::{BTreeMap, HashMap};
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use pax_language::{parse_pax_str, Rule};
    use pax_manifest::pax_runtime_api::{Numeric, PaxValue, Size};
    use pax_manifest::{
        parsing::{
            assemble_component_definition as assemble_component_definition_raw,
            parse_settings_from_component_definition_string,
            parse_timeline_from_component_definition_string, ParsingContext,
        },
        utils, ComponentDefinition, ControlFlowConditionalBranchKind,
        ControlFlowRepeatPredicateDefinition, GradientDefinition, GradientElement,
        GradientShapeDefinition, GradientStopDefinition, InOutInterruption, PaxIdentifier,
        PaxManifest, RouteBranchDescriptor, SettingElement, SettingsBlockElement,
        TemplateNodeDefinition, TemplateNodeId, TimelineBlockElement, TimelineMarker, Token,
        TypeId, ValueDefinition,
    };

    #[cfg(feature = "code_serialization")]
    use pax_manifest::code_serialization::press_code_serialization_template;
    #[cfg(feature = "code_serialization")]
    use pax_manifest::{
        ComponentTemplate, TimelineDefinition, TimelineKeyframe, TimelineSelectorBlockDefinition,
        TimelineSelectorElement, TimelineTrackDefinition, TimelineTrackElement,
    };

    fn write_temp_rust_source(contents: &str) -> std::path::PathBuf {
        static TEMP_SOURCE_COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique_suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = TEMP_SOURCE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "pax-manifest-implicit-handlers-{}-{}-{}.rs",
            std::process::id(),
            unique_suffix,
            counter
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

    fn template_node_id_by_id(
        template: &pax_manifest::ComponentTemplate,
        inline_id: &str,
    ) -> TemplateNodeId {
        template
            .get_ids()
            .into_iter()
            .find(|id| {
                template
                    .get_node(id)
                    .and_then(|node| node.selector_info.id.as_ref())
                    .map(|token| token.token_value.as_str() == inline_id)
                    .unwrap_or(false)
            })
            .cloned()
            .expect("template node id should exist")
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

    fn assemble_component_definition(
        ctx: ParsingContext,
        pax: &str,
        is_main_component: bool,
        template_map: HashMap<String, TypeId>,
        module_path: &str,
        self_type_id: TypeId,
        template_source_file_path: &str,
        rust_source_file_path: &str,
    ) -> (ParsingContext, ComponentDefinition) {
        assemble_component_definition_raw(
            ctx,
            pax,
            is_main_component,
            template_map,
            HashMap::new(),
            None,
            module_path,
            self_type_id,
            template_source_file_path,
            rust_source_file_path,
        )
    }

    fn assemble_component_definition_with_route_branches(
        ctx: ParsingContext,
        pax: &str,
        is_main_component: bool,
        template_map: HashMap<String, TypeId>,
        route_branch_descriptors: HashMap<TypeId, RouteBranchDescriptor>,
        module_path: &str,
        self_type_id: TypeId,
        template_source_file_path: &str,
        rust_source_file_path: &str,
    ) -> (ParsingContext, ComponentDefinition) {
        assemble_component_definition_raw(
            ctx,
            pax,
            is_main_component,
            template_map,
            route_branch_descriptors,
            None,
            module_path,
            self_type_id,
            template_source_file_path,
            rust_source_file_path,
        )
    }

    fn default_route_branch_descriptor() -> RouteBranchDescriptor {
        RouteBranchDescriptor {
            path_property: "path".to_string(),
            default_property: "default".to_string(),
            modal: false,
        }
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
    fn test_type_qualified_object_constructor_remains_supported() {
        let res = utils::parse_value(
            "CornerRadii { top_left: 12 top_right: 8 bottom_right: 4 bottom_left: 2 }",
        );
        if let Ok(Some(ValueDefinition::LiteralValue(PaxValue::Object(fields)))) = res {
            assert_eq!(fields.len(), 4);
            assert_eq!(fields[0].0, "top_left");
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
    fn test_parse_inline_gradient_default_linear() {
        let res = utils::parse_value("@gradient { 0%: RED, 100%: BLUE }");
        if let Ok(Some(ValueDefinition::Gradient(gradient))) = res {
            assert!(matches!(
                &gradient.shape,
                GradientShapeDefinition::Linear {
                    start: None,
                    end: None
                }
            ));
            let stops: Vec<_> = gradient.stops().collect();
            assert_eq!(stops.len(), 2);
            assert!((stops[0].position.expect_percent() - 0.0).abs() < 0.0001);
            assert!((stops[1].position.expect_percent() - 1.0).abs() < 0.0001);
            assert!(matches!(
                &stops[0].color,
                ValueDefinition::LiteralValue(PaxValue::Color(_))
            ));
        } else {
            panic!("unexpected result: {:?}", res);
        }
    }

    #[test]
    fn test_units_must_be_adjacent_to_numbers() {
        assert!(matches!(
            utils::parse_value("10px"),
            Ok(Some(ValueDefinition::LiteralValue(PaxValue::Size(_))))
        ));
        assert!(utils::parse_value("10 px").is_err());
    }

    #[test]
    fn test_parse_inline_gradient_explicit_linear() {
        let res = utils::parse_value(
            "@gradient { linear: { start: [0%, 50%] end: [100%, 50%] } 0%: rgba(255, 0, 0, 255), 100%: {self.active ? RED : BLUE} }",
        );
        if let Ok(Some(ValueDefinition::Gradient(gradient))) = res {
            assert!(matches!(
                &gradient.shape,
                GradientShapeDefinition::Linear {
                    start: Some(_),
                    end: Some(_)
                }
            ));
            let stops: Vec<_> = gradient.stops().collect();
            assert_eq!(stops.len(), 2);
            assert!(matches!(&stops[1].color, ValueDefinition::Expression(_)));
        } else {
            panic!("unexpected result: {:?}", res);
        }
    }

    #[test]
    fn test_parse_inline_gradient_radial() {
        let res = utils::parse_value(
            "@gradient { radial: { start: [50%, 50%] end: [50%, 50%] radius: 180 } 0%: WHITE, 100%: rgba(255, 255, 255, 0) }",
        );
        if let Ok(Some(ValueDefinition::Gradient(gradient))) = res {
            assert!(matches!(
                &gradient.shape,
                GradientShapeDefinition::Radial { .. }
            ));
            assert_eq!(gradient.stops().count(), 2);
        } else {
            panic!("unexpected result: {:?}", res);
        }
    }

    #[test]
    #[should_panic(expected = "@gradient supports only one shape block")]
    fn test_parse_inline_gradient_rejects_multiple_shapes() {
        let _ = utils::parse_value(
            "@gradient { linear: {} radial: { start: [50%, 50%] end: [50%, 50%] radius: 180 } 0%: WHITE, 100%: TRANSPARENT }",
        );
    }

    #[test]
    fn test_number_before_gradient_fill_does_not_consume_fill_as_frames_unit() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let ellipse_type_id = TypeId::build_singleton("Ellipse", Some("Ellipse"));
        let mut template_map = HashMap::new();
        template_map.insert("Ellipse".to_string(), ellipse_type_id);

        let pax = r#"
            <Ellipse
                opacity=1
                fill=@gradient {
                    0%: RED
                    100%: BLUE
                }
            />
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
        let root_id = template.get_root().remove(0);
        let node = template.get_node(&root_id).unwrap();
        let settings = node.settings.as_ref().expect("settings should parse");

        assert!(settings.iter().any(|setting| matches!(
            setting,
            SettingElement::Setting(token, ValueDefinition::LiteralValue(PaxValue::Numeric(_)))
                if token.token_value == "opacity"
        )));
        assert!(settings.iter().any(|setting| matches!(
            setting,
            SettingElement::Setting(token, ValueDefinition::Gradient(_))
                if token.token_value == "fill"
        )));
        assert!(!settings.iter().any(|setting| matches!(
            setting,
            SettingElement::Setting(token, _) if token.token_value == "ill"
        )));
    }

    #[test]
    fn test_static_class_list_lowers_to_one_selector_binding() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let rectangle_type_id = TypeId::build_singleton("Rectangle", Some("Rectangle"));
        let mut template_map = HashMap::new();
        template_map.insert("Rectangle".to_string(), rectangle_type_id);

        let (_, component) = assemble_component_definition(
            ParsingContext::default(),
            r#"<Rectangle class=["card", "elevated", "interactive"] />"#,
            false,
            template_map,
            "crate",
            component_type_id,
            "example.pax",
            file!(),
        );

        let template = component.template.unwrap();
        let root_id = template.get_root().remove(0);
        let node = template.get_node(&root_id).unwrap();
        let classes = node.selector_info.literal_classes();
        assert_eq!(classes, vec!["card", "elevated", "interactive"]);
        assert!(node.settings.is_none());
    }

    #[test]
    fn test_dynamic_class_expression_is_reserved_selector_metadata() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let rectangle_type_id = TypeId::build_singleton("Rectangle", Some("Rectangle"));
        let mut template_map = HashMap::new();
        template_map.insert("Rectangle".to_string(), rectangle_type_id);

        let (_, component) = assemble_component_definition(
            ParsingContext::default(),
            "<Rectangle class={self.current_classes} opacity=0.5 />",
            false,
            template_map,
            "crate",
            component_type_id,
            "example.pax",
            file!(),
        );

        let template = component.template.unwrap();
        let root_id = template.get_root().remove(0);
        let node = template.get_node(&root_id).unwrap();
        assert!(matches!(
            node.selector_info.class_binding,
            Some(ValueDefinition::Expression(_))
        ));
        assert!(!node.settings.as_ref().unwrap().iter().any(
            |setting| matches!(setting, SettingElement::Setting(token, _) if token.token_value == "class"),
        ));
    }

    #[test]
    #[should_panic(expected = "Specified more than one class attribute inline")]
    fn test_duplicate_class_attributes_are_rejected() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let rectangle_type_id = TypeId::build_singleton("Rectangle", Some("Rectangle"));
        let mut template_map = HashMap::new();
        template_map.insert("Rectangle".to_string(), rectangle_type_id);

        let _ = assemble_component_definition(
            ParsingContext::default(),
            r#"<Rectangle class="elevated" class="card" />"#,
            false,
            template_map,
            "crate",
            component_type_id,
            "example.pax",
            file!(),
        );
    }

    #[test]
    #[should_panic(expected = "class cannot be assigned from a selector or settings block")]
    fn test_class_cannot_be_assigned_from_settings() {
        let component = parse_pax_str(
            Rule::pax_component_definition,
            r#"
                <Rectangle />
                @settings {
                    .card { class: "other" }
                }
            "#,
        )
        .expect("component should parse");

        let _ = parse_settings_from_component_definition_string(component);
    }

    #[test]
    #[should_panic(expected = "class cannot be assigned by a timeline")]
    fn test_class_cannot_be_assigned_by_a_timeline() {
        let component = parse_pax_str(
            Rule::pax_component_definition,
            r#"
                <Rectangle />
                @timeline pulse {
                    .card {
                        class: { 0: "card", 100: "active" }
                    }
                }
            "#,
        )
        .expect("component should parse");

        let _ = parse_timeline_from_component_definition_string(component);
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
    fn test_parse_timeline_interruption_setting() {
        let component = parse_pax_str(
            Rule::pax_component_definition,
            r#"
                <Group />

                @timeline leave {
                    interruption: Restart,
                    self {
                        opacity: {
                            0: 1,
                            10: 0,
                        },
                    }
                }
            "#,
        )
        .expect("component should parse");

        let timelines = parse_timeline_from_component_definition_string(component);
        assert_eq!(timelines.len(), 1);
        assert_eq!(timelines[0].interruption, InOutInterruption::Restart);
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
        let common = manifest.get_inline_common_properties(&component_type_id, &root_id, node);

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
    fn test_parse_element_level_transition_bindings() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let group_type_id = TypeId::build_singleton("Group", Some("Group"));
        let mut template_map = HashMap::new();
        template_map.insert("Group".to_string(), group_type_id);

        let pax = r#"
            <Group
                @in=enter
                @out=@timeline {
                    opacity: {
                        0: 1,
                        10: 0,
                    },
                }
            />
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
        let root_id = template.get_root().remove(0);
        let node = template.get_node(&root_id).unwrap();
        let settings = node.settings.as_ref().expect("settings should parse");
        assert!(matches!(
            &settings[0],
            SettingElement::Setting(key, ValueDefinition::Identifier(identifier))
                if key.token_value == "in" && identifier.name == "enter"
        ));
        assert!(matches!(
            &settings[1],
            SettingElement::Setting(key, ValueDefinition::Block(block))
                if key.token_value == "out" && block.elements.len() == 1
        ));
    }

    #[test]
    fn test_element_transition_named_timeline_can_target_sibling() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let group_type_id = TypeId::build_singleton("Group", Some("Group"));
        let text_type_id = TypeId::build_singleton("Text", Some("Text"));
        let mut template_map = HashMap::new();
        template_map.insert("Group".to_string(), group_type_id);
        template_map.insert("Text".to_string(), text_type_id);

        let pax = r#"
            <Group id=panel @in=enter />
            <Text id=caption opacity=0.5 />

            @timeline enter {
                duration: 10,
                self {
                    opacity: {
                        0: 0,
                        10: 1,
                    },
                },
                #caption {
                    opacity: {
                        0: 0,
                        10: 1,
                    },
                },
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
        let panel_id = template_node_id_by_id(template, "panel");
        let caption_id = template_node_id_by_id(template, "caption");
        let panel = template.get_node(&panel_id).unwrap();
        let caption = template.get_node(&caption_id).unwrap();

        let panel_common =
            manifest.get_inline_common_properties(&component_type_id, &panel_id, panel);
        let caption_common =
            manifest.get_inline_common_properties(&component_type_id, &caption_id, caption);

        assert!(matches!(
            panel_common.get("opacity"),
            Some(ValueDefinition::Transition(transition)) if transition.enter.is_some()
        ));
        assert!(matches!(
            caption_common.get("opacity"),
            Some(ValueDefinition::Transition(transition)) if transition.enter.is_some()
        ));

        let panel_config =
            manifest.get_template_node_transition_config(&component_type_id, &panel_id);
        let caption_config =
            manifest.get_template_node_transition_config(&component_type_id, &caption_id);
        assert!(panel_config.has_enter);
        assert!(caption_config.has_enter);
        assert_eq!(panel_config.enter_sources, caption_config.enter_sources);
    }

    #[test]
    fn test_element_transition_inline_timeline_lowers_to_self_transition() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let group_type_id = TypeId::build_singleton("Group", Some("Group"));
        let mut template_map = HashMap::new();
        template_map.insert("Group".to_string(), group_type_id);

        let pax = r#"
            <Group
                id=panel
                opacity=0.5
                @in=@timeline {
                    duration: 10,
                    interruption: Restart,
                    opacity: {
                        0: 0,
                        10: 1,
                    },
                }
            />
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
        let panel_id = template_node_id_by_id(template, "panel");
        let panel = template.get_node(&panel_id).unwrap();
        let common = manifest.get_inline_common_properties(&component_type_id, &panel_id, panel);

        assert!(matches!(
            common.get("opacity"),
            Some(ValueDefinition::Transition(transition))
                if transition.enter.as_ref().map(|track| track.interruption)
                    == Some(InOutInterruption::Restart)
        ));
        let config = manifest.get_template_node_transition_config(&component_type_id, &panel_id);
        assert!(config.has_enter);
        assert_eq!(config.enter_frame_count, 10);
    }

    #[test]
    fn test_element_transition_dynamic_duration_does_not_use_percent_fallback() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let group_type_id = TypeId::build_singleton("Group", Some("Group"));
        let mut template_map = HashMap::new();
        template_map.insert("Group".to_string(), group_type_id);

        let pax = r#"
            <Group
                id=panel
                opacity=1
                @out=@timeline {
                    duration: {self.duration},
                    opacity: {
                        0: 1,
                        100%: 0,
                    },
                }
            />
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
        let panel_id = template_node_id_by_id(template, "panel");
        let panel = template.get_node(&panel_id).unwrap();
        let common = manifest.get_inline_common_properties(&component_type_id, &panel_id, panel);

        assert!(matches!(
            common.get("opacity"),
            Some(ValueDefinition::Transition(transition)) if transition.exit.is_some()
        ));
        let config = manifest.get_template_node_transition_config(&component_type_id, &panel_id);
        assert!(config.has_exit);
        assert_eq!(config.exit_frame_count, 0);
        assert!(matches!(
            config.exit_dynamic_durations.as_slice(),
            [ValueDefinition::Expression(_)]
        ));
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
        let route_type_id = TypeId::build_singleton("Route", Some("Route"));
        let mut template_map = HashMap::new();
        template_map.insert("Text".to_string(), text_type_id);
        template_map.insert("Group".to_string(), group_type_id);
        template_map.insert("Route".to_string(), route_type_id.clone());
        let mut route_branch_descriptors = HashMap::new();
        route_branch_descriptors.insert(route_type_id, default_route_branch_descriptor());

        let pax = r#"
            <Router>
                <Route
                    path="/"
                    metadata=RouteMetadata {
                        title: "Home"
                        description: "The example home route."
                        index: true
                        social_image: "assets/home.png"
                        social_image_alt: "Home preview"
                    }
                >
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

        let (_, component) = assemble_component_definition_with_route_branches(
            ParsingContext::default(),
            pax,
            false,
            template_map,
            route_branch_descriptors,
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
        let home_metadata = control_flow_settings.route_branches[0]
            .metadata
            .as_ref()
            .expect("home route metadata should parse");
        assert_eq!(home_metadata.title, "Home");
        assert_eq!(home_metadata.description, "The example home route.");
        assert!(home_metadata.index);
        assert_eq!(
            home_metadata.social_image.as_deref(),
            Some("assets/home.png")
        );
        assert_eq!(
            home_metadata.social_image_alt.as_deref(),
            Some("Home preview")
        );
        assert_eq!(
            control_flow_settings.route_branches[1].path.as_deref(),
            Some("/settings/*")
        );
        assert!(control_flow_settings.route_branches[2].default);
        assert_eq!(control_flow_settings.route_branches[0].child_ids.len(), 1);
        assert_eq!(control_flow_settings.route_branches[1].child_ids.len(), 1);
        assert_eq!(control_flow_settings.route_branches[2].child_ids.len(), 1);
        assert_eq!(template.get_children(&router_id).unwrap().len(), 3);

        let settings_route_id = control_flow_settings.route_branches[1].child_ids[0].clone();
        assert_eq!(template.get_children(&settings_route_id).unwrap().len(), 2);
    }

    #[test]
    fn test_parse_route_card_preserves_component_shell() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let text_type_id = TypeId::build_singleton("Text", Some("Text"));
        let route_card_type_id = TypeId::build_singleton("RouteCard", Some("RouteCard"));
        let mut template_map = HashMap::new();
        template_map.insert("Text".to_string(), text_type_id);
        template_map.insert("RouteCard".to_string(), route_card_type_id.clone());
        let mut route_branch_descriptors = HashMap::new();
        route_branch_descriptors.insert(
            route_card_type_id.clone(),
            default_route_branch_descriptor(),
        );

        let pax = r#"
            <Router>
                <RouteCard path="/details" edge=RouteCardEdge::Bottom duration=240ms curve=OutBack>
                    <Text id=details />
                </RouteCard>
            </Router>
        "#;

        let (_, component) = assemble_component_definition_with_route_branches(
            ParsingContext::default(),
            pax,
            false,
            template_map,
            route_branch_descriptors,
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
        let router_id = template.get_root().remove(0);
        let router_node = template.get_node(&router_id).unwrap();
        let control_flow_settings = router_node.control_flow_settings.as_ref().unwrap();

        assert_eq!(control_flow_settings.route_branches.len(), 1);
        assert_eq!(
            control_flow_settings.route_branches[0].path.as_deref(),
            Some("/details")
        );
        assert_eq!(control_flow_settings.route_branches[0].child_ids.len(), 1);

        let route_card_id = control_flow_settings.route_branches[0].child_ids[0].clone();
        let route_card = template.get_node(&route_card_id).unwrap();
        assert_eq!(route_card.type_id, route_card_type_id);
        assert_eq!(template.get_children(&route_card_id).unwrap().len(), 1);

        let settings = route_card.settings.as_ref().unwrap();
        assert!(settings.iter().any(|setting| matches!(
            setting,
            SettingElement::Setting(token, _value) if token.token_value == "edge"
        )));
        assert!(settings.iter().any(|setting| matches!(
            setting,
            SettingElement::Setting(token, _value) if token.token_value == "duration"
        )));
        assert!(settings.iter().any(|setting| matches!(
            setting,
            SettingElement::Setting(token, _value) if token.token_value == "curve"
        )));
        assert!(settings.iter().all(|setting| !matches!(
            setting,
            SettingElement::Setting(token, ValueDefinition::Block(_))
                if token.token_value == "in" || token.token_value == "out"
        )));
    }

    #[test]
    fn test_parse_custom_route_branch_descriptor() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let text_type_id = TypeId::build_singleton("Text", Some("Text"));
        let sheet_route_type_id = TypeId::build_singleton("SheetRoute", Some("SheetRoute"));
        let mut template_map = HashMap::new();
        template_map.insert("Text".to_string(), text_type_id);
        template_map.insert("SheetRoute".to_string(), sheet_route_type_id.clone());
        let mut route_branch_descriptors = HashMap::new();
        route_branch_descriptors.insert(
            sheet_route_type_id.clone(),
            RouteBranchDescriptor {
                path_property: "pattern".to_string(),
                default_property: "fallback".to_string(),
                modal: true,
            },
        );

        let pax = r#"
            <Router>
                <SheetRoute pattern="/sheet/:id" motion=2>
                    <Text id=sheet />
                </SheetRoute>
                <SheetRoute fallback=true motion=0 />
            </Router>
        "#;

        let (_, component) = assemble_component_definition_with_route_branches(
            ParsingContext::default(),
            pax,
            false,
            template_map,
            route_branch_descriptors,
            "crate",
            component_type_id,
            "example.pax",
            file!(),
        );

        let template = component.template.unwrap();
        let router_id = template.get_root().remove(0);
        let router_node = template.get_node(&router_id).unwrap();
        let control_flow_settings = router_node.control_flow_settings.as_ref().unwrap();

        assert_eq!(
            control_flow_settings.route_branches[0].path.as_deref(),
            Some("/sheet/:id")
        );
        assert!(control_flow_settings.route_branches[0].modal);
        assert!(control_flow_settings.route_branches[1].default);
        assert!(control_flow_settings.route_branches[1].modal);

        let sheet_route_id = control_flow_settings.route_branches[0].child_ids[0].clone();
        let sheet_route = template.get_node(&sheet_route_id).unwrap();
        assert_eq!(sheet_route.type_id, sheet_route_type_id);
        assert!(sheet_route
            .settings
            .as_ref()
            .unwrap()
            .iter()
            .any(|setting| {
                matches!(
                    setting,
                    SettingElement::Setting(token, _value) if token.token_value == "motion"
                )
            }));
    }

    #[test]
    #[should_panic(expected = "default Route metadata cannot set index: true")]
    fn test_default_route_metadata_cannot_be_indexable() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let text_type_id = TypeId::build_singleton("Text", Some("Text"));
        let mut template_map = HashMap::new();
        template_map.insert("Text".to_string(), text_type_id);

        assemble_component_definition(
            ParsingContext::default(),
            r#"
                <Router>
                    <Route
                        default=true
                        metadata=RouteMetadata {
                            title: "Missing"
                            description: "This page could not be found."
                            index: true
                        }
                    >
                        <Text />
                    </Route>
                </Router>
            "#,
            false,
            template_map,
            "crate",
            component_type_id,
            "example.pax",
            file!(),
        );
    }

    #[test]
    #[should_panic(expected = "RouteMetadata title may only be declared once")]
    fn test_route_metadata_rejects_duplicate_fields() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let text_type_id = TypeId::build_singleton("Text", Some("Text"));
        let mut template_map = HashMap::new();
        template_map.insert("Text".to_string(), text_type_id);

        assemble_component_definition(
            ParsingContext::default(),
            r#"
                <Router>
                    <Route
                        path="/"
                        metadata=RouteMetadata {
                            title: "Home"
                            title: "Drifted home"
                            description: "The home route."
                            index: true
                        }
                    >
                        <Text />
                    </Route>
                </Router>
            "#,
            false,
            template_map,
            "crate",
            component_type_id,
            "example.pax",
            file!(),
        );
    }

    #[test]
    fn test_parse_relative_nested_router_template() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let text_type_id = TypeId::build_singleton("Text", Some("Text"));
        let route_type_id = TypeId::build_singleton("Route", Some("Route"));
        let mut template_map = HashMap::new();
        template_map.insert("Text".to_string(), text_type_id);
        template_map.insert("Route".to_string(), route_type_id.clone());
        let mut route_branch_descriptors = HashMap::new();
        route_branch_descriptors.insert(route_type_id, default_route_branch_descriptor());

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

        let (_, component) = assemble_component_definition_with_route_branches(
            ParsingContext::default(),
            pax,
            false,
            template_map,
            route_branch_descriptors,
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
        let nested_router_id = template
            .get_children(&root_branch)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
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
        let route_type_id = TypeId::build_singleton("Route", Some("Route"));
        let mut template_map = HashMap::new();
        template_map.insert("Text".to_string(), text_type_id);
        template_map.insert("Group".to_string(), group_type_id);
        template_map.insert("Route".to_string(), route_type_id.clone());
        let mut route_branch_descriptors = HashMap::new();
        route_branch_descriptors.insert(route_type_id, default_route_branch_descriptor());

        let pax = r#"
            <Router>
                <Route
                    path="/"
                    metadata=RouteMetadata {
                        title: "Home"
                        description: "Home route"
                        index: true
                    }
                >
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

        let (_, component) = assemble_component_definition_with_route_branches(
            ParsingContext::default(),
            pax,
            false,
            template_map.clone(),
            route_branch_descriptors.clone(),
            "crate",
            component_type_id.clone(),
            "example.pax",
            file!(),
        );

        let rendered = press_code_serialization_template(component).unwrap();
        assert!(rendered.contains("<Router>"));
        assert!(rendered.contains(r#"<Route path="/settings/*">"#));
        assert!(rendered.contains("<Route default=true>"));
        assert!(rendered.contains("metadata=RouteMetadata"));
        assert!(rendered.contains(r#"title: "Home""#));

        let (_, parsed_component) = assemble_component_definition_with_route_branches(
            ParsingContext::default(),
            &rendered,
            false,
            template_map,
            route_branch_descriptors,
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
        assert_eq!(branches[1].child_ids.len(), 1);
        assert_eq!(branches[2].child_ids.len(), 1);
        assert!(branches[2].default);
        assert_eq!(
            branches[0]
                .metadata
                .as_ref()
                .map(|metadata| metadata.title.as_str()),
            Some("Home")
        );
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
                interruption: InOutInterruption::Restart,
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
                                interruption: Default::default(),
                                use_local_property_scope: false,
                            },
                        )],
                    },
                )],
            }],
            route_branch: None,
        };

        let rendered = press_code_serialization_template(component).unwrap();
        assert!(rendered.contains("@timeline orbital"));
        assert!(rendered.contains("playhead: self.phase"));
        assert!(rendered.contains("duration: 120"));
        assert!(rendered.contains("interruption: Restart"));
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
                        interruption: Default::default(),
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
                interruption: Default::default(),
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
                                interruption: Default::default(),
                                use_local_property_scope: false,
                            },
                        )],
                    },
                )],
            }],
            route_branch: None,
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
    #[cfg(feature = "code_serialization")]
    fn test_round_trip_inline_gradient_syntax() {
        let component_type_id = TypeId::build_singleton("Example", Some("Example"));
        let rectangle_type_id = TypeId::build_singleton("Rectangle", Some("Rectangle"));
        let mut template =
            ComponentTemplate::new(component_type_id.clone(), Some("example.pax".to_string()));
        template.add(TemplateNodeDefinition {
            type_id: rectangle_type_id.clone(),
            control_flow_settings: None,
            settings: Some(vec![SettingElement::Setting(
                Token::new_without_location("fill".to_string()),
                ValueDefinition::Gradient(GradientDefinition {
                    shape: GradientShapeDefinition::Linear {
                        start: Some(Box::new(ValueDefinition::LiteralValue(PaxValue::Vec(
                            vec![
                                PaxValue::Size(Size::Percent(Numeric::F64(0.0))),
                                PaxValue::Size(Size::Percent(Numeric::F64(50.0))),
                            ],
                        )))),
                        end: Some(Box::new(ValueDefinition::LiteralValue(PaxValue::Vec(
                            vec![
                                PaxValue::Size(Size::Percent(Numeric::F64(100.0))),
                                PaxValue::Size(Size::Percent(Numeric::F64(50.0))),
                            ],
                        )))),
                    },
                    elements: vec![
                        GradientElement::Stop(GradientStopDefinition {
                            position: Size::Percent(Numeric::F64(0.0)),
                            color: ValueDefinition::LiteralValue(PaxValue::Color(Box::new(
                                pax_manifest::pax_runtime_api::Color::RED,
                            ))),
                        }),
                        GradientElement::Stop(GradientStopDefinition {
                            position: Size::Percent(Numeric::F64(100.0)),
                            color: ValueDefinition::LiteralValue(PaxValue::Color(Box::new(
                                pax_manifest::pax_runtime_api::Color::BLUE,
                            ))),
                        }),
                    ],
                }),
            )]),
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
            timelines: vec![],
            route_branch: None,
        };

        let rendered = press_code_serialization_template(component).unwrap();
        assert!(rendered.contains("fill=@gradient {"));
        assert!(rendered.contains("linear: {"));
        assert!(rendered.contains("start: ["), "{rendered}");
        assert!(rendered.contains("end: ["), "{rendered}");
        assert!(!rendered.contains("start: ("), "{rendered}");
        assert!(!rendered.contains("end: ("), "{rendered}");
        assert!(rendered.contains("0%: RED"));
        assert!(rendered.contains("100%: BLUE"));

        let mut template_map = HashMap::new();
        template_map.insert("Rectangle".to_string(), rectangle_type_id);
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
        let root_id = template.get_root().remove(0);
        let node = template.get_node(&root_id).unwrap();
        let fill = node
            .settings
            .as_ref()
            .unwrap()
            .iter()
            .find_map(|setting| match setting {
                SettingElement::Setting(token, ValueDefinition::Gradient(gradient))
                    if token.token_value == "fill" =>
                {
                    Some(gradient)
                }
                _ => None,
            })
            .expect("inline gradient should round-trip");

        assert!(matches!(
            &fill.shape,
            GradientShapeDefinition::Linear {
                start: Some(_),
                end: Some(_)
            }
        ));
        assert_eq!(fill.stops().count(), 2);
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
    fn test_compile_time_settings_merge_leaves_classes_for_runtime() {
        let mut node = TemplateNodeDefinition {
            type_id: TypeId::build_singleton("example::Text", Some("Text")),
            control_flow_settings: None,
            settings: Some(vec![
                SettingElement::Setting(
                    Token::new_without_location("class".to_string()),
                    ValueDefinition::LiteralValue(PaxValue::String("headline".to_string())),
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
        assert!(!merged_map.contains_key("height"));
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
