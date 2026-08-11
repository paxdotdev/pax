use std::collections::{BTreeMap, HashMap, VecDeque};

use pax_message::serde::{Deserialize, Serialize};

use crate::{
    ComponentDefinition, ComponentTemplate, GradientDefinition, GradientElement,
    GradientShapeDefinition, GradientStopDefinition, LiteralBlockDefinition, PaxManifest,
    RouteBranchDescriptor, SettingElement, SettingsBlockElement, SettingsConditionalBlock,
    SettingsConditionalBranch, TemplateNodeDefinition, TemplateNodeId, TimelineBlockElement,
    TimelineDefinition, TimelineSelectorBlockDefinition, TimelineSelectorElement,
    TimelineTrackDefinition, TimelineTrackElement, Token, TransitionDefinition, TypeDefinition,
    TypeId, ValueDefinition,
};

const MAGIC: &[u8; 8] = b"PAXP\x00IR\x00";
const VERSION: u8 = 3;

/// Runtime-facing semantic program model derived from a rich `PaxManifest`.
///
/// `ProgramIR` is a transitional intermediate representation: it preserves the
/// semantic program structure while stripping obviously source-only and
/// compiler-only manifest data. Debug/designtime flows may continue to mount
/// rich manifests directly until later phases cut execution over to `ProgramIR`.
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(crate = "pax_message::serde")]
pub struct ProgramIR {
    pub components: BTreeMap<TypeId, ProgramComponent>,
    pub main_component_type_id: TypeId,
    pub type_table: BTreeMap<TypeId, TypeDefinition>,
    pub assets_dirs: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(crate = "pax_message::serde")]
pub struct ProgramComponent {
    pub type_id: TypeId,
    pub is_main_component: bool,
    pub is_primitive: bool,
    pub is_struct_only_component: bool,
    pub template: Option<ComponentTemplate>,
    pub settings: Option<Vec<SettingsBlockElement>>,
    #[serde(default)]
    pub timelines: Vec<TimelineDefinition>,
    #[serde(default)]
    pub route_branch: Option<RouteBranchDescriptor>,
}

impl ProgramIR {
    pub fn from_manifest(manifest: &PaxManifest) -> Self {
        manifest.into()
    }
}

impl From<&PaxManifest> for ProgramIR {
    fn from(manifest: &PaxManifest) -> Self {
        Self {
            components: manifest
                .components
                .iter()
                .map(|(type_id, definition)| (type_id.clone(), ProgramComponent::from(definition)))
                .collect(),
            main_component_type_id: manifest.main_component_type_id.clone(),
            type_table: manifest
                .type_table
                .iter()
                .map(|(type_id, definition)| (type_id.clone(), definition.clone()))
                .collect(),
            assets_dirs: manifest.assets_dirs.clone(),
        }
    }
}

impl From<&ComponentDefinition> for ProgramComponent {
    fn from(definition: &ComponentDefinition) -> Self {
        Self {
            type_id: definition.type_id.clone(),
            is_main_component: definition.is_main_component,
            is_primitive: definition.is_primitive,
            is_struct_only_component: definition.is_struct_only_component,
            template: definition
                .template
                .as_ref()
                .map(sanitize_component_template),
            settings: definition
                .settings
                .as_ref()
                .map(sanitize_settings_block_elements),
            timelines: definition.timelines.iter().map(sanitize_timeline).collect(),
            route_branch: definition.route_branch.clone(),
        }
    }
}

fn sanitize_component_template(template: &ComponentTemplate) -> ComponentTemplate {
    let root: VecDeque<_> = template
        .get_root()
        .into_iter()
        .filter(|id| should_keep_template_node(template, id))
        .collect();

    let mut children = HashMap::new();
    let mut nodes = HashMap::new();
    for id in root.iter() {
        collect_sanitized_template_parts(template, id, &mut children, &mut nodes);
    }

    ComponentTemplate::from_parts(
        template.get_containing_component_type_id(),
        root,
        children,
        nodes,
        template.get_next_id(),
        None,
    )
}

