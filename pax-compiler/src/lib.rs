extern crate core;

use lazy_static::lazy_static;
use std::io::Read;
use include_dir::{Dir, include_dir};

pub mod manifest;
pub mod templating;
pub mod parsing;
pub mod expressions;

use manifest::PaxManifest;
use rust_format::{Config, Formatter};

use std::{fs};
use std::any::Any;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::str::FromStr;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use itertools::Itertools;

use std::os::unix::fs::PermissionsExt;

use toml_edit::{Item};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use crate::manifest::{ValueDefinition, ComponentDefinition, EventDefinition, ExpressionSpec, TemplateNodeDefinition, TypeTable, LiteralBlockDefinition, TypeDefinition};
use crate::templating::{press_template_codegen_cartridge_component_factory, press_template_codegen_cartridge_render_node_literal, TemplateArgsCodegenCartridgeComponentFactory, TemplateArgsCodegenCartridgeRenderNodeLiteral};

//relative to pax_dir
pub const REEXPORTS_PARTIAL_RS_PATH: &str = "reexports.partial.rs";

//whitelist of package ids that are relevant to the compiler, e.g. for cloning & patching, for assembling FS paths,
//or for looking up package IDs from a userland Cargo.lock.
const ALL_PKGS: [&'static str; 12] = [
    "pax-cartridge",
    "pax-chassis-macos",
    "pax-chassis-web",
    "pax-cli",
    "pax-compiler",
    "pax-core",
    "pax-lang",
    "pax-macro",
    "pax-message",
    "pax-properties-coproduct",
    "pax-runtime-api",
    "pax-std",
];

/// Returns a sorted and de-duped list of combined_reexports.
fn generate_reexports_partial_rs(pax_dir: &PathBuf, manifest: &PaxManifest) {
    let imports = manifest.import_paths.clone().into_iter().sorted().collect();

    let file_contents = &bundle_reexports_into_namespace_string(&imports);

    let path = pax_dir.join(Path::new(REEXPORTS_PARTIAL_RS_PATH));
    fs::write(path, file_contents).unwrap();
}

fn bundle_reexports_into_namespace_string(sorted_reexports: &Vec<String>) -> String {
    let mut root = NamespaceTrieNode {
        node_string: None,
        children: Default::default(),
    };

    for s in sorted_reexports {
        root.insert(s);
    }

    root.serialize_to_reexports()
}

fn update_property_prefixes_in_place(manifest: &mut PaxManifest, host_crate_info: &HostCrateInfo) {
    let mut updated_type_table = HashMap::new();
    manifest.type_table.iter_mut().for_each(|t|{
        t.1.type_id_escaped = t.1.type_id_escaped.replace("{PREFIX}", "");
        t.1.type_id = t.1.type_id.replace("{PREFIX}", &host_crate_info.import_prefix);
        t.1.property_definitions.iter_mut().for_each(|pd|{
            pd.type_id = pd.type_id.replace("{PREFIX}", &host_crate_info.import_prefix);
        });
        updated_type_table.insert(t.0.replace("{PREFIX}", &host_crate_info.import_prefix), t.1.clone());
    });
    std::mem::swap(&mut manifest.type_table, &mut updated_type_table);
}

// The stable output directory for generated / copied files
const PAX_DIR_PKG_PATH : &str = "pkg";

