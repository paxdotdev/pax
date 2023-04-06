extern crate core;

pub mod manifest;
pub mod reflection;
pub mod templating;
pub mod parsing;
pub mod expressions;

use manifest::PaxManifest;

use std::{fs};
use std::borrow::Borrow;
use std::str::FromStr;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use itertools::Itertools;

use std::os::unix::fs::PermissionsExt;

use include_dir::{Dir, DirEntry, include_dir};
use toml_edit::{Item};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use crate::manifest::{LiteralBlockDefinition, ValueDefinition, ComponentDefinition, EventDefinition, ExpressionSpec, TemplateNodeDefinition, SettingsSelectorBlockDefinition};
use crate::templating::{press_template_codegen_cartridge_component_factory, press_template_codegen_cartridge_render_node_literal, TemplateArgsCodegenCartridgeComponentFactory, TemplateArgsCodegenCartridgeRenderNodeLiteral};

//relative to pax_dir
pub const REEXPORTS_PARTIAL_RS_PATH: &str = "reexports.partial.rs";
/// Returns a sorted and de-duped list of combined_reexports.
fn generate_reexports_partial_rs(pax_dir: &PathBuf, manifest: &PaxManifest) {
    //traverse ComponentDefinitions in manifest
    //gather module_path and PascalIdentifier --
    //  handle `parser` module_path and any sub-paths
    //re-expose module_path::PascalIdentifier underneath `pax_reexports`
    //ensure that this partial.rs file is loaded included under the `pax_app` macro
    let reexport_components: Vec<String> = manifest.components.iter().map(|cd|{
        //e.g.: "some::module::path::SomePascalIdentifier"
        cd.1.module_path.clone() + "::" + &cd.1.pascal_identifier
    }).collect();

    let mut reexport_types : Vec<String> = manifest.components.iter().map(|cd|{
        cd.1.property_definitions.iter().map(|pm|{
            pm.fully_qualified_constituent_types.clone()
        }).flatten().collect::<Vec<_>>()
    }).flatten().collect::<Vec<_>>();

    let mut combined_reexports = reexport_components;
    combined_reexports.append(&mut reexport_types);
    combined_reexports.sort();

    let mut file_contents = "pub mod pax_reexports { \n".to_string();

    //Make combined_reexports unique by pouring into a Set and back
    let set: HashSet<_> = combined_reexports.drain(..).collect();
    combined_reexports.extend(set.into_iter());
    combined_reexports.sort();

    file_contents += &bundle_reexports_into_namespace_string(&combined_reexports);

    file_contents += "}";

    let path = pax_dir.join(Path::new(REEXPORTS_PARTIAL_RS_PATH));
    fs::write(path, file_contents).unwrap();
}

fn bundle_reexports_into_namespace_string(sorted_reexports: &Vec<String>) -> String {

    //0. sort (expected to be passed sorted)
    //1. keep transient stack of nested namespaces.  For each export string (like pax::api::Size)
    //   - Split by "::"
    //   - if no `::`, or `crate::SingleDepth`, export at root of `pax_reexports`, i.e. empty stack
    //   - if `::`,
    //      - push onto stack the first n-1 identifiers as namespace
    //        - when pushing onto stack, write a `pub mod _identifier_ {`
    //      - when last element is reached, write a `pub use _identifier_;`
    //      - keep track of previous or next element, pop from stack for each of `n` mismatched prefix tokens
    //        - when popping from stack, write a `}`
    //        - empty stack entirely at end of vec

    //Author's note: if this logic must be refactored significantly, consider building a tree data structure & serializing it, instead
    //      of doubling down on this sorted/iterator/stack approach.  This ended up fairly arcane and brittle. -zb

    let mut namespace_stack = vec![];
    let mut output_string = "".to_string();

    //identify namespaceless or prelude-qualified types, e.g. `f64`
    fn is_reexport_namespaceless(symbols: &Vec<String>) -> bool {
        symbols.len() == 0
    }

    //identify `crate::RootLevel` reexports, e.g. `crate::Foo` but not `crate::bar::Bar`
    fn is_reexport_crate_root(symbols: &Vec<String>) -> bool {
        symbols[0].eq("crate") && symbols.len() == 2
    }

    fn is_reexport_crate_prefixed(symbols: &Vec<String>) -> bool {
        symbols[0].eq("crate")
    }

    fn get_tabs (i: usize) -> String {
        "\t".repeat(i + 1).to_string()
    }

    fn pop_and_write_brace(namespace_stack: &mut Vec<String>, output_string: &mut String){
        namespace_stack.pop();
        output_string.push_str(&*(get_tabs(namespace_stack.len()) + "}\n"));
    }

    fn dump_stack(namespace_stack: &mut Vec<String>, output_string: &mut String)  {
        while namespace_stack.len() > 0 {
            pop_and_write_brace(namespace_stack, output_string);
        }
    }

    sorted_reexports.iter().enumerate().for_each(|(i,pub_use)| {

        let symbols: Vec<String> = pub_use.split("::").map(|s|{s.to_string()}).collect();

        if is_reexport_namespaceless(&symbols) || is_reexport_crate_root(&symbols) {
            //we can assume we're already at the root of the stack, thanks to the look-ahead stack-popping logic.
            assert!(namespace_stack.len() == 0);
            output_string += &*(get_tabs(namespace_stack.len()) + "pub use " + pub_use + ";\n");
        } else {
            //push necessary symbols to stack
            let starting_index = namespace_stack.len();
            let skip = if symbols[0] == "crate" {1} else {0};

            for k in skip..((symbols.len() - 1) - namespace_stack.len()) {
                //k represents the offset `k` from `starting_index`, where `k + starting_index`
                //should be retrieved from `symbols` and pushed to `namespace_stack`
                let namespace_symbol = symbols.get(k + starting_index).unwrap().clone();
                output_string += &*(get_tabs(namespace_stack.len()) + "pub mod " + &namespace_symbol + " {\n");
                namespace_stack.push(namespace_symbol);
            }

            output_string += &*(get_tabs(namespace_stack.len()) + "pub use " + pub_use + ";\n");

            //look-ahead and pop stack as necessary
            match sorted_reexports.get(i + 1) {
                Some(next_reexport) => {
                    let next_symbols : Vec<String> = next_reexport.split("::").map(|s|{s.to_string()}).collect();
                    if is_reexport_crate_prefixed(&next_symbols) || is_reexport_namespaceless(&next_symbols) {
                        dump_stack(&mut namespace_stack, &mut output_string);
                    } else {
                        //for the CURRENT first n-1 symbols, check against same position in
                        //new_symbols.
                        //for the first mismatched symbol at i, pop k times, where k = (n-1)-i

                        let mut how_many_pops = None;
                        let n_minus_one = symbols.len() - 1;
                        symbols.iter().take(symbols.len() - 1).enumerate().for_each(|(i,symbol)|{
                            if let None = how_many_pops {
                                if let Some(next_symbol) = next_symbols.get(i) {
                                    if !next_symbol.eq(symbol) {
                                        how_many_pops = Some(n_minus_one - i);
                                    }
                                } else {
                                    how_many_pops = Some(n_minus_one - i);
                                }
                            }
                        });

                        if let Some(pops) = how_many_pops {
                            for _ in skip..pops {
                                pop_and_write_brace(&mut namespace_stack, &mut output_string);
                            }
                        }
                    }
                },
                None => {
                    //we're at the end of the vec — dump stack and write braces
                    dump_stack(&mut namespace_stack, &mut output_string);
                }
            }
        }
    });

    output_string
}

