extern crate core;

pub mod expressions;
pub mod manifest;
pub mod parsing;
pub mod templating;

use manifest::PaxManifest;
use pax_runtime_api::CommonProperties;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;

use actix_web::middleware::Logger;
use actix_web::{App, HttpServer};
use env_logger;
use flate2::read::GzDecoder;
use fs_extra::dir::{self, CopyOptions};
use itertools::Itertools;
use rust_format::Formatter;
use std::net::TcpListener;
use tar::Archive;

use include_dir::{include_dir, Dir};
use lazy_static::lazy_static;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use crate::manifest::{
    ComponentDefinition, EventDefinition, ExpressionSpec, LiteralBlockDefinition,
    TemplateNodeDefinition, TypeDefinition, TypeTable, ValueDefinition,
};

use crate::templating::{
    press_template_codegen_cartridge_component_factory,
    press_template_codegen_cartridge_render_node_literal,
    TemplateArgsCodegenCartridgeComponentFactory, TemplateArgsCodegenCartridgeRenderNodeLiteral,
};

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use toml_edit::Item;

lazy_static! {
    #[allow(non_snake_case)]
    static ref PAX_BADGE: ColoredString = "[Pax]".bold().on_black().white();

    static ref DIR_IGNORE_LIST_MACOS : Vec<&'static str> = vec!["target", ".build", ".git"];
    static ref DIR_IGNORE_LIST_WEB : Vec<&'static str> = vec![".git"];
}

static PAX_CREATE_TEMPLATE: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/new-project-template");

const PAX_CREATE_TEMPLATE_DIR_NAME: &str = "new-project-template";
const PKG_DIR_NAME: &str = "pkg";
const BUILD_DIR_NAME: &str = "build";
const PUBLIC_DIR_NAME: &str = "public";
const ASSETS_DIR_NAME: &str = "assets";
const REEXPORTS_PARTIAL_FILE_NAME: &str = "reexports.partial.rs";
const RUST_IOS_DYLIB_FILE_NAME: &str = "libpaxchassisios.dylib";
const RUST_MACOS_DYLIB_FILE_NAME: &str = "libpaxchassismacos.dylib";
const PORTABLE_DYLIB_INSTALL_NAME: &str = "@rpath/PaxCartridge.framework/PaxCartridge";

const XCODE_MACOS_TARGET_DEBUG: &str = "Pax macOS (Development)";
const XCODE_IOS_TARGET_DEBUG: &str = "Pax iOS (Development)";

// These package IDs represent the directory / package names inside the xcframework,
const MACOS_MULTIARCH_PACKAGE_ID: &str = "macos-arm64_x86_64";
const IOS_SIMULATOR_MULTIARCH_PACKAGE_ID: &str = "ios-arm64_x86_64-simulator";
const IOS_PACKAGE_ID: &str = "ios-arm64";

const ERR_SPAWN: &str = "failed to spawn child";

