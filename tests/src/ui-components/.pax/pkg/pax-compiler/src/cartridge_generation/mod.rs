//! # Code Generation Module
//!
//! The `code_generation` module provides structures and functions for generating Pax Cartridges
//! from Pax Manifests. The `generate_and_overwrite_cartridge` function is the main entrypoint.

use crate::helpers::PKG_DIR_NAME;
use itertools::Itertools;
use std::fs;
use std::str::FromStr;

use pax_manifest::{
    cartridge_generation::CommonProperty, ExpressionSpec, HostCrateInfo, PaxManifest, TypeId,
};

use std::path::PathBuf;
use toml_edit::Item;

pub mod templating;

pub const INITIAL_MANIFEST_FILE_NAME: &str = "initial-manifest.json";

pub fn generate_and_overwrite_cartridge(
    pax_dir: &PathBuf,
    manifest: &PaxManifest,
    host_crate_info: &HostCrateInfo,
) -> PathBuf {
    let target_dir = pax_dir.join(PKG_DIR_NAME).join("pax-cartridge");

    #[allow(unused_mut)]
    let mut generated_lib_rs;

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
    generated_lib_rs = templating::press_template_codegen_cartridge_lib(
        templating::TemplateArgsCodegenCartridgeLib {
            imports,
            expression_specs,
            components: manifest.generate_codegen_component_info(),
            common_properties: CommonProperty::get_as_common_property(),
            type_table: manifest.type_table.clone(),
            is_designtime: cfg!(feature = "designtime"),
        },
    );

    // write manifest to cartridge
    let manifest_path = target_dir.join(INITIAL_MANIFEST_FILE_NAME);
    fs::write(manifest_path, serde_json::to_string(manifest).unwrap()).unwrap();

    // Re: formatting the generated Rust code, see prior art at `_format_generated_lib_rs`
    let path = target_dir.join("src/lib.rs");
    fs::write(path.clone(), generated_lib_rs).unwrap();
    path
}