fn update_property_prefixes_in_place(manifest: &mut PaxManifest, host_crate_info: &HostCrateInfo) {
    //update property types in-place
    manifest.components.iter_mut().for_each(|cd| {
        cd.1.property_definitions.iter_mut().for_each(|pm| {
            pm.property_type_info.pascalized_fully_qualified_type = pm.property_type_info.pascalized_fully_qualified_type.replace("{PREFIX}", "__");
            pm.property_type_info.fully_qualified_type = pm.property_type_info.fully_qualified_type.replace("{PREFIX}", &host_crate_info.import_prefix);
        });
    });
}


fn generate_properties_coproduct(pax_dir: &PathBuf, manifest: &PaxManifest, host_crate_info: &HostCrateInfo) {

    let target_dir = pax_dir.join("properties-coproduct");
    clone_properties_coproduct_to_dot_pax(&target_dir).unwrap();

    let target_cargo_full_path = fs::canonicalize(target_dir.join("Cargo.toml")).unwrap();
    let mut target_cargo_toml_contents = toml_edit::Document::from_str(&fs::read_to_string(&target_cargo_full_path).unwrap()).unwrap();


    clean_dependencies_table_of_relative_paths("pax-properties-coproduct", target_cargo_toml_contents["dependencies"].as_table_mut().unwrap(), host_crate_info);

    //insert new entry pointing to userland crate, where `pax_app` is defined
    std::mem::swap(
        target_cargo_toml_contents["dependencies"].get_mut(&host_crate_info.name).unwrap(),
        &mut Item::from_str("{ path=\"../..\" }").unwrap()
    );

    //write patched Cargo.toml
    fs::write(&target_cargo_full_path, &target_cargo_toml_contents.to_string()).unwrap();


    //build tuples for PropertiesCoproduct
    let mut properties_coproduct_tuples : Vec<(String, String)> = manifest.components.iter().map(|comp_def| {
        let mod_path = if &comp_def.1.module_path == "crate" {"".to_string()} else { comp_def.1.module_path.replace("crate::", "") + "::"};
        (
            comp_def.1.pascal_identifier.clone(),
            format!("{}{}{}", &host_crate_info.import_prefix, &mod_path, &comp_def.1.pascal_identifier)
        )
    }).collect();
    let set: HashSet<(String, String)> = properties_coproduct_tuples.drain(..).collect();
    properties_coproduct_tuples.extend(set.into_iter());
    properties_coproduct_tuples.sort();



    //build tuples for TypesCoproduct
    // - include all Property types, representing all possible return types for Expressions
    // - include all T such that T is the iterator type for some Property<Vec<T>>
    let mut types_coproduct_tuples : Vec<(String, String)> = manifest.components.iter().map(|cd|{
        cd.1.property_definitions.iter().map(|pm|{
            (pm.property_type_info.pascalized_fully_qualified_type.clone(),
             pm.property_type_info.fully_qualified_type.clone())
        }).collect::<Vec<_>>()
    }).flatten().collect::<Vec<_>>();

    let mut set: HashSet<_> = types_coproduct_tuples.drain(..).collect();

    #[allow(non_snake_case)]
    let TYPES_COPRODUCT_BUILT_INS = vec![
        ("f64", "f64"),
        ("bool", "bool"),
        ("isize", "isize"),
        ("usize", "usize"),
        ("String", "String"),
        ("Vec_Rc_PropertiesCoproduct___", "std::vec::Vec<std::rc::Rc<PropertiesCoproduct>>"),
        ("Transform2D", "pax_runtime_api::Transform2D"),
        ("Range_isize_", "std::ops::Range<isize>"),
        ("Size2D", "pax_runtime_api::Size2D"),
        ("Size", "pax_runtime_api::Size"),
        ("SizePixels", "pax_runtime_api::SizePixels"),
    ];

    TYPES_COPRODUCT_BUILT_INS.iter().for_each(|builtin| {set.insert((builtin.0.to_string(), builtin.1.to_string()));});
    types_coproduct_tuples.extend(set.into_iter());
    types_coproduct_tuples.sort();

    //press template into String
    let generated_lib_rs = templating::press_template_codegen_properties_coproduct_lib(templating::TemplateArgsCodegenPropertiesCoproductLib {
        properties_coproduct_tuples,
        types_coproduct_tuples,
    });

    //write String to file
    fs::write(target_dir.join("src/lib.rs"), generated_lib_rs).unwrap();

}