//whitelist of package ids that are relevant to the compiler, e.g. for cloning & patching, for assembling FS paths,
//or for looking up package IDs from a userland Cargo.lock.
const ALL_PKGS: [&'static str; 14] = [
    "pax-cartridge",
    "pax-chassis-common",
    "pax-chassis-ios",
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

/// Clone all dependencies to `.pax/pkg`.  Similar in spirit to the Cargo package cache,
/// this temp directory enables Pax to codegen and building in the context of the larger monorepo,
/// working around various constraints with Cargo (for example, limits surrounding the `patch` directive.)
///
/// The packages in `.pax/pkg` are both where we write our codegen (into pax-cartridge and pax-properties-coproduct)
/// and where we build chassis and chassis-interfaces. (for example, running `wasm-pack` inside `.pax/pkg/pax-chassis-web`.
fn clone_all_to_pkg_dir(pax_dir: &PathBuf, pax_version: &Option<String>, ctx: &RunContext) {
    let dest_pkg_root = pax_dir.join(PKG_DIR_NAME);
    for pkg in ALL_PKGS {
        if ctx.is_libdev_mode {
            //Copy all packages from monorepo root on every build.  this allows us to propagate changes
            //to a libdev build without "sticky caches."
            let pax_workspace_root = pax_dir.parent().unwrap().parent().unwrap();
            let src = pax_workspace_root.join(pkg);
            let dest = dest_pkg_root.join(pkg);

            copy_dir_recursively(&src, &dest, &DIR_IGNORE_LIST_MACOS)
                .expect(&format!("Failed to copy from {:?} to {:?}", src, dest));
        } else {
            let dest = dest_pkg_root.join(pkg);
            if !dest.exists() {
                let pax_version = pax_version
                    .as_ref()
                    .expect("Pax version required but not found");
                let tarball_url = format!(
                    "https://crates.io/api/v1/crates/{}/{}/download",
                    pkg, pax_version
                );
                let resp = reqwest::blocking::get(&tarball_url).expect(&format!(
                    "Failed to fetch tarball for {} at version {}",
                    pkg, pax_version
                ));

                let tarball_bytes = resp.bytes().expect("Failed to read tarball bytes");

                // Wrap the byte slice in a Cursor, so it can be used as a Read trait object.
                let cursor = std::io::Cursor::new(&tarball_bytes[..]);

                // Create a GzDecoder to handle the gzip layer.
                let gz = GzDecoder::new(cursor);

                // Pass the GzDecoder to tar::Archive.
                let mut archive = Archive::new(gz);

                // Iterate over the entries in the archive and modify the paths before extracting.
                for entry_result in archive.entries().expect("Failed to read entries") {
                    let mut entry = entry_result.expect("Failed to read entry");
                    let path = match entry
                        .path()
                        .expect("Failed to get path")
                        .components()
                        .skip(1)
                        .collect::<PathBuf>()
                        .as_path()
                        .to_owned()
                    {
                        path if path.to_string_lossy() == "" => continue, // Skip the root folder
                        path => dest.join(path),
                    };
                    if entry.header().entry_type().is_dir() {
                        fs::create_dir_all(&path).expect("Failed to create directory");
                    } else {
                        if let Some(parent) = path.parent() {
                            fs::create_dir_all(&parent).expect("Failed to create parent directory");
                        }
                        entry.unpack(&path).expect("Failed to unpack file");
                    }
                }
            }
        }
    }
}

/// Returns a sorted and de-duped list of combined_reexports.
fn generate_reexports_partial_rs(pax_dir: &PathBuf, manifest: &PaxManifest) {
    let imports = manifest.import_paths.clone().into_iter().sorted().collect();

    let file_contents = &bundle_reexports_into_namespace_string(&imports);

    let path = pax_dir.join(Path::new(REEXPORTS_PARTIAL_FILE_NAME));
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
    manifest.type_table.iter_mut().for_each(|t| {
        t.1.type_id_escaped = t.1.type_id_escaped.replace("{PREFIX}", "");
        t.1.type_id =
            t.1.type_id
                .replace("{PREFIX}", &host_crate_info.import_prefix);
        t.1.property_definitions.iter_mut().for_each(|pd| {
            pd.type_id = pd
                .type_id
                .replace("{PREFIX}", &host_crate_info.import_prefix);
        });
        updated_type_table.insert(
            t.0.replace("{PREFIX}", &host_crate_info.import_prefix),
            t.1.clone(),
        );
    });
    std::mem::swap(&mut manifest.type_table, &mut updated_type_table);
}

fn generate_and_overwrite_properties_coproduct(
    pax_dir: &PathBuf,
    manifest: &PaxManifest,
    host_crate_info: &HostCrateInfo,
) {
    let target_dir = pax_dir.join(PKG_DIR_NAME).join("pax-properties-coproduct");

    let target_cargo_full_path = fs::canonicalize(target_dir.join("Cargo.toml")).unwrap();
    let mut target_cargo_toml_contents =
        toml_edit::Document::from_str(&fs::read_to_string(&target_cargo_full_path).unwrap())
            .unwrap();

    //insert new entry pointing to userland crate, where `pax_app` is defined
    std::mem::swap(
        target_cargo_toml_contents["dependencies"]
            .get_mut(&host_crate_info.name)
            .unwrap(),
        &mut Item::from_str("{ path=\"../../..\" }").unwrap(),
    );

    //write patched Cargo.toml
    fs::write(
        &target_cargo_full_path,
        &target_cargo_toml_contents.to_string(),
    )
    .unwrap();

    //build tuples for PropertiesCoproduct
    let mut properties_coproduct_tuples: Vec<(String, String)> = manifest
        .components
        .iter()
        .map(|comp_def| {
            let mod_path = if &comp_def.1.module_path == "crate" {
                "".to_string()
            } else {
                comp_def.1.module_path.replace("crate::", "") + "::"
            };
            (
                comp_def.1.type_id_escaped.clone(),
                format!(
                    "{}{}{}",
                    &host_crate_info.import_prefix, &mod_path, &comp_def.1.pascal_identifier
                ),
            )
        })
        .collect();
    let set: HashSet<(String, String)> = properties_coproduct_tuples.drain(..).collect();
    properties_coproduct_tuples.extend(set.into_iter());
    properties_coproduct_tuples.sort();

    //build tuples for TypesCoproduct
    // - include all Property types, representing all possible return types for Expressions
    // - include all T such that T is the iterator type for some Property<Vec<T>>
    let mut types_coproduct_tuples: Vec<(String, String)> = manifest
        .components
        .iter()
        .map(|cd| {
            cd.1.get_property_definitions(&manifest.type_table)
                .iter()
                .map(|pm| {
                    let td = pm.get_type_definition(&manifest.type_table);

                    (
                        td.type_id_escaped.clone(),
                        host_crate_info.import_prefix.to_string()
                            + &td.type_id.clone().replace("crate::", ""),
                    )
                })
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect::<Vec<_>>();

    let mut set: HashSet<_> = types_coproduct_tuples.drain(..).collect();

    #[allow(non_snake_case)]
    let TYPES_COPRODUCT_BUILT_INS = vec![
        ("f64", "f64"),
        ("bool", "bool"),
        ("isize", "isize"),
        ("usize", "usize"),
        ("String", "String"),
        (
            "stdCOCOvecCOCOVecLABRstdCOCOrcCOCORcLABRPropertiesCoproductRABRRABR",
            "std::vec::Vec<std::rc::Rc<PropertiesCoproduct>>",
        ),
        ("Transform2D", "pax_runtime_api::Transform2D"),
        ("stdCOCOopsCOCORangeLABRisizeRABR", "std::ops::Range<isize>"),
        ("Size", "pax_runtime_api::Size"),
        ("Rotation", "pax_runtime_api::Rotation"),
        ("SizePixels", "pax_runtime_api::SizePixels"),
        ("Numeric", "pax_runtime_api::Numeric"),
    ];

    TYPES_COPRODUCT_BUILT_INS.iter().for_each(|builtin| {
        set.insert((builtin.0.to_string(), builtin.1.to_string()));
    });
    types_coproduct_tuples.extend(set.into_iter());
    types_coproduct_tuples.sort();

    types_coproduct_tuples = types_coproduct_tuples
        .into_iter()
        .unique_by(|elem| elem.0.to_string())
        .collect::<Vec<(String, String)>>();

    //press template into String
    let generated_lib_rs = templating::press_template_codegen_properties_coproduct_lib(
        templating::TemplateArgsCodegenPropertiesCoproductLib {
            properties_coproduct_tuples,
            types_coproduct_tuples,
        },
    );

    //write String to file
    fs::write(target_dir.join("src/lib.rs"), generated_lib_rs).unwrap();
}

fn generate_and_overwrite_cartridge(
    pax_dir: &PathBuf,
    manifest: &PaxManifest,
    host_crate_info: &HostCrateInfo,
) {
    let target_dir = pax_dir.join(PKG_DIR_NAME).join("pax-cartridge");

    let target_cargo_full_path = fs::canonicalize(target_dir.join("Cargo.toml")).unwrap();
    let mut target_cargo_toml_contents =
        toml_edit::Document::from_str(&fs::read_to_string(&target_cargo_full_path).unwrap())
            .unwrap();

    //insert new entry pointing to userland crate, where `pax_app` is defined
    std::mem::swap(
        target_cargo_toml_contents["dependencies"]
            .get_mut(&host_crate_info.name)
            .unwrap(),
        &mut Item::from_str("{ path=\"../../..\" }").unwrap(),
    );

    //write patched Cargo.toml
    fs::write(
        &target_cargo_full_path,
        &target_cargo_toml_contents.to_string(),
    )
    .unwrap();

    const IMPORTS_BUILTINS: [&str; 26] = [
        "std::cell::RefCell",
        "std::collections::HashMap",
        "std::collections::VecDeque",
        "std::ops::Deref",
        "std::rc::Rc",
        "pax_runtime_api::PropertyInstance",
        "pax_runtime_api::PropertyLiteral",
        "pax_runtime_api::CommonProperties",
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

    let mut imports: Vec<String> = manifest
        .import_paths
        .iter()
        .map(|path| {
            if !imports_builtins_set.contains(&**path) {
                IMPORT_PREFIX.clone() + &path.replace("crate::", "")
            } else {
                "".to_string()
            }
        })
        .collect();

    imports.append(
        &mut IMPORTS_BUILTINS
            .into_iter()
            .map(|ib| ib.to_string())
            .collect::<Vec<String>>(),
    );

    let consts = vec![]; //TODO!

    let mut expression_specs: Vec<ExpressionSpec> = manifest
        .expression_specs
        .as_ref()
        .unwrap()
        .values()
        .map(|es: &ExpressionSpec| es.clone())
        .collect();
    expression_specs = expression_specs.iter().sorted().cloned().collect();

    let component_factories_literal = manifest
        .components
        .values()
        .into_iter()
        .filter(|cd| !cd.is_primitive && !cd.is_struct_only_component)
        .map(|cd| generate_cartridge_component_factory_literal(manifest, cd, host_crate_info))
        .collect();

    //press template into String
    let generated_lib_rs = templating::press_template_codegen_cartridge_lib(
        templating::TemplateArgsCodegenCartridgeLib {
            imports,
            consts,
            expression_specs,
            component_factories_literal,
        },
    );

    // Re: formatting the generated Rust code, see prior art at `_format_generated_lib_rs`
    fs::write(target_dir.join("src/lib.rs"), generated_lib_rs).unwrap();
}

/// Note: this function was abandoned because RustFmt takes unacceptably long to format complex
/// pax-cartridge/src/lib.rs files.  The net effect was a show-stoppingly slow `pax build`.
/// We can problaby mitigate this by: (a) waiting for or eliciting improvements in RustFmt, or (b) figuring out what about our codegen is slowing RustFmt down, and generate our code differently to side-step.
/// This code is left for posterity in case we take another crack at formatting generated code.
fn _format_generated_lib_rs(generated_lib_rs: String) -> String {
    let formatter = rust_format::RustFmt::default();

    if let Ok(out) = formatter.format_str(generated_lib_rs.clone()) {
        out
    } else {
        //if formatting fails (e.g. parsing error, common expected case) then
        //fall back to unformatted generated code
        generated_lib_rs
    }
}

fn generate_cartridge_render_nodes_literal(
    rngc: &RenderNodesGenerationContext,
    host_crate_info: &HostCrateInfo,
) -> String {
    let nodes =
        rngc.active_component_definition.template.as_ref().expect(
            "tried to generate render nodes literal for component, but template was undefined",
        );

    let implicit_root = nodes[0].borrow();
    let children_literal: Vec<String> = implicit_root
        .child_ids
        .iter()
        .map(|child_id| {
            let tnd_map = rngc.active_component_definition.template.as_ref().unwrap();
            let active_tnd = &tnd_map[*child_id];
            recurse_generate_render_nodes_literal(rngc, active_tnd, host_crate_info)
        })
        .collect();

    children_literal.join(",")
}

fn generate_bound_events(
    inline_settings: Option<Vec<(String, ValueDefinition)>>,
) -> HashMap<String, String> {
    let mut ret: HashMap<String, String> = HashMap::new();
    if let Some(ref inline) = inline_settings {
        for (key, value) in inline.iter() {
            if let ValueDefinition::EventBindingTarget(s) = value {
                ret.insert(key.clone().to_string(), s.clone().to_string());
            };
        }
    };
    ret
}

fn recurse_literal_block(
    block: LiteralBlockDefinition,
    type_definition: &TypeDefinition,
    host_crate_info: &HostCrateInfo,
) -> String {
    let qualified_path = host_crate_info.import_prefix.to_string()
        + &type_definition.import_path.clone().replace("crate::", "");

    // Buffer to store the string representation of the struct
    let mut struct_representation = format!("\n{{ let mut ret = {}::default();", qualified_path);

    // Iterating through each (key, value) pair in the settings_key_value_pairs
    for (key, value_definition) in block.settings_key_value_pairs.iter() {
        let fully_qualified_type = host_crate_info.import_prefix.to_string()
            + &type_definition
                .property_definitions
                .iter()
                .find(|pd| &pd.name == key)
                .expect(&format!(
                    "Property {} not found on type {}",
                    key, type_definition.type_id
                ))
                .type_id;

        let value_string = match value_definition {
            ValueDefinition::LiteralValue(value) => {
                format!(
                    "ret.{} = Box::new(PropertyLiteral::new(Into::<{}>::into({})));",
                    key, fully_qualified_type, value
                )
            }
            ValueDefinition::Expression(_, id) | ValueDefinition::Identifier(_, id) => {
                format!(
                    "ret.{} = Box::new(PropertyExpression::new({}));",
                    key,
                    id.expect("Tried to use expression but it wasn't compiled")
                )
            }
            ValueDefinition::Block(inner_block) => format!(
                "ret.{} = Box::new(PropertyLiteral::new(Into::<{}>::into({})));",
                key,
                fully_qualified_type,
                recurse_literal_block(inner_block.clone(), type_definition, host_crate_info),
            ),
            _ => {
                panic!("Incorrect value bound to inline setting")
            }
        };

        struct_representation.push_str(&format!("\n{}", value_string));
    }

    struct_representation.push_str("\n ret }");

    struct_representation
}

fn recurse_generate_render_nodes_literal(
    rngc: &RenderNodesGenerationContext,
    tnd: &TemplateNodeDefinition,
    host_crate_info: &HostCrateInfo,
) -> String {
    //first recurse, populating children_literal : Vec<String>
    let children_literal: Vec<String> = tnd
        .child_ids
        .iter()
        .map(|child_id| {
            let active_tnd =
                &rngc.active_component_definition.template.as_ref().unwrap()[*child_id];
            recurse_generate_render_nodes_literal(rngc, active_tnd, host_crate_info)
        })
        .collect();

    //pull inline event binding and store into map
    let events = generate_bound_events(tnd.settings.clone());
    let args = if tnd.type_id == parsing::TYPE_ID_REPEAT {
        // Repeat
        let rsd = tnd
            .control_flow_settings
            .as_ref()
            .unwrap()
            .repeat_source_definition
            .as_ref()
            .unwrap();
        let id = rsd.vtable_id.unwrap();

        let rse_vec = if let Some(_) = &rsd.symbolic_binding {
            format!("Some(Box::new(PropertyExpression::new({})))", id)
        } else {
            "None".into()
        };

        let rse_range = if let Some(_) = &rsd.range_expression_paxel {
            format!("Some(Box::new(PropertyExpression::new({})))", id)
        } else {
            "None".into()
        };

        let common_properties_literal = CommonProperties::get_default_properties_literal();

        TemplateArgsCodegenCartridgeRenderNodeLiteral {
            is_primitive: true,
            snake_case_type_id: "UNREACHABLE".into(),
            primitive_instance_import_path: Some("RepeatInstance".into()),
            properties_coproduct_variant: "None".to_string(),
            component_properties_struct: "None".to_string(),
            defined_properties: vec![],
            common_properties_literal,
            children_literal,
            slot_index_literal: "None".to_string(),
            conditional_boolean_expression_literal: "None".to_string(),
            pascal_identifier: rngc
                .active_component_definition
                .pascal_identifier
                .to_string(),
            type_id_escaped: escape_identifier(
                rngc.active_component_definition.type_id.to_string(),
            ),
            events,
            repeat_source_expression_literal_vec: rse_vec,
            repeat_source_expression_literal_range: rse_range,
        }
    } else if tnd.type_id == parsing::TYPE_ID_IF {
        // If
        let id = tnd
            .control_flow_settings
            .as_ref()
            .unwrap()
            .condition_expression_vtable_id
            .unwrap();

        let common_properties_literal = CommonProperties::get_default_properties_literal();

        TemplateArgsCodegenCartridgeRenderNodeLiteral {
            is_primitive: true,
            snake_case_type_id: "UNREACHABLE".into(),
            primitive_instance_import_path: Some("ConditionalInstance".into()),
            properties_coproduct_variant: "None".to_string(),
            component_properties_struct: "None".to_string(),
            defined_properties: vec![],
            common_properties_literal,
            children_literal,
            slot_index_literal: "None".to_string(),
            repeat_source_expression_literal_vec: "None".to_string(),
            repeat_source_expression_literal_range: "None".to_string(),
            conditional_boolean_expression_literal: format!(
                "Some(Box::new(PropertyExpression::new({})))",
                id
            ),
            pascal_identifier: rngc
                .active_component_definition
                .pascal_identifier
                .to_string(),
            type_id_escaped: escape_identifier(
                rngc.active_component_definition.type_id.to_string(),
            ),
            events,
        }
    } else if tnd.type_id == parsing::TYPE_ID_SLOT {
        // Slot
        let id = tnd
            .control_flow_settings
            .as_ref()
            .unwrap()
            .slot_index_expression_vtable_id
            .unwrap();

        let common_properties_literal = CommonProperties::get_default_properties_literal();

        TemplateArgsCodegenCartridgeRenderNodeLiteral {
            is_primitive: true,
            snake_case_type_id: "UNREACHABLE".into(),
            primitive_instance_import_path: Some("SlotInstance".into()),
            properties_coproduct_variant: "None".to_string(),
            component_properties_struct: "None".to_string(),
            defined_properties: vec![],
            common_properties_literal,
            children_literal,
            slot_index_literal: format!("Some(Box::new(PropertyExpression::new({})))", id),
            repeat_source_expression_literal_vec: "None".to_string(),
            repeat_source_expression_literal_range: "None".to_string(),
            conditional_boolean_expression_literal: "None".to_string(),
            pascal_identifier: rngc
                .active_component_definition
                .pascal_identifier
                .to_string(),
            type_id_escaped: escape_identifier(
                rngc.active_component_definition.type_id.to_string(),
            ),
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
        let property_ril_tuples: Vec<Option<(String, String)>> = component_for_current_node
            .get_property_definitions(rngc.type_table)
            .iter()
            .map(|pd| {
                let ril_literal_string = {
                    if let Some(merged_settings) = &tnd.settings {
                        if let Some(matched_setting) =
                            merged_settings.iter().find(|avd| avd.0 == pd.name)
                        {
                            match &matched_setting.1 {
                                ValueDefinition::LiteralValue(lv) => {
                                    Some(format!("PropertyLiteral::new({})", lv))
                                }
                                ValueDefinition::Expression(_, id)
                                | ValueDefinition::Identifier(_, id) => {
                                    Some(format!(
                                        "PropertyExpression::new({})",
                                        id.expect("Tried to use expression but it wasn't compiled")
                                    ))
                                }
                                ValueDefinition::Block(block) => {
                                    Some(format!(
                                        "PropertyLiteral::new({})",
                                        recurse_literal_block(
                                            block.clone(),
                                            pd.get_type_definition(&rngc.type_table),
                                            host_crate_info
                                        )
                                    ))
                                }
                                _ => {
                                    panic!("Incorrect value bound to inline setting")
                                }
                            }
                        } else {
                            None
                        }
                    } else {
                        //no inline attributes at all; everything will be default
                        None
                    }
                };


                if let Some(ril_literal_string) = ril_literal_string {
                    Some((pd.name.clone(), ril_literal_string))
                } else {
                    None
                }
            })
            .collect();

        let defined_properties: Vec<(String, String)> = property_ril_tuples
            .iter()
            .filter_map(|p| p.as_ref())
            .cloned()
            .collect();

        let identifiers_and_types = pax_runtime_api::CommonProperties::get_property_identifiers();

        fn default_common_property_value(identifier: &str) -> String {
            if identifier == "transform" {
                "Transform2D::default_wrapped()".to_string()
            } else if identifier == "width" || identifier == "height" {
                "Rc::new(RefCell::new(PropertyLiteral::new(Size::default())))".to_string()
            } else {
                "Default::default()".to_string()
            }
        }

        fn is_optional(identifier: &str) -> bool {
            identifier != "transform" && identifier != "width" && identifier != "height"
        }

        let common_properties_literal: Vec<(String, String)> = identifiers_and_types
            .iter()
            .map(|identifier_and_type| {
                if let Some(inline_settings) = &tnd.settings {
                    if let Some(matched_setting) = inline_settings
                        .iter()
                        .find(|vd| vd.0 == *identifier_and_type.0)
                    {
                        (
                            identifier_and_type.0.to_string(),
                            match &matched_setting.1 {
                                ValueDefinition::LiteralValue(lv) => {
                                    let mut literal_value = format!(
                                        "Rc::new(RefCell::new(PropertyLiteral::new(Into::<{}>::into({}))))",
                                        identifier_and_type.1,
                                        lv,
                                    );
                                    if is_optional(&identifier_and_type.0) {
                                        literal_value = format!("Some({})", literal_value);
                                    }
                                    literal_value
                                }
                                ValueDefinition::Expression(_, id)
                                | ValueDefinition::Identifier(_, id) => {
                                    let mut literal_value = format!(
                                        "Rc::new(RefCell::new(PropertyExpression::new({})))",
                                        id.expect("Tried to use expression but it wasn't compiled")
                                    );

                                    if is_optional(&identifier_and_type.0) {
                                        literal_value = format!("Some({})", literal_value);
                                    }
                                    literal_value
                                }
                                _ => {
                                    panic!("Incorrect value bound to attribute")
                                }
                            },
                        )
                    } else {
                        (
                            identifier_and_type.0.to_string(),
                            default_common_property_value(&identifier_and_type.0),
                        )
                    }
                } else {
                    (
                        identifier_and_type.0.to_string(),
                        default_common_property_value(&identifier_and_type.0),
                    )
                }
            })
            .collect();
        //then, on the post-order traversal, press template string and return
        TemplateArgsCodegenCartridgeRenderNodeLiteral {
            is_primitive: component_for_current_node.is_primitive,
            snake_case_type_id: component_for_current_node.get_snake_case_id(),
            primitive_instance_import_path: component_for_current_node
                .primitive_instance_import_path
                .clone(),
            properties_coproduct_variant: component_for_current_node.type_id_escaped.to_string(),
            component_properties_struct: component_for_current_node.pascal_identifier.to_string(),
            defined_properties,
            common_properties_literal,
            children_literal,
            slot_index_literal: "None".to_string(),
            repeat_source_expression_literal_vec: "None".to_string(),
            repeat_source_expression_literal_range: "None".to_string(),
            conditional_boolean_expression_literal: "None".to_string(),
            pascal_identifier: rngc
                .active_component_definition
                .pascal_identifier
                .to_string(),
            type_id_escaped: escape_identifier(
                rngc.active_component_definition.type_id.to_string(),
            ),
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
            for e in event_list.iter() {
                ret.insert(e.key.clone(), e.value.clone());
            }
        }
        _ => {}
    };
    ret
}

fn generate_cartridge_component_factory_literal(
    manifest: &PaxManifest,
    cd: &ComponentDefinition,
    host_crate_info: &HostCrateInfo,
) -> String {
    let rngc = RenderNodesGenerationContext {
        components: &manifest.components,
        active_component_definition: cd,
        type_table: &manifest.type_table,
    };

    let args = TemplateArgsCodegenCartridgeComponentFactory {
        is_main_component: cd.is_main_component,
        snake_case_type_id: cd.get_snake_case_id(),
        component_properties_struct: cd.pascal_identifier.to_string(),
        properties: cd
            .get_property_definitions(&manifest.type_table)
            .iter()
            .map(|pd| {
                (
                    pd.clone(),
                    pd.get_type_definition(&manifest.type_table)
                        .type_id_escaped
                        .clone(),
                )
            })
            .collect(),
        events: generate_events_map(cd.events.clone()),
        render_nodes_literal: generate_cartridge_render_nodes_literal(&rngc, host_crate_info),
        properties_coproduct_variant: cd.type_id_escaped.to_string(),
    };

    press_template_codegen_cartridge_component_factory(args)
}

fn get_or_create_pax_directory(working_dir: &str) -> PathBuf {
    let working_path = std::path::Path::new(working_dir).join(".pax");
    std::fs::create_dir_all(&working_path).unwrap();
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
}

fn get_host_crate_info(cargo_toml_path: &Path) -> HostCrateInfo {
    let existing_cargo_toml = toml_edit::Document::from_str(
        &fs::read_to_string(fs::canonicalize(cargo_toml_path).unwrap()).unwrap(),
    )
    .expect("Error loading host Cargo.toml");

    let name = existing_cargo_toml["package"]["name"]
        .as_str()
        .unwrap()
        .to_string();
    let identifier = name.replace("-", "_"); //NOTE: perhaps this could be less naive?
    let import_prefix = format!("{}::pax_reexports::", &identifier);

    HostCrateInfo {
        name,
        identifier,
        import_prefix,
    }
}

#[allow(unused)]
static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

/// Executes a shell command to run the feature-flagged parser at the specified path
/// Returns an output object containing bytestreams of stdout/stderr as well as an exit code
pub fn run_parser_binary(path: &str, process_child_ids: Arc<Mutex<Vec<u64>>>) -> Output {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(path)
        .arg("run")
        .arg("--features")
        .arg("parser")
        .arg("--color")
        .arg("always")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(pre_exec_hook);
    }

    let child = cmd.spawn().expect(ERR_SPAWN);

    // child.stdin.take().map(drop);
    let output = wait_with_output(&process_child_ids, child);
    output
}

use colored::{ColoredString, Colorize};

use crate::parsing::escape_identifier;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    version: String,
}

fn get_version_of_whitelisted_packages(path: &str) -> Result<String, &'static str> {
    let mut cmd = Command::new("cargo");
    let output = cmd
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(path)
        .output()
        .expect("Failed to execute `cargo metadata`");

    if !output.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        panic!("Failed to get metadata from Cargo");
    }

    let metadata: Metadata =
        serde_json::from_slice(&output.stdout).expect("Failed to parse JSON from `cargo metadata`");

    let mut tracked_version: Option<String> = None;

    for package in &metadata.packages {
        if ALL_PKGS.contains(&package.name.as_str()) {
            if let Some(ref version) = tracked_version {
                if package.version != *version {
                    panic!(
                        "Version mismatch for {}: expected {}, found {}",
                        package.name, version, package.version
                    );
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
    //First we clone dependencies into the .pax/pkg directory.  We must do this before running
    //the parser binary specifical for libdev in pax-example — see pax-example/Cargo.toml where
    //dependency paths are `.pax/pkg/*`.
    let pax_dir = get_or_create_pax_directory(&ctx.path);

    //Inspect Cargo.lock to find declared pax lib versions.  Note that this is moot for
    //libdev, where we don't care about a crates.io version (and where `cargo metadata` won't work
    //on a cold-start monorepo clone.)
    let pax_version = if ctx.is_libdev_mode {
        None
    } else {
        Some(get_version_of_whitelisted_packages(&ctx.path).unwrap())
    };
    clone_all_to_pkg_dir(&pax_dir, &pax_version, &ctx);

    println!("{} 🛠️  Building parser binary with `cargo`...", *PAX_BADGE);
    // Run parser bin from host project with `--features parser`
    let output = run_parser_binary(&ctx.path, Arc::clone(&ctx.process_child_ids));

    // Forward stderr only
    std::io::stderr()
        .write_all(output.stderr.as_slice())
        .unwrap();

    if !output.status.success() {
        println!("Parsing failed — there is likely a syntax error in the provided pax");
        return Err(());
    }

    let out = String::from_utf8(output.stdout).unwrap();
    let mut manifest: PaxManifest =
        serde_json::from_str(&out).expect(&format!("Malformed JSON from parser: {}", &out));
    let host_cargo_toml_path = Path::new(&ctx.path).join("Cargo.toml");
    let host_crate_info = get_host_crate_info(&host_cargo_toml_path);
    update_property_prefixes_in_place(&mut manifest, &host_crate_info);

    println!("{} 🧮 Compiling expressions", *PAX_BADGE);
    expressions::compile_all_expressions(&mut manifest);

    println!("{} 🦀 Generating Rust", *PAX_BADGE);
    generate_reexports_partial_rs(&pax_dir, &manifest);
    generate_and_overwrite_properties_coproduct(&pax_dir, &manifest, &host_crate_info);
    generate_and_overwrite_cartridge(&pax_dir, &manifest, &host_crate_info);

    //7. Build the appropriate `chassis` from source, with the patched `Cargo.toml`, Properties Coproduct, and Cartridge from above
    println!("{} 🧱 Building cartridge with `cargo`", *PAX_BADGE);
    let res = build_chassis_with_cartridge(&pax_dir, &ctx, Arc::clone(&ctx.process_child_ids));
    if let Err(()) = res {
        return Err(());
    }

    Ok(())
}

fn start_static_http_server(fs_path: PathBuf) -> std::io::Result<()> {
    // Initialize logging

    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::Builder::from_env(env_logger::Env::default())
        .format(|buf, record| writeln!(buf, "{} 🍱 Served {}", *PAX_BADGE, record.args()))
        .init();

    // Create a Runtime
    let runtime = actix_rt::System::new().block_on(async {
        let mut port = 8080;
        let server = loop {
            // Check if the port is available
            if TcpListener::bind(("127.0.0.1", port)).is_ok() {
                // Log the server details
                println!(
                    "{} 🗂️  Serving static files from {}",
                    *PAX_BADGE,
                    &fs_path.to_str().unwrap()
                );
                let address_msg = format!("http://127.0.0.1:{}", port).blue();
                let server_running_at_msg = format!("Server running at {}", address_msg).bold();
                println!("{} 📠 {}", *PAX_BADGE, server_running_at_msg);
                break HttpServer::new(move || {
                    App::new().wrap(Logger::new("| %s | %U")).service(
                        actix_files::Files::new("/*", fs_path.clone()).index_file("index.html"),
                    )
                })
                .bind(("127.0.0.1", port))
                .expect("Error binding to address")
                .workers(2);
            } else {
                port += 1; // Try the next port
            }
        };

        server.run().await
    });

    runtime
}

/// Helper recursive fs copy method, like fs::copy, but suited for our purposes.
/// Includes ability to ignore directories by name during recursion.
fn copy_dir_recursively(
    src: &Path,
    dest: &Path,
    ignore_list: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    if src.is_dir() {
        // If the directory name is in the ignore list, we skip this directory
        if ignore_list.contains(&src.file_name().unwrap().to_str().unwrap()) {
            return Ok(());
        }

        // Create the corresponding directory in the destination,
        // and copy its contents recursively
        fs::create_dir_all(dest)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            let dest_child = dest.join(path.file_name().ok_or("Invalid file name")?);
            copy_dir_recursively(&path, &dest_child, ignore_list)?;
        }
    } else {
        // If source is a file, just copy it to the destination
        fs::copy(src, dest)?;
    }
    Ok(())
}

/// Clean all `.pax` temp files
pub fn perform_clean(path: &str) {
    let path = PathBuf::from(path);
    let pax_dir = path.join(".pax");

    fs::remove_dir_all(&pax_dir).ok();
}

/// Runs `cargo build` (or `wasm-pack build`) with appropriate env in the directory
/// of the generated chassis project inside the specified .pax dir
/// Returns an output object containing bytestreams of stdout/stderr as well as an exit code
pub fn build_chassis_with_cartridge(
    pax_dir: &PathBuf,
    ctx: &RunContext,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
) -> Result<(), ()> {
    let target: &RunTarget = &ctx.target;
    let target_str: &str = target.into();
    let target_str_lower = &target_str.to_lowercase();
    let pax_dir = PathBuf::from(pax_dir.to_str().unwrap());
    let chassis_path = pax_dir
        .join(PKG_DIR_NAME)
        .join(format!("pax-chassis-{}", target_str_lower));

    //approximate `should_also_run` as "dev build," `!should_also_run` as prod.
    //we can improve this with an explicit `--release` flag in the Pax CLI
    #[allow(non_snake_case)]
    let IS_RELEASE: bool = !ctx.should_also_run;
    #[allow(non_snake_case)]
    let BUILD_MODE_NAME: &str = if IS_RELEASE { "release" } else { "debug" };

    let interface_path = pax_dir
        .join(PKG_DIR_NAME)
        .join(format!("pax-chassis-{}", target_str_lower))
        .join("interface");

    //Inject `patch` directive, which allows userland projects to refer to concrete versions like `0.4.0`, while we
    //swap them for our locally cloned filesystem versions during compilation.
    let existing_cargo_toml_path = chassis_path.join("Cargo.toml");
    let existing_cargo_toml_string = fs::read_to_string(&existing_cargo_toml_path).unwrap();
    let mut existing_cargo_toml =
        toml_edit::Document::from_str(&existing_cargo_toml_string).unwrap();

    //In builds where we don't wipe out the `pkg` directory (e.g. those installed from crates.io),
    //the Cargo.toml may already have been patched.  Injecting an additional patch would break cargo.
    if !existing_cargo_toml_string.contains("patch.crates-io") {
        let mut patch_table = toml_edit::table();
        for pkg in ALL_PKGS {
            patch_table[pkg]["path"] = toml_edit::value(format!("../{}", pkg));
        }

        existing_cargo_toml.insert("patch.crates-io", patch_table);
        fs::write(
            existing_cargo_toml_path,
            existing_cargo_toml
                .to_string()
                .replace("\"patch.crates-io\"", "patch.crates-io"),
        )
        .unwrap();
    }

    //string together a shell call to build our chassis, with cartridge inserted via `patch`
    match target {
        RunTarget::macOS | RunTarget::iOS => {
            //0: Rust arch string, for passing to cargo
            //1: Apple arch string, for addressing xcframework
            let target_mappings: &[(&str, &str)] = if let RunTarget::macOS = target {
                &[
                    ("aarch64-apple-darwin", "macos-arm64"),
                    ("x86_64-apple-darwin", "macos-x86_64"),
                ]
            } else {
                &[
                    ("aarch64-apple-ios", "ios-arm64"),
                    ("x86_64-apple-ios", "iossimulator-x86_64"),
                    ("aarch64-apple-ios-sim", "iossimulator-arm64"),
                ]
            };

            let dylib_file_name = if let RunTarget::macOS = target {
                RUST_MACOS_DYLIB_FILE_NAME
            } else {
                RUST_IOS_DYLIB_FILE_NAME
            };

            let mut handles = Vec::new();

            //(arch id, single-platform .dylib path, stdout/stderr from build)
            let build_results: Arc<Mutex<HashMap<u32, (String, String, Output)>>> =
                Arc::new(Mutex::new(HashMap::new()));

            let targets_single_string = target_mappings
                .iter()
                .map(|tm| tm.1.to_string())
                .collect::<Vec<String>>()
                .join(", ")
                .bold();
            println!(
                "{} 🧶 Compiling targets {{{}}} in {} mode using {} threads...\n",
                *PAX_BADGE,
                &targets_single_string,
                &BUILD_MODE_NAME.to_string().bold(),
                target_mappings.len()
            );

            let mut index = 0;
            for target_mapping in target_mappings {
                let chassis_path = chassis_path.clone();
                let pax_dir = pax_dir.clone();

                let process_child_ids_threadsafe = process_child_ids.clone();
                let build_results_threadsafe = build_results.clone();

                let handle = thread::spawn(move || {
                    let mut cmd = Command::new("cargo");
                    cmd.current_dir(&chassis_path)
                        .arg("build")
                        .arg("--color")
                        .arg("always")
                        .arg("--target")
                        .arg(target_mapping.0)
                        .env("PAX_DIR", &pax_dir)
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped());

                    if IS_RELEASE {
                        cmd.arg("--release");
                    }

                    #[cfg(unix)]
                    unsafe {
                        cmd.pre_exec(pre_exec_hook);
                    }

                    let child = cmd.spawn().expect(ERR_SPAWN);

                    //Execute `cargo build`, which generates our dylibs
                    let output = wait_with_output(&process_child_ids_threadsafe, child);

                    let dylib_src = chassis_path
                        .join("target")
                        .join(target_mapping.0)
                        .join(BUILD_MODE_NAME)
                        .join(dylib_file_name);

                    let new_val = (
                        target_mapping.1.to_string(),
                        dylib_src.to_str().unwrap().to_string(),
                        output,
                    );
                    build_results_threadsafe
                        .lock()
                        .unwrap()
                        .insert(index, new_val);
                });
                index = index + 1;
                handles.push(handle);
            }

            let mut index = 0;
            // Wait for all threads to complete and print their outputs
            for handle in handles {
                handle.join().unwrap();
            }

            let results = build_results.lock().unwrap();

            let mut should_abort = false;
            //Print stdout/stderr
            for i in 0..target_mappings.len() {
                let result = results.get(&(i as u32)).unwrap();
                let target = &result.0;
                let output = &result.2;

                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                if stdout != "" || stderr != "" {
                    println!("{} build finished with output:", &target);
                }
                if stdout != "" {
                    println!("{}", &stdout);
                }
                if stderr != "" {
                    eprintln!("{}", &stderr);
                }

                if !output.status.success() {
                    should_abort = true;
                }

                index = index + 1;
            }

            if should_abort {
                eprintln!("Failed to build one or more targets with Cargo. Aborting.");
                return Err(());
            }

            // Update the `install name` of each Rust-built .dylib, instead of the default-output absolute file paths
            // embedded in each .dylib.  This allows our .dylibs to be portably embedded into an SPM module.
            let result = results.iter().try_for_each(|res| {
                let dylib_path = &res.1.1;
                let mut cmd = Command::new("install_name_tool");
                cmd
                    .arg("-id")
                    .arg(PORTABLE_DYLIB_INSTALL_NAME)
                    .arg(dylib_path);

                #[cfg(unix)]
                unsafe {
                    cmd.pre_exec(pre_exec_hook);
                }
                let child = cmd.spawn().unwrap();
                let output = wait_with_output(&process_child_ids, child);
                if !output.status.success() {
                    println!("Failed to rewrite dynamic library install name with install_name_tool.  Aborting.");
                    return Err(());
                }

                Ok(())
            });

            match result {
                Err(_) => {
                    return Err(());
                }
                _ => {}
            };

            // Merge architecture-specific binaries with `lipo` (this is an undocumented requirement
            // of multi-arch builds + xcframeworks for the Apple toolchain; we cannot bundle two
            // macos arch .frameworks in an xcframework; they must lipo'd into a single .framework + dylib.
            // Similarly, iOS binaries require a particular bundling for simulator & device builds.)
            println!(
                "{} 🖇️  Combining architecture-specific binaries with `lipo`...",
                *PAX_BADGE
            );

            if let RunTarget::macOS = target {
                // For macOS, we want to lipo both our arm64 and x86_64 dylibs into a single binary,
                // then bundle that single binary into a single framework within the xcframework.
                let multiarch_dylib_dest = pax_dir
                    .join(PKG_DIR_NAME)
                    .join("pax-chassis-common")
                    .join("pax-swift-cartridge")
                    .join("PaxCartridge.xcframework")
                    .join(MACOS_MULTIARCH_PACKAGE_ID)
                    .join("PaxCartridge.framework")
                    .join("PaxCartridge");

                let lipo_input_paths = results
                    .iter()
                    .map(|res| res.1 .1.clone())
                    .collect::<Vec<String>>();

                // Construct the lipo command
                let mut lipo_command = Command::new("lipo");
                lipo_command
                    .arg("-create")
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped());

                // Add each input path to the command
                for path in &lipo_input_paths {
                    lipo_command.arg(path);
                }

                // Specify the output path
                lipo_command.arg("-output").arg(multiarch_dylib_dest);

                #[cfg(unix)]
                unsafe {
                    lipo_command.pre_exec(pre_exec_hook);
                }
                let child = lipo_command.spawn().expect(ERR_SPAWN);
                let output = wait_with_output(&process_child_ids, child);

                if !output.status.success() {
                    println!("Failed to combine packages with lipo. Aborting.");
                    return Err(());
                }
            } else {
                // For iOS, we want to:
                // 1. lipo together both simulator build architectures
                // 2. copy (a) the lipo'd simulator binary, and (b) the vanilla arm64 iOS binary into the framework
                let simulator_builds = results
                    .iter()
                    .filter(|res| res.1 .0.starts_with("iossimulator-"))
                    .collect::<Vec<_>>();
                let device_build = results
                    .iter()
                    .filter(|res| res.1 .0.starts_with("ios-"))
                    .collect::<Vec<_>>();

                let multiarch_dylib_dest = pax_dir
                    .join(PKG_DIR_NAME)
                    .join("pax-chassis-common")
                    .join("pax-swift-cartridge")
                    .join("PaxCartridge.xcframework")
                    .join(IOS_SIMULATOR_MULTIARCH_PACKAGE_ID)
                    .join("PaxCartridge.framework")
                    .join("PaxCartridge");

                let lipo_input_paths = simulator_builds
                    .iter()
                    .map(|res| res.1 .1.clone())
                    .collect::<Vec<String>>();

                // Construct the lipo command
                let mut lipo_command = Command::new("lipo");
                lipo_command
                    .arg("-create")
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped());

                // Add each input path to the command
                for path in &lipo_input_paths {
                    lipo_command.arg(path);
                }

                // Specify the output path
                lipo_command.arg("-output").arg(multiarch_dylib_dest);

                #[cfg(unix)]
                unsafe {
                    lipo_command.pre_exec(pre_exec_hook);
                }
                let child = lipo_command.spawn().expect(ERR_SPAWN);
                let output = wait_with_output(&process_child_ids, child);
                if !output.status.success() {
                    eprintln!("Failed to combine dylibs with lipo. Aborting.");
                    return Err(());
                }

                //Copy singular device build (iOS, not simulator)
                let device_dylib_src = &device_build[0].1 .1;
                let device_dylib_dest = pax_dir
                    .join(PKG_DIR_NAME)
                    .join("pax-chassis-common")
                    .join("pax-swift-cartridge")
                    .join("PaxCartridge.xcframework")
                    .join(IOS_PACKAGE_ID)
                    .join("PaxCartridge.framework")
                    .join("PaxCartridge");
                let _ = fs::copy(device_dylib_src, device_dylib_dest);
            }

            let (xcodeproj_path, scheme) = if let RunTarget::macOS = target {
                (
                    pax_dir
                        .join(PKG_DIR_NAME)
                        .join("pax-chassis-macos")
                        .join("interface")
                        .join("pax-app-macos")
                        .join("pax-app-macos.xcodeproj"),
                    XCODE_MACOS_TARGET_DEBUG,
                )
            } else {
                (
                    pax_dir
                        .join(PKG_DIR_NAME)
                        .join("pax-chassis-ios")
                        .join("interface")
                        .join("pax-app-ios")
                        .join("pax-app-ios.xcodeproj"),
                    XCODE_IOS_TARGET_DEBUG,
                )
            };

            let configuration = if IS_RELEASE { "Release" } else { "Debug" };

            let build_dest_base = pax_dir.join(BUILD_DIR_NAME).join(BUILD_MODE_NAME);
            let executable_output_dir_path = build_dest_base.join("app");
            let executable_dot_app_path =
                executable_output_dir_path.join(&format!("{}.app", &scheme));
            let _ = fs::create_dir_all(&executable_output_dir_path);

            println!("{} 💻 Building xcodeproject...", *PAX_BADGE);
            let mut cmd = Command::new("xcodebuild");
            cmd.arg("-project")
                .arg(xcodeproj_path)
                .arg("-configuration")
                .arg(configuration)
                .arg("-scheme")
                .arg(scheme)
                .arg(&format!(
                    "CONFIGURATION_BUILD_DIR={}",
                    executable_output_dir_path.to_str().unwrap()
                ))
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::piped());

            if !IS_RELEASE {
                cmd.arg("CODE_SIGNING_REQUIRED=NO")
                    .arg("CODE_SIGN_IDENTITY=");
            }

            if !ctx.verbose {
                cmd.arg("-quiet");
                cmd.arg("GCC_WARN_INHIBIT_ALL_WARNINGS=YES");
            }

            #[cfg(unix)]
            unsafe {
                cmd.pre_exec(pre_exec_hook);
            }
            let child = cmd.spawn().expect(ERR_SPAWN);
            let output = wait_with_output(&process_child_ids, child);

            // Crudely prune out noisy xcodebuild warnings due to an apparent xcode-internal bug at time of authoring, spitting out:
            //   Details:  createItemModels creation requirements should not create capability item model for a capability item model that already exists.
            //       Function: createItemModels(for:itemModelSource:)
            //   Thread:   <_NSMainThread: 0x600000be02c0>{number = 1, name = main}
            //   Please file a bug at https://feedbackassistant.apple.com with this warning message and any useful information you can provide.
            // If we get to a point where xcodebuild isn't spitting these errors, we can drop this block of code and just `.inherit` stderr in
            // the command above.
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if ctx.verbose {
                println!("{}", stderr);
            } else {
                let mut skip_lines = 0;
                for line in stderr.lines() {
                    // Check if this line starts a blacklisted message
                    if line.starts_with("Details:  createItemModels") {
                        skip_lines = 5; // There are 5 lines to skip, including this one
                    }

                    // If skip_lines is non-zero, skip printing and decrement the counter
                    if skip_lines > 0 {
                        skip_lines -= 1;
                        continue;
                    }

                    println!("{}", line);
                }
            }

            if !output.status.success() {
                eprintln!("Failed to build project with xcodebuild. Aborting.");
                return Err(());
            }

            //Copy build artifacts & packages into `build`
            let swift_cart_src = pax_dir
                .join(PKG_DIR_NAME)
                .join("pax-chassis-common")
                .join("pax-swift-cartridge");
            let swift_common_src = pax_dir
                .join(PKG_DIR_NAME)
                .join("pax-chassis-common")
                .join("pax-swift-common");

            let swift_cart_build_dest = build_dest_base
                .join("pax-chassis-common")
                .join("pax-swift-cartridge");
            let swift_common_build_dest = build_dest_base
                .join("pax-chassis-common")
                .join("pax-swift-common");

            let (app_xcodeproj_src, app_xcodeproj_build_dest) = if let RunTarget::macOS = target {
                (
                    pax_dir
                        .join(PKG_DIR_NAME)
                        .join("pax-chassis-macos")
                        .join("interface")
                        .join("pax-app-macos"),
                    build_dest_base
                        .join("pax-chassis-macos")
                        .join("interface")
                        .join("pax-app-macos"),
                )
            } else {
                (
                    pax_dir
                        .join(PKG_DIR_NAME)
                        .join("pax-chassis-ios")
                        .join("interface")
                        .join("pax-app-ios"),
                    build_dest_base
                        .join("pax-chassis-ios")
                        .join("interface")
                        .join("pax-app-ios"),
                )
            };

            let _ = fs::create_dir_all(&swift_cart_build_dest);
            let _ = fs::create_dir_all(&swift_common_build_dest);
            let _ = fs::create_dir_all(&app_xcodeproj_build_dest);

            let _ = copy_dir_recursively(
                &swift_cart_src,
                &swift_cart_build_dest,
                &DIR_IGNORE_LIST_MACOS,
            );
            let _ = copy_dir_recursively(
                &swift_common_src,
                &swift_common_build_dest,
                &DIR_IGNORE_LIST_MACOS,
            );
            let _ = copy_dir_recursively(
                &app_xcodeproj_src,
                &app_xcodeproj_build_dest,
                &DIR_IGNORE_LIST_MACOS,
            );

            // Start  `run` rather than a `build`
            let target_str: &str = target.into();
            if ctx.should_also_run {
                println!("{} 🐇 Running Pax {}...", *PAX_BADGE, target_str);

                if let RunTarget::macOS = target {
                    //
                    // Handle macOS `run`
                    //

                    let system_binary_path =
                        executable_dot_app_path.join(&format!("Contents/MacOS/{}", scheme));

                    let status = Command::new(system_binary_path)
                        .status() // This will wait for the process to complete
                        .expect("failed to execute the app");

                    println!("App exited with: {:?}", status);
                } else {
                    //
                    // Handle iOS `run`
                    //

                    // Get list of devices
                    let mut cmd = Command::new("xcrun");
                    cmd.arg("simctl")
                        .arg("list")
                        .arg("devices")
                        .arg("available")
                        .stdout(std::process::Stdio::piped());

                    #[cfg(unix)]
                    unsafe {
                        cmd.pre_exec(pre_exec_hook);
                    }
                    let child = cmd.spawn().expect(ERR_SPAWN);
                    let output = wait_with_output(&process_child_ids, child);

                    let output_str = std::str::from_utf8(&output.stdout).map_err(|_| ())?;
                    let mut devices: Vec<&str> = output_str
                        .lines()
                        .filter(|line| line.contains("iPhone"))
                        .collect();

                    // Sort to get the newest version
                    devices.sort_by(|a, b| b.cmp(a));

                    // Extract UDID
                    let device_udid_opt = devices
                        .get(0)
                        .and_then(|device| device.split('(').nth(1))
                        .and_then(|udid_with_paren| udid_with_paren.split(')').next());

                    let device_udid = match device_udid_opt {
                        Some(udid) => udid.trim(),
                        None => {
                            return {
                                eprintln!("No installed iOS simulators found on this system.  Install at least one iPhone simulator through xcode and try again.");
                                Err(())
                            }
                        }
                    };

                    // Open the Simulator app
                    let mut cmd = Command::new("open");
                    cmd.arg("-a")
                        .arg("Simulator")
                        .arg("--args")
                        .arg("-CurrentDeviceUDID")
                        .arg(device_udid)
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped());

                    #[cfg(unix)]
                    unsafe {
                        cmd.pre_exec(pre_exec_hook);
                    }
                    let child = cmd.spawn().expect(ERR_SPAWN);
                    let output = wait_with_output(&process_child_ids, child);
                    if !output.status.success() {
                        eprintln!("Error opening iOS simulator. Aborting.");
                        return Err(());
                    }

                    // Boot the relevant simulator
                    let mut cmd = Command::new("xcrun");
                    cmd.arg("simctl")
                        .arg("spawn")
                        .arg(device_udid)
                        .arg("launchctl")
                        .arg("print")
                        .arg("system")
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped());

                    #[cfg(unix)]
                    unsafe {
                        cmd.pre_exec(pre_exec_hook);
                    }
                    let child = cmd.spawn().expect(ERR_SPAWN);
                    let _output = wait_with_output(&process_child_ids, child);
                    // ^ Note that we don't handle errors on this particular command; it will return an error by default
                    // if the simulator isn't running, which isn't an "error" for us.  Instead, defer to the following
                    // polling logic to decide whether the simulator failed to start, which would indeed be an error.

                    // After opening the simulator, wait for the simulator to be booted
                    let max_retries = 5;
                    let retry_period_secs = 5;
                    let mut retries = 0;

                    while !is_simulator_booted(device_udid, &process_child_ids)
                        && retries < max_retries
                    {
                        println!("{} 💤 Waiting for simulator to boot...", *PAX_BADGE);
                        std::thread::sleep(std::time::Duration::from_secs(retry_period_secs));
                        retries = retries + 1;
                    }

                    if retries == max_retries {
                        eprintln!(
                            "Failed to boot the simulator within the expected time. Aborting."
                        );
                        return Err(());
                    }

                    // Install and run app on simulator
                    println!(
                        "{} 📤 Installing and running app from {} on simulator...",
                        *PAX_BADGE,
                        executable_output_dir_path.to_str().unwrap()
                    );

                    let mut cmd = Command::new("xcrun");
                    cmd.arg("simctl")
                        .arg("install")
                        .arg(device_udid)
                        .arg(executable_dot_app_path)
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped());

                    #[cfg(unix)]
                    unsafe {
                        cmd.pre_exec(pre_exec_hook);
                    }
                    let child = cmd.spawn().expect(ERR_SPAWN);
                    let output = wait_with_output(&process_child_ids, child);
                    if !output.status.success() {
                        eprintln!("Error installing app on iOS simulator. Aborting.");
                        return Err(());
                    }

                    let mut cmd = Command::new("xcrun");
                    cmd.arg("simctl")
                        .arg("launch")
                        .arg(device_udid)
                        .arg("dev.pax.pax-app-ios")
                        .stdout(std::process::Stdio::inherit())
                        .stderr(std::process::Stdio::inherit());

                    #[cfg(unix)]
                    unsafe {
                        cmd.pre_exec(pre_exec_hook);
                    }
                    let child = cmd.spawn().expect(ERR_SPAWN);
                    let output = wait_with_output(&process_child_ids, child);
                    if !output.status.success() {
                        eprintln!("Error launching app on iOS simulator. Aborting.");
                        return Err(());
                    }
                    let status = output.status.code().unwrap();

                    println!("App exited with: {:?}", status);
                }
            } else {
                let build_path = executable_output_dir_path.to_str().unwrap().bold();
                println!(
                    "{} 🗂️  Done: {} {} build available at {}",
                    *PAX_BADGE, target_str, BUILD_MODE_NAME, build_path
                );
            }
        }
        RunTarget::Web => {
            let mut cmd = Command::new("wasm-pack");
            cmd.current_dir(&chassis_path)
                .arg("build")
                .arg("--target")
                .arg("web")
                .arg("--out-name")
                .arg("pax-chassis-web")
                .arg("--out-dir")
                .arg(
                    chassis_path
                        .join("interface")
                        .join("public")
                        .to_str()
                        .unwrap(),
                )
                .env("PAX_DIR", &pax_dir)
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit());

            if IS_RELEASE {
                cmd.arg("--release");
            } else {
                cmd.arg("--dev");
            }

            #[cfg(unix)]
            unsafe {
                cmd.pre_exec(pre_exec_hook);
            }

            let child = cmd.spawn().expect(ERR_SPAWN);

            // Execute wasm-pack build
            let output = wait_with_output(&process_child_ids, child);
            if !output.status.success() {
                eprintln!("Failed to build project with wasm-pack. Aborting.");
                return Err(());
            }

            // Copy assets
            let asset_src = pax_dir.join("..").join(ASSETS_DIR_NAME);
            let asset_dest = interface_path.join(PUBLIC_DIR_NAME).join(ASSETS_DIR_NAME);

            // Create target assets directory
            if let Err(e) = fs::create_dir_all(&asset_dest) {
                eprintln!("Error creating directory {:?}: {}", asset_dest, e);
                return Err(());
            }

            // Perform recursive copy from userland `assets/` to built `assets/`
            if let Err(e) = copy_dir_recursively(&asset_src, &asset_dest, &vec![]) {
                eprintln!("Error copying assets: {}", e);
                return Err(());
            }

            //Copy fully built project into .pax/build/web, ready for e.g. publishing
            let build_src = interface_path.join(PUBLIC_DIR_NAME).join(BUILD_MODE_NAME);
            let build_dest = pax_dir
                .join(BUILD_DIR_NAME)
                .join(BUILD_MODE_NAME)
                .join("web");
            let _ = copy_dir_recursively(&build_src, &build_dest, &DIR_IGNORE_LIST_WEB);

            // Start local server if this is a `run` rather than a `build`
            if ctx.should_also_run {
                println!("{} 🐇 Running Pax Web...", *PAX_BADGE);
                let _ = start_static_http_server(interface_path.join(PUBLIC_DIR_NAME));
            } else {
                println!(
                    "{} 🗂️ Done: {} build available at {}",
                    *PAX_BADGE,
                    BUILD_MODE_NAME,
                    build_dest.to_str().unwrap()
                );
            }
        }
    }
    Ok(())
}

