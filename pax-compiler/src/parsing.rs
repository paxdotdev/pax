use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use itertools::{Itertools, MultiPeek};
use pax_manifest::constants::{TYPE_ID_COMMENT, TYPE_ID_IF, TYPE_ID_REPEAT, TYPE_ID_SLOT};
use pax_manifest::escape_identifier;
use std::ops::RangeFrom;

use pax_manifest::{
    get_primitive_type_table, ComponentDefinition, ControlFlowRepeatPredicateDefinition,
    ControlFlowRepeatSourceDefinition, ControlFlowSettingsDefinition, HandlersBlockElement,
    LiteralBlockDefinition, LocationInfo, PropertyDefinition, SettingElement, SettingsBlockElement,
    TemplateNodeDefinition, Token, TokenType, TypeDefinition, TypeTable, ValueDefinition,
};

extern crate pest;
use pest::iterators::{Pair, Pairs};
use pest::{Parser, Span};
use pest_derive::Parser;

use pest::pratt_parser::{Assoc, Op, PrattParser};

#[derive(Parser)]
#[grammar = "pax.pest"]
pub struct PaxParser;

/// Returns (RIL output string, `symbolic id`s found during parse)
/// where a `symbolic id` may be something like `self.num_clicks` or `i`
pub fn run_pratt_parser(input_paxel: &str) -> (String, Vec<String>) {
    // Operator precedence is declared via the ordering here:
    let pratt = PrattParser::new()
        .op(Op::infix(Rule::xo_tern_then, Assoc::Left)
            | Op::infix(Rule::xo_tern_else, Assoc::Right))
        .op(Op::infix(Rule::xo_bool_and, Assoc::Left) | Op::infix(Rule::xo_bool_or, Assoc::Left))
        .op(Op::infix(Rule::xo_add, Assoc::Left) | Op::infix(Rule::xo_sub, Assoc::Left))
        .op(Op::infix(Rule::xo_mul, Assoc::Left) | Op::infix(Rule::xo_div, Assoc::Left))
        .op(Op::infix(Rule::xo_mod, Assoc::Left))
        .op(Op::infix(Rule::xo_exp, Assoc::Right))
        .op(Op::prefix(Rule::xo_neg))
        .op(Op::infix(Rule::xo_rel_eq, Assoc::Left)
            | Op::infix(Rule::xo_rel_neq, Assoc::Left)
            | Op::infix(Rule::xo_rel_lt, Assoc::Left)
            | Op::infix(Rule::xo_rel_lte, Assoc::Left)
            | Op::infix(Rule::xo_rel_gt, Assoc::Left)
            | Op::infix(Rule::xo_rel_gte, Assoc::Left))
        .op(Op::prefix(Rule::xo_bool_not));

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
                    let unit = literal_number_unit.as_str();

                    if unit == "px" {
                        format!("Size::Pixels({}.into())", exp_bod)
                    } else if unit == "%" {
                        format!("Size::Percent({}.into())", exp_bod)
                    } else if unit == "deg" {
                        format!("Rotation::Degrees({}.into())", exp_bod)
                    } else if unit == "rad" {
                        format!("Rotation::Radians({}.into())", exp_bod)
                    } else {
                        unreachable!()
                    }
                } else {
                    exp_bod
                }
            },
            Rule::xo_enum_or_function_call => {
                /* xo_enum_or_function_call = {identifier ~ (("::") ~ identifier)* ~ ("("~xo_enum_or_function_args_list~")")}
                   xo_enum_or_function_args_list = {expression_body ~ ("," ~ expression_body)*} */

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
                    output = output + "(" + &recurse_pratt_parse_to_string(next_pair.into_inner(), pratt_parser, Rc::clone(&symbolic_ids)) + "),"
                }
                output = output + ")";

                output
            },
            Rule::xo_range => {
                /* { op0: (xo_literal | xo_symbol) ~ op1: (xo_range_inclusive | xo_range_exclusive) ~ op2: (xo_literal | xo_symbol)} */
                let mut pairs = primary.into_inner();

                let op0 = pairs.next().unwrap();

                let op0_out = match op0.as_rule() {
                    Rule::xo_literal => {
                        //return the literal exactly as it is
                        op0.as_str().to_string()
                    },
                    Rule::xo_symbol => {
                        symbolic_ids.borrow_mut().push(op0.as_str().to_string());
                        //for symbolic identifiers, remove any "this" or "self", then return string
                        format!("{}.get_as_int()",convert_symbolic_binding_from_paxel_to_ril(op0))
                    },
                    _ => unimplemented!("")
                };

                let op1 = pairs.next().unwrap();
                let op1_out = op1.as_str().to_string();

                let op2 = pairs.next().unwrap();
                let op2_out = match op2.as_rule() {
                    Rule::xo_literal => {
                        //return the literal exactly as it is
                        op2.as_str().to_string()
                    },
                    Rule::xo_symbol => {
                        symbolic_ids.borrow_mut().push(op2.as_str().to_string());
                        //for symbolic identifiers, remove any "this" or "self", then return string
                        format!("{}.get_as_int()",convert_symbolic_binding_from_paxel_to_ril(op2))
                    },
                    _ => unimplemented!("")
                };

                format!("({} as isize){}({} as isize)", &op0_out, &op1_out, &op2_out)
            },
            Rule::xo_literal => {
                let literal_kind = primary.into_inner().next().unwrap();

                match literal_kind.as_rule() {
                    Rule::literal_number_with_unit => {
                        let mut inner = literal_kind.into_inner();

                        let value = inner.next().unwrap().as_str();
                        let unit = inner.next().unwrap().as_str();

                        if unit == "px" {
                            format!("Size::Pixels({}.into())", value)
                        } else if unit == "%" {
                            format!("Size::Percent({}.into())", value)
                        } else if unit == "deg" {
                            format!("Rotation::Degrees({}.into())", value)
                        } else if unit == "rad" {
                            format!("Rotation::Radians({}.into())", value)
                        } else {
                            unreachable!()
                        }
                    },
                    Rule::literal_number => {
                        let mut inner = literal_kind.into_inner();
                        let value = inner.next().unwrap().as_str();
                        format!("Numeric::from({})", value)
                    },
                    Rule::string => {
                        format!("StringBox::from({}).into()",literal_kind.as_str().to_string())
                    },
                    _ => {
                        /* {literal_enum_value | literal_tuple_access | literal_tuple | string } */
                        literal_kind.as_str().to_string()
                    }
                }
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
                format!("{}",convert_symbolic_binding_from_paxel_to_ril(primary))
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
            Rule::xo_bool_and => {format!("({}&&{})", lhs, rhs)},
            Rule::xo_bool_or => {format!("({}||{})", lhs, rhs)},
            Rule::xo_div => {format!("({}/{})", lhs, rhs)},
            Rule::xo_exp => {format!("(({}).pow({}))", lhs, rhs)},
            Rule::xo_mod => {format!("({}%{})", lhs, rhs)},
            Rule::xo_mul => {format!("({}*({}).into())", lhs, rhs)},
            Rule::xo_rel_eq => {format!("({}=={})", lhs, rhs)},
            Rule::xo_rel_gt => {format!("({}>{})", lhs, rhs)},
            Rule::xo_rel_gte => {format!("({}>={})", lhs, rhs)},
            Rule::xo_rel_lt => {format!("({}<{})", lhs, rhs)},
            Rule::xo_rel_lte => {format!("({}<={})", lhs, rhs)},
            Rule::xo_rel_neq => {format!("({}!={})", lhs, rhs)},
            Rule::xo_sub => {format!("({}-{})", lhs, rhs)},
            Rule::xo_tern_then => {format!("if {} {{ {} }}", lhs, rhs)},
            Rule::xo_tern_else => {format!("{} else {{ {} }}", lhs, rhs)},
            _ => unreachable!(),
        })
        .parse(expression)
}

