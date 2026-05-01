use crate::*;
use pax_language::interpreter::parse_pax_expression_from_pair;
use pax_language::{from_pax, parse_pax_expression, parse_pax_str, Pair, Pairs, Rule, Span};
use pax_runtime_api::PaxValue;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;
use syn::{FnArg, ImplItem, ImplItemMethod, Item, Type, Visibility};

/// Parse template nodes out of a component-definition AST into a mutable template context.
pub fn parse_template_from_component_definition_string(
    ctx: &mut TemplateNodeParseContext,
    pax: &str,
    pax_component_definition: Pair<Rule>,
) {
    pax_component_definition
        .into_inner()
        .for_each(|pair| match pair.as_rule() {
            Rule::root_tag_pair => {
                recurse_visit_tag_pairs_for_template(
                    ctx,
                    pair.into_inner().next().unwrap(),
                    pax,
                    TreeLocation::Root,
                );
            }
            _ => {}
        });
}

/// Mutable state used while parsing template nodes for one component.
pub struct TemplateNodeParseContext {
    pub template: ComponentTemplate,
    pub pascal_identifier_to_type_id_map: HashMap<String, TypeId>,
}

fn recurse_visit_tag_pairs_for_template(
    ctx: &mut TemplateNodeParseContext,
    any_tag_pair: Pair<Rule>,
    pax: &str,
    location: TreeLocation,
) {
    match any_tag_pair.as_rule() {
        Rule::matched_tag => {
            //matched_tag => open_tag > pascal_identifier
            let matched_tag = any_tag_pair;
            let open_tag_pair = matched_tag.clone().into_inner().next().unwrap();
            let source_location = Some(span_to_location(&open_tag_pair.as_span()));
            let mut open_tag = open_tag_pair.into_inner();
            let pascal_identifier = open_tag.next().unwrap().as_str();
            let settings = parse_inline_attribute_from_final_pairs_of_tag(open_tag);

            if pascal_identifier == "Router" {
                parse_router_matched_tag(ctx, matched_tag, pax, location);
                return;
            }

            if pascal_identifier == "Route" {
                panic!("Route must be a direct child of Router");
            }

            let template_node = TemplateNodeDefinition {
                type_id: TypeId::build_singleton(
                    &ctx.pascal_identifier_to_type_id_map
                        .get(pascal_identifier)
                        .expect(&format!("Template key not found {}", &pascal_identifier))
                        .to_string(),
                    Some(&pascal_identifier.to_string()),
                ),
                settings: settings.clone(),
                selector_info: TemplateNodeSelectorInfo::from_inline_settings(
                    source_location,
                    &settings,
                ),
                raw_comment_string: None,
                control_flow_settings: None,
            };

            let id = match location {
                TreeLocation::Root => ctx.template.add_root_node_back(template_node),
                TreeLocation::Parent(id) => ctx.template.add_child_back(id, template_node),
            };

            //recurse into inner_nodes
            let prospective_inner_nodes = matched_tag.into_inner().nth(1).unwrap();
            match prospective_inner_nodes.as_rule() {
                Rule::inner_nodes => {
                    let inner_nodes = prospective_inner_nodes;
                    inner_nodes.into_inner().for_each(|sub_tag_pair| {
                        recurse_visit_tag_pairs_for_template(
                            ctx,
                            sub_tag_pair,
                            pax,
                            TreeLocation::Parent(id.clone().get_template_node_id()),
                        );
                    })
                }
                _ => {
                    panic!("wrong prospective inner nodes (or nth)")
                }
            }
        }
        Rule::self_closing_tag => {
            let source_location = Some(span_to_location(&any_tag_pair.as_span()));
            let mut tag_pairs = any_tag_pair.into_inner();
            let pascal_identifier = tag_pairs.next().unwrap().as_str();

            if pascal_identifier == "Router" {
                parse_router_self_closing_tag(ctx, location);
                return;
            }

            if pascal_identifier == "Route" {
                panic!("Route must be a direct child of Router");
            }

            let type_id = if let Some(type_id) =
                ctx.pascal_identifier_to_type_id_map.get(pascal_identifier)
            {
                type_id.clone()
            } else {
                TypeId::build_blank_component(pascal_identifier)
            };
            let settings = parse_inline_attribute_from_final_pairs_of_tag(tag_pairs);
            let template_node = TemplateNodeDefinition {
                type_id,
                settings: settings.clone(),
                selector_info: TemplateNodeSelectorInfo::from_inline_settings(
                    source_location,
                    &settings,
                ),
                raw_comment_string: None,
                control_flow_settings: None,
            };
            let _ = match location {
                TreeLocation::Root => ctx.template.add_root_node_back(template_node),
                TreeLocation::Parent(id) => ctx.template.add_child_back(id, template_node),
            };
        }
        Rule::statement_control_flow => {
            /* statement_control_flow = {(statement_if | statement_for | statement_slot)} */

            let any_tag_pair = any_tag_pair.into_inner().next().unwrap();
            let _template_node_definition = match any_tag_pair.as_rule() {
                Rule::statement_if => {
                    //`if` TemplateNodeDefinition
                    let template_node = TemplateNodeDefinition {
                        control_flow_settings: Some(ControlFlowSettingsDefinition::default()),
                        type_id: TypeId::build_if(),
                        settings: None,
                        selector_info: Default::default(),
                        raw_comment_string: None,
                    };

                    let id = match location {
                        TreeLocation::Root => ctx.template.add_root_node_back(template_node),
                        TreeLocation::Parent(id) => ctx.template.add_child_back(id, template_node),
                    };
                    let if_node_id = id.get_template_node_id();
                    let mut first_condition_expression = None;
                    let mut conditional_branches = Vec::new();

                    for branch in any_tag_pair.into_inner() {
                        let branch_rule = branch.as_rule();
                        let mut branch_inner = branch.into_inner();
                        let (branch_kind, condition_expression, prospective_inner_nodes) =
                            match branch_rule {
                                Rule::statement_if_branch => {
                                    let expression_body = branch_inner.next().unwrap();
                                    let expression_info = ExpressionInfo::new(
                                        parse_pax_expression(expression_body.as_str()).unwrap(),
                                    );
                                    (
                                        ControlFlowConditionalBranchKind::If,
                                        Some(expression_info),
                                        branch_inner.next(),
                                    )
                                }
                                Rule::statement_else_if_branch => {
                                    let expression_body = branch_inner.next().unwrap();
                                    let expression_info = ExpressionInfo::new(
                                        parse_pax_expression(expression_body.as_str()).unwrap(),
                                    );
                                    (
                                        ControlFlowConditionalBranchKind::ElseIf,
                                        Some(expression_info),
                                        branch_inner.next(),
                                    )
                                }
                                Rule::statement_else_branch => (
                                    ControlFlowConditionalBranchKind::Else,
                                    None,
                                    branch_inner.next(),
                                ),
                                _ => unreachable!("Parsing error: {:?}", branch_rule),
                            };

                        if first_condition_expression.is_none() {
                            first_condition_expression = condition_expression.clone();
                        }

                        let existing_children_count = ctx
                            .template
                            .get_children(&if_node_id)
                            .unwrap_or_default()
                            .len();

                        if let Some(inner_nodes) = prospective_inner_nodes {
                            inner_nodes.into_inner().for_each(|sub_tag_pair| {
                                recurse_visit_tag_pairs_for_template(
                                    ctx,
                                    sub_tag_pair,
                                    pax,
                                    TreeLocation::Parent(if_node_id.clone()),
                                );
                            })
                        }

                        let child_ids = ctx
                            .template
                            .get_children(&if_node_id)
                            .unwrap_or_default()
                            .into_iter()
                            .skip(existing_children_count)
                            .collect();
                        conditional_branches.push(ControlFlowConditionalBranchDefinition {
                            branch_kind,
                            condition_expression,
                            child_ids,
                        });
                    }

                    let mut if_node = ctx.template.get_node(&if_node_id).unwrap().clone();
                    if let Some(control_flow_settings) = &mut if_node.control_flow_settings {
                        control_flow_settings.condition_expression = first_condition_expression;
                        control_flow_settings.conditional_branches = conditional_branches;
                    }
                    ctx.template.set_node(if_node_id, if_node);
                }
                Rule::statement_for => {
                    let mut cfavd = ControlFlowSettingsDefinition::default();
                    let mut for_statement = any_tag_pair.clone().into_inner();
                    let mut predicate_declaration = for_statement.next().unwrap().into_inner();

                    if predicate_declaration.clone().count() > 1 {
                        //tuple, like the `elem, i` in `for (elem, i) in self.some_list`
                        let elem = predicate_declaration.next().unwrap();
                        let index = predicate_declaration.next().unwrap();
                        cfavd.repeat_predicate_definition =
                            Some(ControlFlowRepeatPredicateDefinition::ElemIdIndexId(
                                elem.as_str().to_owned(),
                                index.as_str().to_owned(),
                            ));
                    } else {
                        let elem = predicate_declaration.next().unwrap();
                        //single identifier, like the `elem` in `for elem in self.some_list`
                        cfavd.repeat_predicate_definition = Some(
                            ControlFlowRepeatPredicateDefinition::ElemId(elem.as_str().to_owned()),
                        );
                    }

                    let mut prospective_inner_nodes = None;
                    for for_statement_child in for_statement {
                        match for_statement_child.as_rule() {
                            Rule::statement_for_source => {
                                let inner_source = for_statement_child.into_inner().next().unwrap();
                                /* statement_for_source = { xo_range | xo_symbol } */
                                let repeat_source_definition = ExpressionInfo::new(
                                    parse_pax_expression(inner_source.as_str()).unwrap(),
                                );
                                cfavd.repeat_source_expression = Some(repeat_source_definition);
                            }
                            Rule::statement_for_key => {
                                let key_expression =
                                    for_statement_child.into_inner().next().unwrap();
                                cfavd.repeat_key_expression = Some(ExpressionInfo::new(
                                    parse_pax_expression(key_expression.as_str()).unwrap(),
                                ));
                            }
                            Rule::inner_nodes => {
                                prospective_inner_nodes = Some(for_statement_child);
                            }
                            _ => unreachable!(),
                        }
                    }

                    //`for` TemplateNodeDefinition
                    let template_node = TemplateNodeDefinition {
                        type_id: TypeId::build_repeat(),
                        control_flow_settings: Some(cfavd),
                        settings: None,
                        selector_info: Default::default(),
                        raw_comment_string: None,
                    };

                    let id = match location {
                        TreeLocation::Root => ctx.template.add_root_node_back(template_node),
                        TreeLocation::Parent(id) => ctx.template.add_child_back(id, template_node),
                    };

                    if let Some(inner_nodes) = prospective_inner_nodes {
                        inner_nodes.into_inner().for_each(|sub_tag_pair| {
                            recurse_visit_tag_pairs_for_template(
                                ctx,
                                sub_tag_pair,
                                pax,
                                TreeLocation::Parent(id.clone().get_template_node_id()),
                            );
                        })
                    }
                }
                Rule::statement_slot => {
                    let mut statement_slot = any_tag_pair.into_inner();
                    let expression_body = statement_slot.next().unwrap();
                    let slot_expression = ExpressionInfo::new(
                        parse_pax_expression(expression_body.as_str()).unwrap(),
                    );
                    let template_node = TemplateNodeDefinition {
                        control_flow_settings: Some(ControlFlowSettingsDefinition {
                            condition_expression: None,
                            slot_index_expression: Some(slot_expression),
                            repeat_predicate_definition: None,
                            repeat_source_expression: None,
                            repeat_key_expression: None,
                            conditional_branches: vec![],
                            route_branches: vec![],
                        }),
                        type_id: TypeId::build_slot(),
                        settings: None,
                        selector_info: Default::default(),
                        raw_comment_string: None,
                    };

                    let _ = match location {
                        TreeLocation::Root => ctx.template.add_root_node_back(template_node),
                        TreeLocation::Parent(id) => ctx.template.add_child_back(id, template_node),
                    };
                }
                _ => {
                    unreachable!("Parsing error: {:?}", any_tag_pair.as_rule());
                }
            };
        }
        Rule::comment => {
            let template_node = TemplateNodeDefinition {
                control_flow_settings: None,
                type_id: TypeId::build_comment(),
                settings: None,
                selector_info: Default::default(),
                raw_comment_string: Some(any_tag_pair.as_str().to_string()),
            };
            let _ = match location {
                TreeLocation::Root => ctx.template.add_root_node_back(template_node),
                TreeLocation::Parent(id) => ctx.template.add_child_back(id, template_node),
            };
        }
        Rule::node_inner_content => {
            //For example:  `<Text>"I am inner content"</Text>`
            unimplemented!("Inner content not yet supported");
        }
        _ => {
            unreachable!("Parsing error: {:?}", any_tag_pair.as_rule());
        }
    }
}

