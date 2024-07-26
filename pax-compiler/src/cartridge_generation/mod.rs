//! # Code Generation Module
//!
//! The `code_generation` module provides structures and functions for generating Pax Cartridges
//! from Pax Manifests. The `generate_and_overwrite_cartridge` function is the main entrypoint.

use itertools::Itertools;
use std::fs;

use pax_manifest::{cartridge_generation::CommonProperty, ExpressionSpec, PaxManifest};

use std::path::PathBuf;

pub mod templating;

pub const CARTRIDGE_PARTIAL_PATH: &str = "cartridge.partial.rs";

// Generates (codegens) the PaxCartridge definition, abiding by the PaxCartridge trait.
// Side-effect: writes the generated string to disk as .pax/cartridge.partial.rs,
// so that it may be `include!`d by the  #[pax] #[main] macro
pub fn generate_cartridge_partial_rs(pax_dir: &PathBuf, manifest: &PaxManifest) -> PathBuf {
    #[allow(unused_mut)]
    let mut generated_lib_rs;

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
            cartridge_struct_id: manifest.get_main_cartridge_struct_id(),
            definition_to_instance_traverser_struct_id: manifest
                .get_main_definition_to_instance_traverser_struct_id(),
            expression_specs,
            components: manifest.generate_codegen_component_info(),
            common_properties: CommonProperty::get_as_common_property(),
            type_table: manifest.type_table.clone(),
            is_designtime: cfg!(feature = "designtime"),
            manifest_json: serde_json::to_string(manifest).unwrap(),
        },
    );

    let path = pax_dir.join(CARTRIDGE_PARTIAL_PATH);
    fs::write(path.clone(), generated_lib_rs).unwrap();
    path
}