fn generate_cartridge_definition(pax_dir: &PathBuf, manifest: &PaxManifest, host_crate_info: &HostCrateInfo) {
    let target_dir = pax_dir.join("cartridge");
    clone_cartridge_to_dot_pax(&target_dir).unwrap();

    let target_cargo_full_path = fs::canonicalize(target_dir.join("Cargo.toml")).unwrap();
    let mut target_cargo_toml_contents = toml_edit::Document::from_str(&fs::read_to_string(&target_cargo_full_path).unwrap()).unwrap();

    clean_dependencies_table_of_relative_paths("pax-cartridge", target_cargo_toml_contents["dependencies"].as_table_mut().unwrap(), host_crate_info);

    //insert new entry pointing to userland crate, where `pax_app` is defined
    std::mem::swap(
        target_cargo_toml_contents["dependencies"].get_mut(&host_crate_info.name).unwrap(),
        &mut Item::from_str("{ path=\"../..\" }").unwrap()
    );

    //write patched Cargo.toml
    fs::write(&target_cargo_full_path, &target_cargo_toml_contents.to_string()).unwrap();

    //Gather all fully_qualified_constituent_types from manifest; prepend with re-export prefix; make unique
    #[allow(non_snake_case)]
    let IMPORT_PREFIX = format!("{}::pax_reexports::", host_crate_info.identifier);
    let mut imports : Vec<String> = manifest.components.values().map(|comp_def: &ComponentDefinition|{
        comp_def.property_definitions.iter().map(|prop_def|{
            prop_def.fully_qualified_constituent_types.iter().map(|fqct|{
                IMPORT_PREFIX.clone() + fqct
            }).collect::<Vec<String>>()
        }).flatten().collect::<Vec<String>>()
    }).flatten().collect::<Vec<String>>();
    let unique_imports: HashSet<String> = imports.drain(..).collect();
    imports.extend(unique_imports.into_iter().sorted());

    //Also add component property structs to the imports list, with same reexports prefix
    let properties_structs_imports : Vec<String> = manifest.components.values().map(|comp_def: &ComponentDefinition|{
        let module_path = if comp_def.module_path == "crate" {
            "".to_string()
        } else {
            comp_def.module_path.replace("crate::", "") + "::"
        };
        format!("{}{}{}", &IMPORT_PREFIX, &module_path, comp_def.pascal_identifier)
    }).collect::<Vec<String>>();
    imports.extend(properties_structs_imports.into_iter().sorted());

    let consts = vec![];//TODO!

    //Traverse component tree starting at root
    //build a N/PIT in memory for each component (maybe this can be automatically serialized for component factories?)
    // handle each kind of attribute:
    //   Literal(String),
    //      inline into N/PIT
    //   Expression(String),
    //      pencil in the ID; handle the expression separately (build ExpressionSpec & friends)
    //   Identifier(String),
    //      syntactic sugar for an expression with a single dependency, returning that dependency
    //   EventBindingTarget(String),
    //      ensure this gets added to the HandlerRegistry for this component; rely on ugly error messages for now
    //
    // for serialization to RIL, generate InstantiationArgs for each node, special-casing built-ins like Repeat, Slot
    //
    // Also decide whether to join settings blocks in this work
    //
    // Compile expressions during traversal, keeping track of "compile-time stack" for symbol resolution
    //   If `const` is bit off for this work, must first populate symbols via pax_const => PaxManifest
    //     -- must also choose scoping rules; probably just component-level scoping for now
    //
    // Throw errors when symbols in expressions cannot be resolved; ensure path forward to developer-friendly error messages
    //     For reference, Rust's message is:
    //  error[E0425]: cannot find value `not_defined` in this scope
    //         --> pax-compiler/src/main.rs:404:13
    //          |
    //      404 |     let y = not_defined + 6;
    //          |             ^^^^^^^^^^^ not found in this scope
    //     Python uses:
    // NameError: name 'z' is not defined
    //     JavaScript uses:
    // Uncaught ReferenceError: not_defined is not defined

    let mut expression_specs : Vec<ExpressionSpec> = manifest.expression_specs.as_ref().unwrap().values().map(|es: &ExpressionSpec|{es.clone()}).collect();
    expression_specs = expression_specs.iter().sorted().cloned().collect();

    let component_factories_literal =  manifest.components.values().into_iter().filter(|cd|{!cd.is_primitive}).map(|cd|{
        generate_cartridge_component_factory_literal(manifest, cd)
    }).collect();

    //press template into String
    let generated_lib_rs = templating::press_template_codegen_cartridge_lib(templating::TemplateArgsCodegenCartridgeLib {
        imports,
        consts,
        expression_specs,
        component_factories_literal,
    });

    //write String to file
    fs::write(target_dir.join("src/lib.rs"), generated_lib_rs).unwrap();

}


fn generate_cartridge_render_nodes_literal(rngc: &RenderNodesGenerationContext) -> String {
    let nodes = rngc.active_component_definition.template.as_ref().expect("tried to generate render nodes literal for component, but template was undefined");

    let implicit_root = nodes[0].borrow();
    let children_literal : Vec<String> = implicit_root.child_ids.iter().map(|child_id|{
    let tnd_map = rngc.active_component_definition.template.as_ref().unwrap();
    let active_tnd = &tnd_map[*child_id];
        recurse_generate_render_nodes_literal(rngc, active_tnd)
    }).collect();

    children_literal.join(",")
}

fn generate_bound_events(inline_settings: Option<Vec<(String, ValueDefinition)>>) -> HashMap<String, String> {
    let mut ret: HashMap<String, String> = HashMap::new();
     if let Some(ref inline) = inline_settings {
        for (key, value) in inline.iter() {
            if let ValueDefinition::EventBindingTarget(s) = value {
                ret.insert(key.clone().to_string(), s.clone().to_string());
            };
        };
    };
    ret
}

