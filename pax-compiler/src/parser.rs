#[macro_use]
extern crate pest_derive;


extern crate pest;

// #[macro_use]
// extern crate lazy_static;

use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::{env, fs};
use std::hint::unreachable_unchecked;
use std::path::{Path, PathBuf};
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


pub fn handle_primitive(pascal_identifier: &str, module_path: &str, source_id: &str) -> ComponentDefinition {
    ComponentDefinition {
        id: source_id.to_string(),
        pascal_identifier: pascal_identifier.to_string(),
        template: None,
        settings: None,
        root_template_node_id: None,
        module_path: module_path.to_string(),
    }
}

pub fn handle_file(mut ctx: ManifestContext, file: &str, module_path: &str, explicit_path: Option<String>, pascal_identifier: &str, template_map: HashMap<String, String>, source_id: &str) -> (ManifestContext, ComponentDefinition) {

    let path =
        match explicit_path {
            None => {
                //infer path by current filename, e.g. lib.rs => lib.pax
                let mut inferred_path = PathBuf::from(file);
                inferred_path.set_extension("pax");

                let root = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
                let path = Path::new(&root).join("src/").join(&inferred_path);
                let file_name = match path.file_name() {
                    Some(file_name) => file_name,
                    None => panic!("no pax file found"), //TODO: make error message more helpful, e.g. by suggesting where to create a pax file
                };

                path
            },
            Some(provided_path) => {
                //explicit path (relative to src/) was provided
                let root = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
                let path = Path::new(&root).join("src/").join(&provided_path);
                let file_name = match path.file_name() {
                    Some(file_name) => file_name,
                    None => panic!("pax file not found at specified path"), //TODO: make error message more helpful, e.g. by suggesting the use of `src/`-relative paths
                };

                path
            }
        };

    println!("path: {:?}", path);
    let pax = fs::read_to_string(path).unwrap();

    let (ctx, comp_def) = parse_component_from_pax_file(ctx,&pax, pascal_identifier ,true, template_map, source_id, module_path);
    (ctx, comp_def)
}


