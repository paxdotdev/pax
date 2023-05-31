use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{HashSet, HashMap};
use std::ops::{RangeFrom};
use itertools::{Itertools, MultiPeek};

use crate::manifest::{PropertyDefinition, ComponentDefinition, TemplateNodeDefinition, ControlFlowSettingsDefinition, ControlFlowRepeatPredicateDefinition, ValueDefinition, SettingsSelectorBlockDefinition, LiteralBlockDefinition, ControlFlowRepeatSourceDefinition, EventDefinition, TypeDefinition, TypeTable};

use uuid::Uuid;

extern crate pest;
use pest_derive::Parser;
use pest::Parser;
use pest::iterators::{Pair, Pairs};

use pest::{
    pratt_parser::{Assoc, Op, PrattParser},
};
use pax_message::reflection::Reflectable;

#[derive(Parser)]
#[grammar = "pax.pest"]
pub struct PaxParser;

pub fn assemble_primitive_definition(pascal_identifier: &str, module_path: &str, source_id: &str, property_definitions: &Vec<PropertyDefinition>, primitive_instance_import_path: String, self_type_definition: TypeDefinition) -> ComponentDefinition {
    let modified_module_path = if module_path.starts_with("parser") {
        module_path.replacen("parser", "crate", 1)
    } else {
        module_path.to_string()
    };

    let sub_properties : HashMap<String, PropertyDefinition> = property_definitions.iter().map(|pd|{(pd.name.clone(), pd.clone())}).collect();
    let sub_properties = Some(sub_properties);
    let x = TypeDefinition {
        original_type: pascal_identifier.to_string(),
        fully_qualified_type: "".to_string(),
        fully_qualified_type_pascalized: "".to_string(),
        fully_qualified_constituent_types: vec![],
        sub_properties,
    };

    ComponentDefinition {
        is_primitive: true,
        is_type: false,
        primitive_instance_import_path: Some(primitive_instance_import_path),
        is_main_component: false,
        source_id: source_id.to_string(),
        pascal_identifier: pascal_identifier.to_string(),
        template: None,
        settings: None,
        module_path: modified_module_path,
        self_type_definition,
        events: None,
    }
}

/// Returns (RIL output string, `symbolic id`s found during parse)
/// where a `symbolic id` may be something like `self.num_clicks` or `i`
pub fn run_pratt_parser(input_paxel: &str) -> (String, Vec<String>) {

    // Operator precedence is declared via the ordering here:
    let pratt = PrattParser::new()
        .op(Op::infix(Rule::xo_tern_then, Assoc::Left) | Op::infix(Rule::xo_tern_else, Assoc::Right))
        .op(Op::infix(Rule::xo_bool_and, Assoc::Left) | Op::infix(Rule::xo_bool_or, Assoc::Left))
        .op(Op::infix(Rule::xo_add, Assoc::Left) | Op::infix(Rule::xo_sub, Assoc::Left))
        .op(Op::infix(Rule::xo_mul, Assoc::Left) | Op::infix(Rule::xo_div, Assoc::Left))
        .op(Op::infix(Rule::xo_mod, Assoc::Left))
        .op(Op::infix(Rule::xo_exp, Assoc::Right))
        .op(Op::prefix(Rule::xo_neg))
        .op(
            Op::infix(Rule::xo_rel_eq, Assoc::Left) |
            Op::infix(Rule::xo_rel_neq, Assoc::Left) |
            Op::infix(Rule::xo_rel_lt, Assoc::Left) |
            Op::infix(Rule::xo_rel_lte, Assoc::Left) |
            Op::infix(Rule::xo_rel_gt, Assoc::Left) |
            Op::infix(Rule::xo_rel_gte, Assoc::Left)
        )
        .op(Op::prefix(Rule::xo_bool_not));

    let pairs = PaxParser::parse(Rule::expression_body, input_paxel).expect(&format!("unsuccessful pratt parse {}", &input_paxel));

    let symbolic_ids = Rc::new(RefCell::new(vec![]));
    let output = recurse_pratt_parse_to_string(pairs, &pratt, Rc::clone(&symbolic_ids));
    (output, symbolic_ids.take())
}