// This function checks if the simulator with the given UDID is booted
fn is_simulator_booted(device_udid: &str, process_child_ids: &Arc<Mutex<Vec<u64>>>) -> bool {
    let mut cmd = Command::new("xcrun");
    cmd.arg("simctl")
        .arg("list")
        .arg("devices")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(pre_exec_hook);
    }
    let child = cmd.spawn().expect(ERR_SPAWN);
    let output = wait_with_output(&process_child_ids, child);
    if !output.status.success() {
        panic!("Error checking simulator status. This is an unhandled error and may leave orphaned processes.");
    }

    let output_str = String::from_utf8(output.stdout).expect("Failed to convert to string");
    output_str
        .lines()
        .any(|line| line.contains(device_udid) && line.contains("Booted"))
}

pub fn perform_create(ctx: &CreateContext) {
    let full_path = Path::new(&ctx.path);

    // Abort if directory already exists
    if full_path.exists() {
        panic!("Error: destination `{:?}` already exists", full_path);
    }
    let _ = fs::create_dir_all(&full_path);

    // clone template into full_path
    if ctx.is_libdev_mode {
        //For is_libdev_mode, we copy our monorepo @/pax-compiler/new-project-template directory
        //to the target directly.  This enables iterating on new-project-template during libdev
        //without the sticky caches associated with `include_dir`
        let pax_compiler_cargo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let template_src = pax_compiler_cargo_root.join(PAX_CREATE_TEMPLATE_DIR_NAME);

        let mut options = CopyOptions::new();
        options.overwrite = true;

        for entry in std::fs::read_dir(&template_src).expect("Failed to read template directory") {
            let entry_path = entry.expect("Failed to read entry").path();
            if entry_path.is_dir() {
                dir::copy(&entry_path, &full_path, &options).expect("Failed to copy directory");
            } else {
                fs::copy(&entry_path, full_path.join(entry_path.file_name().unwrap()))
                    .expect("Failed to copy file");
            }
        }
    } else {
        // File src is include_dir — recursively extract files from include_dir into full_path
        PAX_CREATE_TEMPLATE
            .extract(&full_path)
            .expect("Failed to extract files");
    }

    //Patch Cargo.toml
    let cargo_template_path = full_path.join("Cargo.toml.template");
    let extracted_cargo_toml_path = full_path.join("Cargo.toml");
    let _ = fs::copy(&cargo_template_path, &extracted_cargo_toml_path);
    let _ = fs::remove_file(&cargo_template_path);

    let crate_name = full_path.file_name().unwrap().to_str().unwrap().to_string();

    // Read the Cargo.toml
    let mut doc = fs::read_to_string(&full_path.join("Cargo.toml"))
        .expect("Failed to read Cargo.toml")
        .parse::<toml_edit::Document>()
        .expect("Failed to parse Cargo.toml");

    // Update the `dependencies` section
    if let Some(deps) = doc
        .as_table_mut()
        .entry("dependencies")
        .or_insert_with(toml_edit::table)
        .as_table_mut()
    {
        let keys: Vec<String> = deps
            .iter()
            .filter_map(|(key, _)| {
                if key.starts_with("pax-") {
                    Some(key.to_string())
                } else {
                    None
                }
            })
            .collect();

        for key in keys {
            let dep_entry = deps.get_mut(&key).unwrap();

            if let toml_edit::Item::Value(toml_edit::Value::InlineTable(ref mut dep_table)) =
                dep_entry
            {
                // This entry is an inline table, update it

                dep_table.insert(
                    "version",
                    toml_edit::Value::String(toml_edit::Formatted::new(ctx.version.clone())),
                );
            } else {
                // If dependency entry is not a table, create a new table with version and path
                let dependency_string = if ctx.is_libdev_mode {
                    format!(
                        "{{ version=\"{}\", path=\"../{}\", optional=true }}",
                        ctx.version, &key
                    )
                } else {
                    format!("{{ version=\"{}\" }}", ctx.version)
                };

                std::mem::swap(
                    deps.get_mut(&key).unwrap(),
                    &mut toml_edit::Item::from_str(&dependency_string).unwrap(),
                );
            }
        }
    }

    // Update the `package` section
    if let Some(package) = doc
        .as_table_mut()
        .entry("package")
        .or_insert_with(toml_edit::table)
        .as_table_mut()
    {
        if let Some(name_item) = package.get_mut("name") {
            *name_item = toml_edit::Item::Value(crate_name.into());
        }
        if let Some(version_item) = package.get_mut("version") {
            *version_item = toml_edit::Item::Value(ctx.version.clone().into());
        }
    }

    // Write the modified Cargo.toml back to disk
    fs::write(&full_path.join("Cargo.toml"), doc.to_string())
        .expect("Failed to write modified Cargo.toml");

    println!(
        "\nCreated new Pax project at {}.\nTo run:\n  `cd {} && pax-cli run --target=web`",
        full_path.to_str().unwrap(),
        full_path.to_str().unwrap()
    );
}

