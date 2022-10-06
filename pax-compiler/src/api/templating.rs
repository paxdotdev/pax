
use serde_derive::{Serialize, Deserialize};
use serde_json;
use include_dir::{include_dir, Dir};
use tera::{Context, Tera};
use std::collections::HashSet;

use super::ExpressionSpec;

static ROOT_PATH : &str = "$CARGO_MANIFEST_DIR/templates";
static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

#[derive(Serialize)]
pub struct TemplateArgsMacroPaxPrimitive {
    pub pascal_identifier: String,
    pub original_tokens: String,
    /// Used to codegen get_property_manifest calls, which allows parser to "reflect"
    pub compile_time_property_definitions: Vec<CompileTimePropertyDefinition>,
}

#[derive(Serialize)]
pub struct TemplateArgsMacroPaxType {
    pub pascal_identifier: String,
    pub original_tokens: String,
}

#[derive(Serialize)]
pub struct CompileTimePropertyDefinition {
    pub scoped_resolvable_types: HashSet<String>,
    pub field_name: String,
    pub original_type: String,
}

#[derive(Serialize)]
pub struct TemplateArgsCodegenPropertiesCoproductLib {
    //e.g. `Rectangle(pax_example::pax_reexports::pax_std::primitives::Rectangle)`
    //      |-------| |--------------------------------------------------------|
    //      tuple.0   tuple.1
    pub properties_coproduct_tuples: Vec<(String, String)>,

    //e.g. `Stroke(    pax_example::pax_reexports::pax_std::types::Stroke)`
    //      |----|     |--------------------------------------------------------|
    //      tuple.0    tuple.1
    pub types_coproduct_tuples: Vec<(String, String)>,
}

#[derive(Serialize)]
pub struct TemplateArgsMacroPax {
    pub raw_pax: String,
    pub pascal_identifier: String,
    pub original_tokens: String,
    pub is_root: bool,
    pub template_dependencies: Vec<String>,
    pub compile_time_property_definitions: Vec<CompileTimePropertyDefinition>,
    pub reexports_snippet: String,
}

#[derive(Serialize)]
pub struct TemplateArgsCodegenCartridgeLib {
    /// List of fully qualified import strings, e.g. pax_example::pax_reexports::...
    pub imports: Vec<String>,

    /// List of fully qualified primitive Instance imports, as annotated by `pax_primitive` macro
    pub primitive_imports: Vec<String>,

    /// List of `const `declarations: full token streams ready to re-write
    pub consts: Vec<String>,

    /// List of compiled expression specs
    pub expression_specs: Vec<ExpressionSpec>,
}




//The following `include_str!()` calls allow `rustc` to "dirty-watch" these template files.
//Otherwise, after changing one of those files, the author would also need to change
//something in _this file_ for `rustc` to detect the changes and recompile the included
//template file.

static TEMPLATE_PAX_PRIMITIVE : &str = include_str!("../../templates/macros/pax_primitive.tera");
pub fn press_template_macro_pax_primitive(args: TemplateArgsMacroPaxPrimitive ) -> String {
    let template = TEMPLATE_DIR.get_file("macros/pax_primitive.tera").unwrap().contents_utf8().unwrap();
    Tera::one_off(template.into(), &tera::Context::from_serialize(args).unwrap(), false).unwrap()
}

static TEMPLATE_PAX_TYPE : &str = include_str!("../../templates/macros/pax_type.tera");
pub fn press_template_macro_pax_type(args: TemplateArgsMacroPaxType ) -> String {
    let template = TEMPLATE_DIR.get_file("macros/pax_type.tera").unwrap().contents_utf8().unwrap();
    Tera::one_off(template.into(), &tera::Context::from_serialize(args).unwrap(), false).unwrap()
}

static TEMPLATE_CODEGEN_PROPERTIES_COPRODUCT_LIB : &str = include_str!("../../templates/codegen/properties-coproduct-lib.tera");
pub fn press_template_codegen_properties_coproduct_lib(args: TemplateArgsCodegenPropertiesCoproductLib ) -> String {
    let template = TEMPLATE_DIR.get_file("codegen/properties-coproduct-lib.tera").unwrap().contents_utf8().unwrap();
    Tera::one_off(template.into(), &tera::Context::from_serialize(args).unwrap(), false).unwrap()
}

static TEMPLATE_CODEGEN_CARTRIDGE_LIB : &str = include_str!("../../templates/codegen/cartridge-lib.tera");
pub fn press_template_codegen_cartridge_lib(args: TemplateArgsCodegenCartridgeLib ) -> String {
    let template = TEMPLATE_DIR.get_file("codegen/cartridge-lib.tera").unwrap().contents_utf8().unwrap();
    Tera::one_off(template.into(), &tera::Context::from_serialize(args).unwrap(), false).unwrap()
}

static TEMPLATE_PAX : &str = include_str!("../../templates/macros/pax.tera");
pub fn press_template_macro_pax(args: TemplateArgsMacroPax) -> String {
    let template = TEMPLATE_DIR.get_file("macros/pax.tera").unwrap().contents_utf8().unwrap();
    Tera::one_off(template.into(), &tera::Context::from_serialize(args).unwrap(), false).unwrap()
}
