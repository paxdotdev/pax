use pax_lang::{parse_pax_str, Pair, Pairs, Parser, PaxParser, Rule};

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

pub fn parse_pascal_identifiers_from_component_definition_string(
    pax: &str,
) -> Result<Vec<String>, String> {
    let pax_component_definition = parse_pax_str(Rule::pax_component_definition, pax)?;

    let pascal_identifiers: Rc<RefCell<HashSet<String>>> = Rc::new(RefCell::new(HashSet::new()));
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
    Ok(unwrapped_hashmap.into_iter().collect())
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
