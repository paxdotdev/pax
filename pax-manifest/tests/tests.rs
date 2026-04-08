#[cfg(test)]
#[cfg(feature = "parsing")]
mod tests {

    use pax_lang::{parse_pax_str, Rule};
    use pax_manifest::pax_runtime_api::PaxValue;
    use pax_manifest::{
        code_serialization::press_code_serialization_template,
        parsing::parse_timeline_from_component_definition_string, utils, ComponentDefinition,
        TimelineBlockElement, TimelineDefinition, TimelineKeyframe, TimelineMarker,
        TimelineSelectorBlockDefinition, TimelineSelectorElement, TimelineTrackDefinition,
        TimelineTrackElement, Token, TypeId, ValueDefinition,
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
}