fn collect_sanitized_template_parts(
    template: &ComponentTemplate,
    id: &TemplateNodeId,
    children: &mut HashMap<TemplateNodeId, VecDeque<TemplateNodeId>>,
    nodes: &mut HashMap<TemplateNodeId, TemplateNodeDefinition>,
) {
    let Some(node) = template.get_node(id) else {
        return;
    };
    let Some(node) = sanitize_template_node(node) else {
        return;
    };
    nodes.insert(id.clone(), node);

    let filtered_children: VecDeque<_> = template
        .get_children(id)
        .unwrap_or_default()
        .into_iter()
        .filter(|child_id| should_keep_template_node(template, child_id))
        .collect();

    if !filtered_children.is_empty() {
        children.insert(id.clone(), filtered_children.clone());
    }

    for child_id in filtered_children {
        collect_sanitized_template_parts(template, &child_id, children, nodes);
    }
}

fn should_keep_template_node(template: &ComponentTemplate, id: &TemplateNodeId) -> bool {
    template
        .get_node(id)
        .map(|node| node.raw_comment_string.is_none())
        .unwrap_or(false)
}

fn sanitize_template_node(node: &TemplateNodeDefinition) -> Option<TemplateNodeDefinition> {
    if node.raw_comment_string.is_some() {
        return None;
    }

    let mut control_flow_settings = node.control_flow_settings.clone();
    if let Some(settings) = &mut control_flow_settings {
        for route in &mut settings.route_branches {
            route.metadata = None;
        }
    }

    Some(TemplateNodeDefinition {
        type_id: node.type_id.clone(),
        control_flow_settings,
        settings: node.settings.as_ref().map(sanitize_setting_elements),
        selector_info: crate::TemplateNodeSelectorInfo {
            source_location: node.selector_info.source_location.clone(),
            id: node.selector_info.id.as_ref().map(strip_token),
            class_binding: node.selector_info.class_binding.clone(),
        },
        raw_comment_string: None,
    })
}

fn sanitize_settings_block_elements(
    elements: &Vec<SettingsBlockElement>,
) -> Vec<SettingsBlockElement> {
    elements
        .iter()
        .filter_map(|element| match element {
            SettingsBlockElement::SelectorBlock(token, block) => {
                Some(SettingsBlockElement::SelectorBlock(
                    strip_token(token),
                    sanitize_literal_block(block),
                ))
            }
            SettingsBlockElement::Handler(token, handlers) => Some(SettingsBlockElement::Handler(
                strip_token(token),
                handlers.iter().map(strip_token).collect(),
            )),
            SettingsBlockElement::Transition(token, value) => Some(
                SettingsBlockElement::Transition(strip_token(token), strip_token(value)),
            ),
            SettingsBlockElement::Conditional(block) => Some(SettingsBlockElement::Conditional(
                SettingsConditionalBlock {
                    branches: block
                        .branches
                        .iter()
                        .map(|branch| SettingsConditionalBranch {
                            condition_expression: branch.condition_expression.clone(),
                            elements: sanitize_settings_block_elements(&branch.elements),
                        })
                        .collect(),
                },
            )),
            SettingsBlockElement::Comment(_) => None,
        })
        .collect()
}

fn sanitize_setting_elements(elements: &Vec<SettingElement>) -> Vec<SettingElement> {
    elements
        .iter()
        .filter_map(|element| match element {
            SettingElement::Setting(token, value) => Some(SettingElement::Setting(
                strip_token(token),
                sanitize_value_definition(value),
            )),
            SettingElement::Comment(_) => None,
        })
        .collect()
}

fn sanitize_literal_block(block: &LiteralBlockDefinition) -> LiteralBlockDefinition {
    LiteralBlockDefinition {
        explicit_type_pascal_identifier: block
            .explicit_type_pascal_identifier
            .as_ref()
            .map(strip_token),
        elements: sanitize_setting_elements(&block.elements),
    }
}

fn sanitize_timeline(definition: &TimelineDefinition) -> TimelineDefinition {
    TimelineDefinition {
        name: definition.name.as_ref().map(strip_token),
        playhead: definition.playhead.as_ref().map(sanitize_value_definition),
        duration: definition.duration.as_ref().map(sanitize_value_definition),
        repeat: definition.repeat,
        interruption: definition.interruption,
        elements: definition
            .elements
            .iter()
            .filter_map(|element| match element {
                TimelineBlockElement::SelectorBlock(token, selector) => {
                    Some(TimelineBlockElement::SelectorBlock(
                        strip_token(token),
                        sanitize_timeline_selector(selector),
                    ))
                }
                TimelineBlockElement::Comment(_) => None,
            })
            .collect(),
    }
}