pub struct CreateContext {
    pub path: String,
    pub is_libdev_mode: bool,
    pub version: String,
}

pub struct RunContext {
    pub target: RunTarget,
    pub path: String,
    pub verbose: bool,
    pub should_also_run: bool,
    pub is_libdev_mode: bool,
    pub process_child_ids: Arc<Mutex<Vec<u64>>>,
}

pub enum RunTarget {
    #[allow(non_camel_case_types)]
    macOS,
    Web,
    #[allow(non_camel_case_types)]
    iOS,
}

impl From<&str> for RunTarget {
    fn from(input: &str) -> Self {
        match input.to_lowercase().as_str() {
            "macos" => RunTarget::macOS,
            "web" => RunTarget::Web,
            "ios" => RunTarget::iOS,
            _ => {
                unreachable!()
            }
        }
    }
}

impl<'a> Into<&'a str> for &'a RunTarget {
    fn into(self) -> &'a str {
        match self {
            RunTarget::Web => "Web",
            RunTarget::macOS => "macOS",
            RunTarget::iOS => "iOS",
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

const ERR_LOCK: &str = "Failed to lock process_child_ids mutex";

pub fn wait_with_output(
    process_child_ids: &Arc<Mutex<Vec<u64>>>,
    child: std::process::Child,
) -> std::process::Output {
    let child_id: u64 = child.id().into();

    // Push the child_id to the shared process_child_ids vector
    process_child_ids.lock().expect(ERR_LOCK).push(child_id);

    // Wait for the child process to complete
    let output = child
        .wait_with_output()
        .expect("Failed to wait for child process");

    // Ensure the child ID is removed after completion
    process_child_ids
        .lock()
        .expect(ERR_LOCK)
        .retain(|&id| id != child_id);

    output
}

#[cfg(unix)]
fn pre_exec_hook() -> Result<(), std::io::Error> {
    // Set a new process group for this command
    unsafe {
        libc::setpgid(0, 0);
    }
    Ok(())
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

        let mut accum: String = "".into();

        self.children.iter().sorted().for_each(|child| {
            if child.1.node_string.as_ref().unwrap() == "crate" {
                //handle crate subtrie by skipping the crate NamespaceTrieNode, traversing directly into its children
                child.1.children.iter().sorted().for_each(|child| {
                    if child.1.children.len() == 0 {
                        //leaf node:  write `pub use ...` entry
                        accum += &format!(
                            "{}pub use {};\n",
                            indent_str,
                            child.1.node_string.as_ref().unwrap()
                        );
                    } else {
                        //non-leaf node:  write `pub mod ...` block
                        accum += &format!(
                            "{}pub mod {} {{\n",
                            indent_str,
                            child
                                .1
                                .node_string
                                .as_ref()
                                .unwrap()
                                .split("::")
                                .last()
                                .unwrap()
                        );
                        accum += &child.1.recurse_serialize_to_reexports(indent + 1);
                        accum += &format!("{}}}\n", indent_str);
                    }
                })
            } else {
                if child.1.children.len() == 0 {
                    //leaf node:  write `pub use ...` entry
                    accum += &format!(
                        "{}pub use {};\n",
                        indent_str,
                        child.1.node_string.as_ref().unwrap()
                    );
                } else {
                    //non-leaf node:  write `pub mod ...` block
                    accum += &format!(
                        "{}pub mod {}{{\n",
                        indent_str,
                        child
                            .1
                            .node_string
                            .as_ref()
                            .unwrap()
                            .split("::")
                            .last()
                            .unwrap()
                    );
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