fn recurse_generate_render_nodes_literal(rngc: &RenderNodesGenerationContext, tnd: &TemplateNodeDefinition) -> String {
    //first recurse, populating children_literal : Vec<String>
    let children_literal : Vec<String> = tnd.child_ids.iter().map(|child_id|{
        let active_tnd = &rngc.active_component_definition.template.as_ref().unwrap()[*child_id];
        recurse_generate_render_nodes_literal(rngc, active_tnd)
    }).collect();

    const DEFAULT_PROPERTY_LITERAL: &str = "PropertyLiteral::new(Default::default())";

    //pull inline event binding and store into map
    let events = generate_bound_events(tnd.settings.clone());
    let args = if tnd.component_id == parsing::COMPONENT_ID_REPEAT {
        // Repeat
        let rsd = tnd.control_flow_settings.as_ref().unwrap().repeat_source_definition.as_ref().unwrap();
        let id = rsd.vtable_id.unwrap();

        //FUTURE: extend 
        let rse_vec = if let Some(_) = &rsd.symbolic_binding {
            format!("Some(Box::new(PropertyExpression::new({})))", id)
        } else {"None".into()};

        let rse_range = if let Some(_) = &rsd.range_expression_paxel {
            format!("Some(Box::new(PropertyExpression::new({})))", id)
        } else {"None".into()};

        TemplateArgsCodegenCartridgeRenderNodeLiteral {
            is_primitive: true,
            snake_case_component_id: "UNREACHABLE".into(),
            primitive_instance_import_path: Some("RepeatInstance".into()),
            properties_coproduct_variant: "None".to_string(),
            component_properties_struct: "None".to_string(),
            properties: vec![],
            transform_ril: DEFAULT_PROPERTY_LITERAL.to_string(),
            size_ril: [DEFAULT_PROPERTY_LITERAL.to_string(), DEFAULT_PROPERTY_LITERAL.to_string()],
            children_literal,
            slot_index_literal: "None".to_string(),
            conditional_boolean_expression_literal: "None".to_string(),
            active_root: rngc.active_component_definition.pascal_identifier.to_string(),
            events,
            repeat_source_expression_literal_vec: rse_vec,
            repeat_source_expression_literal_range: rse_range,
        }
    } else if tnd.component_id == parsing::COMPONENT_ID_IF {
        // If
        let id = tnd.control_flow_settings.as_ref().unwrap().condition_expression_vtable_id.unwrap();

        TemplateArgsCodegenCartridgeRenderNodeLiteral {
            is_primitive: true,
            snake_case_component_id: "UNREACHABLE".into(),
            primitive_instance_import_path: Some("ConditionalInstance".into()),
            properties_coproduct_variant: "None".to_string(),
            component_properties_struct: "None".to_string(),
            properties: vec![],
            transform_ril: DEFAULT_PROPERTY_LITERAL.to_string(),
            size_ril: [DEFAULT_PROPERTY_LITERAL.to_string(), DEFAULT_PROPERTY_LITERAL.to_string()],
            children_literal,
            slot_index_literal: "None".to_string(),
            repeat_source_expression_literal_vec:  "None".to_string(),
            repeat_source_expression_literal_range:  "None".to_string(),
            conditional_boolean_expression_literal: format!("Some(Box::new(PropertyExpression::new({})))", id),
            active_root: rngc.active_component_definition.pascal_identifier.to_string(),
            events,
        }
    } else if tnd.component_id == parsing::COMPONENT_ID_SLOT {
        // Slot
        let id = tnd.control_flow_settings.as_ref().unwrap().slot_index_expression_vtable_id.unwrap();

        TemplateArgsCodegenCartridgeRenderNodeLiteral {
            is_primitive: true,
            snake_case_component_id: "UNREACHABLE".into(),
            primitive_instance_import_path: Some("ConditionalInstance".into()),
            properties_coproduct_variant: "None".to_string(),
            component_properties_struct: "None".to_string(),
            properties: vec![],
            transform_ril: DEFAULT_PROPERTY_LITERAL.to_string(),
            size_ril: [DEFAULT_PROPERTY_LITERAL.to_string(), DEFAULT_PROPERTY_LITERAL.to_string()],
            children_literal,
            slot_index_literal: format!("Some(Box::new(PropertyExpression::new({})))", id),
            repeat_source_expression_literal_vec:  "None".to_string(),
            repeat_source_expression_literal_range:  "None".to_string(),
            conditional_boolean_expression_literal: "None".to_string(),
            active_root: rngc.active_component_definition.pascal_identifier.to_string(),
            events,
        }
    } else {
        //Handle anything that's not a built-in

        let component_for_current_node = rngc.components.get(&tnd.component_id).unwrap();

        //Properties:
        //  - for each property on cfcn, there will either be:
        //     - an explicit, provided value, or
        //     - an implicit, default value
        //  - an explicit value is present IFF an AttributeValueDefinition
        //    for that property is present on the TemplateNodeDefinition.
        //    That AttributeValueDefinition may be an Expression or Literal (we can throw at this
        //    stage for any `Properties` that are bound to something other than an expression / literal)

        // Tuple of property_id, RIL literal string (e.g. `PropertyLiteral::new(...`_
        let property_ril_tuples: Vec<(String, String)> = component_for_current_node.property_definitions.iter().map(|pd| {
            let ril_literal_string = {
                if let Some(inline_settings) = &tnd.settings {
                    if let Some(matched_setting) = inline_settings.iter().find(|avd| { avd.0 == pd.name }) {
                        match &matched_setting.1 {
                            ValueDefinition::LiteralValue(lv) => {
                                format!("PropertyLiteral::new({})", lv)
                            },
                            ValueDefinition::Expression(_, id) |
                            ValueDefinition::Identifier(_, id) => {
                                format!("PropertyExpression::new({})", id.expect("Tried to use expression but it wasn't compiled"))
                            },
                            _ => {
                                panic!("Incorrect value bound to inline setting")
                            }
                        }
                    } else {
                        DEFAULT_PROPERTY_LITERAL.to_string()
                    }
                } else {
                    //no inline attributes at all; everything will be default
                    DEFAULT_PROPERTY_LITERAL.to_string()
                }
            };

            (pd.name.clone(), ril_literal_string)
        }).collect();

        //handle size: "width" and "height"
        let keys = ["width", "height", "transform"];
        let builtins_ril: Vec<String> = keys.iter().map(|builtin_key| {
            if let Some(inline_settings) = &tnd.settings {
                if let Some(matched_setting) = inline_settings.iter().find(|vd| { vd.0 == *builtin_key }) {
                    match &matched_setting.1 {
                        ValueDefinition::LiteralValue(lv) => {
                            format!("PropertyLiteral::new({})", lv)
                        },
                        ValueDefinition::Expression(_, id) |
                        ValueDefinition::Identifier(_, id) => {
                            format!("PropertyExpression::new({})", id.expect("Tried to use expression but it wasn't compiled"))
                        },
                        _ => {
                            panic!("Incorrect value bound to attribute")
                        }
                    }
                } else {
                    DEFAULT_PROPERTY_LITERAL.to_string()
                }
            } else {
                DEFAULT_PROPERTY_LITERAL.to_string()
            }
        }).collect();


        //then, on the post-order traversal, press template string and return
        TemplateArgsCodegenCartridgeRenderNodeLiteral {
            is_primitive: component_for_current_node.is_primitive,
            snake_case_component_id: component_for_current_node.get_snake_case_id(),
            primitive_instance_import_path: component_for_current_node.primitive_instance_import_path.clone(),
            properties_coproduct_variant: component_for_current_node.pascal_identifier.to_string(),
            component_properties_struct: component_for_current_node.pascal_identifier.to_string(),
            properties: property_ril_tuples,
            transform_ril: builtins_ril[2].clone(),
            size_ril: [builtins_ril[0].clone(), builtins_ril[1].clone()],
            children_literal,
            slot_index_literal: "None".to_string(),
            repeat_source_expression_literal_vec: "None".to_string(),
            repeat_source_expression_literal_range:  "None".to_string(),
            conditional_boolean_expression_literal: "None".to_string(),
            active_root: rngc.active_component_definition.pascal_identifier.to_string(),
            events,
        }
    };

    press_template_codegen_cartridge_render_node_literal(args)
}

