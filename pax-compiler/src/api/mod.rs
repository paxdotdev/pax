
extern crate pest;
use pest_derive::Parser;
use pest::Parser;

use std::{env, fs};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use pest::iterators::{Pair, Pairs};
use uuid::Uuid;

pub mod message;
pub use message::*;

pub use lazy_static::lazy_static;

#[derive(Parser)]
#[grammar = "pax.pest"]
pub struct PaxParser;

/*
COMPILATION STAGES

0. Macro codegen
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
//     let pax_component_definition = PaxParser::parse(Rule::pax_component_definition, pax)
//         .expect("unsuccessful parse") // unwrap the parse result
//         .next().unwrap(); // get and unwrap the `file` rule; never fails
//
//     let x = pax_component_definition.into_inner();
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
        source_id: source_id.to_string(),
        pascal_identifier: pascal_identifier.to_string(),
        template: None,
        settings: None,
        root_template_node_id: None,
        module_path: module_path.to_string(),
    }
}


pub enum PaxContents {
    FilePath(String),
    Inline(String),
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

    let (ctx, comp_def) = parse_full_component_definition_string(ctx, &pax, pascal_identifier, true, template_map, source_id, module_path);
    (ctx, comp_def)
}


pub fn parse_pascal_identifiers_from_component_definition_string(pax: &str) -> Vec<String> {
    let pax_component_definition = PaxParser::parse(Rule::pax_component_definition, pax)
        .expect("unsuccessful parse") // unwrap the parse result
        .next().unwrap(); // get and unwrap the `pax_component_definition` rule

    let pascal_identifiers = Rc::new(RefCell::new(HashSet::new()));

    pax_component_definition.into_inner().for_each(|pair|{
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
                                    //for enumerating our pascal_identifiers, we can just ignore (traverse into desc. of) control-flow
                                    sub_tag_pair.into_inner().for_each(|pair|{
                                        recurse_visit_tag_pairs_for_pascal_identifiers(pair, Rc::clone(&pascal_identifiers));
                                    });
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

fn parse_template_from_component_definition_string(ctx: &mut TemplateParseContext, pax: &str)  {
    let pax_component_definition = PaxParser::parse(Rule::pax_component_definition, pax)
        .expect("unsuccessful parse") // unwrap the parse result
        .next().unwrap(); // get and unwrap the `pax_component_definition` rule

    pax_component_definition.into_inner().for_each(|pair|{
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
            let mut open_tag = matched_tag.clone().into_inner().next().unwrap().into_inner();
            let pascal_identifier = open_tag.next().unwrap().as_str();

            let new_id = create_uuid();
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
                            recurse_visit_tag_pairs_for_template(ctx, sub_tag_pair);
                        })
                },
                Rule::closing_tag => {},
                _ => {panic!("wrong .nth")}
            }

            let template_node = TemplateNodeDefinition {
                id: new_id,
                component_id: ctx.pascal_identifier_to_component_id_map.get(pascal_identifier).expect(&format!("Template key not found {}", &pascal_identifier)).to_string(),
                inline_attributes: parse_inline_attribute_from_final_pairs_of_tag(open_tag),
                children_ids: ctx.children_id_tracking_stack.pop().unwrap(),
            };
            ctx.template_node_definitions.push(template_node);

        },
        Rule::self_closing_tag => {
            let mut tag_pairs = any_tag_pair.into_inner();
            let pascal_identifier = tag_pairs.next().unwrap().as_str();
            let new_id = create_uuid();
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
                component_id: ctx.pascal_identifier_to_component_id_map.get(pascal_identifier).expect(&format!("Template key not found {}", &pascal_identifier)).to_string(),
                inline_attributes: parse_inline_attribute_from_final_pairs_of_tag(tag_pairs),
                children_ids: vec![]
            };
            ctx.template_node_definitions.push(template_node);
        },
        Rule::statement_control_flow => {

            //statement_control_flow => statement variant
            let matched_tag = any_tag_pair.into_inner().next().unwrap();

            match matched_tag.as_rule() {
                Rule::statement_if => {

                },
                Rule::statement_for => {
                    println!("Found for!");
                },
                Rule::statement_slot => {

                },
                _ => {
                    unreachable!();
                }
            };
            //
            // let mut open_tag = matched_tag.clone().into_inner().next().unwrap().into_inner();
            // let pascal_identifier = open_tag.next().unwrap().as_str();
            //
            //
            // let new_id = create_uuid();
            // if ctx.is_root {
            //     ctx.root_template_node_id = Some(new_id.clone());
            // }
            // ctx.is_root = false;
            //
            // //add self to parent's children_id_list
            // let mut parents_children_id_list = ctx.children_id_tracking_stack.pop().unwrap();
            // parents_children_id_list.push(new_id.clone());
            // ctx.children_id_tracking_stack.push(parents_children_id_list);
            //
            // //push the frame for this node's children
            // ctx.children_id_tracking_stack.push(vec![]);
            //
            // //recurse into inner_nodes
            // let prospective_inner_nodes = matched_tag.into_inner().nth(1).unwrap();
            // match prospective_inner_nodes.as_rule() {
            //     Rule::inner_nodes => {
            //         let inner_nodes = prospective_inner_nodes;
            //         inner_nodes.into_inner()
            //             .for_each(|sub_tag_pair|{
            //                 match sub_tag_pair.as_rule() {
            //                     Rule::matched_tag | Rule::self_closing_tag | Rule::statement_control_flow => {
            //                         //it's another tag — time to recurse
            //                         recurse_visit_tag_pairs_for_template(ctx, sub_tag_pair);
            //                     },
            //                     // Rule::statement_control_flow => {
            //                     //     //Control flow is also handled as basic TemplateNodeDefinitions,
            //                     //     //maybe with extra handling around expressions/bindings
            //                     //
            //                     //     unimplemented!("Control flow not yet supported");
            //                     // },
            //                     _ => {unreachable!()},
            //                 }
            //             }
            //             )
            //     },
            //     Rule::closing_tag => {},
            //     _ => {panic!("wrong .nth")}
            // }
            //
            // let template_node = TemplateNodeDefinition {
            //     id: new_id,
            //     component_id: ctx.pascal_identifier_to_component_id_map.get(pascal_identifier).expect("Template key not found").to_string(),
            //     inline_attributes: parse_inline_attribute_from_final_pairs_of_tag(open_tag),
            //     children_ids: ctx.children_id_tracking_stack.pop().unwrap(),
            // };
            // ctx.template_node_definitions.push(template_node);
            //
            //
            //
            // todo!("support control flow mapping into TemplateNodeDefinitions");
        }
        _ => {unreachable!()}
    }
}


fn parse_inline_attribute_from_final_pairs_of_tag ( final_pairs_of_tag: Pairs<Rule>) -> Option<Vec<(String, AttributeValueDefinition)>> {
    let vec : Vec<(String, AttributeValueDefinition)> = final_pairs_of_tag.map(|attribute_key_value_pair|{
        let mut kv = attribute_key_value_pair.into_inner();
        //TODO: handle expression
        let key = kv.next().unwrap().as_str().to_string();
        let mut raw_value = kv.next().unwrap().into_inner().next().unwrap();
        let value = match raw_value.as_rule() {
            Rule::string => {AttributeValueDefinition::String(raw_value.as_str().to_string())},
            Rule::expression => {AttributeValueDefinition::Expression(raw_value.as_str().to_string())},
            _ => {unreachable!()}
        };
        (key, value)
    }).collect();

    if vec.len() > 0 {
        Some(vec)
    }else{
        None
    }
}

fn handle_number_string(num_string: &str) -> Number {
    //for now, maybe naively, treat as float IFF there's a `.` in its string representation
    if num_string.contains(".") {
        Number::Float(num_string.parse::<f64>().unwrap())
    } else {
        Number::Int(num_string.parse::<isize>().unwrap())
    }
}

fn handle_unit_string(unit_string: &str) -> Unit {
    match unit_string {
        "px" => Unit::Pixels,
        "%" => Unit::Percent,
        _ => {unimplemented!("Only px or % are currently supported as units")}
    }
}

fn derive_literal_value_from_pair(literal_value_pair: Pair<Rule>) -> SettingsLiteralValue {
    //literal_value = { literal_number_with_unit | literal_number | literal_array | string }
    let inner_literal = literal_value_pair.into_inner().next().unwrap();
    match inner_literal.as_rule() {
        Rule::literal_number_with_unit => {
            let mut tokens = inner_literal.into_inner();
            let num_str = tokens.next().unwrap().as_str();
            let unit_str = tokens.next().unwrap().as_str();
            SettingsLiteralValue::LiteralNumberWithUnit(handle_number_string(num_str), handle_unit_string(unit_str))
        },
        Rule::literal_number => {
            let num_str = inner_literal.as_str();
            SettingsLiteralValue::LiteralNumber(handle_number_string(num_str))
        },
        Rule::literal_array => {
            unimplemented!("literal arrays aren't supported but should be. fix me!")
            //in particular, need to handle heterogenous types (i.e. not allow them)
            //might just be able to rely on rustc to enforce
        },
        Rule::string => {
            SettingsLiteralValue::String(inner_literal.as_str().to_string())
        },
        _ => {unimplemented!()}
    }
}

fn derive_settings_value_definition_from_literal_object_pair(mut literal_object: Pair<Rule>) -> SettingsLiteralBlockDefinition {
    let mut literal_object_pairs = literal_object.into_inner();

    let explicit_type_pascal_identifier = match literal_object_pairs.peek().unwrap().as_rule() {
        Rule::pascal_identifier => {
            Some(literal_object_pairs.next().unwrap().as_str().to_string())
        },
        _ => { None }
    };

    SettingsLiteralBlockDefinition {
        explicit_type_pascal_identifier,
        settings_key_value_pairs: literal_object_pairs.map(|settings_key_value_pair| {
            let mut pairs = settings_key_value_pair.into_inner();
            let setting_key = pairs.next().unwrap().as_str().to_string();
            let raw_value = pairs.next().unwrap().into_inner().next().unwrap();
            let setting_value = match raw_value.as_rule() {
                Rule::literal_value => { SettingsValueDefinition::Literal(derive_literal_value_from_pair(raw_value))},
                Rule::literal_object => { SettingsValueDefinition::Block(
                    //Recurse
                    derive_settings_value_definition_from_literal_object_pair(raw_value)
                )},
                // Rule::literal_enum_value => {SettingsValueDefinition::Enum(raw_value.as_str().to_string())},
                Rule::expression => { SettingsValueDefinition::Expression(raw_value.as_str().to_string())},
                _ => {unreachable!()}
            };

            (setting_key, setting_value)

        }).collect()
    }
}

fn parse_settings_from_component_definition_string(pax: &str) -> Option<Vec<SettingsSelectorBlockDefinition>> {

    let pax_component_definition = PaxParser::parse(Rule::pax_component_definition, pax)
        .expect("unsuccessful parse") // unwrap the parse result
        .next().unwrap(); // get and unwrap the `pax_component_definition` rule

    let mut ret : Vec<SettingsSelectorBlockDefinition> = vec![];

    pax_component_definition.into_inner().for_each(|top_level_pair|{
        match top_level_pair.as_rule() {
            Rule::settings_block_declaration => {


                let mut selector_block_definitions: Vec<SettingsSelectorBlockDefinition> = top_level_pair.into_inner().map(|selector_block| {
                    //selector_block => settings_key_value_pair where v is a SettingsValueDefinition
                    let mut selector_block_pairs = selector_block.into_inner();
                    //first pair is the selector itself
                    let selector = selector_block_pairs.next().unwrap().to_string();
                    let mut literal_object = selector_block_pairs.next().unwrap();

                    SettingsSelectorBlockDefinition {
                        selector,
                        value_block: derive_settings_value_definition_from_literal_object_pair(literal_object),
                    }

                }).collect();

                ret.extend(selector_block_definitions);

            }
            _ => {}
        }

    });
    Some(ret)

}

pub fn create_uuid() -> String {
    Uuid::new_v4().to_string()
}


pub struct ManifestContext {
    /// Used to track which files/sources have been visited during parsing,
    /// to prevent duplicate parsing
    pub visited_source_ids: HashSet<String>,

    pub root_component_id: String,

    pub component_definitions: Vec<ComponentDefinition>,

    pub template_map: HashMap<String, String>,
}

//TODO: support fragments of pax that ARE NOT pax_component_definition (e.g. inline expressions)
pub fn parse_full_component_definition_string(mut ctx: ManifestContext, pax: &str, pascal_identifier: &str, is_root: bool, template_map: HashMap<String, String>, source_id: &str, module_path: &str) -> (ManifestContext, ComponentDefinition) {

    let ast = PaxParser::parse(Rule::pax_component_definition, pax)
        .expect("unsuccessful parse") // unwrap the parse result
        .next().unwrap(); // get and unwrap the `pax_component_definition` rule

    println!("ast: {}", ast);

    if is_root {
        ctx.root_component_id = source_id.to_string();
    }

    let mut tpc = TemplateParseContext {
        pascal_identifier_to_component_id_map: template_map,
        template_node_definitions: vec![],
        root_template_node_id: None,
        is_root: true,
        //each frame of the outer vec represents a list of
        //children for a given node; child order matters because of z-index defaults;
        //a new frame is added when descending the tree
        //but not when iterating over siblings
        children_id_tracking_stack: vec![],
    };

    parse_template_from_component_definition_string(&mut tpc, pax);

    let mut new_def = ComponentDefinition {
        source_id: source_id.into(),
        pascal_identifier: pascal_identifier.to_string(),
        template: Some(tpc.template_node_definitions),
        settings: parse_settings_from_component_definition_string(pax),
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