fn sanitize_timeline_selector(
    selector: &TimelineSelectorBlockDefinition,
) -> TimelineSelectorBlockDefinition {
    TimelineSelectorBlockDefinition {
        elements: selector
            .elements
            .iter()
            .filter_map(|element| match element {
                TimelineSelectorElement::Track(token, track) => {
                    Some(TimelineSelectorElement::Track(
                        strip_token(token),
                        sanitize_timeline_track(track),
                    ))
                }
                TimelineSelectorElement::Comment(_) => None,
            })
            .collect(),
    }
}

fn sanitize_timeline_track(track: &TimelineTrackDefinition) -> TimelineTrackDefinition {
    TimelineTrackDefinition {
        elements: track
            .elements
            .iter()
            .filter_map(|element| match element {
                TimelineTrackElement::Keyframe(keyframe) => {
                    Some(TimelineTrackElement::Keyframe(crate::TimelineKeyframe {
                        marker: keyframe.marker.clone(),
                        value: sanitize_value_definition(&keyframe.value),
                        easing: keyframe.easing.as_ref().map(strip_token),
                    }))
                }
                TimelineTrackElement::Comment(_) => None,
            })
            .collect(),
        playhead: track
            .playhead
            .as_ref()
            .map(|value| Box::new(sanitize_value_definition(value))),
        duration: track
            .duration
            .as_ref()
            .map(|value| Box::new(sanitize_value_definition(value))),
        repeat: track.repeat,
        starting_value: track
            .starting_value
            .as_ref()
            .map(|value| Box::new(sanitize_value_definition(value))),
        interruption: track.interruption,
        use_local_property_scope: track.use_local_property_scope,
    }
}

fn sanitize_gradient(definition: &GradientDefinition) -> GradientDefinition {
    GradientDefinition {
        shape: sanitize_gradient_shape(&definition.shape),
        elements: definition
            .elements
            .iter()
            .filter_map(|element| match element {
                GradientElement::Stop(stop) => {
                    Some(GradientElement::Stop(GradientStopDefinition {
                        position: stop.position.clone(),
                        color: sanitize_value_definition(&stop.color),
                    }))
                }
                GradientElement::Comment(_) => None,
            })
            .collect(),
    }
}

fn sanitize_gradient_shape(shape: &GradientShapeDefinition) -> GradientShapeDefinition {
    match shape {
        GradientShapeDefinition::Linear { start, end } => GradientShapeDefinition::Linear {
            start: start
                .as_ref()
                .map(|value| Box::new(sanitize_value_definition(value))),
            end: end
                .as_ref()
                .map(|value| Box::new(sanitize_value_definition(value))),
        },
        GradientShapeDefinition::Radial { start, end, radius } => GradientShapeDefinition::Radial {
            start: Box::new(sanitize_value_definition(start)),
            end: Box::new(sanitize_value_definition(end)),
            radius: Box::new(sanitize_value_definition(radius)),
        },
    }
}

fn sanitize_value_definition(value: &ValueDefinition) -> ValueDefinition {
    match value {
        ValueDefinition::Undefined => ValueDefinition::Undefined,
        ValueDefinition::LiteralValue(value) => ValueDefinition::LiteralValue(value.clone()),
        ValueDefinition::Block(block) => ValueDefinition::Block(sanitize_literal_block(block)),
        ValueDefinition::Timeline(track) => {
            ValueDefinition::Timeline(sanitize_timeline_track(track))
        }
        ValueDefinition::Gradient(gradient) => {
            ValueDefinition::Gradient(sanitize_gradient(gradient))
        }
        ValueDefinition::Transition(transition) => {
            ValueDefinition::Transition(sanitize_transition_definition(transition))
        }
        ValueDefinition::Expression(info) => ValueDefinition::Expression(info.clone()),
        ValueDefinition::Identifier(identifier) => ValueDefinition::Identifier(identifier.clone()),
        ValueDefinition::DoubleBinding(identifier) => {
            ValueDefinition::DoubleBinding(identifier.clone())
        }
        ValueDefinition::EventBindingTarget(identifier) => {
            ValueDefinition::EventBindingTarget(identifier.clone())
        }
    }
}