struct RenderNodesGenerationContext<'a> {
    components: &'a std::collections::HashMap<String, ComponentDefinition>,
    active_component_definition: &'a ComponentDefinition,
}

fn generate_events_map(events: Option<Vec<EventDefinition>>) -> HashMap<String, Vec<String>> {
    let mut ret = HashMap::new();
    let _ = match events {
        Some(event_list) => {
            for e in event_list.iter(){
                ret.insert(e.key.clone(), e.value.clone());
            }
        },
        _ => {},
    };
    ret
}


fn generate_cartridge_component_factory_literal(manifest: &PaxManifest, cd: &ComponentDefinition) -> String {

    let rngc = RenderNodesGenerationContext {
        components: &manifest.components,
        active_component_definition: cd,
    };

    let args = TemplateArgsCodegenCartridgeComponentFactory {
        is_root: cd.is_root,
        snake_case_component_id: cd.get_snake_case_id(),
        component_properties_struct: cd.pascal_identifier.to_string(),
        properties: cd.property_definitions.clone(),
        events: generate_events_map(cd.events.clone()),
        render_nodes_literal: generate_cartridge_render_nodes_literal(&rngc),
        properties_coproduct_variant: cd.pascal_identifier.to_string()
    };

    press_template_codegen_cartridge_component_factory(args)

}

fn clean_dependencies_table_of_relative_paths(crate_name: &str, dependencies: &mut toml_edit::Table, host_crate_info: &HostCrateInfo) {
    dependencies.iter_mut().for_each(|dep| {

        let dep_name = dep.0.to_string();
        let is_cloned_dep = dep_name.contains("pax-properties-coproduct") || dep_name.contains("pax-cartridge");

        match dep.1.get_mut("path") {
            Some(existing_path) => {
                if  !existing_path.is_none() && !is_cloned_dep && host_crate_info.is_lib_dev_mode {
                    //in "library dev mode," instead of removing relative paths, we want to prepend them with an extra `../'
                    //This allows us to compile `pax-example` against the latest local Pax lib code,
                    //instead of relying on crates.io

                    //Two twists:
                    // 1. because of the extra nesting of Chassis dirs, they require yet an extra prepended "../"
                    //    (this could be made more elegant by flattening `chassis/MacOS` into `chassis-macos`, etc.
                    // 2. because we are copying `pax-properties-coproduct` and `pax-cartridge` from source (rather than referring to the crates at the root of `pax/*`)
                    //    we DO want to remove relative paths for these dependencies

                    let existing_str = existing_path.as_str().unwrap();
                    let mut new_str = "\"../../".to_string() + existing_str + "\"";

                    if crate_name == "pax-chassis" {
                        //add yet another `../`
                        new_str = new_str.replacen("../", "../../", 1);
                    }

                    std::mem::swap(
                        existing_path,
                        &mut Item::from_str(&new_str).unwrap()
                    );
                } else {
                    std::mem::swap(
                        existing_path,
                        &mut Item::None,
                    );
                }
            },
            _ => {}
        }
    });
}

