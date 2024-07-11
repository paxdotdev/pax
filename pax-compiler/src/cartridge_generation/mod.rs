//! # Code Generation Module
//!
//! The `code_generation` module provides structures and functions for generating Pax Cartridges
//! from Pax Manifests.

use itertools::Itertools;
use std::fs;
use std::str::FromStr;
use pax_manifest::{ExpressionSpec, HostCrateInfo, PaxManifest, TypeId};
use pax_manifest::cartridge_generation::CommonProperty;

pub mod templating;

pub fn generate_cartridge_as_string(
    manifest: &PaxManifest,
    host_crate_info: &HostCrateInfo,
) -> String {

    let mut imports: Vec<String> = manifest
        .import_paths
        .iter()
        .map(|e| {
            TypeId::build_singleton(e, None)
                .fully_qualify_type_id(host_crate_info)
                .import_path()
                .unwrap()
        })
        .collect();

    imports.append(
        &mut pax_manifest::IMPORTS_BUILTINS
            .into_iter()
            .map(|ib| ib.to_string())
            .collect::<Vec<String>>(),
    );

    let mut expression_specs: Vec<ExpressionSpec> = manifest
        .expression_specs
        .as_ref()
        .unwrap()
        .values()
        .map(|es: &ExpressionSpec| es.clone())
        .collect();
    expression_specs = expression_specs.iter().sorted().cloned().collect();

    //press template into String
    let generated_tokens = templating::press_template_codegen_cartridge_lib(
        templating::TemplateArgsCodegenCartridgeLib {
            imports,
            expression_specs,
            components: manifest.generate_codegen_component_info(),
            common_properties: CommonProperty::get_as_common_property(),
            type_table: manifest.type_table.clone(),
            is_designtime: cfg!(feature = "designtime"),
        },
    );

    generated_tokens
    // write manifest to cartridge
    // let manifest_path = target_dir.join(INITIAL_MANIFEST_FILE_NAME);
    // let manifest_string = serde_json::to_string(manifest).unwrap();
    // // fs::write(manifest_path, manifest_str).unwrap();
    // // Re: formatting the generated Rust code, see prior art at `_format_generated_lib_rs`
    // // let path = target_dir.join("src/lib.rs");
    // // fs::write(path.clone(), generated_lib_rs).unwrap();
    // // path
    // manifest_string
}