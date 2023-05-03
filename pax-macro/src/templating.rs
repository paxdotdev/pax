use sailfish::TemplateOnce;

use serde_derive::{Serialize};
#[allow(unused)]
use serde_json;

use std::collections::HashSet;

#[derive(Serialize)]
pub struct StaticPropertyDefinition {
    pub scoped_resolvable_types: HashSet<String>,
    pub field_name: String,
    pub original_type: String,
}


#[derive(TemplateOnce)]
#[template(path = "../templates/pax_primitive.stpl", escape=false)]
pub struct TemplateArgsMacroPaxPrimitive {
    pub pascal_identifier: String,
    pub original_tokens: String,
    /// Used to codegen get_property_manifest calls, which allows parser to "reflect"
    pub static_property_definitions: Vec<StaticPropertyDefinition>,
    /// For example: "pax_std_primitives::RectangleInstance" for Rectangle (pax_std::primitives::Rectangle)
    pub primitive_instance_import_path: String,
}

#[derive(TemplateOnce)]
#[template(path = "../templates/pax_type.stpl", escape=false)]
pub struct TemplateArgsMacroPaxType {
    pub pascal_identifier: String,
    pub original_tokens: String,
    pub type_dependencies: Vec<String>,
    pub static_property_definitions: Vec<StaticPropertyDefinition>,
    pub should_derive_default: bool,
    pub should_derive_clone: bool,
}

#[derive(TemplateOnce)]
#[template(path = "../templates/pax.stpl", escape=false)]
pub struct TemplateArgsMacroPax {
    pub raw_pax: String,
    pub pascal_identifier: String,
    pub is_root: bool,
    pub template_dependencies: Vec<String>,
    pub static_property_definitions: Vec<StaticPropertyDefinition>,
    pub reexports_snippet: String,
}




//The following `include_str!()` calls allow `rustc` to "dirty-watch" these template files.
//Otherwise, after changing one of those files, the author would also need to change
//something in _this file_ for `rustc` to detect the changes and recompile the included
//template file.
//
// static TEMPLATE_PAX_PRIMITIVE : &str = include_str!("../templates/pax_primitive.stpl");
// pub fn press_template_macro_pax_primitive(args: TemplateArgsMacroPaxPrimitive ) -> String {
//     let template = TEMPLATE_DIR.get_file("macros/pax_primitive.tera").unwrap().contents_utf8().unwrap();
//     Tera::one_off(template.into(), &tera::Context::from_serialize(args).unwrap(), false).unwrap()
// }
//
// static TEMPLATE_PAX_TYPE : &str = include_str!("../templates/pax_type.stpl");
// pub fn press_template_macro_pax_type(args: TemplateArgsMacroPaxType ) -> String {
//     let template = TEMPLATE_DIR.get_file("macros/pax_type.tera").unwrap().contents_utf8().unwrap();
//     Tera::one_off(template.into(), &tera::Context::from_serialize(args).unwrap(), false).unwrap()
// }
//
//
// static TEMPLATE_PAX : &str = include_str!("../templates/pax.stpl");
// pub fn press_template_macro_pax(args: TemplateArgsMacroPax) -> String {
//     let template = TEMPLATE_DIR.get_file("macros/pax.tera").unwrap().contents_utf8().unwrap();
//     Tera::one_off(template.into(), &tera::Context::from_serialize(args).unwrap(), false).unwrap()
// }