fn parse_template_from_component_definition_string(ctx: &mut TemplateNodeParseContext, pax: &str) {
    let pax_component_definition = PaxParser::parse(Rule::pax_component_definition, pax)
        .expect(&format!("unsuccessful parse from {}", &pax)) // unwrap the parse result
        .next()
        .unwrap(); // get and unwrap the `pax_component_definition` rule

    let mut roots_ids = vec![];

    //insert placeholder for IMPLICIT_ROOT.  We will fill this back in on the post-order.
    ctx.template_node_definitions
        .insert(0, TemplateNodeDefinition::default());

    pax_component_definition
        .into_inner()
        .for_each(|pair| match pair.as_rule() {
            Rule::root_tag_pair => {
                ctx.child_id_tracking_stack.push(vec![]);
                let next_id = ctx.uid_gen.peek().unwrap();
                roots_ids.push(*next_id);
                recurse_visit_tag_pairs_for_template(ctx, pair.into_inner().next().unwrap(), pax);
            }
            _ => {}
        });

    // This IMPLICIT_ROOT placeholder node, at index 0 of the TND vec,
    // is a container for the child_ids that act as "multi-roots," which enables
    // templates to be authored without requiring a single top-level container
    ctx.template_node_definitions.remove(&0);
    ctx.template_node_definitions.insert(
        0,
        TemplateNodeDefinition {
            id: 0,
            child_ids: roots_ids,
            type_id: "IMPLICIT_ROOT".to_string(),
            control_flow_settings: None,
            settings: None,
            pascal_identifier: "<UNREACHABLE>".to_string(),
            raw_comment_string: None,
        },
    );
}

