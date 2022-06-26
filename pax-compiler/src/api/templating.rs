
use serde_derive::Serialize;
use serde_json;
use include_dir::{include_dir, Dir};
use tera::{Context, Tera};


static ROOT_PATH : &str = "$CARGO_MANIFEST_DIR/templates";
static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");



#[derive(Serialize)]
pub struct TemplateArgsMacroPaxPrimitive {
    pub pascal_identifier: String,
    pub original_tokens: String,

}


#[derive(Serialize)]
pub struct TemplateArgsMacroPax {
    pub raw_pax: String,
    pub pascal_identifier: String,
    pub original_tokens: String,
    pub is_root: bool,
    pub dependencies: Vec<String>,

    /// List of tuples of local Property types
    /// `(a,b)` where `a is the identifier (name) of the property, and
    /// `b` is the type of that property, qualified to local namespace (as Pascal identifiers or primitive type identifiers)
    /// Used to codegen get_property_manifest calls, which allows parser binary to "reflect"
    pub local_property_definitions: Vec<(String, String)>,

    pub pub_mod_types: String,
}

static TEMPLATE_PAX_PRIMITIVE : &str = include_str!("../../templates/macros/pax_primitive");
pub fn press_template_macro_pax_primitive(args: TemplateArgsMacroPaxPrimitive ) -> String {
    let template = TEMPLATE_DIR.get_file("macros/pax_primitive").unwrap().contents_utf8().unwrap();
    Tera::one_off(template.into(), &tera::Context::from_serialize(args).unwrap(), false).unwrap()
}

//Included to allows `rustc` to "dirty-watch" these template files.
//Otherwise, after changing one of those files, the author must also change
//something in _this file_ for `rustc` to detect the changes and recompile the included
//template file.
static TEMPLATE_PAX : &str = include_str!("../../templates/macros/pax");
pub fn press_template_macro_pax_root(args: TemplateArgsMacroPax) -> String {
    let template = TEMPLATE_DIR.get_file("macros/pax").unwrap().contents_utf8().unwrap();
    Tera::one_off(template.into(), &tera::Context::from_serialize(args).unwrap(), false).unwrap()
}