fn parse_router_matched_tag(
    ctx: &mut TemplateNodeParseContext,
    matched_tag: Pair<Rule>,
    pax: &str,
    location: TreeLocation,
) {
    let template_node = TemplateNodeDefinition {
        type_id: TypeId::build_router(),
        settings: None,
        selector_info: Default::default(),
        raw_comment_string: None,
        control_flow_settings: Some(ControlFlowSettingsDefinition::default()),
    };

    let id = match location {
        TreeLocation::Root => ctx.template.add_root_node_back(template_node),
        TreeLocation::Parent(id) => ctx.template.add_child_back(id, template_node),
    };
    let router_node_id = id.get_template_node_id();
    let prospective_inner_nodes = matched_tag.into_inner().nth(1).unwrap();
    let mut route_branches = Vec::new();

    match prospective_inner_nodes.as_rule() {
        Rule::inner_nodes => {
            for sub_tag_pair in prospective_inner_nodes.into_inner() {
                match sub_tag_pair.as_rule() {
                    Rule::matched_tag => {
                        let route_pascal_identifier = sub_tag_pair
                            .clone()
                            .into_inner()
                            .next()
                            .unwrap()
                            .into_inner()
                            .next()
                            .unwrap()
                            .as_str()
                            .to_string();

                        if route_pascal_identifier != "Route" {
                            panic!("Router expects direct Route children");
                        }

                        route_branches.push(parse_route_branch_from_matched_tag(
                            ctx,
                            sub_tag_pair,
                            pax,
                            &router_node_id,
                        ));
                    }
                    Rule::self_closing_tag => {
                        let route_pascal_identifier = sub_tag_pair
                            .clone()
                            .into_inner()
                            .next()
                            .unwrap()
                            .as_str()
                            .to_string();

                        if route_pascal_identifier != "Route" {
                            panic!("Router expects direct Route children");
                        }

                        route_branches.push(parse_route_branch_from_self_closing_tag(sub_tag_pair));
                    }
                    Rule::comment => recurse_visit_tag_pairs_for_template(
                        ctx,
                        sub_tag_pair,
                        pax,
                        TreeLocation::Parent(router_node_id.clone()),
                    ),
                    _ => panic!("Router expects direct Route children"),
                }
            }
        }
        _ => panic!("wrong prospective inner nodes (or nth)"),
    }

    let mut router_node = ctx.template.get_node(&router_node_id).unwrap().clone();
    if let Some(control_flow_settings) = &mut router_node.control_flow_settings {
        control_flow_settings.route_branches = route_branches;
    }
    ctx.template.set_node(router_node_id, router_node);
}

