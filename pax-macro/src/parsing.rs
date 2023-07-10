

extern crate pest;
use pest_derive::Parser;
use pest::Parser;
use pest::iterators::{Pair, Pairs};

use pest::{
    pratt_parser::{Assoc, Op, PrattParser},
};

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashSet;

#[derive(Parser)]
#[grammar = "../../pax-compiler/src/pax.pest"]
pub struct PaxMacroParser;


pub fn parse_literal_and_expression_dependencies(pax: &str) -> Vec<String> {

    let pax_component_definition = PaxMacroParser::parse(Rule::pax_component_definition, pax)
        .expect(&format!("unsuccessful parse from {}", &pax))
        .next().unwrap();


    let pascal_identifiers = Rc::new(RefCell::new(HashSet::new()));


    pax_component_definition.into_inner().for_each(|pair|{
        match pair.as_rule() {
            Rule::root_tag_pair => {
                recurse_visit_tag_pairs_for_literal_and_expression_dependencies(
                    pair.into_inner().next().unwrap(),
                    Rc::clone(&pascal_identifiers),
                );
            },
            _ => {}
        }
    });
    let unwrapped_hashmap = Rc::try_unwrap(pascal_identifiers).unwrap().into_inner();
    unwrapped_hashmap.into_iter().collect()



}

fn recurse_visit_tag_pairs_for_literal_and_expression_dependencies(any_tag_pair: Pair<Rule>, pascal_identifiers: Rc<RefCell<HashSet<String>>>)  {
    match any_tag_pair.as_rule() {
        Rule::matched_tag => {
            //matched_tag => open_tag => pascal_identifier
            let matched_tag = any_tag_pair;
            let open_tag = matched_tag.clone().into_inner().next().unwrap();
            let pascal_identifier = open_tag.into_inner().next().unwrap().as_str();
            pascal_identifiers.borrow_mut().insert(pascal_identifier.to_string());

            let prospective_inner_nodes = matched_tag.clone().into_inner().nth(1).unwrap();
            match prospective_inner_nodes.as_rule() {
                Rule::inner_nodes => {
                    let inner_nodes = prospective_inner_nodes;
                    inner_nodes.into_inner()
                        .for_each(|sub_tag_pair|{
                            match sub_tag_pair.as_rule() {
                                Rule::matched_tag | Rule::self_closing_tag | Rule::statement_control_flow => {
                                    //it's another tag — time to recurse
                                    recurse_visit_tag_pairs_for_pascal_identifiers(sub_tag_pair, Rc::clone(&pascal_identifiers));
                                },
                                Rule::node_inner_content => {
                                    //literal or expression content; no pascal identifiers to worry about here
                                }
                                _ => {unreachable!("Parsing error 88779273: {:?}", sub_tag_pair.as_rule());},
                            }
                        }
                        )
                },
                _ => {unreachable!("Parsing error 45834823: {:?}", matched_tag.clone().into_inner())}
            }
        },
        Rule::self_closing_tag => {
            let pascal_identifier = any_tag_pair.into_inner().next().unwrap().as_str();
            pascal_identifiers.borrow_mut().insert(pascal_identifier.to_string());
        },
        Rule::statement_control_flow => {
            let matched_tag = any_tag_pair.into_inner().next().unwrap();

            let n = match matched_tag.as_rule() {
                Rule::statement_if => {
                    1
                },
                Rule::statement_for => {
                    2
                },
                Rule::statement_slot => {
                    0
                },
                _ => {
                    unreachable!("Parsing error 944491032: {:?}", matched_tag.as_rule());
                }
            };

            let prospective_inner_nodes = matched_tag.into_inner().nth(n).expect("WRONG nth");
            match prospective_inner_nodes.as_rule() {
                Rule::inner_nodes => {
                    let inner_nodes = prospective_inner_nodes;
                    inner_nodes.into_inner()
                        .for_each(|sub_tag_pair|{
                            recurse_visit_tag_pairs_for_pascal_identifiers(sub_tag_pair, Rc::clone(&pascal_identifiers));
                        })
                },
                Rule::expression_body => {
                    //This space intentionally left blank.
                    //e.g. for `slot` -- not necessary to worry about for PascalIdentifiers
                },
                _ => {unreachable!("Parsing error 4449292922: {:?}", prospective_inner_nodes.as_rule());}
            }

        },
        _ => {unreachable!("Parsing error 123123121: {:?}", any_tag_pair.as_rule());}
    }
}


pub fn parse_pascal_identifiers_from_component_definition_string(pax: &str) -> Vec<String> {
    let pax_component_definition = PaxMacroParser::parse(Rule::pax_component_definition, pax)
        .expect(&format!("unsuccessful parse from {}", &pax)) // unwrap the parse result
        .next().unwrap(); // get and unwrap the `pax_component_definition` rule

    let pascal_identifiers = Rc::new(RefCell::new(HashSet::new()));


    pax_component_definition.into_inner().for_each(|pair|{
        match pair.as_rule() {
            Rule::root_tag_pair => {
                recurse_visit_tag_pairs_for_pascal_identifiers(
                    pair.into_inner().next().unwrap(),
                    Rc::clone(&pascal_identifiers),
                );
            },
            _ => {}
        }
    });
    let unwrapped_hashmap = Rc::try_unwrap(pascal_identifiers).unwrap().into_inner();
    unwrapped_hashmap.into_iter().collect()
}