fn generate_chassis(pax_dir: &PathBuf, target: &RunTarget, host_crate_info: &HostCrateInfo, libdevmode: bool) {
    //1. clone (git or raw fs) pax-chassis-whatever into .pax/chassis/
    let chassis_dir = pax_dir.join("chassis");
    std::fs::create_dir_all(&chassis_dir).expect("Failed to create chassis directory.  Check filesystem permissions?");

    let target_str : &str = target.into();
    let relative_chassis_specific_target_dir = chassis_dir.join(target_str);

    clone_target_chassis_to_dot_pax(&relative_chassis_specific_target_dir, target_str, libdevmode).unwrap();

    //2. patch Cargo.toml
    let existing_cargo_toml_path = fs::canonicalize(relative_chassis_specific_target_dir.join("Cargo.toml")).unwrap();
    let mut existing_cargo_toml = toml_edit::Document::from_str(&fs::read_to_string(&existing_cargo_toml_path).unwrap()).unwrap();

    //remove all relative `path` entries from dependencies, so that we may patch.
    clean_dependencies_table_of_relative_paths("pax-chassis", existing_cargo_toml["dependencies"].as_table_mut().unwrap(), host_crate_info);

    //add `patch`
    let mut patch_table = toml_edit::table();
    patch_table["pax-cartridge"]["path"] = toml_edit::value("../../cartridge");
    patch_table["pax-properties-coproduct"]["path"] = toml_edit::value("../../properties-coproduct");
    existing_cargo_toml.insert("patch.crates-io", patch_table);

    //3. write Cargo.toml back to disk & done
    //   hack out the double-quotes inserted by toml_edit along the way
    fs::write(existing_cargo_toml_path, existing_cargo_toml.to_string().replace("\"patch.crates-io\"", "patch.crates-io") ).unwrap();
}

/// Instead of the built-in Dir#extract method, which aborts when a file exists,
/// this implementation will continue extracting, as well as overwrite existing files
fn persistent_extract<S: AsRef<Path>>(dir: &Dir, base_path: S) -> std::io::Result<()> {
    let base_path = base_path.as_ref();

    for entry in dir.entries() {
        let path = base_path.join(entry.path());

        match entry {
            DirEntry::Dir(d) => {
                fs::create_dir_all(&path).ok();
                persistent_extract(d, base_path).ok();
            }
            DirEntry::File(f) => {
                fs::write(path, f.contents()).ok();
            }
        }
    }

    Ok(())
}


/// Simple recursive fs copy function, since std::fs::copy doesn't recurse for us
fn libdev_chassis_copy(src: &PathBuf, dest: &PathBuf) {
    for entry_wrapped in fs::read_dir(src).unwrap() {
        let entry = entry_wrapped.unwrap();
        let file_name = entry.file_name();
        let src_path= &entry.path();
        if entry.file_type().unwrap().is_dir() {
            libdev_chassis_copy(src_path, &dest.join(&file_name));
        } else {
            fs::create_dir_all(dest).ok();
            fs::copy(src_path, dest.join(&file_name)).unwrap();
        }
    }
}


static CHASSIS_MACOS_LIBDEV: &str = "../pax-chassis-macos";
static CHASSIS_MACOS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../pax-chassis-macos");
//NOTE: including this whole pax-chassis-web directory, plus node_modules, adds >100MB to the size of the
//      compiler binary; also extends build times for Web and build times for pax-compiler itself.
//      These are all development dependencies, namely around webpack/typescript -- this could be
//      improved with a "production build" of `pax-chassis-web` that gets included into the compiler
static CHASSIS_WEB_LIBDEV: &str = "../pax-chassis-web";
static CHASSIS_WEB_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../pax-chassis-web");
/// Clone the relevant chassis (and dev harness) to the local .pax directory
/// The chassis is the final compiled Rust library (thus the point where `patch`es must occur)
/// and the encapsulated dev harness is the actual dev executable
fn clone_target_chassis_to_dot_pax(relative_chassis_specific_target_dir: &PathBuf, target_str: &str, libdevmode: bool) -> std::io::Result<()> {

    // fs::remove_dir_all(&relative_chassis_specific_target_dir);
    fs::create_dir_all(&relative_chassis_specific_target_dir).unwrap();

    //Note: zb spent too long tangling with this -- seems like fs::remove* and fs::create* work
    //      only with the relative path, while Dir::extract requires a canonicalized path.  At least: this works on macOS,
    //      and failed silently/partially in all explored configurations until this one
    let chassis_specific_dir = fs::canonicalize(&relative_chassis_specific_target_dir).expect("Invalid path");

    // println!("Cloning {} chassis to {:?}", target_str, chassis_specific_dir);
    match RunTarget::from(target_str) {
        RunTarget::MacOS => {

            if libdevmode {
                // We can assume we're in the pax monorepo — thus we can raw-copy ../pax-chassis-* into .pax,
                // instead of relying on include_dir (which has a very sticky cache and requires constant `cargo clean`ing to clear)
                // This feature allows us to make edits e.g. to @/pax-chassis-macos and rest assured that they are copied into @/pax-example/.pax/chassis/MacOS with every libdev build
                libdev_chassis_copy(&fs::canonicalize(CHASSIS_MACOS_LIBDEV).expect("cannot pass --libdev outside of pax monorepo environment."), &chassis_specific_dir);
            } else {
                persistent_extract(&CHASSIS_MACOS_DIR, &chassis_specific_dir).unwrap();
            }

            // CHASSIS_MACOS_DIR.extract(&chassis_specific_dir).unwrap();
            //HACK: patch the relative directory for the cdylib, because in a rust monorepo the `target` dir
            //      is at the monorepo root, while in this isolated project it will be in `pax-chassis-macos`.
            let pbx_path = &chassis_specific_dir.join("pax-dev-harness-macos").join("pax-dev-harness-macos.xcodeproj").join("project.pbxproj");
            fs::write(pbx_path, fs::read_to_string(pbx_path).unwrap().replace("../../target", "../target")).unwrap();

            //write +x permission to copied run-debuggable-mac-app
            fs::set_permissions(chassis_specific_dir.join("pax-dev-harness-macos").join("run-debuggable-mac-app.sh"), fs::Permissions::from_mode(0o777)).unwrap();
        }
        RunTarget::Web => {
            if libdevmode {
                // We can assume we're in the pax monorepo — thus we can raw-copy ../pax-chassis-* into .pax,
                // instead of relying on include_dir (which has a very sticky cache and requires constant `cargo clean`ing to clear)
                // This feature allows us to make edits e.g. to @/pax-chassis-web and rest assured that they are copied into @/pax-example/.pax/chassis/Web with every libdev build
                libdev_chassis_copy(&fs::canonicalize(CHASSIS_WEB_LIBDEV).expect("cannot pass --libdev outside of pax monorepo environment."), &chassis_specific_dir);
            } else {
                persistent_extract(&CHASSIS_WEB_DIR, &chassis_specific_dir).unwrap();
            }

            //write +x permission to copied run-debuggable-mac-app
            fs::set_permissions(chassis_specific_dir.join("pax-dev-harness-web").join("run-web.sh"), fs::Permissions::from_mode(0o777)).unwrap();
        }
    }
    Ok(())


}