fn parse_router_self_closing_tag(ctx: &mut TemplateNodeParseContext, location: TreeLocation) {
    let template_node = TemplateNodeDefinition {
        type_id: TypeId::build_router(),
        settings: None,
        selector_info: Default::default(),
        raw_comment_string: None,
        control_flow_settings: Some(ControlFlowSettingsDefinition::default()),
    };

    match location {
        TreeLocation::Root => {
            ctx.template.add_root_node_back(template_node);
        }
        TreeLocation::Parent(id) => {
            ctx.template.add_child_back(id, template_node);
        }
    };
}

fn parse_route_branch_from_matched_tag(
    ctx: &mut TemplateNodeParseContext,
    matched_tag: Pair<Rule>,
    pax: &str,
    router_node_id: &TemplateNodeId,
) -> ControlFlowRouteBranchDefinition {
    let mut open_tag = matched_tag
        .clone()
        .into_inner()
        .next()
        .unwrap()
        .into_inner();
    let _ = open_tag.next().unwrap();
    let route_settings = parse_inline_attribute_from_final_pairs_of_tag(open_tag);
    let existing_children_count = ctx
        .template
        .get_children(router_node_id)
        .unwrap_or_default()
        .len();

    if let Some(inner_nodes) = matched_tag.into_inner().nth(1) {
        inner_nodes.into_inner().for_each(|sub_tag_pair| {
            recurse_visit_tag_pairs_for_template(
                ctx,
                sub_tag_pair,
                pax,
                TreeLocation::Parent(router_node_id.clone()),
            );
        });
    }

    let child_ids = ctx
        .template
        .get_children(router_node_id)
        .unwrap_or_default()
        .into_iter()
        .skip(existing_children_count)
        .collect::<Vec<_>>();

    parse_route_branch_settings(route_settings, child_ids)
}

fn parse_route_branch_from_self_closing_tag(
    self_closing_tag: Pair<Rule>,
) -> ControlFlowRouteBranchDefinition {
    let mut tag_pairs = self_closing_tag.into_inner();
    let _ = tag_pairs.next().unwrap();
    let route_settings = parse_inline_attribute_from_final_pairs_of_tag(tag_pairs);
    parse_route_branch_settings(route_settings, vec![])
}

fn parse_route_branch_settings(
    settings: Option<Vec<SettingElement>>,
    child_ids: Vec<TemplateNodeId>,
) -> ControlFlowRouteBranchDefinition {
    let mut path = None;
    let mut is_default = false;

    for setting in settings.unwrap_or_default() {
        let SettingElement::Setting(token, value) = setting else {
            continue;
        };

        match token.token_value.as_str() {
            "path" => {
                path = Some(parse_route_path_setting(&value));
            }
            "default" => {
                is_default = parse_route_default_setting(&value);
            }
            other => panic!("Unsupported Route attribute {other}"),
        }
    }

    if is_default && path.is_some() {
        panic!("default Route cannot also declare path");
    }

    if !is_default && path.is_none() {
        panic!("Route requires path or default=true");
    }

    ControlFlowRouteBranchDefinition {
        path,
        default: is_default,
        child_ids,
    }
}

fn parse_route_path_setting(value: &ValueDefinition) -> String {
    match value {
        ValueDefinition::LiteralValue(PaxValue::String(path)) => {
            if path.is_empty() {
                panic!("Route path must not be empty");
            }
            path.clone()
        }
        _ => panic!("Route path must be a string literal"),
    }
}

fn parse_route_default_setting(value: &ValueDefinition) -> bool {
    match value {
        ValueDefinition::LiteralValue(PaxValue::Bool(value)) => *value,
        _ => panic!("Route default must be a boolean literal"),
    }
}