fn clone_all_dependencies_to_tmp(pax_dir: &PathBuf, pax_version: &str, host_crate_info: &HostCrateInfo) {

    let dest_pkg_root = pax_dir.join(PAX_DIR_PKG_PATH);
    for pkg in ALL_PKGS {

        if host_crate_info.is_lib_dev_mode {
            //Copy all packages from monorepo root on every build.  this allows us to propagate changes
            //to a libdev build without "sticky caches."
            //
            //Note that this may incur a penalty on libdev build times,
            //since cargo will want to rebuild the whole workspace from scratch on every build.  If we want to optimize this,
            //consider a "double buffered" approach, where we copy everything into a fresh new buffer (B), call it `.pax/pkg-tmp`, while leaving (A) `.pax/pkg`
            //unchanged on disk.  Bytewise check each file found in B against a prospective match in A, and copy only if different.  (B) could also be stored on a virtual
            //FS in memory, to reduce disk churn.
            let pax_workspace_root = pax_dir.parent().unwrap().parent().unwrap();
            let src = pax_workspace_root.join(pkg);
            let dest = dest_pkg_root.join(pkg);
            copy_dir_to(&src, &dest).expect(&format!("Failed to copy from {:?} to {:?}", src, dest));
        } else {
            let dest = dest_pkg_root.join(pkg);
            if !dest.exists() {
                let tarball_url = format!("https://crates.io/api/v1/crates/{}/{}/download", pkg, pax_version);
                let resp = reqwest::blocking::get(&tarball_url)
                    .expect(&format!("Failed to fetch tarball for {} at version {}", pkg, pax_version));

                let tarball_bytes = resp.bytes().expect("Failed to read tarball bytes");
                let mut tar = tar::Archive::new(&tarball_bytes[..]);
                tar.unpack(&dest).expect(&format!("Failed to unpack tarball for {}", pkg));
            }
        }

    }
}

