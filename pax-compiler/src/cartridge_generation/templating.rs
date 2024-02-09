use include_dir::{include_dir, Dir};
#[allow(unused_imports)]
use pax_runtime::api::serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use serde_json;
use tera::{Context, Tera};

use pax_manifest::{cartridge_generation::{CommonProperty, ComponentInfo}, ExpressionSpec, TypeTable};

static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/cartridge_generation");
static CARTRIDGE_TEMPLATE: &str = "cartridge.tera";
static MACROS_TEMPLATE: &str = "macros.tera";


#[derive(Serialize)]
#[serde(crate = "pax_runtime::api::serde")]
pub struct TemplateArgsCodegenCartridgeLib {
    /// List of fully qualified import strings, e.g. pax_example::pax_reexports::...
    pub imports: Vec<String>,

    /// List of compiled expression specs
    pub expression_specs: Vec<ExpressionSpec>,

    // List of relevant component information for codegen (e.g handlers)
    pub components: Vec<ComponentInfo>,

    // Information about known common properties
    pub common_properties: Vec<CommonProperty>,

    // Information about known types and their properties
    pub type_table: TypeTable,
}

#[allow(unused)]
static TEMPLATE_CODEGEN_CARTRIDGE_LIB: &str =
    include_str!("../../templates/cartridge_generation/cartridge.tera");
pub fn press_template_codegen_cartridge_lib(args: TemplateArgsCodegenCartridgeLib) -> String {
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

    tera.render(
        CARTRIDGE_TEMPLATE,
        &Context::from_serialize(args).unwrap(),
    )
    .expect("Failed to render template")
}