static CARTRIDGE_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../pax-cartridge");
/// Clone the template pax-cartridge directory into .pax, for further codegen
fn clone_cartridge_to_dot_pax(relative_cartridge_target_dir: &PathBuf) -> std::io::Result<()> {
    // fs::remove_dir_all(&relative_cartridge_target_dir);
    fs::create_dir_all(&relative_cartridge_target_dir).unwrap();

    let target_dir = fs::canonicalize(&relative_cartridge_target_dir).expect("Invalid path for generated pax cartridge");

    // println!("Cloning cartridge to {:?}", target_dir);

    persistent_extract(&CARTRIDGE_DIR, &target_dir).unwrap();

    Ok(())
}

static PROPERTIES_COPRODUCT_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../pax-properties-coproduct");
/// Clone a copy of the relevant chassis (and dev harness) to the local .pax directory
/// The chassis is the final compiled Rust library (thus the point where `patch`es must occur)
/// and the encapsulated dev harness is the actual dev executable
fn clone_properties_coproduct_to_dot_pax(relative_cartridge_target_dir: &PathBuf) -> std::io::Result<()> {
    // fs::remove_dir_all(&relative_cartridge_target_dir);
    fs::create_dir_all(&relative_cartridge_target_dir).unwrap();

    let target_dir = fs::canonicalize(&relative_cartridge_target_dir).expect("Invalid path for generated pax cartridge");

    persistent_extract(&PROPERTIES_COPRODUCT_DIR, &target_dir).unwrap();

    Ok(())
}

fn get_or_create_pax_directory(working_dir: &str) -> PathBuf {
    let working_path = std::path::Path::new(working_dir).join(".pax");
    std::fs::create_dir_all( &working_path).unwrap();
    fs::canonicalize(working_path).unwrap()
}

/// Pulled from host Cargo.toml
struct HostCrateInfo {
    /// for example: `pax-example`
    name: String,
    /// for example: `pax_example`
    identifier: String,
    /// for example: `some_crate::pax_reexports`,
    import_prefix: String,
    /// describes whether we're developing inside pax/pax-example, which is
    /// used at least to special-case relative paths for compiled projects
    is_lib_dev_mode: bool,
}

fn get_host_crate_info(cargo_toml_path: &Path) -> HostCrateInfo {
    let existing_cargo_toml = toml_edit::Document::from_str(&fs::read_to_string(
        fs::canonicalize(cargo_toml_path).unwrap()).unwrap()).expect("Error loading host Cargo.toml");

    let name = existing_cargo_toml["package"]["name"].as_str().unwrap().to_string();
    let identifier = name.replace("-", "_"); //NOTE: perhaps this could be less naive?

    let import_prefix = format!("{}::pax_reexports::", &identifier);

    let is_lib_dev_mode = cargo_toml_path.to_str().unwrap().ends_with("pax-example/Cargo.toml");

    HostCrateInfo {
        name,
        identifier,
        import_prefix,
        is_lib_dev_mode,
    }
}

#[allow(unused)]
static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

pub fn perform_clean(path: &str) -> Result<(), ()> {
    let pax_dir = get_or_create_pax_directory(path);
    fs::remove_dir_all(pax_dir).unwrap();
    Ok(())
}

/// Executes a shell command to run the feature-flagged parser at the specified path
/// Returns an output object containing bytestreams of stdout/stderr as well as an exit code
pub fn run_parser_binary(path: &str) -> Output {
    let cargo_run_parser_process = Command::new("cargo")
        .current_dir(path)
        .arg("run")
        .arg("--features")
        .arg("parser")
        .arg("--color")
        .arg("always")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to execute parser binary");

    cargo_run_parser_process.wait_with_output().unwrap()
}


use colored::Colorize;


