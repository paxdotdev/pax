#[macro_use]
extern crate pest_derive;


extern crate pest;

// #[macro_use]
// extern crate lazy_static;

use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashSet;
use std::fs;
use std::hint::unreachable_unchecked;
use std::rc::Rc;
use pest::iterators::Pair;

use uuid::Uuid;


use pest::Parser;
use pax_message::{ComponentDefinition, PaxManifest, SettingsDefinition, TemplateNodeDefinition};
// use pest::prec_climber::PrecClimber;

#[derive(Parser)]
#[grammar = "pax.pest"]
pub struct PaxParser;



/*
COMPILATION STAGES

0. Process template
    - build render tree by parsing template file
        - unroll @{} into a vanilla tree (e.g. `<repeat>` instead of `foreach`)
        - defer inlined properties & expressions to `process properties` step, except for `id`s
    - semantize: map node keys to known rendernode types
    - fails upon malformed tree or unknown node types
1. Process properties
    - link properties to nodes of render tree
    - first parse "stylesheet" properties
    - semantize: map selectors to known template nodes+types, then property keys/values to known node properies + FromString=>values
    - then override with inlined properties from template
    - fails upon type mismatches, empty-set selectors, heterogenous multi-element selectors
2. Process expressions
    - parse & lambda-ize expressions, applying a la properties above
    - return primitive types
    - fails upon return type mismatch, malformed expression
 */
//
//
// pub struct PaxParser<'a> {
//     inner_str: &'a str,
// }
//
// impl<'a> PaxParser<'a> {
//     pub fn new(pax: &str) -> Self {
//         PaxParser {
//             inner_str: pax
//         }
//     }
//     ///Parses `template` of the encapsulated Pax string, returning
//     ///the root node as a Definition entity
//     pub fn parse_template(&self) -> TemplateNodeDefinition {
//         self.inner_str
//     }
// }


fn visit_template_tag_pair(pair: Pair<Rule>)  { // -> TemplateNodeDefinition
    //TODO: determine if matched or self-closing
    //      extract
    // match pair.as_rule() {
    //     Rule::matched_tag => {
    //
    //         pair.into_inner().for_each(|matched_tag_pair| {
    //
    //             match matched_tag_pair.as_rule() {
    //                 Rule::open_tag => {
    //                     //register this tag in manifest
    //                 },
    //                 Rule::inner_nodes => {
    //                     //recursively visit template tag pair, passing/returning manifest
    //                     visit_template_tag_pair(matched_tag_pair);
    //                 },
    //                 Rule::statement_control_flow => {
    //                     //will need to support expressions (-> bool, -> iter)
    //                     unimplemented!("control flow support not yet implemented in parser");
    //                 },
    //                 _ => {},
    //             }
    //         })
    //     },
    //     Rule::self_closing_tag => {
    //         pair.into_inner()
    //
    //     },
    //     _ => {
    //         unreachable!();
    //     }
    // }
}


//TODO: should we process in chunks of `files` or `components`?
//      for now they're enforced to be the same thing (at least due to
//      the magic resolution of foo.pax from foo.rs, which admittedly could be changed.)
//
//
// pub fn parse_pax_for_template(pax: &str) {//-> TemplateNodeDefinition {
//
//     let pax_file = PaxParser::parse(Rule::pax_file, pax)
//         .expect("unsuccessful parse") // unwrap the parse result
//         .next().unwrap(); // get and unwrap the `file` rule; never fails
//
//     let x = pax_file.into_inner();
//     x.for_each(|pair|{
//         match pair.as_rule() {
//             Rule::root_tag_pair => {
//                 println!("root tag inner: {:?}", pair.into_inner());
//             }
//             _ => {}
//         }
//     });
//
//
//
//
//     // parsed.
//
//     // unimplemented!()
//     // TemplateNodeDefinition {
//     //     id:
//     // }
// }
//


pub fn parse_pascal_identifiers_from_pax_file(pax: &str) -> Vec<String> {
    // let mut ret = vec![];

    let pax_file = PaxParser::parse(Rule::pax_file, pax)
        .expect("unsuccessful parse") // unwrap the parse result
        .next().unwrap(); // get and unwrap the `pax_file` rule

    println!("{:?}", pax_file);

    let mut pascal_identifiers = Rc::new(RefCell::new(HashSet::new()));

    pax_file.into_inner().for_each(|pair|{
        match pair.as_rule() {
            Rule::root_tag_pair => {
                recurse_visit_tag_pairs_for_pascal_identifiers(
                    pair.into_inner().next().unwrap(),
                    Rc::clone(&pascal_identifiers)
                );
            }
            _ => {}
        }
    });
    let unwrapped_hashmap = Rc::try_unwrap(pascal_identifiers).unwrap().into_inner();
    unwrapped_hashmap.into_iter().collect()
}