fn parse_literal_function(literal_function_full: Pair<Rule>) -> Token {
    let literal_function = literal_function_full.clone().into_inner().next().unwrap();

    let location_info = span_to_location(&literal_function.as_span());
    let literal_function_token = Token::new(literal_function.as_str().to_string(), location_info);
    literal_function_token
}

fn parse_event_id(event_id_full: Pair<Rule>) -> Token {
    let event_id = event_id_full.clone().into_inner().next().unwrap();

    let event_id_location = span_to_location(&event_id.as_span());
    let event_id_token = Token::new(event_id.as_str().to_string(), event_id_location);
    event_id_token
}

fn parse_inline_attribute_from_final_pairs_of_tag(
    final_pairs_of_tag: Pairs<Rule>,
) -> Option<Vec<SettingElement>> {
    let vec: Vec<SettingElement> = final_pairs_of_tag
        .map(|attribute_key_value_pair| {
            match attribute_key_value_pair
                .clone()
                .into_inner()
                .next()
                .unwrap()
                .as_rule()
            {
                Rule::double_binding => {
                    let mut kv = attribute_key_value_pair.into_inner();
                    let mut double_binding = kv.next().unwrap().into_inner();

                    let setting: Pair<Rule> = double_binding.next().unwrap();
                    let property = double_binding.next().unwrap();
                    let setting_location = span_to_location(&setting.as_span());
                    let setting_token = Token::new(setting.as_str().to_string(), setting_location);

                    SettingElement::Setting(
                        setting_token,
                        ValueDefinition::DoubleBinding(PaxIdentifier::new(property.as_str())),
                    )
                }
                Rule::attribute_event_binding => {
                    // attribute_event_binding = {event_id ~ "=" ~ literal_function}
                    let mut kv = attribute_key_value_pair.into_inner();
                    let mut attribute_event_binding = kv.next().unwrap().into_inner();

                    let event_id_token = parse_event_id(attribute_event_binding.next().unwrap());

                    let literal_function = attribute_event_binding.next().unwrap().as_str();
                    SettingElement::Setting(
                        event_id_token,
                        ValueDefinition::EventBindingTarget(PaxIdentifier::new(literal_function)),
                    )
                }
                _ => {
                    //Vanilla `key=value` setting pair

                    let mut kv = attribute_key_value_pair.into_inner();
                    let key = kv.next().unwrap();
                    let key_location = span_to_location(&key.as_span());
                    let key_token = Token::new(key.as_str().to_string(), key_location);
                    let value_outer =
                        kv.next()
                            .expect(&format!("key: {}, kvs: {}", key.as_str(), kv));
                    let value = value_outer.clone().into_inner().next().expect(&format!(
                        "key: {}, value: {}",
                        key.as_str(),
                        value_outer.as_str()
                    ));
                    let value_definition = parse_setting_value_definition(key.as_str(), value);
                    SettingElement::Setting(key_token, value_definition)
                }
            }
        })
        .collect();

    if vec.len() > 0 {
        Some(vec)
    } else {
        None
    }
}

fn parse_setting_value_definition(setting_key: &str, value: Pair<Rule>) -> ValueDefinition {
    reject_padding_tuple_syntax(setting_key, &value);
    parse_value_definition(value)
}

fn reject_padding_tuple_syntax(setting_key: &str, value: &Pair<Rule>) {
    if setting_key == "padding" && contains_tuple_value(value.clone()) {
        panic!("padding does not support tuple syntax; use padding=[x, y] for axis padding");
    }
}

fn contains_tuple_value(value: Pair<Rule>) -> bool {
    matches!(value.as_rule(), Rule::literal_tuple | Rule::xo_tuple)
        || value.into_inner().any(contains_tuple_value)
}

pub fn parse_value_definition(value: Pair<Rule>) -> ValueDefinition {
    match value.as_rule() {
        Rule::timeline_keyframe_value | Rule::timeline_block_setting_value => {
            parse_value_definition(value.into_inner().next().unwrap())
        }
        Rule::timeline_symbol => ValueDefinition::Identifier(PaxIdentifier::new(value.as_str())),
        Rule::timeline_inline_value => {
            let timeline_track = value.into_inner().next().unwrap();
            ValueDefinition::Timeline(derive_timeline_track_definition(timeline_track))
        }
        Rule::literal_value => {
            let inner = value.into_inner().next().unwrap();
            match inner.as_rule() {
                Rule::literal_object => {
                    let literal = from_pax(inner.as_str());
                    match literal {
                        Ok(value) => ValueDefinition::LiteralValue(value),
                        Err(_) => ValueDefinition::Block(
                            derive_value_definition_from_literal_object_pair(inner),
                        ),
                    }
                }
                _ => {
                    let literal = from_pax(inner.as_str())
                        .expect(&format!("Unable to parse literal: {:?}", inner));
                    ValueDefinition::LiteralValue(literal)
                }
            }
        }
        Rule::expression_body => {
            let expression =
                parse_pax_expression_from_pair(value).expect("Unable to parse expression");
            ValueDefinition::Expression(ExpressionInfo::new(expression))
        }
        Rule::identifier => {
            let identifier = PaxIdentifier::new(value.as_str());
            ValueDefinition::Identifier(identifier)
        }
        _ => {
            unreachable!(
                "Unexpected attribute value pair rule: {:?}",
                value.as_rule()
            );
        }
    }
}

fn parse_timeline_marker(marker: Pair<Rule>) -> TimelineMarker {
    match marker.as_rule() {
        Rule::timeline_marker => parse_timeline_marker(marker.into_inner().next().unwrap()),
        Rule::literal_number_integer => TimelineMarker::Frame(
            marker
                .as_str()
                .parse()
                .expect("timeline frame markers must be integers"),
        ),
        Rule::timeline_duration => match from_pax(marker.as_str())
            .expect("timeline duration markers must be valid Pax duration literals")
        {
            PaxValue::Duration(duration) => TimelineMarker::Duration(duration),
            _ => unreachable!("timeline duration markers must parse to Duration"),
        },
        Rule::timeline_percent => {
            let raw = marker.as_str().trim_end_matches('%');
            TimelineMarker::Percent(
                raw.parse()
                    .expect("timeline percent markers must be numeric"),
            )
        }
        _ => unreachable!("Unexpected timeline marker rule: {:?}", marker.as_rule()),
    }
}

