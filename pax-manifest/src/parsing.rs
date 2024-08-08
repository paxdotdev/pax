use crate::*;
use pax_lang::{
    get_pax_pratt_parser, parse_pax_str, Pair, Pairs, PaxParser, PrattParser, Rule, Span,
};
use pax_runtime_api::{Color, Fill, Size, Stroke};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::rc::Rc;

use pax_lang::Parser;

/// Returns (RIL output string, `symbolic id`s found during parse)
/// where a `symbolic id` may be something like `self.num_clicks` or `i`
pub fn run_pratt_parser(input_paxel: &str) -> (String, Vec<String>) {
    let pratt = get_pax_pratt_parser();
    let pairs = PaxParser::parse(Rule::expression_body, input_paxel)
        .expect(&format!("unsuccessful pratt parse {}", &input_paxel));

    let symbolic_ids = Rc::new(RefCell::new(vec![]));
    let output = recurse_pratt_parse_to_string(pairs, &pratt, Rc::clone(&symbolic_ids));
    (output, symbolic_ids.take())
}

/// Removes leading `self.` or `this.`, escapes remaining symbol to be a suitable atomic identifier
fn convert_symbolic_binding_from_paxel_to_ril(xo_symbol: Pair<Rule>) -> String {
    let mut pairs = xo_symbol.clone().into_inner();
    let maybe_this_or_self = pairs.next().unwrap().as_str();

    let self_or_this_removed = if maybe_this_or_self == "this" || maybe_this_or_self == "self" {
        let mut output = "".to_string();

        //accumulate remaining identifiers, having skipped `this` or `self` with the original `.next()`
        pairs.for_each(|pair| output += &*(".".to_owned() + pair.as_str()));

        //remove initial fencepost "."
        output.replacen(".", "", 1)
    } else {
        //remove original binding; no self or this
        xo_symbol.as_str().to_string()
    };

    escape_identifier(self_or_this_removed)
}