/// For the specified file path or current working directory, first compile Pax project,
/// then run it with a patched build of the `chassis` appropriate for the specified platform
/// See: pax-compiler-sequence-diagram.png
pub fn perform_build(ctx: &RunContext) -> Result<(), ()> {

    #[allow(non_snake_case)]
    let PAX_BADGE = "[Pax]".bold().on_black().white();

    println!("{} 🛠 Building Rust project with cargo...", &PAX_BADGE);
    let pax_dir = get_or_create_pax_directory(&ctx.path);

    // Run parser bin from host project with `--features parser`
    let output = run_parser_binary(&ctx.path);

    // Forward stderr only
    std::io::stderr().write_all(output.stderr.as_slice()).unwrap();
    assert_eq!(output.status.code().unwrap(), 0, "Parsing failed — there is likely a syntax error in the provided pax");

    let out = String::from_utf8(output.stdout).unwrap();
    let mut manifest : PaxManifest = serde_json::from_str(&out).expect(&format!("Malformed JSON from parser: {}", &out));
    let host_cargo_toml_path = Path::new(&ctx.path).join("Cargo.toml");
    let host_crate_info = get_host_crate_info(&host_cargo_toml_path);
    update_property_prefixes_in_place(&mut manifest, &host_crate_info);

    println!("{} 🧮 Compiling expressions", &PAX_BADGE);
    expressions::compile_all_expressions(&mut manifest);

    println!("{} 🦀 Generating Rust", &PAX_BADGE);
    generate_reexports_partial_rs(&pax_dir, &manifest);
    generate_properties_coproduct(&pax_dir, &manifest, &host_crate_info);
    generate_cartridge_definition(&pax_dir, &manifest, &host_crate_info);
    generate_chassis(&pax_dir, &ctx.target, &host_crate_info, ctx.libdevmode);

    //7. Build the appropriate `chassis` from source, with the patched `Cargo.toml`, Properties Coproduct, and Cartridge from above
    println!("{} 🧱 Building cartridge with cargo", &PAX_BADGE);
    let output = build_chassis_with_cartridge(&pax_dir, &ctx.target);
    //forward stderr only
    std::io::stderr().write_all(output.stderr.as_slice()).unwrap();
    assert_eq!(output.status.code().unwrap(), 0);

    if ctx.should_also_run {
        //8a::run: compile and run dev harness, with freshly built chassis plugged in
        println!("{} 🏃‍ Running fully compiled {} app...", &PAX_BADGE, <&RunTarget as Into<&str>>::into(&ctx.target));

    } else {
        //8b::compile: compile and write executable binary / package to disk at specified or implicit path
        println!("{} 🛠 Building fully compiled {} app...", &PAX_BADGE, <&RunTarget as Into<&str>>::into(&ctx.target));
    }
    build_harness_with_chassis(&pax_dir, &ctx, &Harness::Development);

    Ok(())
}

#[derive(Debug)]
pub enum Harness {
    Development,
}

fn build_harness_with_chassis(pax_dir: &PathBuf, ctx: &RunContext, harness: &Harness) {

    let target_str : &str = ctx.target.borrow().into();
    let target_str_lower: &str = &target_str.to_lowercase();

    let harness_path = pax_dir
        .join("chassis")
        .join({let s : &str = ctx.target.borrow().into(); s})
        .join({
            match harness {
                Harness::Development => {
                    format!("pax-dev-harness-{}", target_str_lower)
                }
            }
        });

    let script = match harness {
        Harness::Development => {
            match ctx.target {
                RunTarget::Web => "./run-web.sh",
                RunTarget::MacOS => "./run-debuggable-mac-app.sh",
            }
        }
    };

    let is_web = if let RunTarget::Web = ctx.target { true } else { false };
    let target_folder : &str = ctx.target.borrow().into();
    let path = fs::canonicalize(std::path::Path::new(&ctx.path)).unwrap();
    let output_path = path.join("build").join(target_folder);
    let output_path_val = output_path.to_str().unwrap();

    let verbose_val = format!("{}",ctx.verbose);
    let exclude_arch_val =  if std::env::consts::ARCH == "aarch64" {
        "x86_64"
    } else {
        "arm64"
    };
    let should_also_run = &format!("{}",ctx.should_also_run);
    if is_web {
        Command::new(script)
            .current_dir(&harness_path)
            .arg(should_also_run)
            .arg(output_path_val)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .expect("failed to run harness")
            .wait()
            .expect("failed to run harness");
    } else {
        Command::new(script)
            .current_dir(&harness_path)
            .arg(verbose_val)
            .arg(exclude_arch_val)
            .arg(should_also_run)
            .arg(output_path_val)
            .stdout(std::process::Stdio::inherit())
            .stderr(if ctx.verbose { std::process::Stdio::inherit() } else {std::process::Stdio::piped()})
            .spawn()
            .expect("failed to run harness")
            .wait()
            .expect("failed to run harness");
    }
}

/// Runs `cargo build` (or `wasm-pack build`) with appropriate env in the directory
/// of the generated chassis project inside the specified .pax dir
/// Returns an output object containing bytestreams of stdout/stderr as well as an exit code
pub fn build_chassis_with_cartridge(pax_dir: &PathBuf, target: &RunTarget) -> Output {

    let pax_dir = PathBuf::from(pax_dir.to_str().unwrap());
    let chassis_path = pax_dir.join("chassis").join({let s: & str = target.into(); s});
    //string together a shell call like the following:
    let cargo_run_chassis_build = match target {
        RunTarget::MacOS => {
            Command::new("cargo")
                .current_dir(&chassis_path)
                .arg("build")
                .arg("--color")
                .arg("always")
                .env("PAX_DIR", &pax_dir)
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .spawn()
                .expect("failed to build chassis")
        },
        RunTarget::Web => {
            Command::new("wasm-pack")
                .current_dir(&chassis_path)
                .arg("build")
                .arg("--release")
                .arg("-d")
                .arg(pax_dir.join("chassis").join("Web").join("pax-dev-harness-web").join("dist").to_str().unwrap()) //--release -d pax-dev-harness-web/dist
                .env("PAX_DIR", &pax_dir)
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .spawn()
                .expect("failed to build chassis")
        }
    };

    cargo_run_chassis_build.wait_with_output().unwrap()
}

pub struct RunContext {
    pub target: RunTarget,
    pub path: String,
    pub verbose: bool,
    pub should_also_run: bool,
    pub libdevmode: bool,
}

pub enum RunTarget {
    MacOS,
    Web,
}

impl From<&str> for RunTarget {
    fn from(input: &str) -> Self {
        match input.to_lowercase().as_str() {
            "macos" => {
                RunTarget::MacOS
            },
            "web" => {
                RunTarget::Web
            }
            _ => {unreachable!()}
        }
    }
}

impl<'a> Into<&'a str> for &'a RunTarget {
    fn into(self) -> &'a str {
        match self {
            RunTarget::Web => {
                "Web"
            },
            RunTarget::MacOS => {
                "MacOS"
            },
        }
    }
}