fn recurse_visit_tag_pairs_for_pascal_identifiers(any_tag_pair: Pair<Rule>, pascal_identifiers: Rc<RefCell<HashSet<String>>>)  {
    match any_tag_pair.as_rule() {
        Rule::matched_tag => {
            //matched_tag => open_tag => pascal_identifier
            let matched_tag = any_tag_pair;
            let open_tag = matched_tag.clone().into_inner().next().unwrap();
            let pascal_identifier = open_tag.into_inner().next().unwrap().as_str();
            pascal_identifiers.borrow_mut().insert(pascal_identifier.to_string());

            let prospective_inner_nodes = matched_tag.clone().into_inner().nth(1).unwrap();
            match prospective_inner_nodes.as_rule() {
                Rule::inner_nodes => {
                    let inner_nodes = prospective_inner_nodes;
                    inner_nodes.into_inner()
                        .for_each(|sub_tag_pair|{
                            match sub_tag_pair.as_rule() {
                                Rule::matched_tag | Rule::self_closing_tag | Rule::statement_control_flow => {
                                    //it's another tag — time to recurse
                                    recurse_visit_tag_pairs_for_pascal_identifiers(sub_tag_pair, Rc::clone(&pascal_identifiers));
                                },
                                Rule::node_inner_content => {
                                    //literal or expression content; no pascal identifiers to worry about here
                                }
                                _ => {unreachable!("Parsing error 88779273: {:?}", sub_tag_pair.as_rule());},
                            }
                        }
                        )
                },
                _ => {unreachable!("Parsing error 45834823: {:?}", matched_tag.clone().into_inner())}
            }
        },
        Rule::self_closing_tag => {
            let pascal_identifier = any_tag_pair.into_inner().next().unwrap().as_str();
            pascal_identifiers.borrow_mut().insert(pascal_identifier.to_string());
        },
        Rule::statement_control_flow => {
            let matched_tag = any_tag_pair.into_inner().next().unwrap();

            let n = match matched_tag.as_rule() {
                Rule::statement_if => {
                    1
                },
                Rule::statement_for => {
                    2
                },
                Rule::statement_slot => {
                    0
                },
                _ => {
                    unreachable!("Parsing error 944491032: {:?}", matched_tag.as_rule());
                }
            };

            let prospective_inner_nodes = matched_tag.into_inner().nth(n).expect("WRONG nth");
            match prospective_inner_nodes.as_rule() {
                Rule::inner_nodes => {
                    let inner_nodes = prospective_inner_nodes;
                    inner_nodes.into_inner()
                        .for_each(|sub_tag_pair|{
                            recurse_visit_tag_pairs_for_pascal_identifiers(sub_tag_pair, Rc::clone(&pascal_identifiers));
                        })
                },
                Rule::expression_body => {
                    //This space intentionally left blank.
                    //e.g. for `slot` -- not necessary to worry about for PascalIdentifiers
                },
                _ => {unreachable!("Parsing error 4449292922: {:?}", prospective_inner_nodes.as_rule());}
            }

        },
        _ => {unreachable!("Parsing error 123123121: {:?}", any_tag_pair.as_rule());}
    }
}


pub fn pratt_parse_for_scoped_types(input_paxel: &str) -> Vec<String> {
    // NOTE! This declaration below is duplicated from pax_compiler as a hack.
    //       Ideally: revisit the divide between pax-macro and pax-compiler

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

    let scoped_types = Rc::new(RefCell::new(vec![]));
    recurse_pratt_parse_for_scoped_types(pairs, &pratt, Rc::clone(&scoped_types));
    scoped_ids.take()
}

