extern crate pest;
#[macro_use]
extern crate pest_derive;

#[macro_use]
extern crate lazy_static;

use std::fs;


use pest::Parser;
use pest::prec_climber::PrecClimber;
use pest::error::Error;

#[derive(Parser)]
#[grammar = "dash.pest"]
pub struct TemplateParser;


/*

TODO:
    [ ] CLI + args parsing
        [ ] Pass path to HTML file or multiple files, or to a .json config
    [ ] Template parser
    [ ] Properties parser
    [ ] Expression parser

*/
//
// lazy_static! {
//     static ref PREC_CLIMBER: PrecClimber<Rule> = {
//         use Rule::*;
//         use Assoc::*;
//
//         PrecClimber::new(vec![
//             Operator::new(add, Left) | Operator::new(subtract, Left),
//             Operator::new(multiply, Left) | Operator::new(divide, Left),
//             Operator::new(power, Right)
//         ])
//     };
// }


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

fn main() {

    let unparsed_file = fs::read_to_string("dev-samples/basic-plus.dash").expect("cannot read file");

    let file /*: pest::iterators::Pair<>*/ = TemplateParser::parse(Rule::dash_file, &unparsed_file)
        .expect("unsuccessful parse") // unwrap the parse result
        .next().unwrap(); // get and unwrap the `file` rule; never fails


    print!("{:#?}", file);
    //
    //
    // let unparsed_file = fs::read_to_string("dev-samples/data.json").expect("cannot read file");
    //
    // let json: JSONValue = parse_json_file(&unparsed_file).expect("unsuccessful parse");
    //
    // println!("{}", serialize_jsonvalue(&json));

    //start semantizing
    let render_tree = unimplemented!();

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