fn derive_timeline_keyframe_definition(timeline_keyframe: Pair<Rule>) -> TimelineKeyframe {
    let mut pairs = timeline_keyframe.into_inner();
    let marker = parse_timeline_marker(pairs.next().unwrap());
    let value = parse_value_definition(pairs.next().unwrap());
    let easing = pairs.next().map(|easing| {
        let location = span_to_location(&easing.as_span());
        Token::new(easing.as_str().to_string(), location)
    });
    TimelineKeyframe {
        marker,
        value,
        easing,
    }
}

fn apply_timeline_setting_to_track(track: &mut TimelineTrackDefinition, setting: Pair<Rule>) {
    let mut pairs = setting.into_inner();
    let key = pairs.next().unwrap().into_inner().next().unwrap();
    let value = pairs.next().unwrap();
    let parsed_value = parse_value_definition(value);

    match key.as_str() {
        "frames" => {
            panic!(
                "timeline setting `frames` has been removed; use `duration` with a unitless frame count or `f` unit"
            )
        }
        "duration" => {
            track.duration = Some(Box::new(parsed_value));
        }
        "loop" => {
            if let ValueDefinition::LiteralValue(PaxValue::Bool(value)) = parsed_value {
                track.repeat = Some(value);
            }
        }
        "playhead" => {
            track.playhead = Some(Box::new(parsed_value));
        }
        _ => {}
    }
}

fn derive_timeline_track_definition(timeline_track: Pair<Rule>) -> TimelineTrackDefinition {
    let mut track = TimelineTrackDefinition {
        elements: vec![],
        playhead: None,
        duration: None,
        repeat: None,
        starting_value: None,
        use_local_property_scope: false,
    };

    for pair in timeline_track.into_inner() {
        match pair.as_rule() {
            Rule::timeline_block_setting => apply_timeline_setting_to_track(&mut track, pair),
            Rule::timeline_keyframe => track.elements.push(TimelineTrackElement::Keyframe(
                derive_timeline_keyframe_definition(pair),
            )),
            Rule::comment => track
                .elements
                .push(TimelineTrackElement::Comment(pair.as_str().to_string())),
            _ => unreachable!("Unexpected timeline track rule: {:?}", pair.as_rule()),
        }
    }

    track
}

fn derive_timeline_selector_block_definition(
    timeline_selector_body: Pair<Rule>,
) -> TimelineSelectorBlockDefinition {
    TimelineSelectorBlockDefinition {
        elements: timeline_selector_body
            .into_inner()
            .map(|pair| match pair.as_rule() {
                Rule::timeline_property_key_value_pair => {
                    let mut pairs = pair.into_inner();
                    let property_key = pairs.next().unwrap().into_inner().next().unwrap();
                    let property_key_location = span_to_location(&property_key.as_span());
                    let property_key_token =
                        Token::new(property_key.as_str().to_string(), property_key_location);
                    let track = derive_timeline_track_definition(pairs.next().unwrap());
                    TimelineSelectorElement::Track(property_key_token, track)
                }
                Rule::comment => TimelineSelectorElement::Comment(pair.as_str().to_string()),
                _ => unreachable!("Unexpected timeline selector rule: {:?}", pair.as_rule()),
            })
            .collect(),
    }
}

pub fn parse_timeline_from_component_definition_string(
    pax_component_definition: Pair<Rule>,
) -> Vec<TimelineDefinition> {
    let mut timelines = Vec::new();

    pax_component_definition
        .into_inner()
        .for_each(|top_level_pair| {
            if top_level_pair.as_rule() != Rule::timeline_block_declaration {
                return;
            }

            let mut timeline = TimelineDefinition::default();
            let mut pairs = top_level_pair.into_inner().peekable();

            if matches!(
                pairs.peek().map(|pair| pair.as_rule()),
                Some(Rule::identifier)
            ) {
                let raw_name = pairs.next().unwrap();
                timeline.name = Some(Token::new(
                    raw_name.as_str().to_string(),
                    span_to_location(&raw_name.as_span()),
                ));
            }

            for timeline_entity in pairs {
                match timeline_entity.as_rule() {
                    Rule::timeline_block_setting => {
                        let mut pairs = timeline_entity.into_inner();
                        let key = pairs.next().unwrap().into_inner().next().unwrap();
                        let value = pairs.next().unwrap();
                        let parsed_value = parse_value_definition(value);
                        match key.as_str() {
                            "frames" => {
                                panic!(
                                    "timeline setting `frames` has been removed; use `duration` with a unitless frame count or `f` unit"
                                )
                            }
                            "duration" => {
                                timeline.duration = Some(parsed_value);
                            }
                            "loop" => {
                                if let ValueDefinition::LiteralValue(PaxValue::Bool(value)) =
                                    parsed_value
                                {
                                    timeline.repeat = value;
                                }
                            }
                            "playhead" => {
                                timeline.playhead = Some(parsed_value);
                            }
                            _ => {}
                        }
                    }
                    Rule::timeline_selector_block => {
                        let mut selector_block_pairs = timeline_entity.into_inner();
                        let raw_target = selector_block_pairs.next().unwrap();
                        let raw_value_location = span_to_location(&raw_target.as_span());
                        let target: String = raw_target
                            .as_str()
                            .chars()
                            .filter(|c| !c.is_whitespace())
                            .collect();
                        let token = Token::new(target, raw_value_location);
                        let selector_body = selector_block_pairs.next().unwrap();
                        timeline.elements.push(TimelineBlockElement::SelectorBlock(
                            token,
                            derive_timeline_selector_block_definition(selector_body),
                        ));
                    }
                    Rule::comment => {
                        timeline.elements.push(TimelineBlockElement::Comment(
                            timeline_entity.as_str().to_string(),
                        ));
                    }
                    _ => {
                        unreachable!(
                            "Unexpected timeline block rule: {:?}",
                            timeline_entity.as_rule()
                        );
                    }
                }
            }

            if !timeline.elements.is_empty()
                || timeline.duration.is_some()
                || timeline.playhead.is_some()
                || !timeline.repeat
                || timeline.name.is_some()
            {
                timelines.push(timeline);
            }
        });

    timelines
}

fn derive_value_definition_from_literal_object_pair(
    literal_object: Pair<Rule>,
) -> LiteralBlockDefinition {
    derive_value_definition_from_literal_object_pair_inner(literal_object, false)
}

fn derive_settings_block_value_definition_from_literal_object_pair(
    literal_object: Pair<Rule>,
) -> LiteralBlockDefinition {
    derive_value_definition_from_literal_object_pair_inner(literal_object, true)
}