struct TemplateNodeParseContext {
    pub template_node_definitions: HashMap<usize, TemplateNodeDefinition>,
    pub pascal_identifier_to_type_id_map: HashMap<String, String>,
    //each frame of the outer vec represents a list of
    //children for a given node;
    //a new frame is added when descending the tree
    //but not when iterating over siblings
    pub child_id_tracking_stack: Vec<Vec<usize>>,
    pub uid_gen: MultiPeek<RangeFrom<usize>>,
}

fn recurse_visit_tag_pairs_for_template(
    ctx: &mut TemplateNodeParseContext,
    any_tag_pair: Pair<Rule>,
    pax: &str,
) {
    let new_id = ctx.uid_gen.next().unwrap();
    //insert blank placeholder
    ctx.template_node_definitions
        .insert(new_id, TemplateNodeDefinition::default());

    //add self to parent's children_id_list
    let mut parents_children_id_list = ctx.child_id_tracking_stack.pop().unwrap();
    parents_children_id_list.push(new_id);
    ctx.child_id_tracking_stack.push(parents_children_id_list);

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

            //push the empty frame for this node's children
            ctx.child_id_tracking_stack.push(vec![]);

            //recurse into inner_nodes
            let prospective_inner_nodes = matched_tag.into_inner().nth(1).unwrap();
            match prospective_inner_nodes.as_rule() {
                Rule::inner_nodes => {
                    let inner_nodes = prospective_inner_nodes;
                    inner_nodes.into_inner().for_each(|sub_tag_pair| {
                        recurse_visit_tag_pairs_for_template(ctx, sub_tag_pair, pax);
                    })
                }
                _ => {
                    panic!("wrong prospective inner nodes (or nth)")
                }
            }

            let mut template_node = TemplateNodeDefinition {
                id: new_id,
                control_flow_settings: None,
                type_id: ctx
                    .pascal_identifier_to_type_id_map
                    .get(pascal_identifier)
                    .expect(&format!("Template key not found {}", &pascal_identifier))
                    .to_string(),
                settings: parse_inline_attribute_from_final_pairs_of_tag(open_tag, pax),
                child_ids: ctx.child_id_tracking_stack.pop().unwrap(),
                pascal_identifier: pascal_identifier.to_string(),
                raw_comment_string: None,
            };
            std::mem::swap(
                ctx.template_node_definitions.get_mut(&new_id).unwrap(),
                &mut template_node,
            );
        }
        Rule::self_closing_tag => {
            let mut tag_pairs = any_tag_pair.into_inner();
            let pascal_identifier = tag_pairs.next().unwrap().as_str();

            let mut template_node = TemplateNodeDefinition {
                id: new_id,
                control_flow_settings: None,
                type_id: ctx
                    .pascal_identifier_to_type_id_map
                    .get(pascal_identifier)
                    .expect(&format!("Template key not found {}", &pascal_identifier))
                    .to_string(),
                settings: parse_inline_attribute_from_final_pairs_of_tag(tag_pairs, pax),
                child_ids: vec![],
                pascal_identifier: pascal_identifier.to_string(),
                raw_comment_string: None,
            };
            std::mem::swap(
                ctx.template_node_definitions.get_mut(&new_id).unwrap(),
                &mut template_node,
            );
        }
        Rule::statement_control_flow => {
            /* statement_control_flow = {(statement_if | statement_for | statement_slot)} */

            //push the empty frame for this node's children
            ctx.child_id_tracking_stack.push(vec![]);

            let any_tag_pair = any_tag_pair.into_inner().next().unwrap();
            let mut template_node_definition = match any_tag_pair.as_rule() {
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
                    let prospective_inner_nodes = statement_if.next();

                    if let Some(inner_nodes) = prospective_inner_nodes {
                        inner_nodes.into_inner().for_each(|sub_tag_pair| {
                            recurse_visit_tag_pairs_for_template(ctx, sub_tag_pair, pax);
                        })
                    }

                    //`if` TemplateNodeDefinition
                    TemplateNodeDefinition {
                        id: new_id,
                        control_flow_settings: Some(ControlFlowSettingsDefinition {
                            condition_expression_paxel: Some(expression_body_token),
                            condition_expression_vtable_id: None, //This will be written back to this data structure later, during expression compilation
                            slot_index_expression_paxel: None,
                            slot_index_expression_vtable_id: None,
                            repeat_predicate_definition: None,
                            repeat_source_definition: None,
                        }),
                        type_id: TYPE_ID_IF.to_string(),
                        settings: None,
                        child_ids: ctx.child_id_tracking_stack.pop().unwrap(),
                        pascal_identifier: "Conditional".to_string(),
                        raw_comment_string: None,
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
                            ControlFlowRepeatSourceDefinition {
                                range_expression_paxel: Some(inner_source_token),
                                vtable_id: None, //This will be written back to this data structure later, during expression compilation
                                symbolic_binding: None,
                            }
                        }
                        Rule::xo_symbol => {
                            inner_source_token.token_value =
                                convert_symbolic_binding_from_paxel_to_ril(inner_source);
                            ControlFlowRepeatSourceDefinition {
                                range_expression_paxel: None,
                                vtable_id: None,
                                symbolic_binding: Some(inner_source_token),
                            }
                        }
                        _ => {
                            unreachable!()
                        }
                    };

                    cfavd.repeat_source_definition = Some(repeat_source_definition);

                    if let Some(inner_nodes) = prospective_inner_nodes {
                        inner_nodes.into_inner().for_each(|sub_tag_pair| {
                            recurse_visit_tag_pairs_for_template(ctx, sub_tag_pair, pax);
                        })
                    }

                    //`for` TemplateNodeDefinition
                    TemplateNodeDefinition {
                        id: new_id,
                        type_id: TYPE_ID_REPEAT.to_string(),
                        control_flow_settings: Some(cfavd),
                        settings: None,
                        child_ids: ctx.child_id_tracking_stack.pop().unwrap(),
                        pascal_identifier: "Repeat".to_string(),
                        raw_comment_string: None,
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

                    if let Some(inner_nodes) = prospective_inner_nodes {
                        inner_nodes.into_inner().for_each(|sub_tag_pair| {
                            recurse_visit_tag_pairs_for_template(ctx, sub_tag_pair, pax);
                        })
                    }

                    TemplateNodeDefinition {
                        id: *&new_id,
                        control_flow_settings: Some(ControlFlowSettingsDefinition {
                            condition_expression_paxel: None,
                            condition_expression_vtable_id: None,
                            slot_index_expression_paxel: Some(expression_body_token),
                            slot_index_expression_vtable_id: None, //This will be written back to this data structure later, during expression compilation
                            repeat_predicate_definition: None,
                            repeat_source_definition: None,
                        }),
                        type_id: TYPE_ID_SLOT.to_string(),
                        settings: None,
                        child_ids: ctx.child_id_tracking_stack.pop().unwrap(),
                        pascal_identifier: "Slot".to_string(),
                        raw_comment_string: None,
                    }
                }
                _ => {
                    unreachable!("Parsing error: {:?}", any_tag_pair.as_rule());
                }
            };

            std::mem::swap(
                ctx.template_node_definitions.get_mut(&new_id).unwrap(),
                &mut template_node_definition,
            );
        }
        Rule::comment => {
            let mut template_node = TemplateNodeDefinition {
                id: *&new_id,
                control_flow_settings: None,
                type_id: TYPE_ID_COMMENT.to_string(),
                settings: None,
                child_ids: vec![],
                pascal_identifier: "Comment".to_string(),
                raw_comment_string: Some(any_tag_pair.as_str().to_string()),
            };
            std::mem::swap(
                ctx.template_node_definitions.get_mut(&new_id).unwrap(),
                &mut template_node,
            );
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
                Rule::attribute_event_binding => {
                    // attribute_event_binding = {attribute_event_id ~ "=" ~ xo_symbol}
                    let mut kv = attribute_key_value_pair.into_inner();
                    let mut attribute_event_binding = kv.next().unwrap().into_inner();
                    let event_id_full = attribute_event_binding.next().unwrap();
                    let event_id = event_id_full.clone().into_inner().last().unwrap();

                    let event_id_location = span_to_location(&event_id.as_span());
                    let event_id_token = Token::new_with_raw_value(
                        event_id.as_str().to_string(),
                        event_id_full.as_str().to_string(),
                        TokenType::EventId,
                        event_id_location,
                        pax,
                    );

                    let literal_function_full = attribute_event_binding.next().unwrap();

                    let literal_function =
                        literal_function_full.clone().into_inner().next().unwrap();

                    let location_info = span_to_location(&literal_function.as_span());
                    let literal_function_token = Token::new_with_raw_value(
                        literal_function.as_str().to_string(),
                        literal_function_full.as_str().to_string(),
                        TokenType::Handler,
                        location_info,
                        pax,
                    );
                    SettingElement::Setting(
                        event_id_token,
                        ValueDefinition::EventBindingTarget(literal_function_token),
                    )
                }
                Rule::id_binding => {
                    let mut kv = attribute_key_value_pair.into_inner().next().unwrap().into_inner();
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
                },
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
                            ValueDefinition::Expression(expression_token, None)
                        }
                        Rule::identifier => {
                            let identifier_token = Token::new(
                                value.as_str().to_string(),
                                TokenType::Identifier,
                                location_info,
                                pax,
                            );
                            ValueDefinition::Identifier(identifier_token, None)
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
                                ValueDefinition::Expression(token, None)
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

fn parse_settings_from_component_definition_string(pax: &str) -> Option<Vec<SettingsBlockElement>> {
    let pax_component_definition = PaxParser::parse(Rule::pax_component_definition, pax)
        .expect(&format!("unsuccessful parse from {}", &pax)) // unwrap the parse result
        .next()
        .unwrap(); // get and unwrap the `pax_component_definition` rule

    let mut ret: Vec<SettingsBlockElement> = vec![];

    pax_component_definition
        .into_inner()
        .for_each(|top_level_pair| {
            match top_level_pair.as_rule() {
                Rule::settings_block_declaration => {
                    let selector_block_definitions: Vec<SettingsBlockElement> = top_level_pair
                        .into_inner()
                        .map(|selector_block| {
                            match selector_block.as_rule() {
                                Rule::selector_block => {
                                    //selector_block => settings_key_value_pair where v is a ValueDefinition
                                    let mut selector_block_pairs = selector_block.into_inner();
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

                                    SettingsBlockElement::SelectorBlock(
                                        token,
                                        derive_value_definition_from_literal_object_pair(
                                            literal_object,
                                            pax,
                                        ),
                                    )
                                }
                                Rule::comment => {
                                    let comment = selector_block.as_str().to_string();
                                    SettingsBlockElement::Comment(comment)
                                }
                                _ => {
                                    unreachable!("Parsing error: {:?}", selector_block.as_rule());
                                }
                            }
                        })
                        .collect();

                    ret.extend(selector_block_definitions);
                }
                _ => {}
            }
        });
    Some(ret)
}

fn parse_events_from_component_definition_string(pax: &str) -> Option<Vec<HandlersBlockElement>> {
    let pax_component_definition = PaxParser::parse(Rule::pax_component_definition, pax)
        .expect(&format!("unsuccessful parse from {}", &pax)) // unwrap the parse result
        .next()
        .unwrap(); // get and unwrap the `pax_component_definition` rule

    let mut ret: Vec<HandlersBlockElement> = vec![];

    pax_component_definition
        .into_inner()
        .for_each(|top_level_pair| match top_level_pair.as_rule() {
            Rule::handlers_block_declaration => {
                let event_definitions: Vec<HandlersBlockElement> = top_level_pair
                    .into_inner()
                    .map(
                        |handlers_key_value_pair| match handlers_key_value_pair.as_rule() {
                            Rule::handlers_key_value_pair => {
                                let mut pairs = handlers_key_value_pair.into_inner();
                                let key = pairs.next().unwrap().into_inner().next().unwrap();
                                let key_value = key.as_str().to_string();
                                let key_location = span_to_location(&key.as_span());
                                let raw_values = pairs.next().unwrap().into_inner().next().unwrap();
                                let value = match raw_values.as_rule() {
                                    Rule::literal_function => {
                                        let raw_value =
                                            raw_values.as_str().replace(",", "").trim().to_string();
                                        let value = raw_values.into_inner().next().unwrap();
                                        let raw_value_location = span_to_location(&value.as_span());
                                        let token = Token::new_with_raw_value(
                                            value.as_str().to_string(),
                                            raw_value,
                                            TokenType::Handler,
                                            raw_value_location,
                                            pax,
                                        );
                                        vec![token]
                                    }
                                    Rule::function_list => raw_values
                                        .into_inner()
                                        .map(|literal_function| {
                                            let raw_value_full = literal_function
                                                .as_str()
                                                .replace(",", "")
                                                .trim()
                                                .to_string();
                                            let value: Pair<'_, Rule> =
                                                literal_function.into_inner().next().unwrap();
                                            let raw_value_location =
                                                span_to_location(&value.as_span());
                                            let token = Token::new_with_raw_value(
                                                value.as_str().to_string(),
                                                raw_value_full,
                                                TokenType::Handler,
                                                raw_value_location,
                                                pax,
                                            );
                                            token
                                        })
                                        .collect(),
                                    _ => {
                                        unreachable!("Parsing error: {:?}", raw_values.as_rule());
                                    }
                                };
                                HandlersBlockElement::Handler(
                                    Token::new(key_value, TokenType::EventId, key_location, pax),
                                    value,
                                )
                            }
                            Rule::comment => {
                                let comment = handlers_key_value_pair.as_str().to_string();
                                HandlersBlockElement::Comment(comment)
                            }
                            _ => {
                                unreachable!(
                                    "Parsing error: {:?}",
                                    handlers_key_value_pair.as_rule()
                                );
                            }
                        },
                    )
                    .collect();

                ret.extend(event_definitions);
            }
            _ => {}
        });
    Some(ret)
}

pub struct ParsingContext {
    /// Used to track which files/sources have been visited during parsing,
    /// to prevent duplicate parsing
    pub visited_type_ids: HashSet<String>,

    pub main_component_type_id: String,

    pub component_definitions: HashMap<String, ComponentDefinition>,

    pub template_map: HashMap<String, String>,

    pub template_node_definitions: HashMap<usize, TemplateNodeDefinition>,

    pub type_table: TypeTable,

    pub import_paths: HashSet<String>,
}

impl Default for ParsingContext {
    fn default() -> Self {
        Self {
            main_component_type_id: "".into(),
            visited_type_ids: HashSet::new(),
            component_definitions: HashMap::new(),
            template_map: HashMap::new(),
            type_table: get_primitive_type_table(),
            template_node_definitions: HashMap::new(),
            import_paths: HashSet::new(),
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

/// Extract all errors from a Pax parse result
pub fn extract_errors(pairs: pest::iterators::Pairs<Rule>) -> Vec<ParsingError> {
    let mut errors = vec![];

    for pair in pairs {
        let error = match pair.as_rule() {
            Rule::block_level_error => Some((
                format!("{:?}", pair.as_rule()),
                "Unexpected template structure encountered.".to_string(),
            )),
            Rule::attribute_key_value_pair_error => Some((
                format!("{:?}", pair.as_rule()),
                "Attribute key-value pair is malformed.".to_string(),
            )),
            Rule::inner_tag_error => Some((
                format!("{:?}", pair.as_rule()),
                "Inner tag doesn't match any expected format.".to_string(),
            )),
            Rule::selector_block_error => Some((
                format!("{:?}", pair.as_rule()),
                "Selector block structure is not well-defined.".to_string(),
            )),
            Rule::handler_key_value_pair_error => Some((
                format!("{:?}", pair.as_rule()),
                "Event handler key-value pair is malformed.".to_string(),
            )),
            Rule::expression_body_error => Some((
                format!("{:?}", pair.as_rule()),
                "Expression inside curly braces is not well defined.".to_string(),
            )),
            Rule::open_tag_error => Some((
                format!("{:?}", pair.as_rule()),
                "Open tag is malformed".to_string(),
            )),
            Rule::tag_error => Some((
                format!("{:?}", pair.as_rule()),
                "Tag structure is unexpected.".to_string(),
            )),
            _ => None,
        };
        if let Some((error_name, error_message)) = error {
            let span = pair.as_span();
            let ((line_start, start_col), (line_end, end_col)) =
                (pair.line_col(), span.end_pos().line_col());
            let error = ParsingError {
                error_name,
                error_message,
                matched_string: span.as_str().to_string(),
                start: (line_start, start_col),
                end: (line_end, end_col),
            };
            errors.push(error);
        }
        errors.extend(extract_errors(pair.into_inner()));
    }

    errors
}

/// From a raw string of Pax representing a single component, parse a complete ComponentDefinition
pub fn assemble_component_definition(
    mut ctx: ParsingContext,
    pax: &str,
    pascal_identifier: &str,
    is_main_component: bool,
    template_map: HashMap<String, String>,
    module_path: &str,
    self_type_id: &str,
    component_source_file_path: &str,
) -> (ParsingContext, ComponentDefinition) {
    let _ast = PaxParser::parse(Rule::pax_component_definition, pax)
        .expect(&format!("unsuccessful parse from {}", &pax)) // unwrap the parse result
        .next()
        .unwrap(); // get and unwrap the `pax_component_definition` rule

    let errors = extract_errors(_ast.clone().into_inner());
    if !errors.is_empty() {
        let mut error_messages = String::new();

        for error in &errors {
            let msg = format!(
                "error: {}\n   --> {}:{}\n    |\n{}  | {}\n    |{}\n\n",
                error.error_message,
                error.start.0,
                error.start.1,
                error.start.0,
                error.matched_string,
                "^".repeat(error.matched_string.len())
            );
            error_messages.push_str(&msg);
        }

        panic!("{}", error_messages);
    }

    if is_main_component {
        ctx.main_component_type_id = self_type_id.to_string();
    }

    let mut tpc = TemplateNodeParseContext {
        pascal_identifier_to_type_id_map: template_map,
        template_node_definitions: HashMap::new(),
        //each frame of the outer vec represents a list of
        //children for a given node; child order matters because of z-index defaults;
        //a new frame is added when descending the tree
        //but not when iterating over siblings
        child_id_tracking_stack: vec![],
        uid_gen: (1..).multipeek(),
    };

    parse_template_from_component_definition_string(&mut tpc, pax);
    let modified_module_path = if module_path.starts_with("parser") {
        module_path.replacen("parser", "crate", 1)
    } else {
        module_path.to_string()
    };

    //populate template_node_definitions vec, needed for traversing node tree at codegen-time
    ctx.template_node_definitions = tpc.template_node_definitions.clone();

    let new_def = ComponentDefinition {
        is_primitive: false,
        is_struct_only_component: false,
        is_main_component,
        primitive_instance_import_path: None,
        type_id: self_type_id.to_string(),
        type_id_escaped: escape_identifier(self_type_id.to_string()),
        pascal_identifier: pascal_identifier.to_string(),
        template: Some(tpc.template_node_definitions),
        settings: parse_settings_from_component_definition_string(pax),
        handlers: parse_events_from_component_definition_string(pax),
        module_path: modified_module_path,
        next_template_id: Some(*tpc.uid_gen.peek().unwrap()),
        template_source_file_path: Some(component_source_file_path.to_string()),
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
    pascal_identifier: &str,
    module_path: &str,
    self_type_id: &str,
) -> (ParsingContext, ComponentDefinition) {
    let modified_module_path = clean_module_path(module_path);

    let new_def = ComponentDefinition {
        type_id: self_type_id.to_string(),
        type_id_escaped: escape_identifier(self_type_id.to_string()),
        is_main_component: false,
        is_primitive: false,
        is_struct_only_component: true,
        pascal_identifier: pascal_identifier.to_string(),
        module_path: modified_module_path,
        primitive_instance_import_path: None,
        template: None,
        settings: None,
        handlers: None,
        next_template_id: None,
        template_source_file_path: None,
    };

    (ctx, new_def)
}

pub fn assemble_primitive_definition(
    pascal_identifier: &str,
    module_path: &str,
    primitive_instance_import_path: String,
    self_type_id: &str,
) -> ComponentDefinition {
    let modified_module_path = clean_module_path(module_path);

    ComponentDefinition {
        is_primitive: true,
        is_struct_only_component: false,
        primitive_instance_import_path: Some(primitive_instance_import_path),
        is_main_component: false,
        type_id: self_type_id.to_string(),
        type_id_escaped: escape_identifier(self_type_id.to_string()),
        pascal_identifier: pascal_identifier.to_string(),
        template: None,
        settings: None,
        module_path: modified_module_path,
        handlers: None,
        next_template_id: None,
        template_source_file_path: None,
    }
}

pub fn assemble_type_definition(
    mut ctx: ParsingContext,
    property_definitions: Vec<PropertyDefinition>,
    inner_iterable_type_id: Option<String>,
    self_type_id: &str,
    import_path: String,
) -> (ParsingContext, TypeDefinition) {
    let type_id_escaped = escape_identifier(self_type_id.to_string());

    let new_def = TypeDefinition {
        type_id: self_type_id.to_string(),
        type_id_escaped,
        inner_iterable_type_id,
        property_definitions,
        import_path,
    };

    ctx.type_table
        .insert(self_type_id.to_string(), new_def.clone());

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
        //Default impl for primitives and pax_runtime::api
        let type_id = Self::get_type_id();
        let td = TypeDefinition {
            type_id: type_id.to_string(),
            type_id_escaped: escape_identifier(type_id.to_string()),
            inner_iterable_type_id: None,
            property_definitions: vec![],
            import_path: type_id.to_string(),
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

    fn get_type_id() -> String {
        //This default is used by primitives but expected to
        //be overridden by userland Pax components / primitives
        Self::get_import_path()
    }

    fn get_iterable_type_id() -> Option<String> {
        //Most types do not have an iterable type (e.g. the T in Vec<T>) —
        //it is the responsibility of iterable types to override this fn
        None
    }
}

impl Reflectable for usize {
    fn get_self_pascal_identifier() -> String {
        "usize".to_string()
    }
}
impl Reflectable for isize {
    fn get_self_pascal_identifier() -> String {
        "isize".to_string()
    }
}
impl Reflectable for i128 {
    fn get_self_pascal_identifier() -> String {
        "i128".to_string()
    }
}
impl Reflectable for u128 {
    fn get_self_pascal_identifier() -> String {
        "u128".to_string()
    }
}
impl Reflectable for i64 {
    fn get_self_pascal_identifier() -> String {
        "i64".to_string()
    }
}
impl Reflectable for u64 {
    fn get_self_pascal_identifier() -> String {
        "u64".to_string()
    }
}
impl Reflectable for i32 {
    fn get_self_pascal_identifier() -> String {
        "i32".to_string()
    }
}
impl Reflectable for u32 {
    fn get_self_pascal_identifier() -> String {
        "u32".to_string()
    }
}
impl Reflectable for i8 {
    fn get_self_pascal_identifier() -> String {
        "i8".to_string()
    }
}
impl Reflectable for u8 {
    fn get_self_pascal_identifier() -> String {
        "u8".to_string()
    }
}
impl Reflectable for f64 {
    fn get_self_pascal_identifier() -> String {
        "f64".to_string()
    }
}
impl Reflectable for f32 {
    fn get_self_pascal_identifier() -> String {
        "f32".to_string()
    }
}
impl Reflectable for bool {
    fn get_self_pascal_identifier() -> String {
        "bool".to_string()
    }
}
impl Reflectable for std::string::String {
    fn get_import_path() -> String {
        "std::string::String".to_string()
    }
    fn get_self_pascal_identifier() -> String {
        "String".to_string()
    }
}
impl<T> Reflectable for std::rc::Rc<T> {
    fn get_import_path() -> String {
        "std::rc::Rc".to_string()
    }
    fn get_self_pascal_identifier() -> String {
        "Rc".to_string()
    }
}

impl<T: Reflectable> Reflectable for std::option::Option<T> {
    fn parse_to_manifest(mut ctx: ParsingContext) -> (ParsingContext, Vec<PropertyDefinition>) {
        let type_id = Self::get_type_id();
        let td = TypeDefinition {
            type_id: type_id.to_string(),
            type_id_escaped: escape_identifier(type_id.to_string()),
            inner_iterable_type_id: None,
            property_definitions: vec![],
            import_path: type_id.to_string(),
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

    fn get_type_id() -> String {
        format!("std::option::Option<{}{}>", "{PREFIX}", &T::get_type_id())
    }
}

impl Reflectable for pax_runtime::api::Size {
    fn get_import_path() -> String {
        "pax_engine::api::Size".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Size".to_string()
    }
}

impl Reflectable for pax_runtime::api::Rotation {
    fn get_import_path() -> String {
        "pax_engine::api::Rotation".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Rotation".to_string()
    }
}

impl Reflectable for pax_runtime::api::Numeric {
    fn get_import_path() -> String {
        "pax_engine::api::Numeric".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Numeric".to_string()
    }
}

impl Reflectable for kurbo::Point {
    fn get_import_path() -> String {
        "kurbo::Point".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Point".to_string()
    }
}

impl Reflectable for pax_runtime::api::Transform2D {
    fn get_import_path() -> String {
        "pax_engine::api::Transform2D".to_string()
    }

    fn get_self_pascal_identifier() -> String {
        "Transform2D".to_string()
    }
}

impl Reflectable for pax_runtime::api::StringBox {
    fn get_import_path() -> String {
        "pax_engine::api::StringBox".to_string()
    }
    fn get_self_pascal_identifier() -> String {
        "StringBox".to_string()
    }
}

impl<T: Reflectable> Reflectable for std::vec::Vec<T> {
    fn parse_to_manifest(mut ctx: ParsingContext) -> (ParsingContext, Vec<PropertyDefinition>) {
        let type_id = Self::get_type_id();
        let td = TypeDefinition {
            type_id: type_id.to_string(),
            type_id_escaped: escape_identifier(type_id.to_string()),
            import_path: Self::get_import_path(),
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
    fn get_type_id() -> String {
        //Need to encode generics contents as part of unique id for iterables
        format!(
            "std::vec::Vec<{}{}>",
            "{PREFIX}",
            &Self::get_iterable_type_id().unwrap()
        )
    }
    fn get_iterable_type_id() -> Option<String> {
        Some(T::get_type_id())
    }
}