fn recurse_visit_tag_pairs_for_pascal_identifiers(any_tag_pair: Pair<Rule>, pascal_identifiers: Rc<RefCell<HashSet<String>>>)  {
    match any_tag_pair.as_rule() {
        Rule::matched_tag => {
            //matched_tag => open_tag > pascal_identifier
            let matched_tag = any_tag_pair;
            let open_tag = matched_tag.clone().into_inner().next().unwrap();
            let pascal_identifier = open_tag.into_inner().next().unwrap().as_str();
            pascal_identifiers.borrow_mut().insert(pascal_identifier.to_string());

            //recurse into inner_nodes
            let prospective_inner_nodes = matched_tag.into_inner().nth(1).unwrap();
            match prospective_inner_nodes.as_rule() {
                Rule::inner_nodes => {
                    let inner_nodes = prospective_inner_nodes;
                    inner_nodes.into_inner()
                        .for_each(|sub_tag_pair|{
                            match sub_tag_pair.as_rule() {
                                Rule::matched_tag | Rule::self_closing_tag => {
                                    //it's another tag — time to recurse
                                    recurse_visit_tag_pairs_for_pascal_identifiers(sub_tag_pair, Rc::clone(&pascal_identifiers));
                                },
                                Rule::statement_control_flow => {
                                    unimplemented!("Control flow not yet supported");
                                },
                                _ => {unreachable!()},
                            }
                        }
                    )
                },
                Rule::closing_tag => {},
                _ => {panic!("wrong .nth")}
            }
        },
        Rule::self_closing_tag => {
            //pascal_identifier
            let pascal_identifier = any_tag_pair.into_inner().next().unwrap().as_str();
            pascal_identifiers.borrow_mut().insert(pascal_identifier.to_string());
        },
        _ => {unreachable!()}
    }
}

fn parse_template_from_pax_file(pax: &str, symbol_name: &str) -> Vec<TemplateNodeDefinition> {


    vec![]
}


fn parse_settings_from_pax_file(pax: &str) -> Option<Vec<SettingsDefinition>> {

    None
}

pub fn get_uuid() -> String {
    Uuid::new_v4().to_string()
}



pub struct ManifestContext {
    /// Used to track which files/sources have been visited during parsing,
    /// to prevent duplicate parsing
    pub visited_source_ids: HashSet<String>,
    pub component_definitions: Vec<ComponentDefinition>,

}


//TODO: support fragments of pax that ARE NOT pax_file (e.g. inline expressions)
pub fn parse_component_from_pax_file(pax: &str, symbol_name: &str, is_root: bool) -> ComponentDefinition {

    let ast = PaxParser::parse(Rule::pax_file, pax)
        .expect("unsuccessful parse") // unwrap the parse result
        .next().unwrap(); // get and unwrap the `pax_file` rule

    let new_id = Uuid::new_v4().to_string();
    //
    // if is_root {
    //     todo!(pack this ID into the manifest as root_component_id)
    // }

    let mut ret = ComponentDefinition {
        id: new_id,
        name: symbol_name.to_string(),
        template: parse_template_from_pax_file(pax, symbol_name),
        settings: parse_settings_from_pax_file(pax),
    };

    // TODO:
    //     from pax-compiler, start process: `TCP_CALLBACK_PORT=22520 cargo run parser --features="parser"`
    //     THEN from inside the parser binary: parse entire project starting with "lib.pax"
    //     THEN phone home the manifest to pax-compiler via the provided TCP port

    //TODO:
    //     how do we latch onto in-file dependencies?
    //     one approach (the only non-static-analysis approach?) is to code-gen the expected dep (RIL)
    //     e.g. for `<Repeat>` => `Repeat {}`
    //   SO, in the case where we're code-genning the
    //


    //recommended piping into `less` or similar
    print!("{:#?}", ast);

    unimplemented!();

}

//
// enum JSONValue<'a> {
//     Object(Vec<(&'a str, JSONValue<'a>)>),
//     Array(Vec<JSONValue<'a>>),
//     String(&'a str),
//     Number(f64),
//     Boolean(bool),
//     Null,
// }
//
// fn serialize_jsonvalue(val: &JSONValue) -> String {
//     use JSONValue::*;
//
//     match val {
//         Object(o) => {
//             let contents: Vec<_> = o
//                 .iter()
//                 .map(|(name, value)|
//                      format!("\"{}\":{}", name, serialize_jsonvalue(value)))
//                 .collect();
//             format!("{{{}}}", contents.join(","))
//         }
//         Array(a) => {
//             let contents: Vec<_> = a.iter().map(serialize_jsonvalue).collect();
//             format!("[{}]", contents.join(","))
//         }
//         String(s) => format!("\"{}\"", s),
//         Number(n) => format!("{}", n),
//         Boolean(b) => format!("{}", b),
//         Null => format!("null"),
//     }
// }
//
//
//
// fn parse_json_file(file: &str) -> Result<JSONValue, Error<Rule>> {
//     let json = JSONParser::parse(Rule::json, file)?.next().unwrap();
//
//     use pest::iterators::Pair;
//
//     fn parse_value(pair: Pair<Rule>) -> JSONValue {
//         match pair.as_rule() {
//             Rule::object => JSONValue::Object(
//                 pair.into_inner()
//                     .map(|pair| {
//                         let mut inner_rules = pair.into_inner();
//                         let name = inner_rules
//                             .next()
//                             .unwrap()
//                             .into_inner()
//                             .next()
//                             .unwrap()
//                             .as_str();
//                         let value = parse_value(inner_rules.next().unwrap());
//                         (name, value)
//                     })
//                     .collect(),
//             ),
//             Rule::array => JSONValue::Array(pair.into_inner().map(parse_value).collect()),
//             Rule::string => JSONValue::String(pair.into_inner().next().unwrap().as_str()),
//             Rule::number => JSONValue::Number(pair.as_str().parse().unwrap()),
//             Rule::boolean => JSONValue::Boolean(pair.as_str().parse().unwrap()),
//             Rule::null => JSONValue::Null,
//             Rule::json
//             | Rule::EOI
//             | Rule::pair
//             | Rule::value
//             | Rule::inner
//             | Rule::char
//             | Rule::WHITESPACE => unreachable!(),
//         }
//     }
//
//     Ok(parse_value(json))
// }