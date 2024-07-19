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

pub const CARTRIDGE_PARTIAL_PATH: &str = "cartridge.partial.rs";

// Generates (codegens) the PaxCartridge definition, abiding by the PaxCartridge trait.
// Side-effect: writes the generated string to disk as .pax/cartridge.partial.rs,
// so that it may be `include!`d by the  #[pax] #[main] macro
pub fn generate_cartridge_partial_rs(
    pax_dir: &PathBuf,
    manifest: &PaxManifest,
    host_crate_info: &HostCrateInfo,
) -> PathBuf {

    #[allow(unused_mut)]
        let mut generated_lib_rs;

    // let target_cargo_full_path = fs::canonicalize(target_dir.join("Cargo.toml")).unwrap();
    // let mut target_cargo_toml_contents =
    //     toml_edit::Document::from_str(&fs::read_to_string(&target_cargo_full_path).unwrap())
    //         .unwrap();
    //
    // //write patched Cargo.toml
    // fs::write(
    //     &target_cargo_full_path,
    //     &target_cargo_toml_contents.to_string(),
    // )
    //     .unwrap();


    let mut expression_specs: Vec<ExpressionSpec> = manifest
        .expression_specs
        .as_ref()
        .unwrap()
        .values()
        .map(|es: &ExpressionSpec| es.clone())
        .collect();
    expression_specs = expression_specs.iter().sorted().cloned().collect();

    //press template into String
    generated_lib_rs = templating::press_template_codegen_cartridge_snippet(
        templating::TemplateArgsCodegenCartridgeSnippet {
            cartridge_struct_id: format!("{}{}", &manifest.main_component_type_id.get_pascal_identifier().unwrap(), "Cartridge"),
            expression_specs,
            components: manifest.generate_codegen_component_info(),
            common_properties: CommonProperty::get_as_common_property(),
            type_table: manifest.type_table.clone(),
            is_designtime: cfg!(feature = "designtime"),
            manifest_json: serde_json::to_string(manifest).unwrap(),
        },
    );

    // write manifest to fs
    // let manifest_path = pax_dir.join(INITIAL_MANIFEST_FILE_NAME);
    // let manifest_str = serde_json::to_string(manifest).unwrap();
    // fs::write(manifest_path, manifest_str).unwrap();

    // Re: formatting the generated Rust code, see prior art at `_format_generated_lib_rs`
    let path = pax_dir.join(CARTRIDGE_PARTIAL_PATH);
    fs::write(path.clone(), generated_lib_rs).unwrap();
    path
}