fn sanitize_transition_definition(transition: &TransitionDefinition) -> TransitionDefinition {
    TransitionDefinition {
        enter: transition.enter.as_ref().map(sanitize_timeline_track),
        exit: transition.exit.as_ref().map(sanitize_timeline_track),
        starting_value: transition
            .starting_value
            .as_ref()
            .map(|value| Box::new(sanitize_value_definition(value))),
    }
}

fn strip_token(token: &Token) -> Token {
    Token::new_without_location(token.token_value.clone())
}

pub mod binary {
    use super::{ProgramIR, MAGIC, VERSION};
    use crate::binary::{decode_with_header, encode_with_header, Result};

    pub fn to_vec(program: &ProgramIR) -> Result<Vec<u8>> {
        encode_with_header(MAGIC, VERSION, program)
    }

    pub fn from_slice(bytes: &[u8]) -> Result<ProgramIR> {
        decode_with_header(bytes, MAGIC, VERSION)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};

    use pax_runtime_api::{Color, Numeric, PaxValue, Size};

    use super::ProgramIR;
    use crate::{
        binary::Result as BinaryResult, ComponentDefinition, ComponentTemplate, GradientDefinition,
        GradientElement, GradientShapeDefinition, GradientStopDefinition, LiteralBlockDefinition,
        LocationInfo, PaxIdentifier, PaxManifest, SettingsBlockElement, TemplateNodeDefinition,
        TimelineBlockElement, TimelineDefinition, TimelineKeyframe, TimelineMarker,
        TimelineSelectorBlockDefinition, TimelineSelectorElement, TimelineTrackDefinition,
        TimelineTrackElement, Token, TypeId, ValueDefinition,
    };

    fn test_location() -> LocationInfo {
        LocationInfo {
            start_line_col: (1, 1),
            end_line_col: (1, 5),
        }
    }

