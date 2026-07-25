use include_dir::{include_dir, Dir};
#[allow(unused_imports)]
use pax_runtime::api::serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use serde_json;
use std::collections::HashMap;
use tera::{Context, Tera};

use pax_manifest::{
    cartridge_generation::{CommonProperty, ComponentInfo},
    TypeTable,
};

static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/cartridge_generation");
static CARTRIDGE_TEMPLATE: &str = "cartridge.tera";
static MACROS_TEMPLATE: &str = "macros.tera";

#[serde_with::serde_as]
#[derive(Serialize)]
pub struct TemplateArgsCodegenCartridgeSnippet {
    /// Identifier (name) for the cartridge struct to generate, i.e. the
    /// struct that will implement PaxCartridge
    pub cartridge_struct_id: String,

    pub definition_to_instance_traverser_struct_id: String,

    // List of relevant component information for codegen (e.g handlers)
    pub components: Vec<ComponentInfo>,

    // Information about known common properties
    pub common_properties: Vec<CommonProperty>,

    // Information about known types and their properties
    #[serde_as(as = "HashMap<serde_with::json::JsonString, _>")]
    pub type_table: TypeTable,

    // Whether this is a designtime cartridge
    pub is_designtime: bool,

    // JSON string representation of the manifest, used at least for designtime builds
    pub userland_manifest_json: String,

    // Rust expression that constructs the manifest directly, used for lean release builds
    pub userland_manifest_rust: String,

    // Whether the generated cartridge should construct the manifest from generated Rust
    pub use_rust_manifest: bool,

    /// Customizable import path for pax_engine, for codegen
    pub engine_import_path: String,
}

#[allow(unused)]
static TEMPLATE_CODEGEN_CARTRIDGE_SNIPPET: &str =
    include_str!("../../templates/cartridge_generation/cartridge.tera");
pub fn press_template_codegen_cartridge_snippet(
    args: TemplateArgsCodegenCartridgeSnippet,
) -> String {
    let mut tera = Tera::default();
    tera.add_raw_template(
        MACROS_TEMPLATE,
        TEMPLATE_DIR
            .get_file(MACROS_TEMPLATE)
            .unwrap()
            .contents_utf8()
            .unwrap(),
    )
    .expect("Failed to add macros.tera");

    tera.add_raw_template(
        CARTRIDGE_TEMPLATE,
        TEMPLATE_DIR
            .get_file(CARTRIDGE_TEMPLATE)
            .unwrap()
            .contents_utf8()
            .unwrap(),
    )
    .expect("Failed to add cartridge.tera");

    tera.render(CARTRIDGE_TEMPLATE, &Context::from_serialize(args).unwrap())
        .expect("Failed to render template")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cartridge_args(
        is_designtime: bool,
        use_rust_manifest: bool,
    ) -> TemplateArgsCodegenCartridgeSnippet {
        TemplateArgsCodegenCartridgeSnippet {
            cartridge_struct_id: "TestCartridge".to_string(),
            definition_to_instance_traverser_struct_id: "TestTraverser".to_string(),
            components: Vec::new(),
            common_properties: Vec::new(),
            type_table: TypeTable::default(),
            is_designtime,
            userland_manifest_json: "{}".to_string(),
            userland_manifest_rust: "pax_engine::pax_manifest::PaxManifest::default()".to_string(),
            use_rust_manifest,
            engine_import_path: "pax_engine".to_string(),
        }
    }

    #[test]
    fn generated_cartridge_owns_lint_scope_in_debug_and_release_modes() {
        for (is_designtime, use_rust_manifest) in [(true, false), (false, true)] {
            let generated = press_template_codegen_cartridge_snippet(cartridge_args(
                is_designtime,
                use_rust_manifest,
            ));

            syn::parse_file(&generated).expect("generated cartridge should remain valid Rust");
            assert!(generated.contains(
                "#[allow(dead_code, non_snake_case, non_upper_case_globals, unused_imports, unused_variables)]\nmod __pax_generated_cartridge"
            ));
            assert!(generated.contains(
                "use __pax_generated_cartridge::{init_definition_to_instance_traverser, init_manifest};"
            ));
            assert!(generated.contains("std::cell::Ref<'_, pax_manifest::PaxManifest>"));
            assert!(!generated.contains("std::cell::Ref<pax_manifest::PaxManifest>"));

            if use_rust_manifest {
                assert!(generated.contains("pax_engine::pax_manifest::PaxManifest::default()"));
                assert!(!generated.contains("userland_manifest_json"));
            } else {
                assert!(generated.contains("userland_manifest_json"));
                assert!(generated.contains("_project_query"));
            }
        }

        let macros = TEMPLATE_DIR
            .get_file(MACROS_TEMPLATE)
            .expect("cartridge macros template should exist")
            .contents_utf8()
            .expect("cartridge macros template should be UTF-8");
        assert!(macros.contains("if let Ok(properties)"));
        assert!(!macros.contains("if let Ok(mut properties)"));
    }
}
