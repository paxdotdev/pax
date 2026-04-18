#[cfg(test)]
#[cfg(feature = "parsing")]
mod tests {
    use std::collections::HashMap;

    use pax_lang::{parse_pax_str, Rule};
    use pax_manifest::pax_runtime_api::PaxValue;
    use pax_manifest::{
        code_serialization::press_code_serialization_template,
        parsing::{
            assemble_component_definition, parse_timeline_from_component_definition_string,
            ParsingContext,
        },
        utils, ComponentDefinition, ComponentTemplate, PaxIdentifier, SettingElement,
        TemplateNodeDefinition, TimelineBlockElement, TimelineDefinition, TimelineKeyframe,
        TimelineMarker, TimelineSelectorBlockDefinition, TimelineSelectorElement,
        TimelineTrackDefinition, TimelineTrackElement, Token, TypeId, ValueDefinition,
    };

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
            utils::parse_value("@timeline { frames: 90, loop: false, 0: 0, Linear, 100%: 1 }");
        if let Ok(Some(ValueDefinition::Timeline(track))) = res {
            let keyframes: Vec<_> = track.keyframes().collect();
            assert_eq!(track.frames, Some(90));
            assert_eq!(track.repeat, Some(false));
            assert_eq!(keyframes.len(), 2);
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
    fn test_parse_with_extra() {
        let res = utils::parse_value("{5 + 3}this_shouldn't succeed");
        assert!(matches!(res, Err(_)));
    }

    #[test]
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
                frames: Some(120),
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
                                frames: None,
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
        assert!(rendered.contains("frames: 120"));
        assert!(rendered.contains("0: 0.40, Linear"));
        assert!(rendered.contains("100%: 1.00"));
    }

    #[test]
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
                                marker: TimelineMarker::Frame(0),
                                value: ValueDefinition::LiteralValue(PaxValue::Numeric(
                                    0.35.into(),
                                )),
                                easing: Some(Token::new_without_location("Linear".to_string())),
                            }),
                            TimelineTrackElement::Keyframe(TimelineKeyframe {
                                marker: TimelineMarker::Percent(50.0),
                                value: ValueDefinition::LiteralValue(PaxValue::Numeric(1.0.into())),
                                easing: Some(Token::new_without_location("OutQuad".to_string())),
                            }),
                            TimelineTrackElement::Keyframe(TimelineKeyframe {
                                marker: TimelineMarker::Percent(100.0),
                                value: ValueDefinition::LiteralValue(PaxValue::Numeric(
                                    0.35.into(),
                                )),
                                easing: None,
                            }),
                        ],
                        playhead: Some(Box::new(ValueDefinition::Identifier(PaxIdentifier::new(
                            "self.phase",
                        )))),
                        frames: Some(90),
                        repeat: Some(false),
                        starting_value: None,
                        use_local_property_scope: false,
                    }),
                ),
            ]),
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
                frames: Some(120),
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
                                frames: None,
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
        assert!(rendered.contains("frames: 90"));
        assert!(rendered.contains("loop: false"));
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
        );

        assert_eq!(parsed_component.timelines.len(), 1);
        assert_eq!(parsed_component.timelines[0].frames, Some(120));
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

        assert_eq!(opacity_track.frames, Some(90));
        assert_eq!(opacity_track.repeat, Some(false));
        assert!(matches!(
            opacity_track.playhead.as_deref(),
            Some(ValueDefinition::Identifier(identifier)) if identifier.name == "self.phase"
        ));
        assert_eq!(opacity_track.keyframes().count(), 3);
    }
}
