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
#[serde(crate = "pax_runtime::api::serde")]
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
    pub manifest_json: String,
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