fn derive_value_definition_from_literal_object_pair_inner(
    literal_object: Pair<Rule>,
    validate_setting_syntax: bool,
) -> LiteralBlockDefinition {
    let mut literal_object_pairs = literal_object.into_inner();

    if let None = literal_object_pairs.peek() {
        return LiteralBlockDefinition {
            explicit_type_pascal_identifier: None,
            elements: vec![],
        };
    }

    let explicit_type_pascal_identifier = match literal_object_pairs.peek().unwrap().as_rule() {
        Rule::pascal_identifier => {
            let raw_value = literal_object_pairs.next().unwrap();
            let raw_value_location = span_to_location(&raw_value.as_span());
            let token = Token::new(raw_value.as_str().to_string(), raw_value_location);
            Some(token)
        }
        _ => None,
    };

    LiteralBlockDefinition {
        explicit_type_pascal_identifier,
        elements: literal_object_pairs
            .map(
                |settings_key_value_pair| match settings_key_value_pair.as_rule() {
                    Rule::settings_key_value_pair => {
                        let mut pairs = settings_key_value_pair.into_inner();

                        let setting_key = pairs.next().unwrap().into_inner().next().unwrap();
                        let setting_key_location = span_to_location(&setting_key.as_span());
                        let setting_key_token =
                            Token::new(setting_key.as_str().to_string(), setting_key_location);
                        let value = pairs.next().unwrap().into_inner().next().unwrap();
                        let setting_value_definition = if validate_setting_syntax {
                            parse_setting_value_definition(setting_key.as_str(), value)
                        } else {
                            parse_value_definition(value)
                        };

                        SettingElement::Setting(setting_key_token, setting_value_definition)
                    }
                    Rule::comment => {
                        let comment = settings_key_value_pair.as_str().to_string();
                        SettingElement::Comment(comment)
                    }
                    _ => {
                        unreachable!(
                            "Parsing error 2314314145: {:?}",
                            settings_key_value_pair.as_rule()
                        );
                    }
                },
            )
            .collect(),
    }
}

pub fn parse_settings_from_component_definition_string(
    pax_component_definition: Pair<Rule>,
) -> Vec<SettingsBlockElement> {
    let mut settings: Vec<SettingsBlockElement> = vec![];

    pax_component_definition
        .into_inner()
        .for_each(|top_level_pair| {
            match top_level_pair.as_rule() {
                Rule::settings_block_declaration => {
                    top_level_pair
                        .into_inner()
                        .for_each(|top_level_settings_block_entity| {
                            match top_level_settings_block_entity.as_rule() {
                                Rule::settings_event_binding => {
                                    //event handler binding in the form of `@pre_render: handle_pre_render`
                                    let mut settings_event_binding_pairs =
                                        top_level_settings_block_entity.into_inner();
                                    let event_id_token = parse_event_id(
                                        settings_event_binding_pairs.next().unwrap(),
                                    );
                                    let literal_function_token = parse_literal_function(
                                        settings_event_binding_pairs.next().unwrap(),
                                    );
                                    let event_name = event_id_token.token_value.as_str();
                                    if matches!(event_name, "in" | "out") {
                                        settings.push(SettingsBlockElement::Transition(
                                            event_id_token,
                                            literal_function_token,
                                        ));
                                    } else {
                                        let handler_element: SettingsBlockElement =
                                            SettingsBlockElement::Handler(
                                                event_id_token,
                                                vec![literal_function_token],
                                            );
                                        settings.push(handler_element);
                                    }
                                }
                                Rule::selector_block => {
                                    //selector_block => settings_key_value_pair where v is a ValueDefinition
                                    let mut selector_block_pairs =
                                        top_level_settings_block_entity.into_inner();
                                    //first pair is the selector itself
                                    let raw_selector = selector_block_pairs.next().unwrap();
                                    let raw_value_location =
                                        span_to_location(&raw_selector.as_span());
                                    let selector: String = raw_selector
                                        .as_str()
                                        .chars()
                                        .filter(|c| !c.is_whitespace())
                                        .collect();
                                    let token = Token::new(selector, raw_value_location);
                                    let literal_object = selector_block_pairs.next().unwrap();

                                    settings.push(SettingsBlockElement::SelectorBlock(
                                        token,
                                        derive_settings_block_value_definition_from_literal_object_pair(
                                            literal_object,
                                        ),
                                    ));
                                }
                                Rule::comment => {
                                    let comment =
                                        top_level_settings_block_entity.as_str().to_string();
                                    settings.push(SettingsBlockElement::Comment(comment));
                                }
                                _ => {
                                    unreachable!(
                                        "Parsing error: {:?}",
                                        top_level_settings_block_entity.as_rule()
                                    );
                                }
                            }
                        });
                }
                _ => {}
            }
        });
    settings
}

/// Accumulator for manifest parsing across components and reflected types.
pub struct ParsingContext {
    /// Used to track which files/sources have been visited during parsing,
    /// to prevent duplicate parsing
    pub visited_type_ids: HashSet<TypeId>,

    pub main_component_type_id: TypeId,

    pub component_definitions: BTreeMap<TypeId, ComponentDefinition>,

    pub template_map: HashMap<String, TypeId>,

    pub template_node_definitions: ComponentTemplate,

    pub type_table: TypeTable,

    pub assets_dirs: Vec<String>,
}

impl Default for ParsingContext {
    fn default() -> Self {
        Self {
            main_component_type_id: TypeId::default(),
            visited_type_ids: HashSet::new(),
            component_definitions: BTreeMap::new(),
            template_map: HashMap::new(),
            type_table: get_primitive_type_table(),
            template_node_definitions: ComponentTemplate::default(),
            assets_dirs: vec![],
        }
    }
}

#[derive(Debug)]
/// Source-mapped parsing error payload used by designer/editor integrations.
pub struct ParsingError {
    pub error_name: String,
    pub error_message: String,
    pub matched_string: String,
    pub start: (usize, usize),
    pub end: (usize, usize),
}

const IMPLICIT_LIFECYCLE_HANDLER_CANDIDATES: [(&str, [&str; 2]); 4] = [
    ("mount", ["on_mount", "mount"]),
    ("tick", ["on_tick", "tick"]),
    ("pre_render", ["on_pre_render", "pre_render"]),
    ("unmount", ["on_unmount", "unmount"]),
];