fn generate_and_overwrite_properties_coproduct(pax_dir: &PathBuf, manifest: &PaxManifest, host_crate_info: &HostCrateInfo) {

    let target_dir = pax_dir.join(PAX_DIR_PKG_PATH).join("pax-properties-coproduct");

    let target_cargo_full_path = fs::canonicalize(target_dir.join("Cargo.toml")).unwrap();
    let mut target_cargo_toml_contents = toml_edit::Document::from_str(&fs::read_to_string(&target_cargo_full_path).unwrap()).unwrap();

    //insert new entry pointing to userland crate, where `pax_app` is defined
    std::mem::swap(
        target_cargo_toml_contents["dependencies"].get_mut(&host_crate_info.name).unwrap(),
        &mut Item::from_str("{ path=\"../../..\" }").unwrap()
    );

    //write patched Cargo.toml
    fs::write(&target_cargo_full_path, &target_cargo_toml_contents.to_string()).unwrap();

    //build tuples for PropertiesCoproduct
    let mut properties_coproduct_tuples : Vec<(String, String)> = manifest.components.iter().map(|comp_def| {
        let mod_path = if &comp_def.1.module_path == "crate" {"".to_string()} else { comp_def.1.module_path.replace("crate::", "") + "::"};
        (
            comp_def.1.type_id_escaped.clone(),
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
        cd.1.get_property_definitions(&manifest.type_table).iter().map(|pm|{
            let td = pm.get_type_definition(&manifest.type_table);

            (
                td.type_id_escaped.clone(),
                host_crate_info.import_prefix.to_string() + &td.type_id.clone().replace("crate::", "")
            )
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
        ("stdCOCOvecCOCOVecLABRstdCOCOrcCOCORcLABRPropertiesCoproductRABRRABR", "std::vec::Vec<std::rc::Rc<PropertiesCoproduct>>"),
        ("Transform2D", "pax_runtime_api::Transform2D"),
        ("stdCOCOopsCOCORangeLABRisizeRABR", "std::ops::Range<isize>"),
        ("Size2D", "pax_runtime_api::Size2D"),
        ("Size", "pax_runtime_api::Size"),
        ("SizePixels", "pax_runtime_api::SizePixels"),
        ("Numeric", "pax_runtime_api::Numeric"),
    ];

    TYPES_COPRODUCT_BUILT_INS.iter().for_each(|builtin| {set.insert((builtin.0.to_string(), builtin.1.to_string()));});
    types_coproduct_tuples.extend(set.into_iter());
    types_coproduct_tuples.sort();

    types_coproduct_tuples = types_coproduct_tuples.into_iter().unique_by(|elem|{elem.0.to_string()}).collect::<Vec<(String, String)>>();

    //press template into String
    let generated_lib_rs = templating::press_template_codegen_properties_coproduct_lib(templating::TemplateArgsCodegenPropertiesCoproductLib {
        properties_coproduct_tuples,
        types_coproduct_tuples,
    });

    //write String to file
    fs::write(target_dir.join("src/lib.rs"), generated_lib_rs).unwrap();

}

fn generate_and_overwrite_cartridge(pax_dir: &PathBuf, manifest: &PaxManifest, host_crate_info: &HostCrateInfo) {
    let target_dir = pax_dir.join(PAX_DIR_PKG_PATH).join("pax-cartridge");

    let target_cargo_full_path = fs::canonicalize(target_dir.join("Cargo.toml")).unwrap();
    let mut target_cargo_toml_contents = toml_edit::Document::from_str(&fs::read_to_string(&target_cargo_full_path).unwrap()).unwrap();

    //insert new entry pointing to userland crate, where `pax_app` is defined
    std::mem::swap(
        target_cargo_toml_contents["dependencies"].get_mut(&host_crate_info.name).unwrap(),
        &mut Item::from_str("{ path=\"../../..\" }").unwrap()
    );

    //write patched Cargo.toml
    fs::write(&target_cargo_full_path, &target_cargo_toml_contents.to_string()).unwrap();

    const IMPORTS_BUILTINS : [&str; 27] = [
        "std::cell::RefCell",
        "std::collections::HashMap",
        "std::collections::VecDeque",
        "std::ops::Deref",
        "std::rc::Rc",
        "pax_runtime_api::PropertyInstance",
        "pax_runtime_api::PropertyLiteral",
        "pax_runtime_api::Size2D",
        "pax_runtime_api::Transform2D",
        "pax_core::ComponentInstance",
        "pax_core::RenderNodePtr",
        "pax_core::PropertyExpression",
        "pax_core::RenderNodePtrList",
        "pax_core::RenderTreeContext",
        "pax_core::ExpressionContext",
        "pax_core::PaxEngine",
        "pax_core::RenderNode",
        "pax_core::InstanceRegistry",
        "pax_core::HandlerRegistry",
        "pax_core::InstantiationArgs",
        "pax_core::ConditionalInstance",
        "pax_core::SlotInstance",
        "pax_core::StackFrame",
        "pax_core::pax_properties_coproduct::PropertiesCoproduct",
        "pax_core::pax_properties_coproduct::TypesCoproduct",
        "pax_core::repeat::RepeatInstance",
        "piet_common::RenderContext",
    ];

    let imports_builtins_set: HashSet<&str> = IMPORTS_BUILTINS.into_iter().collect();

    #[allow(non_snake_case)]
    let IMPORT_PREFIX = format!("{}::pax_reexports::", host_crate_info.identifier);

    let mut imports : Vec<String> = manifest.import_paths.iter().map(|path|{
        if ! imports_builtins_set.contains(&**path) {
            IMPORT_PREFIX.clone() + &path.replace("crate::", "")
        } else {
            "".to_string()
        }
    }).collect();

    imports.append(&mut IMPORTS_BUILTINS.into_iter().map(|ib|{
        ib.to_string()
    }).collect::<Vec<String>>());

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

    let component_factories_literal =  manifest.components.values().into_iter().filter(|cd|{!cd.is_primitive && !cd.is_struct_only_component}).map(|cd|{
        generate_cartridge_component_factory_literal(manifest, cd, host_crate_info)
    }).collect();

    //press template into String
    let generated_lib_rs = templating::press_template_codegen_cartridge_lib(templating::TemplateArgsCodegenCartridgeLib {
        imports,
        consts,
        expression_specs,
        component_factories_literal,
    });

    // Re: formatting the generated output, see prior art at `_format_generated_lib_rs`
    //write String to file
    fs::write(target_dir.join("src/lib.rs"), generated_lib_rs).unwrap();
}

/// Note: this function was abandoned because RustFmt takes unacceptably long to format complex
/// pax-cartridge/src/lib.rs files.  The net effect was a show-stoppingly slow `pax build`.
/// We can problaby mitigate this by: (a) waiting for or eliciting improvements in RustFmt, or (b) figuring out what about our codegen is slowing RustFmt down, and generate our code differently to side-step.
/// This code is left for posterity in case we take another crack at formatting generated code.
fn _format_generated_lib_rs(generated_lib_rs: String) -> String {
    let mut formatter = rust_format::RustFmt::default();

    if let Ok(out) = formatter.format_str(generated_lib_rs.clone()) {
        out
    } else {
        //if formatting fails (e.g. parsing error, common expected case) then
        //fall back to unformatted generated code
        generated_lib_rs
    }
}

fn generate_cartridge_render_nodes_literal(rngc: &RenderNodesGenerationContext,  host_crate_info: &HostCrateInfo) -> String {
    let nodes = rngc.active_component_definition.template.as_ref().expect("tried to generate render nodes literal for component, but template was undefined");

    let implicit_root = nodes[0].borrow();
    let children_literal : Vec<String> = implicit_root.child_ids.iter().map(|child_id|{
    let tnd_map = rngc.active_component_definition.template.as_ref().unwrap();
    let active_tnd = &tnd_map[*child_id];
        recurse_generate_render_nodes_literal(rngc, active_tnd,  host_crate_info)
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

fn recurse_literal_block(block: LiteralBlockDefinition, type_definition: &TypeDefinition, host_crate_info: &HostCrateInfo) -> String {
    let qualified_path =
        host_crate_info.import_prefix.to_string() + &type_definition.import_path.clone().replace("crate::", "");

    // Buffer to store the string representation of the struct
    let mut struct_representation = format!("\n{{ let mut ret = {}::default();", qualified_path);

    // Iterating through each (key, value) pair in the settings_key_value_pairs
    for (key, value_definition) in block.settings_key_value_pairs.iter() {
        let value_string = match value_definition {
            ValueDefinition::LiteralValue(value) => format!("ret.{} = Box::new(PropertyLiteral::new({}));", key, value),
            ValueDefinition::Expression(_, id) |
            ValueDefinition::Identifier(_, id) => {
                format!("ret.{} = Box::new(PropertyExpression::new({}));", key, id.expect("Tried to use expression but it wasn't compiled"))
            },
            ValueDefinition::Block(inner_block) => format!("ret.{} = Box::new(PropertyLiteral::new({}));", key, recurse_literal_block(inner_block.clone(), type_definition, host_crate_info)),
            _ => {
                panic!("Incorrect value bound to inline setting")
            }
        };

        struct_representation.push_str(&format!("\n{}", value_string));
    }

    struct_representation.push_str("\n ret }");

    struct_representation
}


fn recurse_generate_render_nodes_literal(rngc: &RenderNodesGenerationContext, tnd: &TemplateNodeDefinition,  host_crate_info: &HostCrateInfo) -> String {
    //first recurse, populating children_literal : Vec<String>
    let children_literal : Vec<String> = tnd.child_ids.iter().map(|child_id|{
        let active_tnd = &rngc.active_component_definition.template.as_ref().unwrap()[*child_id];
        recurse_generate_render_nodes_literal(rngc, active_tnd,  host_crate_info)
    }).collect();

    const DEFAULT_PROPERTY_LITERAL: &str = "PropertyLiteral::new(Default::default())";

    //pull inline event binding and store into map
    let events = generate_bound_events(tnd.settings.clone());
    let args = if tnd.type_id == parsing::TYPE_ID_REPEAT {
        // Repeat
        let rsd = tnd.control_flow_settings.as_ref().unwrap().repeat_source_definition.as_ref().unwrap();
        let id = rsd.vtable_id.unwrap();

        let rse_vec = if let Some(_) = &rsd.symbolic_binding {
            format!("Some(Box::new(PropertyExpression::new({})))", id)
        } else {"None".into()};

        let rse_range = if let Some(_) = &rsd.range_expression_paxel {
            format!("Some(Box::new(PropertyExpression::new({})))", id)
        } else {"None".into()};

        TemplateArgsCodegenCartridgeRenderNodeLiteral {
            is_primitive: true,
            snake_case_type_id: "UNREACHABLE".into(),
            primitive_instance_import_path: Some("RepeatInstance".into()),
            properties_coproduct_variant: "None".to_string(),
            component_properties_struct: "None".to_string(),
            properties: vec![],
            transform_ril: DEFAULT_PROPERTY_LITERAL.to_string(),
            size_ril: [DEFAULT_PROPERTY_LITERAL.to_string(), DEFAULT_PROPERTY_LITERAL.to_string()],
            children_literal,
            slot_index_literal: "None".to_string(),
            conditional_boolean_expression_literal: "None".to_string(),
            pascal_identifier: rngc.active_component_definition.pascal_identifier.to_string(),
            type_id_escaped: escape_identifier(rngc.active_component_definition.type_id.to_string()),
            events,
            repeat_source_expression_literal_vec: rse_vec,
            repeat_source_expression_literal_range: rse_range,
        }
    } else if tnd.type_id == parsing::TYPE_ID_IF {
        // If
        let id = tnd.control_flow_settings.as_ref().unwrap().condition_expression_vtable_id.unwrap();

        TemplateArgsCodegenCartridgeRenderNodeLiteral {
            is_primitive: true,
            snake_case_type_id: "UNREACHABLE".into(),
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
            pascal_identifier: rngc.active_component_definition.pascal_identifier.to_string(),
            type_id_escaped: escape_identifier(rngc.active_component_definition.type_id.to_string()),
            events,
        }
    } else if tnd.type_id == parsing::TYPE_ID_SLOT {
        // Slot
        let id = tnd.control_flow_settings.as_ref().unwrap().slot_index_expression_vtable_id.unwrap();

        TemplateArgsCodegenCartridgeRenderNodeLiteral {
            is_primitive: true,
            snake_case_type_id: "UNREACHABLE".into(),
            primitive_instance_import_path: Some("SlotInstance".into()),
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
            pascal_identifier: rngc.active_component_definition.pascal_identifier.to_string(),
            type_id_escaped: escape_identifier(rngc.active_component_definition.type_id.to_string()),
            events,
        }
    } else {
        //Handle anything that's not a built-in

        let component_for_current_node = rngc.components.get(&tnd.type_id).unwrap();

        //Properties:
        //  - for each property on cfcn, there will either be:
        //     - an explicit, provided value, or
        //     - an implicit, default value
        //  - an explicit value is present IFF an AttributeValueDefinition
        //    for that property is present on the TemplateNodeDefinition.
        //    That AttributeValueDefinition may be an Expression or Literal (we can throw at this
        //    stage for any `Properties` that are bound to something other than an expression / literal)

        // Tuple of property_id, RIL literal string (e.g. `PropertyLiteral::new(...`_
        let property_ril_tuples: Vec<(String, String)> = component_for_current_node.get_property_definitions(rngc.type_table).iter().map(|pd| {
            let ril_literal_string = {
                if let Some(merged_settings) = &tnd.settings {
                    if let Some(matched_setting) = merged_settings.iter().find(|avd| { avd.0 == pd.name }) {
                        match &matched_setting.1 {
                            ValueDefinition::LiteralValue(lv) => {
                                format!("PropertyLiteral::new({})", lv)
                            },
                            ValueDefinition::Expression(_, id) |
                            ValueDefinition::Identifier(_, id) => {
                                format!("PropertyExpression::new({})", id.expect("Tried to use expression but it wasn't compiled"))
                            },
                            ValueDefinition::Block(block) => {
                                format!("PropertyLiteral::new({})", recurse_literal_block(block.clone(),pd.get_type_definition(&rngc.type_table),  host_crate_info))
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
            snake_case_type_id: component_for_current_node.get_snake_case_id(),
            primitive_instance_import_path: component_for_current_node.primitive_instance_import_path.clone(),
            properties_coproduct_variant: component_for_current_node.type_id_escaped.to_string(),
            component_properties_struct: component_for_current_node.pascal_identifier.to_string(),
            properties: property_ril_tuples,
            transform_ril: builtins_ril[2].clone(),
            size_ril: [builtins_ril[0].clone(), builtins_ril[1].clone()],
            children_literal,
            slot_index_literal: "None".to_string(),
            repeat_source_expression_literal_vec: "None".to_string(),
            repeat_source_expression_literal_range:  "None".to_string(),
            conditional_boolean_expression_literal: "None".to_string(),
            pascal_identifier: rngc.active_component_definition.pascal_identifier.to_string(),
            type_id_escaped: escape_identifier(rngc.active_component_definition.type_id.to_string()),
            events,
        }
    };

    press_template_codegen_cartridge_render_node_literal(args)
}

struct RenderNodesGenerationContext<'a> {
    components: &'a std::collections::HashMap<String, ComponentDefinition>,
    active_component_definition: &'a ComponentDefinition,
    type_table: &'a TypeTable,
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

fn generate_cartridge_component_factory_literal(manifest: &PaxManifest, cd: &ComponentDefinition,  host_crate_info: &HostCrateInfo) -> String {
    let rngc = RenderNodesGenerationContext {
        components: &manifest.components,
        active_component_definition: cd,
        type_table: &manifest.type_table,
    };

    let args = TemplateArgsCodegenCartridgeComponentFactory {
        is_main_component: cd.is_main_component,
        snake_case_type_id: cd.get_snake_case_id(),
        component_properties_struct: cd.pascal_identifier.to_string(),
        properties: cd.get_property_definitions(&manifest.type_table).iter().map(|pd|{
            (pd.clone(),pd.get_type_definition(&manifest.type_table).type_id_escaped.clone())
        }).collect(),
        events: generate_events_map(cd.events.clone()),
        render_nodes_literal: generate_cartridge_render_nodes_literal(&rngc,  host_crate_info),
        properties_coproduct_variant: cd.type_id_escaped.to_string()
    };

    press_template_codegen_cartridge_component_factory(args)
}

fn transform_file_content(src_path: &PathBuf, src_content: &str, host_crate_info: &HostCrateInfo) -> String {
    let src_path_str = src_path.to_str().unwrap();
    if src_path_str.ends_with("pax-properties-coproduct/Cargo.toml") || src_path_str.ends_with("pax-cartridge/Cargo.toml") {
        let mut target_cargo_toml_contents = toml_edit::Document::from_str(src_content).unwrap();

        //insert new entry pointing to userland crate, where `pax_app` is defined
        std::mem::swap(
            target_cargo_toml_contents["dependencies"].get_mut(&host_crate_info.name).unwrap(),
            &mut Item::from_str("{ path=\"../..\" }").unwrap()
        );

        target_cargo_toml_contents.to_string()
    } else {
        src_content.to_string()
    }
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
use crate::manifest::Unit::Percent;
use crate::parsing::escape_identifier;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Debug, Deserialize)]
struct Package {
    id: String,
    name: String,
    version: String,
    dependencies: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
struct Dependency {
    name: String,
    req: String,  // This gives the version requirement, not exact version.
}

fn get_version_of_whitelisted_packages(path: &str) -> Result<String, &'static str> {
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(path)
        .output()
        .expect("Failed to execute `cargo metadata`");

    if !output.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        panic!("Failed to get metadata from Cargo");
    }

    let metadata: Metadata = serde_json::from_slice(&output.stdout)
        .expect("Failed to parse JSON from `cargo metadata`");

    let mut tracked_version: Option<String> = None;

    for package in &metadata.packages {
        if ALL_PKGS.contains(&package.name.as_str()) {
            if let Some(ref version) = tracked_version {
                if package.version != *version {
                    panic!("Version mismatch for {}: expected {}, found {}",
                           package.name, version, package.version);
                }
            } else {
                tracked_version = Some(package.version.clone());
            }
        }
    }

    tracked_version.ok_or("Cannot build a Pax project without a `pax-*` dependency somewhere in your project's dependency graph.  Add e.g. `pax-lang` to your Cargo.toml to resolve this error.")
}

/// For the specified file path or current working directory, first compile Pax project,
/// then run it with a patched build of the `chassis` appropriate for the specified platform
/// See: pax-compiler-sequence-diagram.png
pub fn perform_build(ctx: &RunContext) -> Result<(), ()> {

    #[allow(non_snake_case)]
    let PAX_BADGE = "[Pax]".bold().on_black().white();

    println!("{} 🛠 Running `cargo build`...", &PAX_BADGE);
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

    //Inspect Cargo.lock to find declared pax lib versions
    let pax_version = get_version_of_whitelisted_packages(&ctx.path).unwrap();

    println!("{} 🧮 Compiling expressions", &PAX_BADGE);
    expressions::compile_all_expressions(&mut manifest);

    println!("{} 🦀 Generating Rust", &PAX_BADGE);
    clone_all_dependencies_to_tmp(&pax_dir, &pax_version, &host_crate_info);
    generate_reexports_partial_rs(&pax_dir, &manifest);
    generate_and_overwrite_properties_coproduct(&pax_dir, &manifest, &host_crate_info);
    generate_and_overwrite_cartridge(&pax_dir, &manifest, &host_crate_info);

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

fn copy_dir_to(src_dir: &Path, dst_dir: &Path) -> std::io::Result<()> {
    if !dst_dir.exists() {
        fs::create_dir_all(dst_dir)?;
    }

    for entry_result in fs::read_dir(src_dir)? {
        let entry = entry_result?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst_dir.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_to(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

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
        .join(PAX_DIR_PKG_PATH)
        .join(format!("pax-chassis-{}",target_str_lower))
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

    let output_path = pax_dir.join("build").join(target_folder);
    let output_path_str = output_path.to_str().unwrap();

    std::fs::create_dir_all(&output_path);

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
            .arg(output_path_str)
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
            .arg(output_path_str)
            .stdout(std::process::Stdio::inherit())
            .stderr(if ctx.verbose { std::process::Stdio::inherit() } else {std::process::Stdio::piped()})
            .spawn()
            .expect("failed to run harness")
            .wait()
            .expect("failed to run harness");
    }
}

pub fn perform_clean(path: &str) {
    let path = PathBuf::from(path);
    let pax_dir = path.join(".pax");

    //Sledgehammer approach: nuke the .pax directory
    fs::remove_dir_all(&pax_dir);
}

/// Runs `cargo build` (or `wasm-pack build`) with appropriate env in the directory
/// of the generated chassis project inside the specified .pax dir
/// Returns an output object containing bytestreams of stdout/stderr as well as an exit code
pub fn build_chassis_with_cartridge(pax_dir: &PathBuf, target: &RunTarget) -> Output {

    let target_str : &str = target.into();
    let target_str_lower = &target_str.to_lowercase();
    let pax_dir = PathBuf::from(pax_dir.to_str().unwrap());
    let chassis_path = pax_dir.join(PAX_DIR_PKG_PATH).join(format!("pax-chassis-{}", target_str_lower));

    //Inject `patch` directive, which allows userland projects to refer to concrete versions like `0.4.0`, while we
    //swap them for our locally cloned filesystem versions during compilation.
    let existing_cargo_toml_path = chassis_path.join("Cargo.toml");
    let mut existing_cargo_toml = toml_edit::Document::from_str(&fs::read_to_string(&existing_cargo_toml_path).unwrap()).unwrap();
    let mut patch_table = toml_edit::table();
    for pkg in ALL_PKGS {
        patch_table[pkg]["path"] = toml_edit::value(format!("../{}", pkg));
    }
    existing_cargo_toml.insert("patch.crates-io", patch_table);
    fs::write(existing_cargo_toml_path, existing_cargo_toml.to_string().replace("\"patch.crates-io\"", "patch.crates-io") ).unwrap();

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
                .arg(chassis_path.join("pax-dev-harness-web").join("dist").to_str().unwrap()) //--release -d pax-dev-harness-web/dist
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

struct NamespaceTrieNode {
    pub node_string: Option<String>,
    pub children: HashMap<String, NamespaceTrieNode>,
}

impl PartialEq for NamespaceTrieNode {
    fn eq(&self, other: &Self) -> bool {
        self.node_string == other.node_string
    }
}

impl Eq for NamespaceTrieNode {}

impl PartialOrd for NamespaceTrieNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NamespaceTrieNode {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.node_string, &other.node_string) {
            (Some(a), Some(b)) => a.cmp(b),
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            (None, None) => Ordering::Equal,
        }
    }
}

impl NamespaceTrieNode {
    pub fn insert(&mut self, namespace_string: &str) {
        let mut segments = namespace_string.split("::");
        let first_segment = segments.next().unwrap();

        let mut current_node = self;
        current_node = current_node.get_or_create_child(first_segment);

        for segment in segments {
            current_node = current_node.get_or_create_child(segment);
        }
    }

    pub fn get_or_create_child(&mut self, segment: &str) -> &mut NamespaceTrieNode {
        self.children
            .entry(segment.to_string())
            .or_insert_with(|| NamespaceTrieNode {
                node_string: Some(if let Some(ns) = self.node_string.as_ref() {
                    ns.to_string() + "::" + segment
                } else {
                    segment.to_string()
                }),
                children: HashMap::new(),
            })
    }

    pub fn serialize_to_reexports(&self) -> String {
        "pub mod pax_reexports {\n".to_string() + &self.recurse_serialize_to_reexports(1) + "\n}"
    }

    pub fn recurse_serialize_to_reexports(&self, indent: usize) -> String {

        let indent_str = "    ".repeat(indent);

        let mut accum : String = "".into();

        self.children.iter().sorted().for_each(|child|{
            if child.1.node_string.as_ref().unwrap() == "crate" {
                //handle crate subtrie by skipping the crate NamespaceTrieNode, traversing directly into its children
                child.1.children.iter().sorted().for_each(|child| {
                    if child.1.children.len() == 0 {
                        //leaf node:  write `pub use ...` entry
                        accum += &format!("{}pub use {};\n", indent_str, child.1.node_string.as_ref().unwrap());
                    } else {
                        //non-leaf node:  write `pub mod ...` block
                        accum += &format!("{}pub mod {} {{\n", indent_str, child.1.node_string.as_ref().unwrap().split("::").last().unwrap());
                        accum += &child.1.recurse_serialize_to_reexports(indent + 1);
                        accum += &format!("{}}}\n", indent_str);
                    }
                })

            }else {
                if child.1.children.len() == 0 {
                    //leaf node:  write `pub use ...` entry
                    accum += &format!("{}pub use {};\n", indent_str, child.1.node_string.as_ref().unwrap());
                } else {
                    //non-leaf node:  write `pub mod ...` block
                    accum += &format!("{}pub mod {}{{\n", indent_str, child.1.node_string.as_ref().unwrap().split("::").last().unwrap());
                    accum += &child.1.recurse_serialize_to_reexports(indent + 1);
                    accum += &format!("{}}}\n", indent_str);
                }
            };
        });

        accum
    }
}

#[cfg(test)]
mod tests {
    use super::NamespaceTrieNode;
    use std::collections::HashMap;

    #[test]
    fn test_serialize_to_reexports() {
        let input_vec = vec![
            "crate::Example",
            "crate::fireworks::Fireworks",
            "crate::grids::Grids",
            "crate::grids::RectDef",
            "crate::hello_rgb::HelloRGB",
            "f64",
            "pax_std::primitives::Ellipse",
            "pax_std::primitives::Group",
            "pax_std::primitives::Rectangle",
            "pax_std::types::Color",
            "pax_std::types::Stroke",
            "std::vec::Vec",
            "usize",
        ];

        let mut root_node = NamespaceTrieNode {
            node_string: None,
            children: HashMap::new(),
        };

        for namespace_string in input_vec {
            root_node.insert(&namespace_string);
        }

        let output = root_node.serialize_to_reexports();

        let expected_output = r#"pub mod pax_reexports {
    pub use crate::Example;
    pub mod fireworks {
        pub use crate::fireworks::Fireworks;
    }
    pub mod grids {
        pub use crate::grids::Grids;
        pub use crate::grids::RectDef;
    }
    pub mod hello_rgb {
        pub use crate::hello_rgb::HelloRGB;
    }
    pub use f64;
    pub mod pax_std{
        pub mod primitives{
            pub use pax_std::primitives::Ellipse;
            pub use pax_std::primitives::Group;
            pub use pax_std::primitives::Rectangle;
        }
        pub mod types{
            pub use pax_std::types::Color;
            pub use pax_std::types::Stroke;
        }
    }
    pub mod std{
        pub mod vec{
            pub use std::vec::Vec;
        }
    }
    pub use usize;

}"#;

        assert_eq!(output, expected_output);
    }
}