/// Workhorse method for compiling Expressions into Rust Intermediate Language (RIL, a string of Rust)
fn recurse_pratt_parse_to_string<'a>(
    expression: Pairs<Rule>,
    pratt_parser: &PrattParser<Rule>,
    symbolic_ids: Rc<RefCell<Vec<String>>>,
) -> String {
    pratt_parser
        .map_primary(move |primary| match primary.as_rule() {
            /* expression_grouped | xo_enum_or_function_call | xo_range     */
            Rule::expression_grouped => {
                /* expression_grouped = { "(" ~ expression_body ~ ")" ~ literal_number_unit? } */
                let mut inner = primary.into_inner();

                let exp_bod = recurse_pratt_parse_to_string(inner.next().unwrap().into_inner(), pratt_parser, Rc::clone(&symbolic_ids));
                if let Some(literal_number_unit) = inner.next() {
                    let unit = literal_number_unit.as_str().trim();

                if unit == "px" {
                    format!("Size::Pixels({}.try_coerce().unwrap()).to_pax_any()", exp_bod)
                } else if unit == "%" {
                    format!("Percent({}.try_coerce().unwrap()).to_pax_any()", exp_bod)
                } else if unit == "deg" {
                    format!("Rotation::Degrees({}.try_coerce().unwrap()).to_pax_any()", exp_bod)
                } else if unit == "rad" {
                    format!("Rotation::Radians({}.try_coerce().unwrap()).to_pax_any()", exp_bod)
                } else {
                    unreachable!("unit: {}", unit)
                }
                } else {
                    exp_bod
                }
            },
            Rule::xo_enum_or_function_call => {
                /* xo_enum_or_function_call = {identifier ~ (("::") ~ identifier)* ~ ("("~xo_enum_or_function_args_list~")")}
                   xo_enum_or_function_args_list = {expression_body ~ ("," ~ expression_body)*} */

                if !primary.as_str().contains("(") {
                    //If no args, we just want to return this xo_enum_or_function_call wrapped in
                    //a PaxAny
                    format!("({}).to_pax_any()", primary.as_str())
                } else {
                    //prepend identifiers; recurse-pratt-parse `xo_function_args`' `expression_body`s
                    let mut pairs = primary.into_inner();

                    let mut output = "".to_string();
                    let mut next_pair = pairs.next().unwrap();

                    while let Rule::identifier = next_pair.as_rule() {
                        output = output + next_pair.as_str();
                        next_pair = pairs.next().unwrap();
                        if let Rule::identifier = next_pair.as_rule() {
                            //look-ahead
                            output = output + "::";
                        }
                    };

                    let mut expression_body_pairs = next_pair.into_inner();

                    output = output + "(";
                    while let Some(next_pair) = expression_body_pairs.next() {
                        output = output + "(" + &recurse_pratt_parse_to_string(next_pair.into_inner(), pratt_parser, Rc::clone(&symbolic_ids)) + ").try_coerce().unwrap(),"
                    }
                    output = output + ")";

                    format!("({}).to_pax_any()", output)
                }


            },
            Rule::xo_range => {
                /* { op0: (xo_literal | xo_symbol) ~ op1: (xo_range_inclusive | xo_range_exclusive) ~ op2: (xo_literal | xo_symbol)} */
                let mut pairs = primary.into_inner();

                let op0 = pairs.next().unwrap();

                let op0_out = match op0.as_rule() {
                    Rule::xo_literal => {
                        //return the literal exactly as it is
                        format!("({}isize)", op0.as_str().trim())
                    },
                    Rule::xo_symbol => {
                        symbolic_ids.borrow_mut().push(op0.as_str().trim().to_string());
                        //for symbolic identifiers, remove any "this" or "self", then return string
                        format!("({}).to_pax_any().try_coerce::<isize>().unwrap()",convert_symbolic_binding_from_paxel_to_ril(op0))
                    },
                    _ => unimplemented!("")
                };

                let op1 = pairs.next().unwrap();
                let op1_out = op1.as_str().trim().to_string();

                let op2 = pairs.next().unwrap();
                let op2_out = match op2.as_rule() {
                    Rule::xo_literal => {
                        //return the literal exactly as it is
                        format!("({}isize)", op2.as_str().trim())
                    },
                    Rule::xo_symbol => {
                        symbolic_ids.borrow_mut().push(op2.as_str().trim().to_string());
                        //for symbolic identifiers, remove any "this" or "self", then return string
                        format!("({}).to_pax_any().try_coerce::<isize>().unwrap()",convert_symbolic_binding_from_paxel_to_ril(op2))
                    },
                    _ => unimplemented!("")
                };

                format!("(({}){}({})).to_pax_any()", &op0_out, &op1_out, &op2_out)
            },
            Rule::xo_literal => {
                let literal_kind = primary.into_inner().next().unwrap();

                match literal_kind.as_rule() {
                    Rule::literal_number_with_unit => {
                        let mut inner = literal_kind.into_inner();

                        let value = inner.next().unwrap().as_str().trim();
                        let unit = inner.next().unwrap().as_str().trim();

                        if unit == "px" {
                            format!("Size::Pixels({}.into()).to_pax_any()", value)
                        } else if unit == "%" {
                            format!("Percent({}.into()).to_pax_any()", value)
                        } else if unit == "deg" {
                            format!("Rotation::Degrees({}.into()).to_pax_any()", value)
                        } else if unit == "rad" {
                            format!("Rotation::Radians({}.into()).to_pax_any()", value)
                        } else {
                            unreachable!()
                        }
                    },
                    Rule::literal_number => {
                        let mut inner = literal_kind.into_inner();
                        let value = inner.next().unwrap().as_str().trim();
                        format!("({}).to_pax_any()", value)
                    },
                    Rule::string => {
                        format!("({}).to_string().to_pax_any()",literal_kind.as_str().trim().to_string())
                    },
                    Rule::literal_color => {
                        let mut inner = literal_kind.into_inner();
                        let next_pair = inner.next().unwrap();
                        if let Rule::literal_color_const = next_pair.as_rule() {
                            //Return color consts like WHITE underneath the Color enum
                            format!("Color::{}.to_pax_any()", next_pair.as_str())
                        } else {
                            //Recurse-pratt-parse the args list for color funcs, a la enums
                            let func = next_pair.as_str().split("(").next().unwrap().to_string();
                            let mut accum = "".to_string();
                            let mut inner = next_pair.into_inner();
                            while let Some(next_pair) = inner.next() {
                                // literal_color_channel = {literal_number_with_unit | literal_number_integer}
                                // while this case is trivial, recurse_pratt_parse is used here to manage literal_number_with_unit without duplicating code here
                                let literal_representation = next_pair.into_inner();
                                accum = accum + &recurse_pratt_parse_to_string(literal_representation, pratt_parser, Rc::clone(&symbolic_ids)) + ".try_coerce().unwrap(),"
                            }
                            format!("Color::{}({}).to_pax_any()", &func, &accum )
                        }

                    },
                    _ => {
                        /* {literal_enum_value | literal_tuple_access | literal_tuple | string } */
                        literal_kind.as_str().to_string()
                    }
                }
            },
            Rule::xo_color_space_func => {
                let mut inner = primary.clone().into_inner();

                //Recurse-pratt-parse the args list for color funcs, a la enums
                let func = primary.as_str().split("(").next().unwrap().to_string();
                let mut accum = "".to_string();

                while let Some(next_pair) = inner.next() {
                    // literal_color_channel = {literal_number_with_unit | literal_number_integer}
                    // while this case is trivial, recurse_pratt_parse is used here to manage literal_number_with_unit without duplicating code here
                    let literal_representation = next_pair.into_inner();
                    accum = accum + &recurse_pratt_parse_to_string(literal_representation, pratt_parser, Rc::clone(&symbolic_ids)) + ".try_coerce().unwrap(),"
                }
                format!("Color::{}({}).to_pax_any()", &func, &accum )
            },
            Rule::xo_object => {
                let mut output : String = "".to_string();

                let mut inner = primary.into_inner();
                let maybe_identifier = inner.next().unwrap();
                let rule = maybe_identifier.as_rule();

                //for parsing xo_object_settings_key_value_pair
                //iterate over key-value pairs; recurse into expressions
                fn handle_xoskvp<'a>(xoskvp: Pair<Rule>, pratt_parser: &PrattParser<Rule>, symbolic_ids: Rc<RefCell<Vec<String>>>) -> String {
                    let mut inner_kvp = xoskvp.into_inner();
                    let settings_key = inner_kvp.next().unwrap().as_str().to_string();
                    let expression_body = inner_kvp.next().unwrap().into_inner();

                    let ril = recurse_pratt_parse_to_string(expression_body, pratt_parser, Rc::clone(&symbolic_ids));
                    format!("{}: {},\n",settings_key, ril)
                }

                if let Rule::identifier = rule {
                    //explicit type declaration, like `SomeType {...}`
                    unimplemented!("Explicit struct type declarations are not yet supported.  Instead of `SomeType {{ ... }}`, try using simply `{{ ... }}`.");
                } else {
                    //no explicit type declaration, like `{...}`
                    // -- this token is the first k/v pair of object declaration; handle as such
                    let ril = handle_xoskvp(maybe_identifier, pratt_parser, Rc::clone(&symbolic_ids));
                    output += &ril;
                }

                let mut remaining_kvps = inner.into_iter();

                while let Some(xoskkvp) = remaining_kvps.next() {
                    let ril =  handle_xoskvp(xoskkvp, pratt_parser, Rc::clone(&symbolic_ids));
                    output += &ril;
                }

                output
            },
            Rule::xo_symbol => {
                symbolic_ids.borrow_mut().push(primary.as_str().to_string());
                format!("({}).to_pax_any()",convert_symbolic_binding_from_paxel_to_ril(primary))
            },
            Rule::xo_tuple => {
                let mut tuple = primary.into_inner();
                let exp0 = tuple.next().unwrap();
                let exp1 = tuple.next().unwrap();
                let exp0 = recurse_pratt_parse_to_string( exp0.into_inner(), pratt_parser, Rc::clone(&symbolic_ids));
                let exp1 = recurse_pratt_parse_to_string( exp1.into_inner(), pratt_parser, Rc::clone(&symbolic_ids));
                format!("({},{})", exp0, exp1)
            },
            Rule::xo_list => {
                let mut list = primary.into_inner();
                let mut vec = Vec::new();

                while let Some(item) = list.next() {
                    let item_str = recurse_pratt_parse_to_string(item.into_inner(), pratt_parser, Rc::clone(&symbolic_ids));
                    vec.push(item_str);
                }
                format!("vec![{}]", vec.join(","))
            },
            Rule::expression_body => {
                recurse_pratt_parse_to_string(primary.into_inner(), pratt_parser, Rc::clone(&symbolic_ids))
            },
            _ => unreachable!("{}",primary.as_str()),
        })
        .map_prefix(|op, rhs| match op.as_rule() {
            Rule::xo_neg => format!("(-{})", rhs),
            Rule::xo_bool_not => format!("(!{})", rhs),
            _ => unreachable!(),
        })
        // .map_postfix(|lhs, op| match op.as_rule() {
        //     Rule::fac => format!("({}!)", lhs),
        //     _ => unreachable!(),
        // })
        .map_infix(|lhs, op, rhs| match op.as_rule() {
            Rule::xo_add => {format!("({}+{})", lhs, rhs)},
            Rule::xo_bool_and => {format!("(({}).op_and({}))", lhs, rhs)},
            Rule::xo_bool_or => {format!("(({}).op_or({}))", lhs, rhs)},
            Rule::xo_div => {format!("({}/{})", lhs, rhs)},
            Rule::xo_exp => {format!("(({}).pow({}))", lhs, rhs)},
            Rule::xo_mod => {format!("({}%{})", lhs, rhs)},
            Rule::xo_mul => {format!("({}*{})", lhs, rhs)},
            Rule::xo_rel_eq => {format!("pax_engine::api::pax_value::PaxAny::Builtin(pax_engine::api::pax_value::PaxValue::Bool({}=={}))", lhs, rhs)},
            Rule::xo_rel_gt => {format!("pax_engine::api::pax_value::PaxAny::Builtin(pax_engine::api::pax_value::PaxValue::Bool({}>{}))", lhs, rhs)},
            Rule::xo_rel_gte => {format!("pax_engine::api::pax_value::PaxAny::Builtin(pax_engine::api::pax_value::PaxValue::Bool({}>={}))", lhs, rhs)},
            Rule::xo_rel_lt => {format!("pax_engine::api::pax_value::PaxAny::Builtin(pax_engine::api::pax_value::PaxValue::Bool({}<{}))", lhs, rhs)},
            Rule::xo_rel_lte => {format!("pax_engine::api::pax_value::PaxAny::Builtin(pax_engine::api::pax_value::PaxValue::Bool({}<={}))", lhs, rhs)},
            Rule::xo_rel_neq => {format!("pax_engine::api::pax_value::PaxAny::Builtin(pax_engine::api::pax_value::PaxValue::Bool({}!={}))", lhs, rhs)},
            Rule::xo_sub => {format!("({}-{})", lhs, rhs)},
            Rule::xo_tern_then => {format!("if {}.into() {{ {} }}", lhs, rhs)},
            Rule::xo_tern_else => {format!("{} else {{ {} }}", lhs, rhs)},
            _ => unreachable!(),
        })
        .parse(expression)
}

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
            let mut open_tag = matched_tag
                .clone()
                .into_inner()
                .next()
                .unwrap()
                .into_inner();
            let pascal_identifier = open_tag.next().unwrap().as_str();

            let template_node = TemplateNodeDefinition {
                type_id: TypeId::build_singleton(
                    &ctx.pascal_identifier_to_type_id_map
                        .get(pascal_identifier)
                        .expect(&format!("Template key not found {}", &pascal_identifier))
                        .to_string(),
                    Some(&pascal_identifier.to_string()),
                ),
                settings: parse_inline_attribute_from_final_pairs_of_tag(open_tag, pax),
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
            let mut tag_pairs = any_tag_pair.into_inner();
            let pascal_identifier = tag_pairs.next().unwrap().as_str();

            let type_id = if let Some(type_id) =
                ctx.pascal_identifier_to_type_id_map.get(pascal_identifier)
            {
                type_id.clone()
            } else {
                TypeId::build_blank_component(pascal_identifier)
            };
            let template_node = TemplateNodeDefinition {
                type_id,
                settings: parse_inline_attribute_from_final_pairs_of_tag(tag_pairs, pax),
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
                    let mut statement_if = any_tag_pair.into_inner();
                    let expression_body = statement_if.next().unwrap();
                    let expression_body_location = span_to_location(&expression_body.as_span());
                    let expression_body_token = Token::new(
                        expression_body.as_str().to_string(),
                        TokenType::IfExpression,
                        expression_body_location,
                        pax,
                    );

                    //`if` TemplateNodeDefinition
                    let template_node = TemplateNodeDefinition {
                        control_flow_settings: Some(ControlFlowSettingsDefinition {
                            condition_expression_paxel: Some(expression_body_token),
                            condition_expression_info: None, //This will be written back to this data structure later, during expression compilation
                            slot_index_expression_paxel: None,
                            slot_index_expression_info: None,
                            repeat_predicate_definition: None,
                            repeat_source_definition: None,
                        }),
                        type_id: TypeId::build_if(),
                        settings: None,
                        raw_comment_string: None,
                    };

                    let id = match location {
                        TreeLocation::Root => ctx.template.add_root_node_back(template_node),
                        TreeLocation::Parent(id) => ctx.template.add_child_back(id, template_node),
                    };

                    let prospective_inner_nodes = statement_if.next();

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
                Rule::statement_for => {
                    let mut cfavd = ControlFlowSettingsDefinition::default();
                    let mut for_statement = any_tag_pair.clone().into_inner();
                    let mut predicate_declaration = for_statement.next().unwrap().into_inner();
                    let source = for_statement.next().unwrap();

                    let prospective_inner_nodes = for_statement.next();

                    if predicate_declaration.clone().count() > 1 {
                        //tuple, like the `elem, i` in `for (elem, i) in self.some_list`
                        let elem = predicate_declaration.next().unwrap();
                        let elem_location = span_to_location(&elem.as_span());
                        let elem_token = Token::new(
                            elem.as_str().to_string(),
                            TokenType::ForPredicate,
                            elem_location,
                            pax,
                        );
                        let index = predicate_declaration.next().unwrap();
                        let index_location = span_to_location(&index.as_span());
                        let index_token = Token::new(
                            index.as_str().to_string(),
                            TokenType::ForPredicate,
                            index_location,
                            pax,
                        );
                        cfavd.repeat_predicate_definition =
                            Some(ControlFlowRepeatPredicateDefinition::ElemIdIndexId(
                                elem_token,
                                index_token,
                            ));
                    } else {
                        let elem = predicate_declaration.next().unwrap();
                        //single identifier, like the `elem` in `for elem in self.some_list`
                        let predicate_declaration_location = span_to_location(&elem.as_span());
                        let predicate_declaration_token = Token::new(
                            elem.as_str().to_string(),
                            TokenType::ForPredicate,
                            predicate_declaration_location,
                            pax,
                        );
                        cfavd.repeat_predicate_definition =
                            Some(ControlFlowRepeatPredicateDefinition::ElemId(
                                predicate_declaration_token,
                            ));
                    }

                    let inner_source = source.into_inner().next().unwrap();
                    let inner_source_location = span_to_location(&inner_source.as_span());
                    let mut inner_source_token = Token::new(
                        inner_source.as_str().to_string(),
                        TokenType::ForSource,
                        inner_source_location,
                        pax,
                    );
                    /* statement_for_source = { xo_range | xo_symbol } */
                    let repeat_source_definition = match inner_source.as_rule() {
                        Rule::xo_range => {
                            let mut inner = inner_source.into_inner();
                            let left = inner.next().unwrap();
                            //skip middle
                            inner.next();
                            let right = inner.next().unwrap();
                            let mut range_symbolic_bindings = vec![];
                            if matches!(left.as_rule(), Rule::xo_symbol) {
                                let inner_source_location = span_to_location(&left.as_span());
                                let left_range_token = Token::new(
                                    convert_symbolic_binding_from_paxel_to_ril(left),
                                    TokenType::ForSource,
                                    inner_source_location,
                                    pax,
                                );
                                range_symbolic_bindings.push(left_range_token);
                            }
                            if matches!(right.as_rule(), Rule::xo_symbol) {
                                let inner_source_location = span_to_location(&right.as_span());
                                let left_range_token = Token::new(
                                    convert_symbolic_binding_from_paxel_to_ril(right),
                                    TokenType::ForSource,
                                    inner_source_location,
                                    pax,
                                );
                                range_symbolic_bindings.push(left_range_token);
                            }
                            ControlFlowRepeatSourceDefinition {
                                range_expression_paxel: Some(inner_source_token),
                                range_symbolic_bindings,
                                expression_info: None, //This will be written back to this data structure later, during expression compilation
                                symbolic_binding: None,
                            }
                        }
                        Rule::xo_symbol => {
                            inner_source_token.token_value =
                                convert_symbolic_binding_from_paxel_to_ril(inner_source);
                            ControlFlowRepeatSourceDefinition {
                                range_expression_paxel: None,
                                range_symbolic_bindings: vec![],
                                expression_info: None,
                                symbolic_binding: Some(inner_source_token),
                            }
                        }
                        _ => {
                            unreachable!()
                        }
                    };

                    cfavd.repeat_source_definition = Some(repeat_source_definition);

                    //`for` TemplateNodeDefinition
                    let template_node = TemplateNodeDefinition {
                        type_id: TypeId::build_repeat(),
                        control_flow_settings: Some(cfavd),
                        settings: None,
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
                    let expression_body_location = span_to_location(&expression_body.as_span());
                    let expression_body_token = Token::new(
                        expression_body.as_str().to_string(),
                        TokenType::SlotExpression,
                        expression_body_location,
                        pax,
                    );
                    let prospective_inner_nodes = statement_slot.next();

                    let template_node = TemplateNodeDefinition {
                        control_flow_settings: Some(ControlFlowSettingsDefinition {
                            condition_expression_paxel: None,
                            condition_expression_info: None,
                            slot_index_expression_paxel: Some(expression_body_token),
                            slot_index_expression_info: None, //This will be written back to this data structure later, during expression compilation
                            repeat_predicate_definition: None,
                            repeat_source_definition: None,
                        }),
                        type_id: TypeId::build_slot(),
                        settings: None,
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

fn parse_literal_function(literal_function_full: Pair<Rule>, pax: &str) -> Token {
    let literal_function = literal_function_full.clone().into_inner().next().unwrap();

    let location_info = span_to_location(&literal_function.as_span());
    let literal_function_token = Token::new_with_raw_value(
        literal_function.as_str().to_string(),
        literal_function_full.as_str().to_string().replace(",", ""),
        TokenType::Handler,
        location_info,
        pax,
    );
    literal_function_token
}

fn parse_event_id(event_id_full: Pair<Rule>, pax: &str) -> Token {
    let event_id = event_id_full.clone().into_inner().next().unwrap();

    let event_id_location = span_to_location(&event_id.as_span());
    let event_id_token = Token::new_with_raw_value(
        event_id.as_str().to_string(),
        event_id_full.as_str().to_string(),
        TokenType::EventId,
        event_id_location,
        pax,
    );
    event_id_token
}

fn parse_inline_attribute_from_final_pairs_of_tag(
    final_pairs_of_tag: Pairs<Rule>,
    pax: &str,
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

                    let setting = double_binding.next().unwrap();
                    let setting_location = span_to_location(&setting.as_span());
                    let property = double_binding.next().unwrap();
                    let property_location = span_to_location(&property.as_span());

                    let setting_token = Token::new(
                        setting.as_str().to_string(),
                        TokenType::SettingKey,
                        setting_location,
                        pax,
                    );

                    let property_token = Token::new_with_raw_value(
                        clean_and_split_symbols(property.as_str())
                            .get(0)
                            .expect("No identifier found")
                            .to_string(),
                        property.as_str().to_string(),
                        TokenType::Identifier,
                        property_location,
                        pax,
                    );

                    SettingElement::Setting(
                        setting_token,
                        ValueDefinition::DoubleBinding(property_token),
                    )
                }
                Rule::attribute_event_binding => {
                    // attribute_event_binding = {event_id ~ "=" ~ literal_function}
                    let mut kv = attribute_key_value_pair.into_inner();
                    let mut attribute_event_binding = kv.next().unwrap().into_inner();

                    let event_id_token =
                        parse_event_id(attribute_event_binding.next().unwrap(), pax);

                    let literal_function_token =
                        parse_literal_function(attribute_event_binding.next().unwrap(), pax);
                    SettingElement::Setting(
                        event_id_token,
                        ValueDefinition::EventBindingTarget(literal_function_token),
                    )
                }
                Rule::id_binding => {
                    let mut kv = attribute_key_value_pair
                        .into_inner()
                        .next()
                        .unwrap()
                        .into_inner();
                    let id_binding_key = kv.next().unwrap();
                    let id_binding_key_location = span_to_location(&id_binding_key.as_span());
                    let id_binding_key_token = Token::new(
                        id_binding_key.as_str().to_string(),
                        TokenType::SettingKey,
                        id_binding_key_location,
                        pax,
                    );
                    let id_binding_value = kv.next().unwrap();
                    let id_binding_value_location = span_to_location(&id_binding_value.as_span());
                    let id_binding_value_token = Token::new(
                        id_binding_value.as_str().to_string(),
                        TokenType::LiteralValue,
                        id_binding_value_location,
                        pax,
                    );
                    SettingElement::Setting(
                        id_binding_key_token,
                        ValueDefinition::LiteralValue(id_binding_value_token),
                    )
                }
                _ => {
                    //Vanilla `key=value` setting pair

                    let mut kv = attribute_key_value_pair.into_inner();
                    let key = kv.next().unwrap();
                    let key_location = span_to_location(&key.as_span());
                    let key_token = Token::new(
                        key.as_str().to_string(),
                        TokenType::SettingKey,
                        key_location,
                        pax,
                    );
                    // exact tokens used, includes {} from expression wrapped
                    let raw_value = kv.peek().unwrap().as_str();
                    let value = kv.next().unwrap().into_inner().next().unwrap();
                    let location_info = span_to_location(&value.as_span());
                    let value_definition = match value.as_rule() {
                        Rule::literal_value => {
                            //we want to pratt-parse literals, mostly to unpack `px` and `%` (recursively)
                            let (output_string, _) =
                                crate::parsing::run_pratt_parser(value.as_str());
                            let literal_value_token = Token::new_with_raw_value(
                                output_string,
                                raw_value.to_string(),
                                TokenType::LiteralValue,
                                location_info,
                                pax,
                            );
                            ValueDefinition::LiteralValue(literal_value_token)
                        }
                        Rule::literal_object => ValueDefinition::Block(
                            derive_value_definition_from_literal_object_pair(value, pax),
                        ),
                        Rule::expression_body => {
                            let expression_token = Token::new_with_raw_value(
                                value.as_str().to_string(),
                                raw_value.to_string(),
                                TokenType::Expression,
                                location_info,
                                pax,
                            );
                            ValueDefinition::Expression(expression_token)
                        }
                        Rule::identifier => {
                            let identifier_token = Token::new_with_raw_value(
                                clean_and_split_symbols(value.as_str())
                                    .get(0)
                                    .expect("No identifier found")
                                    .to_string(),
                                value.as_str().to_string(),
                                TokenType::Identifier,
                                location_info,
                                pax,
                            );
                            ValueDefinition::Identifier(identifier_token)
                        }
                        _ => {
                            unreachable!("Parsing error 3342638857230: {:?}", value.as_rule());
                        }
                    };
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

fn derive_value_definition_from_literal_object_pair(
    literal_object: Pair<Rule>,
    pax: &str,
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
            let token = Token::new(
                raw_value.as_str().to_string(),
                TokenType::PascalIdentifier,
                raw_value_location,
                pax,
            );
            Some(token)
        }
        _ => None,
    };

    LiteralBlockDefinition {
        explicit_type_pascal_identifier,
        elements: literal_object_pairs
            .map(|settings_key_value_pair| {
                match settings_key_value_pair.as_rule() {
                    Rule::settings_key_value_pair => {
                        let mut pairs = settings_key_value_pair.into_inner();

                        let setting_key = pairs.next().unwrap().into_inner().next().unwrap();
                        let setting_key_location = span_to_location(&setting_key.as_span());
                        let setting_key_token = Token::new(
                            setting_key.as_str().to_string(),
                            TokenType::SettingKey,
                            setting_key_location,
                            pax,
                        );
                        let raw_value = pairs.peek().unwrap().as_str();
                        let value = pairs.next().unwrap().into_inner().next().unwrap();
                        let location_info = span_to_location(&value.as_span());
                        let setting_value_definition = match value.as_rule() {
                            Rule::literal_value => {
                                //we want to pratt-parse literals, mostly to unpack `px` and `%` (recursively)
                                let (output_string, _) =
                                    crate::parsing::run_pratt_parser(value.as_str());
                                let token = Token::new_with_raw_value(
                                    output_string,
                                    raw_value.to_string(),
                                    TokenType::LiteralValue,
                                    location_info,
                                    pax,
                                );
                                ValueDefinition::LiteralValue(token)
                            }
                            Rule::literal_object => {
                                ValueDefinition::Block(
                                    //Recurse
                                    derive_value_definition_from_literal_object_pair(value, pax),
                                )
                            }
                            // Rule::literal_enum_value => {ValueDefinition::Enum(raw_value.as_str().to_string())},
                            Rule::expression_body => {
                                let token = Token::new_with_raw_value(
                                    value.as_str().to_string(),
                                    raw_value.to_string(),
                                    TokenType::Expression,
                                    location_info,
                                    pax,
                                );
                                ValueDefinition::Expression(token)
                            }
                            _ => {
                                unreachable!("Parsing error 231453468: {:?}", value.as_rule());
                            }
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
                }
            })
            .collect(),
    }
}

pub fn parse_settings_from_component_definition_string(
    pax: &str,
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
                                        pax,
                                    );
                                    let literal_function_token = parse_literal_function(
                                        settings_event_binding_pairs.next().unwrap(),
                                        pax,
                                    );
                                    let handler_element: SettingsBlockElement =
                                        SettingsBlockElement::Handler(
                                            event_id_token,
                                            vec![literal_function_token],
                                        );
                                    settings.push(handler_element);
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
                                    let token = Token::new(
                                        selector,
                                        TokenType::Selector,
                                        raw_value_location,
                                        pax,
                                    );
                                    let literal_object = selector_block_pairs.next().unwrap();

                                    settings.push(SettingsBlockElement::SelectorBlock(
                                        token,
                                        derive_value_definition_from_literal_object_pair(
                                            literal_object,
                                            pax,
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

pub struct ParsingContext {
    /// Used to track which files/sources have been visited during parsing,
    /// to prevent duplicate parsing
    pub visited_type_ids: HashSet<TypeId>,

    pub main_component_type_id: TypeId,

    pub component_definitions: BTreeMap<TypeId, ComponentDefinition>,

    pub template_map: HashMap<String, TypeId>,

    pub template_node_definitions: ComponentTemplate,

    pub type_table: TypeTable,
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
        }
    }
}

#[derive(Debug)]
pub struct ParsingError {
    pub error_name: String,
    pub error_message: String,
    pub matched_string: String,
    pub start: (usize, usize),
    pub end: (usize, usize),
}

/// From a raw string of Pax representing a single component, parse a complete ComponentDefinition
pub fn assemble_component_definition(
    mut ctx: ParsingContext,
    pax: &str,
    is_main_component: bool,
    template_map: HashMap<String, TypeId>,
    module_path: &str,
    self_type_id: TypeId,
    component_source_file_path: &str,
) -> (ParsingContext, ComponentDefinition) {
    let mut tpc = TemplateNodeParseContext {
        pascal_identifier_to_type_id_map: template_map,
        template: ComponentTemplate::new(
            self_type_id.clone(),
            Some(component_source_file_path.to_owned()),
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

    let settings = parse_settings_from_component_definition_string(pax, ast);

    let new_def = ComponentDefinition {
        is_primitive: false,
        is_struct_only_component: false,
        is_main_component,
        primitive_instance_import_path: None,
        type_id: self_type_id,
        template: Some(tpc.template),
        settings: Some(settings),
        module_path: modified_module_path,
    };

    (ctx, new_def)
}

pub fn clean_module_path(module_path: &str) -> String {
    if module_path.starts_with("parser") {
        module_path.replacen("parser", "crate", 1)
    } else {
        module_path.to_string()
    }
}

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
    };
    (ctx, new_def)
}

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
        module_path: modified_module_path,
    }
}

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

/// This trait is used only to extend primitives like u64
/// with the parser-time method `parse_to_manifest`.  This
/// allows the parser binary to codegen calls to `::parse_to_manifest()` even
/// on primitive types
pub trait Reflectable {
    fn parse_to_manifest(mut ctx: ParsingContext) -> (ParsingContext, Vec<PropertyDefinition>) {
        //Default impl for primitives and pax_runtime_api
        let type_id = Self::get_type_id();
        let td = TypeDefinition {
            type_id: type_id.clone(),
            inner_iterable_type_id: None,
            property_definitions: vec![],
        };

        if !ctx.type_table.contains_key(&type_id) {
            ctx.type_table.insert(type_id, td);
        }

        (ctx, vec![])
    }

    ///The import path is the fully namespace-qualified path for a type, like `std::vec::Vec`
    ///This is distinct from type_id ONLY when the type has generics, like Vec, where
    ///the type_id is distinct across a Vec<Foo> and a Vec<Bar>.  In both cases of Vec,
    ///the import_path will remain the same.
    fn get_import_path() -> String {
        //This default is used by primitives but expected to
        //be overridden by userland Pax components / primitives
        Self::get_self_pascal_identifier()
    }

    fn get_self_pascal_identifier() -> String;

    fn get_type_id() -> TypeId;

    fn get_iterable_type_id() -> Option<TypeId> {
        //Most types do not have an iterable type (e.g. the T in Vec<T>) —
        //it is the responsibility of iterable types to override this fn
        None
    }
}

impl Reflectable for () {
    fn get_self_pascal_identifier() -> String {
        "()".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}

impl Reflectable for usize {
    fn get_self_pascal_identifier() -> String {
        "usize".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for isize {
    fn get_self_pascal_identifier() -> String {
        "isize".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for i128 {
    fn get_self_pascal_identifier() -> String {
        "i128".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for u128 {
    fn get_self_pascal_identifier() -> String {
        "u128".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for i64 {
    fn get_self_pascal_identifier() -> String {
        "i64".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for u64 {
    fn get_self_pascal_identifier() -> String {
        "u64".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for i32 {
    fn get_self_pascal_identifier() -> String {
        "i32".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for u32 {
    fn get_self_pascal_identifier() -> String {
        "u32".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for i8 {
    fn get_self_pascal_identifier() -> String {
        "i8".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for u8 {
    fn get_self_pascal_identifier() -> String {
        "u8".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for f64 {
    fn get_self_pascal_identifier() -> String {
        "f64".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for f32 {
    fn get_self_pascal_identifier() -> String {
        "f32".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}
impl Reflectable for bool {
    fn get_self_pascal_identifier() -> String {
        "bool".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}

impl Reflectable for char {
    fn get_self_pascal_identifier() -> String {
        "char".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_primitive(&Self::get_self_pascal_identifier())
    }
}

impl Reflectable for std::string::String {
    fn get_import_path() -> String {
        "std::string::String".to_string()
    }
    fn get_self_pascal_identifier() -> String {
        "String".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl<T> Reflectable for std::rc::Rc<T> {
    fn get_import_path() -> String {
        "std::rc::Rc".to_string()
    }
    fn get_self_pascal_identifier() -> String {
        "Rc".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl<T: Reflectable> Reflectable for std::option::Option<T> {
    fn parse_to_manifest(mut ctx: ParsingContext) -> (ParsingContext, Vec<PropertyDefinition>) {
        let type_id = Self::get_type_id();
        let td = TypeDefinition {
            type_id: type_id.clone(),
            inner_iterable_type_id: None,
            property_definitions: vec![],
        };

        if !ctx.type_table.contains_key(&type_id) {
            ctx.type_table.insert(type_id, td);
        }

        let (ctx, _) = T::parse_to_manifest(ctx);
        (ctx, vec![]) //Option itself has no PAXEL-addressable properties
    }
    fn get_import_path() -> String {
        "std::option::Option".to_string()
    }
    fn get_self_pascal_identifier() -> String {
        "Option".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_option(&format!("{}", &T::get_type_id()))
    }
}

impl Reflectable for TypeId {
    fn get_import_path() -> String {
        "pax_manifest::TypeId".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "TypeId".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for TemplateNodeId {
    fn get_import_path() -> String {
        "pax_manifest::TemplateNodeId".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "TemplateNodeId".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for Fill {
    fn get_import_path() -> String {
        "pax_engine::api::Fill".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Fill".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for Stroke {
    fn parse_to_manifest(mut ctx: ParsingContext) -> (ParsingContext, Vec<PropertyDefinition>) {
        let type_id = Self::get_type_id();
        let mut flags = PropertyDefinitionFlags::default();
        flags.is_property_wrapped = true;
        let td = TypeDefinition {
            type_id: type_id.clone(),
            inner_iterable_type_id: None,
            property_definitions: vec![
                PropertyDefinition {
                    name: "color".to_string(),
                    flags: flags.clone(),
                    type_id: Color::get_type_id(),
                },
                PropertyDefinition {
                    name: "width".to_string(),
                    flags: flags,
                    type_id: Size::get_type_id(),
                },
            ],
        };

        if !ctx.type_table.contains_key(&type_id) {
            ctx.type_table.insert(type_id, td);
        }
        let color_type_id = Color::get_type_id();
        if !ctx.type_table.contains_key(&color_type_id) {
            ctx.type_table.insert(
                color_type_id.clone(),
                TypeDefinition {
                    type_id: color_type_id,
                    inner_iterable_type_id: None,
                    property_definitions: vec![],
                },
            );
        }
        let size_type_id = Size::get_type_id();
        if !ctx.type_table.contains_key(&size_type_id) {
            ctx.type_table.insert(
                size_type_id.clone(),
                TypeDefinition {
                    type_id: size_type_id,
                    inner_iterable_type_id: None,
                    property_definitions: vec![],
                },
            );
        }

        (ctx, vec![])
    }

    fn get_import_path() -> String {
        "pax_engine::api::Stroke".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Stroke".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for pax_runtime_api::Size {
    fn get_import_path() -> String {
        "pax_engine::api::Size".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Size".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for pax_runtime_api::Color {
    fn get_import_path() -> String {
        "pax_engine::api::Color".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Color".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for pax_runtime_api::ColorChannel {
    fn get_import_path() -> String {
        "pax_engine::api::ColorChannel".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "ColorChannel".to_string()
    }

    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for pax_runtime_api::Rotation {
    fn get_import_path() -> String {
        "pax_engine::api::Rotation".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Rotation".to_string()
    }
    fn get_type_id() -> TypeId {
        let type_id = TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        );

        type_id
    }
}

impl Reflectable for pax_runtime_api::Numeric {
    fn get_import_path() -> String {
        "pax_engine::api::Numeric".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Numeric".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for kurbo::Point {
    fn get_import_path() -> String {
        "kurbo::Point".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Point".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl Reflectable for pax_runtime_api::Transform2D {
    fn get_import_path() -> String {
        "pax_engine::api::Transform2D".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Transform2D".to_string()
    }
    fn get_type_id() -> TypeId {
        TypeId::build_singleton(
            &Self::get_import_path(),
            Some(&Self::get_self_pascal_identifier()),
        )
    }
}

impl<T: Reflectable> Reflectable for std::vec::Vec<T> {
    fn parse_to_manifest(mut ctx: ParsingContext) -> (ParsingContext, Vec<PropertyDefinition>) {
        let type_id = Self::get_type_id();
        let td = TypeDefinition {
            type_id: type_id.clone(),
            inner_iterable_type_id: Self::get_iterable_type_id(),
            property_definitions: vec![],
        };

        if !ctx.type_table.contains_key(&type_id) {
            ctx.type_table.insert(type_id, td);
        }

        // Also parse iterable type
        T::parse_to_manifest(ctx)
    }
    fn get_import_path() -> String {
        "std::vec::Vec".to_string()
    }
    fn get_self_pascal_identifier() -> String {
        "Vec".to_string()
    }
    fn get_type_id() -> TypeId {
        //Need to encode generics contents as part of unique id for iterables
        TypeId::build_vector(&format!("{}", &Self::get_iterable_type_id().unwrap()))
    }
    fn get_iterable_type_id() -> Option<TypeId> {
        Some(T::get_type_id())
    }
}

impl<T: Reflectable> Reflectable for VecDeque<T> {
    fn parse_to_manifest(mut ctx: ParsingContext) -> (ParsingContext, Vec<PropertyDefinition>) {
        let type_id = Self::get_type_id();
        let td = TypeDefinition {
            type_id: type_id.clone(),
            inner_iterable_type_id: Self::get_iterable_type_id(),
            property_definitions: vec![],
        };

        if !ctx.type_table.contains_key(&type_id) {
            ctx.type_table.insert(type_id, td);
        }

        // Also parse iterable type
        T::parse_to_manifest(ctx)
    }
    fn get_import_path() -> String {
        "std::collections::VecDeque".to_string()
    }
    fn get_self_pascal_identifier() -> String {
        "VecDeque".to_string()
    }
    fn get_type_id() -> TypeId {
        //Need to encode generics contents as part of unique id for iterables
        TypeId::build_vector(&format!("{}", &Self::get_iterable_type_id().unwrap()))
    }
    fn get_iterable_type_id() -> Option<TypeId> {
        Some(T::get_type_id())
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