fn add_implicit_lifecycle_handlers(
    settings: &mut Vec<SettingsBlockElement>,
    module_path: &str,
    self_type_id: &TypeId,
    rust_source_file_path: &str,
) {
    let explicit_events = settings
        .iter()
        .filter_map(|setting| match setting {
            SettingsBlockElement::Handler(key, _) => Some(key.token_value.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();

    let available_handlers =
        collect_component_lifecycle_handler_names(module_path, self_type_id, rust_source_file_path);

    for (event_name, candidates) in IMPLICIT_LIFECYCLE_HANDLER_CANDIDATES {
        if explicit_events.contains(event_name) {
            continue;
        }

        if let Some(handler_name) = candidates
            .iter()
            .find(|candidate| available_handlers.contains(**candidate))
        {
            settings.push(SettingsBlockElement::Handler(
                Token::new_without_location(event_name.to_string()),
                vec![Token::new_without_location((*handler_name).to_string())],
            ));
        }
    }
}

pub fn augment_settings_with_implicit_lifecycle_handlers(
    settings: &mut Vec<SettingsBlockElement>,
    module_path: &str,
    self_type_id: &TypeId,
    rust_source_file_path: &str,
) {
    let modified_module_path = clean_module_path(module_path);
    add_implicit_lifecycle_handlers(
        settings,
        &modified_module_path,
        self_type_id,
        rust_source_file_path,
    );
}

fn collect_component_lifecycle_handler_names(
    target_module_path: &str,
    self_type_id: &TypeId,
    rust_source_file_path: &str,
) -> HashSet<String> {
    let source = match fs::read_to_string(rust_source_file_path) {
        Ok(source) => source,
        Err(err) => {
            log::warn!(
                "Failed to read Rust source `{}` while inferring implicit lifecycle handlers: {}",
                rust_source_file_path,
                err
            );
            return HashSet::new();
        }
    };

    let parsed_file = match syn::parse_file(&source) {
        Ok(parsed_file) => parsed_file,
        Err(err) => {
            log::warn!(
                "Failed to parse Rust source `{}` while inferring implicit lifecycle handlers: {}",
                rust_source_file_path,
                err
            );
            return HashSet::new();
        }
    };

    let root_module_path = source_root_module_path(rust_source_file_path)
        .unwrap_or_else(|| target_module_path.to_string());
    let target_import_path = self_type_id
        .import_path()
        .unwrap_or_else(|| self_type_id.to_string());
    let target_ident = self_type_id
        .get_pascal_identifier()
        .unwrap_or_else(|| self_type_id.to_string());
    let mut handlers = HashSet::new();

    collect_component_lifecycle_handler_names_from_items(
        &parsed_file.items,
        &root_module_path,
        target_module_path,
        &target_import_path,
        &target_ident,
        &mut handlers,
    );

    handlers
}

fn collect_component_lifecycle_handler_names_from_items(
    items: &[Item],
    current_module_path: &str,
    target_module_path: &str,
    target_import_path: &str,
    target_ident: &str,
    handlers: &mut HashSet<String>,
) {
    for item in items {
        match item {
            Item::Impl(item_impl)
                if impl_targets_component(
                    item_impl,
                    current_module_path,
                    target_module_path,
                    target_import_path,
                    target_ident,
                ) =>
            {
                for impl_item in &item_impl.items {
                    if let ImplItem::Method(method) = impl_item {
                        if is_valid_implicit_lifecycle_method(method) {
                            handlers.insert(method.sig.ident.to_string());
                        }
                    }
                }
            }
            Item::Mod(item_mod) => {
                if let Some((_, module_items)) = &item_mod.content {
                    let child_module_path = format!("{}::{}", current_module_path, item_mod.ident);
                    collect_component_lifecycle_handler_names_from_items(
                        module_items,
                        &child_module_path,
                        target_module_path,
                        target_import_path,
                        target_ident,
                        handlers,
                    );
                }
            }
            _ => {}
        }
    }
}

fn impl_targets_component(
    item_impl: &syn::ItemImpl,
    current_module_path: &str,
    target_module_path: &str,
    target_import_path: &str,
    target_ident: &str,
) -> bool {
    if item_impl.trait_.is_some() {
        return false;
    }

    let Type::Path(type_path) = item_impl.self_ty.as_ref() else {
        return false;
    };

    let canonical_path = canonicalize_path(&type_path.path, current_module_path);
    if canonical_path == target_import_path {
        return true;
    }

    type_path.path.segments.len() == 1
        && type_path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == target_ident)
        && current_module_path == target_module_path
}

fn canonicalize_path(path: &syn::Path, current_module_path: &str) -> String {
    let segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();

    if segments.is_empty() {
        return current_module_path.to_string();
    }

    match segments[0].as_str() {
        "crate" => segments.join("::"),
        "self" => {
            if segments.len() == 1 {
                current_module_path.to_string()
            } else {
                format!("{}::{}", current_module_path, segments[1..].join("::"))
            }
        }
        "super" => {
            let mut module_segments = current_module_path
                .split("::")
                .map(str::to_string)
                .collect::<Vec<_>>();
            let mut index = 0;
            while index < segments.len() && segments[index] == "super" {
                if module_segments.len() > 1 {
                    module_segments.pop();
                }
                index += 1;
            }
            module_segments.extend(segments[index..].iter().cloned());
            module_segments.join("::")
        }
        _ => format!("{}::{}", current_module_path, segments.join("::")),
    }
}

fn is_valid_implicit_lifecycle_method(method: &ImplItemMethod) -> bool {
    matches!(method.vis, Visibility::Public(_))
        && method.sig.inputs.len() == 2
        && matches!(method.sig.inputs.first(), Some(FnArg::Receiver(_)))
        && method
            .sig
            .inputs
            .iter()
            .nth(1)
            .is_some_and(is_node_context_arg)
}

fn is_node_context_arg(arg: &FnArg) -> bool {
    match arg {
        FnArg::Typed(arg) => type_ends_with_ident(arg.ty.as_ref(), "NodeContext"),
        FnArg::Receiver(_) => false,
    }
}

fn type_ends_with_ident(ty: &Type, ident: &str) -> bool {
    match ty {
        Type::Reference(reference) => type_ends_with_ident(reference.elem.as_ref(), ident),
        Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == ident),
        _ => false,
    }
}

fn source_root_module_path(rust_source_file_path: &str) -> Option<String> {
    let path = Path::new(rust_source_file_path);
    let components = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    let src_index = components
        .iter()
        .rposition(|component| component == "src")?;
    let relative_components = &components[(src_index + 1)..];
    let file_name = relative_components.last()?;

    let mut module_segments = vec!["crate".to_string()];
    module_segments.extend(
        relative_components[..relative_components.len().saturating_sub(1)]
            .iter()
            .cloned(),
    );

    match file_name.as_str() {
        "lib.rs" | "main.rs" | "mod.rs" => {}
        file_name if file_name.ends_with(".rs") => {
            module_segments.push(file_name.trim_end_matches(".rs").to_string());
        }
        _ => return None,
    }

    Some(module_segments.join("::"))
}

/// From a raw string of Pax representing a single component, parse a complete ComponentDefinition
pub fn assemble_component_definition(
    mut ctx: ParsingContext,
    pax: &str,
    is_main_component: bool,
    template_map: HashMap<String, TypeId>,
    module_path: &str,
    self_type_id: TypeId,
    template_source_file_path: &str,
    rust_source_file_path: &str,
) -> (ParsingContext, ComponentDefinition) {
    let mut tpc = TemplateNodeParseContext {
        pascal_identifier_to_type_id_map: template_map,
        template: ComponentTemplate::new(
            self_type_id.clone(),
            Some(template_source_file_path.to_owned()),
        ),
    };

    let ast = parse_pax_str(Rule::pax_component_definition, pax).expect("Unsuccessful parse");

    parse_template_from_component_definition_string(&mut tpc, pax, ast.clone());
    let modified_module_path = if module_path.starts_with("parser") {
        module_path.replacen("parser", "crate", 1)
    } else {
        module_path.to_string()
    };

    //populate template_node_definitions vec, needed for traversing node tree at codegen-time
    ctx.template_node_definitions = tpc.template.clone();

    let mut settings = parse_settings_from_component_definition_string(ast.clone());
    let timelines = parse_timeline_from_component_definition_string(ast);
    add_implicit_lifecycle_handlers(
        &mut settings,
        &modified_module_path,
        &self_type_id,
        rust_source_file_path,
    );

    let new_def = ComponentDefinition {
        is_primitive: false,
        is_struct_only_component: false,
        is_main_component,
        primitive_instance_import_path: None,
        type_id: self_type_id,
        template: Some(tpc.template),
        settings: Some(settings),
        timelines,
        module_path: modified_module_path,
    };

    (ctx, new_def)
}

/// Convert macro-parser module roots into crate-relative paths.
pub fn clean_module_path(module_path: &str) -> String {
    if module_path.starts_with("parser") {
        module_path.replacen("parser", "crate", 1)
    } else {
        module_path.to_string()
    }
}

/// Build a component definition for a `#[pax]` data struct with no template.
pub fn assemble_struct_only_component_definition(
    ctx: ParsingContext,
    module_path: &str,
    self_type_id: TypeId,
) -> (ParsingContext, ComponentDefinition) {
    let modified_module_path = clean_module_path(module_path);

    let new_def = ComponentDefinition {
        type_id: self_type_id,
        is_main_component: false,
        is_primitive: false,
        is_struct_only_component: true,
        module_path: modified_module_path,
        primitive_instance_import_path: None,
        template: None,
        settings: None,
        timelines: vec![],
    };
    (ctx, new_def)
}

/// Build a component definition for a built-in primitive.
pub fn assemble_primitive_definition(
    module_path: &str,
    primitive_instance_import_path: String,
    self_type_id: TypeId,
) -> ComponentDefinition {
    let modified_module_path = clean_module_path(module_path);

    ComponentDefinition {
        is_primitive: true,
        is_struct_only_component: false,
        primitive_instance_import_path: Some(primitive_instance_import_path),
        is_main_component: false,
        type_id: self_type_id,
        template: None,
        settings: None,
        timelines: vec![],
        module_path: modified_module_path,
    }
}

/// Insert and return a reflected type definition.
pub fn assemble_type_definition(
    mut ctx: ParsingContext,
    property_definitions: Vec<PropertyDefinition>,
    inner_iterable_type_id: Option<TypeId>,
    self_type_id: TypeId,
) -> (ParsingContext, TypeDefinition) {
    let new_def = TypeDefinition {
        type_id: self_type_id.clone(),
        inner_iterable_type_id,
        property_definitions,
    };

    ctx.type_table.insert(self_type_id, new_def.clone());

    (ctx, new_def)
}

/// Given a Pest Span returns starting and ending (line,col)
fn span_to_location(span: &Span) -> LocationInfo {
    let start = (
        span.start_pos().line_col().0 - 1,
        span.start_pos().line_col().1 - 1,
    );
    let end = (
        span.end_pos().line_col().0 - 1,
        span.end_pos().line_col().1 - 1,
    );
    LocationInfo {
        start_line_col: start,
        end_line_col: end,
    }
}

pub fn clean_and_split_symbols(possibly_nested_symbols: &str) -> Vec<String> {
    let entire_symbol = if possibly_nested_symbols.starts_with("self.") {
        possibly_nested_symbols.replacen("self.", "", 1)
    } else if possibly_nested_symbols.starts_with("this.") {
        possibly_nested_symbols.replacen("this.", "", 1)
    } else {
        possibly_nested_symbols.to_string()
    };

    let trimmed_symbol = entire_symbol.trim();

    trimmed_symbol
        .split(".")
        .map(|atomic_symbol| atomic_symbol.to_string())
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod padding_syntax_tests {
    use super::*;

    fn parse_inline_settings(source: &str) -> Option<Vec<SettingElement>> {
        let tag = parse_pax_str(Rule::self_closing_tag, source).unwrap();
        let mut pairs = tag.into_inner();
        pairs.next();
        parse_inline_attribute_from_final_pairs_of_tag(pairs)
    }

    #[test]
    fn padding_setting_allows_list_literal() {
        let settings = parse_inline_settings("<Group padding=[5px, 10px]/>").unwrap();

        let SettingElement::Setting(token, ValueDefinition::LiteralValue(PaxValue::Vec(values))) =
            &settings[0]
        else {
            panic!("expected padding list literal");
        };

        assert_eq!(token.token_value, "padding");
        assert_eq!(values.len(), 2);
    }

    #[test]
    #[should_panic(expected = "padding does not support tuple syntax")]
    fn padding_setting_rejects_tuple_literal() {
        parse_inline_settings("<Group padding=(5px, 10px)/>");
    }

    #[test]
    #[should_panic(expected = "padding does not support tuple syntax")]
    fn padding_setting_rejects_tuple_expression() {
        parse_inline_settings("<Group padding={(5px, 10px)}/>");
    }
}
