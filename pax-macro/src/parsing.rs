extern crate pest;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Parser)]
#[grammar = "pax.pest"]
pub struct PaxMacroParser;

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

pub fn parse_pascal_identifiers_from_component_definition_string(pax: &str) -> Vec<String> {
    let pax_component_definition = PaxMacroParser::parse(Rule::pax_component_definition, pax)
        .expect(&format!("unsuccessful parse from {}", &pax))
        .next()
        .unwrap();

    let errors = extract_errors(pax_component_definition.clone().into_inner());
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

    let pascal_identifiers = Rc::new(RefCell::new(HashSet::new()));
    pax_component_definition
        .into_inner()
        .for_each(|pair| match pair.as_rule() {
            Rule::root_tag_pair => {
                recurse_visit_tag_pairs_for_pascal_identifiers(
                    pair.into_inner().next().unwrap(),
                    Rc::clone(&pascal_identifiers),
                );
            }
            _ => {}
        });

    let unwrapped_hashmap = Rc::try_unwrap(pascal_identifiers).unwrap().into_inner();
    unwrapped_hashmap.into_iter().collect()
}

fn recurse_visit_tag_pairs_for_pascal_identifiers(
    any_tag_pair: Pair<Rule>,
    pascal_identifiers: Rc<RefCell<HashSet<String>>>,
) {
    match any_tag_pair.as_rule() {
        Rule::matched_tag => {
            //matched_tag => open_tag => pascal_identifier
            let matched_tag = any_tag_pair;
            let open_tag = matched_tag.clone().into_inner().next().unwrap();
            let pascal_identifier = open_tag.into_inner().next().unwrap().as_str();
            pascal_identifiers
                .borrow_mut()
                .insert(pascal_identifier.to_string());

            let prospective_inner_nodes = matched_tag.clone().into_inner().nth(1).unwrap();
            match prospective_inner_nodes.as_rule() {
                Rule::inner_nodes => {
                    let inner_nodes = prospective_inner_nodes;
                    inner_nodes.into_inner().for_each(|sub_tag_pair| {
                        match sub_tag_pair.as_rule() {
                            Rule::matched_tag
                            | Rule::self_closing_tag
                            | Rule::statement_control_flow => {
                                //it's another tag — time to recurse
                                recurse_visit_tag_pairs_for_pascal_identifiers(
                                    sub_tag_pair,
                                    Rc::clone(&pascal_identifiers),
                                );
                            }
                            Rule::node_inner_content => {
                                //literal or expression content; no pascal identifiers to worry about here
                            }
                            Rule::comment => {}
                            _ => {
                                unreachable!(
                                    "Parsing error 88779273: {:?}",
                                    sub_tag_pair.as_rule()
                                );
                            }
                        }
                    })
                }
                Rule::comment => {}
                _ => {
                    unreachable!(
                        "Parsing error 45834823: {:?}",
                        matched_tag.clone().into_inner()
                    )
                }
            }
        }
        Rule::self_closing_tag => {
            let pascal_identifier = any_tag_pair.into_inner().next().unwrap().as_str();
            pascal_identifiers
                .borrow_mut()
                .insert(pascal_identifier.to_string());
        }
        Rule::statement_control_flow => {
            let matched_tag = any_tag_pair.into_inner().next().unwrap();

            let n = match matched_tag.as_rule() {
                Rule::statement_if => 1,
                Rule::statement_for => 2,
                Rule::statement_slot => 0,
                _ => {
                    unreachable!("Parsing error 944491032: {:?}", matched_tag.as_rule());
                }
            };

            let prospective_inner_nodes = matched_tag.into_inner().nth(n).expect("WRONG nth");
            match prospective_inner_nodes.as_rule() {
                Rule::inner_nodes => {
                    let inner_nodes = prospective_inner_nodes;
                    inner_nodes.into_inner().for_each(|sub_tag_pair| {
                        recurse_visit_tag_pairs_for_pascal_identifiers(
                            sub_tag_pair,
                            Rc::clone(&pascal_identifiers),
                        );
                    })
                }
                Rule::expression_body => {
                    //This space intentionally left blank.
                    //e.g. for `slot` -- not necessary to worry about for PascalIdentifiers
                }
                _ => {
                    unreachable!(
                        "Parsing error 4449292922: {:?}",
                        prospective_inner_nodes.as_rule()
                    );
                }
            }
        }
        Rule::comment => {}
        _ => {
            unreachable!("Parsing error 123123121: {:?}", any_tag_pair.as_rule());
        }
    }
}