    fn build_manifest() -> PaxManifest {
        let component_type_id = TypeId::build_singleton("example::Main", Some("Main"));
        let child_type_id = TypeId::build_singleton("example::Child", Some("Child"));
        let mut template =
            ComponentTemplate::new(component_type_id.clone(), Some("src/main.pax".to_string()));

        template.add(TemplateNodeDefinition {
            type_id: TypeId::build_comment(),
            control_flow_settings: None,
            settings: None,
            selector_info: Default::default(),
            raw_comment_string: Some("// comment".to_string()),
        });

        template.add(TemplateNodeDefinition {
            type_id: child_type_id,
            control_flow_settings: None,
            settings: Some(vec![
                crate::SettingElement::Comment("inline comment".to_string()),
                crate::SettingElement::Setting(
                    Token::new("fill".to_string(), test_location()),
                    ValueDefinition::Block(LiteralBlockDefinition {
                        explicit_type_pascal_identifier: Some(Token::new(
                            "Color".to_string(),
                            test_location(),
                        )),
                        elements: vec![
                            crate::SettingElement::Comment("nested comment".to_string()),
                            crate::SettingElement::Setting(
                                Token::new("value".to_string(), test_location()),
                                ValueDefinition::LiteralValue(PaxValue::Numeric(1.into())),
                            ),
                        ],
                    }),
                ),
            ]),
            selector_info: Default::default(),
            raw_comment_string: None,
        });

        let mut components = BTreeMap::new();
        components.insert(
            component_type_id.clone(),
            ComponentDefinition {
                type_id: component_type_id.clone(),
                is_main_component: true,
                is_primitive: false,
                is_struct_only_component: false,
                module_path: "example::main".to_string(),
                primitive_instance_import_path: Some("example::instance".to_string()),
                template: Some(template),
                settings: Some(vec![
                    SettingsBlockElement::Comment("settings comment".to_string()),
                    SettingsBlockElement::Handler(
                        Token::new("click".to_string(), test_location()),
                        vec![Token::new("handle_click".to_string(), test_location())],
                    ),
                    SettingsBlockElement::SelectorBlock(
                        Token::new("#id".to_string(), test_location()),
                        LiteralBlockDefinition {
                            explicit_type_pascal_identifier: None,
                            elements: vec![
                                crate::SettingElement::Comment("selector comment".to_string()),
                                crate::SettingElement::Setting(
                                    Token::new("value".to_string(), test_location()),
                                    ValueDefinition::Identifier(PaxIdentifier::new("self.value")),
                                ),
                                crate::SettingElement::Setting(
                                    Token::new("fill".to_string(), test_location()),
                                    ValueDefinition::Gradient(GradientDefinition {
                                        shape: GradientShapeDefinition::Linear {
                                            start: None,
                                            end: None,
                                        },
                                        elements: vec![
                                            GradientElement::Comment(
                                                "gradient comment".to_string(),
                                            ),
                                            GradientElement::Stop(GradientStopDefinition {
                                                position: Size::Percent(Numeric::F64(0.0)),
                                                color: ValueDefinition::LiteralValue(
                                                    PaxValue::Color(Box::new(Color::RED)),
                                                ),
                                            }),
                                            GradientElement::Stop(GradientStopDefinition {
                                                position: Size::Percent(Numeric::F64(100.0)),
                                                color: ValueDefinition::LiteralValue(
                                                    PaxValue::Color(Box::new(Color::BLUE)),
                                                ),
                                            }),
                                        ],
                                    }),
                                ),
                            ],
                        },
                    ),
                ]),
                timelines: vec![TimelineDefinition {
                    name: Some(Token::new("spin".to_string(), test_location())),
                    playhead: Some(ValueDefinition::Identifier(PaxIdentifier::new(
                        "self.elapsed_frames",
                    ))),
                    duration: Some(ValueDefinition::LiteralValue(PaxValue::Numeric(120.into()))),
                    repeat: true,
                    interruption: crate::InOutInterruption::Restart,
                    elements: vec![
                        TimelineBlockElement::Comment("timeline comment".to_string()),
                        TimelineBlockElement::SelectorBlock(
                            Token::new("#id".to_string(), test_location()),
                            TimelineSelectorBlockDefinition {
                                elements: vec![
                                    TimelineSelectorElement::Comment(
                                        "selector comment".to_string(),
                                    ),
                                    TimelineSelectorElement::Track(
                                        Token::new("rotate".to_string(), test_location()),
                                        TimelineTrackDefinition {
                                            elements: vec![
                                                TimelineTrackElement::Comment(
                                                    "track comment".to_string(),
                                                ),
                                                TimelineTrackElement::Keyframe(TimelineKeyframe {
                                                    marker: TimelineMarker::Frame(0),
                                                    value: ValueDefinition::LiteralValue(
                                                        PaxValue::Numeric(0.into()),
                                                    ),
                                                    easing: Some(Token::new(
                                                        "Linear".to_string(),
                                                        test_location(),
                                                    )),
                                                }),
                                            ],
                                            playhead: None,
                                            duration: None,
                                            repeat: None,
                                            starting_value: None,
                                            interruption: crate::InOutInterruption::Restart,
                                            use_local_property_scope: false,
                                        },
                                    ),
                                ],
                            },
                        ),
                    ],
                }],
                route_branch: None,
            },
        );

        PaxManifest {
            components,
            main_component_type_id: component_type_id,
            type_table: HashMap::new(),
            assets_dirs: vec!["assets".to_string()],
            engine_import_path: "pax_engine".to_string(),
        }
    }