pub fn parse_pascal_identifiers_from_pax_file(pax: &str) -> Vec<String> {
    // let mut ret = vec![];

    let pax_file = PaxParser::parse(Rule::pax_file, pax)
        .expect("unsuccessful parse") // unwrap the parse result
        .next().unwrap(); // get and unwrap the `pax_file` rule

    println!("{:?}", pax_file);

    let pascal_identifiers = Rc::new(RefCell::new(HashSet::new()));

    pax_file.into_inner().for_each(|pair|{
        match pair.as_rule() {
            Rule::root_tag_pair => {
                recurse_visit_tag_pairs_for_pascal_identifiers(
                    pair.into_inner().next().unwrap(),
                    Rc::clone(&pascal_identifiers),
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

fn parse_template_from_pax_file(ctx: &mut TemplateParseContext,pax: &str)  {
    let pax_file = PaxParser::parse(Rule::pax_file, pax)
        .expect("unsuccessful parse") // unwrap the parse result
        .next().unwrap(); // get and unwrap the `pax_file` rule

    pax_file.into_inner().for_each(|pair|{
        match pair.as_rule() {
            Rule::root_tag_pair => {
                ctx.children_id_tracking_stack.push(vec![]);
                recurse_visit_tag_pairs_for_template(
                    ctx,
                    pair.into_inner().next().unwrap(),

                );
            }
            _ => {}
        }
    });

}

struct TemplateParseContext {
    pub template_node_definitions: Vec<TemplateNodeDefinition>,
    pub is_root: bool,
    pub pascal_identifier_to_component_id_map: HashMap<String, String>,
    pub root_template_node_id: Option<String>,
    //each frame of the outer vec represents a list of
    //children for a given node;
    //a new frame is added when descending the tree
    //but not when iterating over siblings
    pub children_id_tracking_stack: Vec<Vec<String>>,
}

fn recurse_visit_tag_pairs_for_template(ctx: &mut TemplateParseContext, any_tag_pair: Pair<Rule>)  {
    match any_tag_pair.as_rule() {
        Rule::matched_tag => {
            //matched_tag => open_tag > pascal_identifier
            let matched_tag = any_tag_pair;
            let open_tag = matched_tag.clone().into_inner().next().unwrap();
            let pascal_identifier = open_tag.into_inner().next().unwrap().as_str();


            let new_id = get_uuid();
            if ctx.is_root {
                ctx.root_template_node_id = Some(new_id.clone());
            }
            ctx.is_root = false;

            //add self to parent's children_id_list
            let mut parents_children_id_list = ctx.children_id_tracking_stack.pop().unwrap();
            parents_children_id_list.push(new_id.clone());
            ctx.children_id_tracking_stack.push(parents_children_id_list);

            //push the frame for this node's children
            ctx.children_id_tracking_stack.push(vec![]);

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
                                    recurse_visit_tag_pairs_for_template(ctx, sub_tag_pair);
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

            let template_node = TemplateNodeDefinition {
                id: new_id,
                component_id: ctx.pascal_identifier_to_component_id_map.get(pascal_identifier).expect("Template key not found").to_string(),
                inline_attributes: None,
                children_ids: ctx.children_id_tracking_stack.pop().unwrap(),
            };
            ctx.template_node_definitions.push(template_node);

        },
        Rule::self_closing_tag => {
            let pascal_identifier = any_tag_pair.into_inner().next().unwrap().as_str();
            let new_id = get_uuid();
            if ctx.is_root {
                ctx.root_template_node_id = Some(new_id.clone());
            }
            ctx.is_root = false;

            //add self to parent's children_id_list
            let mut parents_children_id_list = ctx.children_id_tracking_stack.pop().unwrap();
            parents_children_id_list.push(new_id.clone());
            ctx.children_id_tracking_stack.push(parents_children_id_list);

            let template_node = TemplateNodeDefinition {
                id: new_id,
                component_id: ctx.pascal_identifier_to_component_id_map.get(pascal_identifier).expect("Template key not found").to_string(),
                inline_attributes: None,
                children_ids: vec![]
            };
            ctx.template_node_definitions.push(template_node);
        },
        _ => {unreachable!()}
    }
}


fn parse_settings_from_pax_file(pax: &str) -> Option<Vec<SettingsDefinition>> {
    None
}

pub fn get_uuid() -> String {
    Uuid::new_v4().to_string()
}


#[derive(Debug)]
pub struct ManifestContext {
    /// Used to track which files/sources have been visited during parsing,
    /// to prevent duplicate parsing
    pub root_component_id: String,
    pub visited_source_ids: HashSet<String>,
    pub component_definitions: Vec<ComponentDefinition>,

}

//TODO: support fragments of pax that ARE NOT pax_file (e.g. inline expressions)
pub fn parse_component_from_pax_file(mut ctx: ManifestContext, pax: &str, symbol_name: &str, is_root: bool, template_map: HashMap<String, String>, source_id: &str, module_path: &str) -> (ManifestContext, ComponentDefinition) {

    let ast = PaxParser::parse(Rule::pax_file, pax)
        .expect("unsuccessful parse") // unwrap the parse result
        .next().unwrap(); // get and unwrap the `pax_file` rule

    if is_root {
        ctx.root_component_id = source_id.to_string();
    }

    let mut tpc = TemplateParseContext {
        pascal_identifier_to_component_id_map: template_map,
        template_node_definitions: vec![],
        root_template_node_id: None,
        is_root: true,
        //each frame of the outer vec represents a list of
        //children for a given node;
        //a new frame is added when descending the tree
        //but not when iterating over siblings
        children_id_tracking_stack: vec![],
    };

    parse_template_from_pax_file(&mut tpc, pax);

    let mut new_def = ComponentDefinition {
        id: source_id.into(),
        pascal_identifier: symbol_name.to_string(),
        template: Some(tpc.template_node_definitions),
        settings: parse_settings_from_pax_file(pax),
        module_path: module_path.to_string(),
        root_template_node_id: tpc.root_template_node_id,
    };

    // TODO:
    //     from pax-compiler, start process: `TCP_CALLBACK_PORT=22520 cargo run parser --features="parser"`
    //     THEN from inside the parser binary: parse entire project starting with "lib.pax"
    //     THEN phone home the manifest to pax-compiler via the provided TCP port

    //recommended piping into `less` or similar
    // print!("{:#?}", ast);

    (ctx, new_def)

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