/// Removes leading `self.` or `this.`
/// Converts any remaining `.` to `_DOT_`
fn convert_symbolic_binding_from_paxel_to_ril(xo_symbol: Pair<Rule>) -> String {
    let mut pairs = xo_symbol.clone().into_inner();
    let maybe_this_or_self = pairs.next().unwrap().as_str();

    let self_or_this_removed = if maybe_this_or_self == "this" || maybe_this_or_self == "self" {
        let mut output = "".to_string();

        //accumulate remaining identifiers, having skipped `this` or `self` with the original `.next()`
        pairs.for_each(|pair|{
            output += &*(".".to_owned() + pair.as_str())
        });

        //remove initial fencepost "."
        output.replacen(".", "", 1)
    } else {
        //remove original binding; no self or this
        xo_symbol.as_str().to_string()
    };

    self_or_this_removed.replace(".", "_DOT_")
}

/// Workhorse method for compiling Expressions into Rust Intermediate Language (RIL, a string of Rust)
fn recurse_pratt_parse_to_string<'a>(expression: Pairs<Rule>, pratt_parser: &PrattParser<Rule>, symbolic_ids: Rc<RefCell<Vec<String>>>) -> String {
    pratt_parser
        .map_primary(move |primary| match primary.as_rule() {
            /* expression_grouped | xo_function_call | xo_range     */
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
                    } else {
                        unreachable!()
                    }
                } else {
                    exp_bod
                }
            },
            Rule::xo_function_call => {
                /* xo_function_call = {identifier ~ (("::") ~ identifier)* ~ ("("~xo_function_args_list~")")}
                   xo_function_args_list = {expression_body ~ ("," ~ expression_body)*} */

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
                        //for symbolic identifiers, remove any "this" or "self", then return string
                        convert_symbolic_binding_from_paxel_to_ril(op0)
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
                        //for symbolic identifiers, remove any "this" or "self", then return string
                        convert_symbolic_binding_from_paxel_to_ril(op2)
                    },
                    _ => unimplemented!("")
                };

                format!("{}", op0_out + &op1_out + &op2_out)
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
                        } else {
                            unreachable!()
                        }
                    },
                    Rule::literal_number => {
                        let mut inner = literal_kind.into_inner();
                        let value = inner.next().unwrap().as_str();
                        format!("Numeric::from({})", value)
                    }
                    _ => {
                        /* {literal_enum_value | literal_tuple_access | string | literal_tuple } */
                        literal_kind.as_str().to_string() + ".try_into().unwrap()"
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
                    let ril = handle_xoskvp(maybe_identifier, pratt_parser.clone(), Rc::clone(&symbolic_ids));
                    output += &ril;
                }

                let mut remaining_kvps = inner.into_iter();

                while let Some(xoskkvp) = remaining_kvps.next() {
                    let ril =  handle_xoskvp(xoskkvp, pratt_parser.clone(), Rc::clone(&symbolic_ids));
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
            Rule::expression_body => {
                recurse_pratt_parse_to_string(primary.into_inner(), pratt_parser.clone(), Rc::clone(&symbolic_ids))
            },
            _ => unreachable!(),
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




fn parse_template_from_component_definition_string(ctx: &mut TemplateNodeParseContext, pax: &str)  {
    let pax_component_definition = PaxParser::parse(Rule::pax_component_definition, pax)
        .expect(&format!("unsuccessful parse from {}", &pax)) // unwrap the parse result
        .next().unwrap(); // get and unwrap the `pax_component_definition` rule

    let mut roots_ids = vec![];

    //insert placeholder for IMPLICIT_ROOT.  We will fill this back in on the post-order.
    ctx.template_node_definitions.insert(0, TemplateNodeDefinition::default());

    pax_component_definition.into_inner().for_each(|pair|{
        match pair.as_rule() {
            Rule::root_tag_pair => {
                ctx.child_id_tracking_stack.push(vec![]);
                let next_id = ctx.uid_gen.peek().unwrap();
                roots_ids.push(*next_id);
                recurse_visit_tag_pairs_for_template(
                    ctx,
                    pair.into_inner().next().unwrap(),
                );
            }
            _ => {}
        }
    });

    // This IMPLICIT_ROOT placeholder node, at index 0 of the TND vec,
    // is a container for the child_ids that act as "multi-roots," which enables
    // templates to be authored without requiring a single top-level container
    ctx.template_node_definitions.remove(0);
    ctx.template_node_definitions.insert(0,
        TemplateNodeDefinition {
            id: 0,
            child_ids: roots_ids,
            component_id: "IMPLICIT_ROOT".to_string(),
            control_flow_settings: None,
            settings: None,
            pascal_identifier: "<UNREACHABLE>".to_string()
        }
    );


}

struct TemplateNodeParseContext {
    pub template_node_definitions: Vec<TemplateNodeDefinition>,
    pub pascal_identifier_to_component_id_map: HashMap<String, String>,
    //each frame of the outer vec represents a list of
    //children for a given node;
    //a new frame is added when descending the tree
    //but not when iterating over siblings
    pub child_id_tracking_stack: Vec<Vec<usize>>,
    pub uid_gen: MultiPeek<RangeFrom<usize>>,
}

pub static COMPONENT_ID_IF : &str = "IF";
pub static COMPONENT_ID_REPEAT : &str = "REPEAT";
pub static COMPONENT_ID_SLOT : &str = "SLOT";

fn recurse_visit_tag_pairs_for_template(ctx: &mut TemplateNodeParseContext, any_tag_pair: Pair<Rule>)  {
    let new_id = ctx.uid_gen.next().unwrap();
    //insert blank placeholder
    ctx.template_node_definitions.insert(new_id, TemplateNodeDefinition::default());

    //add self to parent's children_id_list
    let mut parents_children_id_list = ctx.child_id_tracking_stack.pop().unwrap();
    parents_children_id_list.push(new_id.clone());
    ctx.child_id_tracking_stack.push(parents_children_id_list);

    match any_tag_pair.as_rule() {
        Rule::matched_tag => {
            //matched_tag => open_tag > pascal_identifier
            let matched_tag = any_tag_pair;
            let mut open_tag = matched_tag.clone().into_inner().next().unwrap().into_inner();
            let pascal_identifier = open_tag.next().unwrap().as_str();

            //push the empty frame for this node's children
            ctx.child_id_tracking_stack.push(vec![]);

            //recurse into inner_nodes
            let prospective_inner_nodes = matched_tag.into_inner().nth(1).unwrap();
            match prospective_inner_nodes.as_rule() {
                Rule::inner_nodes => {
                    let inner_nodes = prospective_inner_nodes;
                    inner_nodes.into_inner()
                        .for_each(|sub_tag_pair|{
                            recurse_visit_tag_pairs_for_template(ctx, sub_tag_pair);
                        })
                },
                _ => {panic!("wrong prospective inner nodes (or nth)")}
            }

            let mut template_node = TemplateNodeDefinition {
                id: new_id,
                control_flow_settings: None,
                component_id: ctx.pascal_identifier_to_component_id_map.get(pascal_identifier.clone()).expect(&format!("Template key not found {}", &pascal_identifier)).to_string(),
                settings: parse_inline_attribute_from_final_pairs_of_tag(open_tag),
                child_ids: ctx.child_id_tracking_stack.pop().unwrap(),
                pascal_identifier: pascal_identifier.to_string(),
            };
            std::mem::swap(ctx.template_node_definitions.get_mut(new_id).unwrap(),  &mut template_node);
        },
        Rule::self_closing_tag => {
            let mut tag_pairs = any_tag_pair.into_inner();
            let pascal_identifier = tag_pairs.next().unwrap().as_str();

            let mut template_node = TemplateNodeDefinition {
                id: new_id,
                control_flow_settings: None,
                component_id: ctx.pascal_identifier_to_component_id_map.get(pascal_identifier).expect(&format!("Template key not found {}", &pascal_identifier)).to_string(),
                settings: parse_inline_attribute_from_final_pairs_of_tag(tag_pairs),
                child_ids: vec![],
                pascal_identifier: pascal_identifier.to_string(),
            };
            std::mem::swap(ctx.template_node_definitions.get_mut(new_id).unwrap(),  &mut template_node);
        },
        Rule::statement_control_flow => {
            /* statement_control_flow = {(statement_if | statement_for | statement_slot)} */

            //push the empty frame for this node's children
            ctx.child_id_tracking_stack.push(vec![]);

            let any_tag_pair = any_tag_pair.into_inner().next().unwrap();
            let mut template_node_definition = match any_tag_pair.as_rule() {
                Rule::statement_if => {

                    let mut statement_if = any_tag_pair.into_inner();
                    let expression_body = statement_if.next().unwrap().as_str().to_string();
                    let prospective_inner_nodes = statement_if.next();

                    if let Some(inner_nodes) = prospective_inner_nodes {
                        inner_nodes.into_inner()
                            .for_each(|sub_tag_pair|{
                                recurse_visit_tag_pairs_for_template(ctx, sub_tag_pair);
                            })
                    }

                    //`if` TemplateNodeDefinition
                    TemplateNodeDefinition {
                        id: new_id.clone(),
                        control_flow_settings: Some(ControlFlowSettingsDefinition {
                            condition_expression_paxel: Some(expression_body),
                            condition_expression_vtable_id: None, //This will be written back to this data structure later, during expression compilation
                            slot_index_expression_paxel: None,
                            slot_index_expression_vtable_id: None,
                            repeat_predicate_definition: None,
                            repeat_source_definition: None
                        }),
                        component_id: COMPONENT_ID_IF.to_string(),
                        settings: None,
                        child_ids: ctx.child_id_tracking_stack.pop().unwrap(),
                        pascal_identifier: "Conditional".to_string(),
                    }
                },
                Rule::statement_for => {
                    let mut cfavd = ControlFlowSettingsDefinition::default();
                    let mut for_statement = any_tag_pair.clone().into_inner();
                    let mut predicate_declaration = for_statement.next().unwrap().into_inner();
                    let source = for_statement.next().unwrap();

                    let prospective_inner_nodes = for_statement.next();

                    if predicate_declaration.clone().count() > 1 {
                        //tuple, like the `elem, i` in `for (elem, i) in self.some_list`
                        cfavd.repeat_predicate_definition = Some(ControlFlowRepeatPredicateDefinition::ElemIdIndexId(
                            (&predicate_declaration.next().unwrap().as_str()).to_string(),
                            (&predicate_declaration.next().unwrap().as_str()).to_string()
                        ));

                    } else {
                        //single identifier, like the `elem` in `for elem in self.some_list`
                        cfavd.repeat_predicate_definition = Some(ControlFlowRepeatPredicateDefinition::ElemId(predicate_declaration.as_str().to_string()));
                    }

                    let inner_source = source.into_inner().next().unwrap();
                    /* statement_for_source = { xo_range | xo_symbol } */
                    let repeat_source_definition = match inner_source.as_rule() {
                        Rule::xo_range => {
                            ControlFlowRepeatSourceDefinition {
                                range_expression_paxel: Some(inner_source.as_str().to_string()),
                                vtable_id: None, //This will be written back to this data structure later, during expression compilation
                                symbolic_binding: None,
                            }
                        },
                        Rule::xo_symbol => {
                            ControlFlowRepeatSourceDefinition {
                                range_expression_paxel: None,
                                vtable_id: None,
                                symbolic_binding: Some(convert_symbolic_binding_from_paxel_to_ril(inner_source)),
                            }
                        },
                        _ => {unreachable!()}
                    };

                    cfavd.repeat_source_definition = Some(repeat_source_definition);

                    if let Some(inner_nodes) = prospective_inner_nodes {
                        inner_nodes.into_inner()
                            .for_each(|sub_tag_pair|{
                                recurse_visit_tag_pairs_for_template(ctx, sub_tag_pair);
                            })
                    }

                    //`for` TemplateNodeDefinition
                    TemplateNodeDefinition {
                        id: new_id.clone(),
                        component_id: COMPONENT_ID_REPEAT.to_string(),
                        control_flow_settings: Some(cfavd),
                        settings: None,
                        child_ids: ctx.child_id_tracking_stack.pop().unwrap(),
                        pascal_identifier: "Repeat".to_string(),
                    }
                },
                Rule::statement_slot => {
                    let mut statement_slot = any_tag_pair.into_inner();
                    let expression_body = statement_slot.next().unwrap().as_str().to_string();
                    let prospective_inner_nodes = statement_slot.next();

                    if let Some(inner_nodes) = prospective_inner_nodes {
                        inner_nodes.into_inner()
                            .for_each(|sub_tag_pair|{
                                recurse_visit_tag_pairs_for_template(ctx, sub_tag_pair);
                            })
                    }

                    TemplateNodeDefinition {
                        id: new_id.clone(),
                        control_flow_settings: Some(ControlFlowSettingsDefinition {
                            condition_expression_paxel: None,
                            condition_expression_vtable_id: None,
                            slot_index_expression_paxel: Some(expression_body),
                            slot_index_expression_vtable_id: None, //This will be written back to this data structure later, during expression compilation
                            repeat_predicate_definition: None,
                            repeat_source_definition: None
                        }),
                        component_id: COMPONENT_ID_SLOT.to_string(),
                        settings: None,
                        child_ids: ctx.child_id_tracking_stack.pop().unwrap(),
                        pascal_identifier: "Slot".to_string(),
                    }
                },
                _ => {
                    unreachable!("Parsing error: {:?}", any_tag_pair.as_rule());
                }
            };

            std::mem::swap(ctx.template_node_definitions.get_mut(new_id).unwrap(),  &mut template_node_definition);
        },
        Rule::node_inner_content => {
            //For example:  `<Text>"I am inner content"</Text>`
            unimplemented!("Inner content not yet supported");
        },
        _ => {unreachable!("Parsing error: {:?}", any_tag_pair.as_rule());}
    }
}

fn parse_inline_attribute_from_final_pairs_of_tag ( final_pairs_of_tag: Pairs<Rule>) -> Option<Vec<(String, ValueDefinition)>> {
    let vec : Vec<(String, ValueDefinition)> = final_pairs_of_tag.map(|attribute_key_value_pair|{
        match attribute_key_value_pair.clone().into_inner().next().unwrap().as_rule() {
            Rule::attribute_event_binding => {
                // attribute_event_binding = {attribute_event_id ~ "=" ~ xo_symbol}
                let mut kv = attribute_key_value_pair.into_inner();
                let mut attribute_event_binding = kv.next().unwrap().into_inner();
                let event_id = attribute_event_binding.next().unwrap().into_inner().last().unwrap().as_str().to_string();
                let symbolic_binding = attribute_event_binding.next().unwrap().into_inner().next().unwrap().as_str().to_string();
                (event_id, ValueDefinition::EventBindingTarget(symbolic_binding))
            },
            _ => { //Vanilla `key=value` setting pair

                let mut kv = attribute_key_value_pair.into_inner();
                let key = kv.next().unwrap().as_str().to_string();
                let raw_value = kv.next().unwrap().into_inner().next().unwrap();
                let value = match raw_value.as_rule() {
                    Rule::literal_value => {
                        //we want to pratt-parse literals, mostly to unpack `px` and `%` (recursively)
                        let (output_string,  _) = crate::parsing::run_pratt_parser(raw_value.as_str());
                        ValueDefinition::LiteralValue(output_string)
                    },
                    Rule::literal_object => {
                        ValueDefinition::Block(
                            derive_value_definition_from_literal_object_pair(raw_value)
                        )
                    },
                    Rule::expression_body => { ValueDefinition::Expression(raw_value.as_str().to_string(), None)},
                    Rule::identifier => { ValueDefinition::Identifier(raw_value.as_str().to_string(), None)},
                    _ => {unreachable!("Parsing error 3342638857230: {:?}", raw_value.as_rule());}
                };
                (key, value)
            }
        }

    }).collect();

    if vec.len() > 0 {
        Some(vec)
    }else{
        None
    }
}

fn derive_value_definition_from_literal_object_pair(literal_object: Pair<Rule>) -> LiteralBlockDefinition {
    let mut literal_object_pairs = literal_object.into_inner();

    let explicit_type_pascal_identifier = match literal_object_pairs.peek().unwrap().as_rule() {
        Rule::pascal_identifier => {
            Some(literal_object_pairs.next().unwrap().as_str().to_string())
        },
        _ => { None }
    };

    LiteralBlockDefinition {
        explicit_type_pascal_identifier,
        settings_key_value_pairs: literal_object_pairs.map(|settings_key_value_pair| {
            let mut pairs = settings_key_value_pair.into_inner();
            let setting_key = pairs.next().unwrap().into_inner().next().unwrap().as_str().to_string();
            let raw_value = pairs.next().unwrap().into_inner().next().unwrap();
            let setting_value = match raw_value.as_rule() {
                Rule::literal_value => {
                    //we want to pratt-parse literals, mostly to unpack `px` and `%` (recursively)
                    let (output_string,  _) = crate::parsing::run_pratt_parser(raw_value.as_str());
                    ValueDefinition::LiteralValue(output_string)},
                Rule::literal_object => { ValueDefinition::Block(
                    //Recurse
                    derive_value_definition_from_literal_object_pair(raw_value)
                )},
                // Rule::literal_enum_value => {ValueDefinition::Enum(raw_value.as_str().to_string())},
                Rule::expression_body => {
                    ValueDefinition::Expression(raw_value.as_str().to_string(), None)},
                _ => {unreachable!("Parsing error 231453468: {:?}", raw_value.as_rule());}
            };

            (setting_key, setting_value)

        }).collect()
    }
}

fn parse_settings_from_component_definition_string(pax: &str) -> Option<Vec<SettingsSelectorBlockDefinition>> {

    let pax_component_definition = PaxParser::parse(Rule::pax_component_definition, pax)
        .expect(&format!("unsuccessful parse from {}", &pax)) // unwrap the parse result
        .next().unwrap(); // get and unwrap the `pax_component_definition` rule

    let mut ret : Vec<SettingsSelectorBlockDefinition> = vec![];

    pax_component_definition.into_inner().for_each(|top_level_pair|{
        match top_level_pair.as_rule() {
            Rule::settings_block_declaration => {

                let selector_block_definitions: Vec<SettingsSelectorBlockDefinition> = top_level_pair.into_inner().map(|selector_block| {
                    //selector_block => settings_key_value_pair where v is a ValueDefinition
                    let mut selector_block_pairs = selector_block.into_inner();
                    //first pair is the selector itself
                    let raw_selector = selector_block_pairs.next().unwrap().as_str();
                    let selector: String = raw_selector.chars().filter(|c| !c.is_whitespace()).collect();
                    let literal_object = selector_block_pairs.next().unwrap();

                    SettingsSelectorBlockDefinition {
                        selector: selector.clone(),
                        value_block: derive_value_definition_from_literal_object_pair(literal_object),
                    }
                }).collect();

                ret.extend(selector_block_definitions);
            }
            _ => {}
        }
    });
    Some(ret)
}

fn parse_events_from_component_definition_string(pax: &str) -> Option<Vec<EventDefinition>> {

    let pax_component_definition = PaxParser::parse(Rule::pax_component_definition, pax)
        .expect(&format!("unsuccessful parse from {}", &pax)) // unwrap the parse result
        .next().unwrap(); // get and unwrap the `pax_component_definition` rule

    let mut ret : Vec<EventDefinition> = vec![];

    pax_component_definition.into_inner().for_each(|top_level_pair|{
        match top_level_pair.as_rule() {
            Rule::events_block_declaration => {

                let event_definitions: Vec<EventDefinition> = top_level_pair.into_inner().map(|events_key_value_pair| {
                    let mut pairs = events_key_value_pair.into_inner();
                    let key = pairs.next().unwrap().into_inner().next().unwrap().as_str().to_string();
                    let raw_values = pairs.next().unwrap().into_inner().next().unwrap();
                    let value = match raw_values.as_rule() {
                        Rule::literal_function => {
                            vec![raw_values.into_inner().next().unwrap().as_str().to_string()]
                        },
                        Rule::function_list => {
                            raw_values.into_inner().map(|literal_function|{
                                literal_function.into_inner().next().unwrap().as_str().to_string()
                            }).collect()
                        },
                        _ => {unreachable!("Parsing error: {:?}", raw_values.as_rule());}
                    };
                    EventDefinition {
                        key,
                        value,
                    }
                }).collect();

                ret.extend(event_definitions);
            }
            _ => {}
        }
    });
    Some(ret)
}

pub fn create_uuid() -> String {
    Uuid::new_v4().to_string()
}

pub struct ParsingContext {
    /// Used to track which files/sources have been visited during parsing,
    /// to prevent duplicate parsing
    pub visited_source_ids: HashSet<String>,

    pub main_component_id: String,

    pub component_definitions: HashMap<String, ComponentDefinition>,

    pub template_map: HashMap<String, String>,
    pub source_type_map: HashMap<String, TypeDefinition>,

    //(SourceID, associated Strings)
    pub all_property_definitions: HashMap<String, Vec<PropertyDefinition>>,

    pub template_node_definitions: Vec<TemplateNodeDefinition>,
}

impl Default for ParsingContext {
    fn default() -> Self {
        Self {
            main_component_id: "".into(),
            visited_source_ids: HashSet::new(),
            component_definitions: HashMap::new(),
            template_map: HashMap::new(),
            source_type_map: Default::default(),
            all_property_definitions: HashMap::new(),
            template_node_definitions: vec![],
        }
    }
}

/// From a raw string of Pax representing a single component, parse a complete ComponentDefinition
pub fn assemble_component_definition(mut ctx: ParsingContext, pax: &str, pascal_identifier: &str, is_main_component: bool, template_map: HashMap<String, String>, source_id: &str, module_path: &str, self_type_definition: TypeDefinition) -> (ParsingContext, ComponentDefinition) {
    let _ast = PaxParser::parse(Rule::pax_component_definition, pax)
        .expect(&format!("unsuccessful parse from {}", &pax)) // unwrap the parse result
        .next().unwrap(); // get and unwrap the `pax_component_definition` rule

    if is_main_component {
        ctx.main_component_id = source_id.to_string();
    }

    let mut tpc = TemplateNodeParseContext {
        pascal_identifier_to_component_id_map: template_map,
        template_node_definitions: vec![],
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

    let property_definitions = ctx.all_property_definitions.get(source_id).unwrap().clone();

    //populate template_node_definitions vec, needed for traversing node tree at codegen-time
    ctx.template_node_definitions = tpc.template_node_definitions.clone();

    let new_def = ComponentDefinition {
        is_primitive: false,
        is_type: false,
        is_main_component,
        primitive_instance_import_path: None,
        source_id: source_id.into(),
        pascal_identifier: pascal_identifier.to_string(),
        template: Some(tpc.template_node_definitions),
        settings: parse_settings_from_component_definition_string(pax),
        events: parse_events_from_component_definition_string(pax),
        module_path: modified_module_path,
        self_type_definition,
    };

    (ctx, new_def)
}

pub fn assemble_pax_type_definition(ctx: ParsingContext, pascal_identifier: &str, source_id: &str, module_path: &str) -> (ParsingContext, ComponentDefinition) {

    let modified_module_path = if module_path.starts_with("parser") {
        module_path.replacen("parser", "crate", 1)
    } else {
        module_path.to_string()
    };

    let property_definitions = ctx.all_property_definitions.get(source_id).unwrap().clone();

    let new_def = ComponentDefinition {
        source_id: source_id.into(),
        is_main_component: false,
        is_primitive: false,
        is_type: true,
        pascal_identifier: pascal_identifier.to_string(),
        module_path: modified_module_path,
        primitive_instance_import_path: None,
        template: None,
        settings: None,
        events: None,
        self_type_definition: Default::default(),
    };

    (ctx, new_def)
}

pub fn assemble_type_definition(
    ctx: ParsingContext,
    original_type: &str,
    fully_qualified_constituent_types: Vec<String>,
    dep_to_fqd_map: &HashMap<&str, String>,
    sub_properties: Option<HashMap<String, PropertyDefinition>>,
) -> (ParsingContext, TypeDefinition) {

    let mut fully_qualified_type = original_type.to_string();
    //extract dep_to_fqd_map into a Vec<String>; string-replace each looked-up value present in
    //unexpanded_path, ensuring that each looked-up value is not preceded by a `::`
    dep_to_fqd_map.keys().for_each(|key| {
        fully_qualified_type.clone().match_indices(key).for_each(|i|{
            if i.0 < 2 || {let maybe_coco : String = fully_qualified_type.chars().skip((i.0 as i64) as usize - 2).take(2).collect(); maybe_coco != "::" } {
                let new_value = "{PREFIX}".to_string() + &dep_to_fqd_map.get(key).unwrap();
                let starting_index : i64 = i.0 as i64;
                let end_index_exclusive = starting_index + key.len() as i64;
                fully_qualified_type.replace_range(starting_index as usize..end_index_exclusive as usize, &new_value);
            }
        });
    });

    let fully_qualified_type_pascalized = escape_identifier(fully_qualified_type.clone());

    let new_def = TypeDefinition {
        original_type: original_type.to_string(),
        fully_qualified_type,
        fully_qualified_type_pascalized,
        fully_qualified_constituent_types,
        sub_properties,
    };

    (ctx, new_def)
}


pub fn escape_identifier(input: String) -> String {
    input
        .replace("(","LPAR")
        .replace("::","COCO")
        .replace(")","RPAR")
        .replace("<","LABR")
        .replace(">","RABR")
        .replace(",","COMM")
        .replace(".","PERI")
        .replace("[","LSQB")
        .replace("]","RSQB")
}

/// This trait is used only to extend primitives like u64
/// with the parser-time method `parse_type_to_manifest`.  This
/// allows the parser binary to codegen calls to `::parse_type_to_manifest()` even
/// on primitive types
pub trait TypeParsable {
    fn parse_type_to_manifest(mut ctx: ParsingContext) -> ParsingContext {
        //Default impl: no-op
        ctx
    }
}

impl TypeParsable for usize {}
impl TypeParsable for isize {}
impl TypeParsable for i128 {}
impl TypeParsable for u128 {}
impl TypeParsable for i64 {}
impl TypeParsable for u64 {}
impl TypeParsable for i32 {}
impl TypeParsable for u32 {}
impl TypeParsable for i8 {}
impl TypeParsable for u8 {}
impl TypeParsable for f64 {}
impl TypeParsable for f32 {}
impl TypeParsable for bool {}
impl TypeParsable for std::string::String {}
impl<T> TypeParsable for std::rc::Rc<T> {}
impl<T: Reflectable> TypeParsable for std::vec::Vec<T> {}