    #[test]
    fn program_ir_strips_source_and_comment_metadata() {
        let manifest = build_manifest();
        let ir = ProgramIR::from_manifest(&manifest);

        assert_eq!(ir.assets_dirs, vec!["assets".to_string()]);
        assert_eq!(ir.components.len(), 1);

        let component = ir
            .components
            .get(&manifest.main_component_type_id)
            .expect("component should exist");

        let template = component.template.as_ref().expect("template should exist");
        assert_eq!(template.get_file_path(), None);
        assert_eq!(template.get_root().len(), 1);

        let only_root = template.get_root().into_iter().next().unwrap();
        let root_node = template
            .get_node(&only_root)
            .expect("root node should exist");
        let settings = root_node.settings.as_ref().expect("settings should remain");
        assert_eq!(settings.len(), 1);
        match &settings[0] {
            crate::SettingElement::Setting(token, ValueDefinition::Block(block)) => {
                assert_eq!(token.token_location, None);
                assert_eq!(block.elements.len(), 1);
                assert_eq!(
                    block
                        .explicit_type_pascal_identifier
                        .as_ref()
                        .and_then(|token| token.token_location.clone()),
                    None
                );
            }
            _ => panic!("expected sanitized block setting"),
        }

        let component_settings = component.settings.as_ref().expect("settings should remain");
        assert_eq!(component_settings.len(), 2);
        match &component_settings[0] {
            SettingsBlockElement::Handler(token, handlers) => {
                assert_eq!(token.token_location, None);
                assert!(handlers
                    .iter()
                    .all(|handler| handler.token_location.is_none()));
            }
            _ => panic!("expected handler setting"),
        }
        match &component_settings[1] {
            SettingsBlockElement::SelectorBlock(token, block) => {
                assert_eq!(token.token_location, None);
                assert_eq!(block.elements.len(), 2);
                let Some(crate::SettingElement::Setting(_, ValueDefinition::Gradient(gradient))) =
                    block.elements.get(1)
                else {
                    panic!("expected sanitized gradient setting");
                };
                assert!(matches!(
                    &gradient.shape,
                    GradientShapeDefinition::Linear {
                        start: None,
                        end: None
                    }
                ));
                assert_eq!(gradient.elements.len(), 2);
            }
            _ => panic!("expected selector setting"),
        }

        let timeline = component.timelines.first().expect("timeline should remain");
        assert_eq!(
            timeline
                .name
                .as_ref()
                .and_then(|token| token.token_location.clone()),
            None
        );
        assert_eq!(timeline.elements.len(), 1);
        match &timeline.elements[0] {
            TimelineBlockElement::SelectorBlock(token, selector) => {
                assert_eq!(token.token_location, None);
                assert_eq!(selector.elements.len(), 1);
                match &selector.elements[0] {
                    TimelineSelectorElement::Track(track_token, track) => {
                        assert_eq!(track_token.token_location, None);
                        assert_eq!(track.elements.len(), 1);
                        match &track.elements[0] {
                            TimelineTrackElement::Keyframe(keyframe) => {
                                assert_eq!(
                                    keyframe
                                        .easing
                                        .as_ref()
                                        .and_then(|token| token.token_location.clone()),
                                    None
                                );
                            }
                            _ => panic!("expected sanitized keyframe"),
                        }
                    }
                    _ => panic!("expected sanitized track"),
                }
            }
            _ => panic!("expected sanitized timeline selector"),
        }
    }

    #[test]
    fn program_ir_binary_round_trips() -> BinaryResult<()> {
        let manifest = build_manifest();
        let ir = ProgramIR::from_manifest(&manifest);

        let bytes = super::binary::to_vec(&ir)?;
        let decoded = super::binary::from_slice(&bytes)?;

        assert_eq!(decoded.main_component_type_id, ir.main_component_type_id);
        assert_eq!(decoded.assets_dirs, ir.assets_dirs);
        assert_eq!(decoded.components.len(), ir.components.len());
        assert_eq!(decoded.type_table.len(), ir.type_table.len());

        let component = decoded
            .components
            .get(&decoded.main_component_type_id)
            .expect("decoded component should exist");
        assert!(component.template.is_some());
        assert_eq!(
            component.timelines[0].interruption,
            crate::InOutInterruption::Restart
        );
        let TimelineBlockElement::SelectorBlock(_, selector) = &component.timelines[0].elements[0]
        else {
            panic!("expected decoded selector block")
        };
        let TimelineSelectorElement::Track(_, track) = &selector.elements[0] else {
            panic!("expected decoded timeline track")
        };
        assert_eq!(track.interruption, crate::InOutInterruption::Restart);
        assert_eq!(
            component
                .template
                .as_ref()
                .expect("decoded template should exist")
                .get_root()
                .len(),
            1
        );

        Ok(())
    }
}