fn  recurse_pratt_parse_for_scoped_types<'a>(expression: Pairs<Rule>, pratt_parser: &PrattParser<Rule>, scoped_types: Rc<RefCell<Vec<String>>>) {
    pratt_parser
        .map_primary(move |primary| match primary.as_rule() {
            /* expression_grouped | xo_function_call | xo_range     */
            Rule::expression_grouped => {
                /* expression_grouped = { "(" ~ expression_body ~ ")" ~ literal_number_unit? } */
                let mut inner = primary.into_inner();
                recurse_pratt_parse_for_scoped_types(inner.next().unwrap().into_inner(), pratt_parser, Rc::clone(&scoped_types));
            },
            Rule::xo_function_call => {
                /* xo_function_call = {identifier ~ (("::") ~ identifier)* ~ ("("~xo_function_args_list~")")}
                   xo_function_args_list = {expression_body ~ ("," ~ expression_body)*} */

                let mut accum = "".to_string();
                let mut last_identifier = "".to_string();

                let mut pairs = primary.into_inner();

                while let Some(next_pair) = pairs.next() {
                    if matches!(next_pair, Rule::identifier) {
                        accum += &last_identifier;
                        last_identifier = "::".to_string() + next_pair.as_str();
                    }
                }

                if accum != "" {
                    (*scoped_types).borrow_mut().push(accum);
                }

                let mut expression_body_pairs = next_pair.into_inner();

                while let Some(next_pair) = expression_body_pairs.next() {
                    recurse_pratt_parse_for_scoped_types(next_pair.into_inner(), pratt_parser, Rc::clone(&scoped_types));
                }
            },
            Rule::xo_range => {
                /* { op0: (xo_literal | xo_symbol) ~ op1: (xo_range_inclusive | xo_range_exclusive) ~ op2: (xo_literal | xo_symbol)} */
                let mut pairs = primary.into_inner();

                let op0 = pairs.next().unwrap();
                recurse_pratt_parse_for_scoped_types(op0.into_inner(), pratt_parser, Rc::clone(&scoped_types));

                let _operator = pairs.next().unwrap();

                let op2 = pairs.next().unwrap();
                recurse_pratt_parse_for_scoped_types(op2.into_inner(), pratt_parser, Rc::clone(&scoped_types)

            },
            Rule::xo_literal => {
                let literal_kind = primary.into_inner().next().unwrap();

                match literal_kind.as_rule() {
                    Rule::literal_enum_value => {
                        let mut pairs = literal_kind.into_inner();
                        //We will have at least 2 identifiers
                        //For a chain of n identifiers, we want to accum the first n-1

                        while let Some(next_pair) = pairs.next() {
                            if matches!(next_pair, Rule::identifier) {
                                accum += &last_identifier;
                                last_identifier = "::".to_string() + next_pair.as_str();
                            }
                        }
                    },
                    Rule::literal_tuple => {
                        let mut pairs = literal_kind.into_inner();
                        let lit0 = pairs.next().unwrap();
                        let lit1 = pairs.next().unwrap();

                        recurse_pratt_parse_for_scoped_types(lit0.into_inner(), pratt_parser, Rc::clone(&scoped_types));
                        recurse_pratt_parse_for_scoped_types(lit1.into_inner(), pratt_parser, Rc::clone(&scoped_types));
                    }
                    _ => {
                        literal_kind.as_str().to_string() + ".try_into().unwrap()"
                    }
                }
            },
            Rule::xo_object => {

                //recurse only
                let mut output : String = "".to_string();

                let mut inner = primary.into_inner();
                let maybe_identifier = inner.next().unwrap();
                let rule = maybe_identifier.as_rule();

                //for parsing xo_object_settings_key_value_pair
                //iterate over key-value pairs; recurse into expressions
                fn handle_xoskvp<'a>(xoskvp: Pair<Rule>, pratt_parser: &PrattParser<Rule>, scoped_types: Rc<RefCell<Vec<String>>>) -> String {
                    let mut inner_kvp = xoskvp.into_inner();
                    let settings_key = inner_kvp.next().unwrap().as_str().to_string();
                    let expression_body = inner_kvp.next().unwrap().into_inner();

                    recurse_pratt_parse_for_scoped_types(expression_body, pratt_parser, Rc::clone(&scoped_types));
                }

                handle_xoskvp(maybe_identifier, pratt_parser.clone(), Rc::clone(&scoped_types));

                let mut remaining_kvps = inner.into_iter();

                while let Some(xoskkvp) = remaining_kvps.next() {
                    let ril =  handle_xoskvp(xoskkvp, pratt_parser.clone(), Rc::clone(&scoped_types));
                    output += &ril;
                }


            },
            Rule::xo_symbol => {
                //no-op
            },
            Rule::xo_tuple => {
                //recurse only
                let mut tuple = primary.into_inner();
                let exp0 = tuple.next().unwrap();
                let exp1 = tuple.next().unwrap();
                recurse_pratt_parse_for_scoped_types( exp0.into_inner(), pratt_parser, Rc::clone(&scoped_types));
                recurse_pratt_parse_for_scoped_types( exp1.into_inner(), pratt_parser, Rc::clone(&scoped_types));
            },
            Rule::expression_body => {
                //recurse
                recurse_pratt_parse_for_scoped_types(primary.into_inner(), pratt_parser.clone(), Rc::clone(&scoped_types))
            },
            _ => unreachable!(),
        })
        .map_prefix(|op, rhs| match op.as_rule() {
            _ => {},
        })
        // .map_postfix(|lhs, op| match op.as_rule() {
        //     Rule::fac => format!("({}!)", lhs),
        //     _ => unreachable!(),
        // })
        .map_infix(|lhs, op, rhs| match op.as_rule() {
            _ => {},
        })
        .parse(expression)
}