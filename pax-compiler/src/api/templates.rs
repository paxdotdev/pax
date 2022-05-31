
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
pub struct TemplateArgsMacroPaxRoot {
    pub raw_pax: String,
    pub pascal_identifier: String,
    pub original_tokens: String,
    pub dependencies: Vec<String>,
}


pub fn press_template_macro_pax_primitive(args: TemplateArgsMacroPaxPrimitive ) -> String {
    let template = TEMPLATE_DIR.get_file("macros/pax_primitive").unwrap().contents_utf8().unwrap();
    Tera::one_off(template.into(), &tera::Context::from_serialize(args).unwrap(), false).unwrap()
}


pub fn press_template_macro_pax_root(args: TemplateArgsMacroPaxRoot ) -> String {
    let template = TEMPLATE_DIR.get_file("macros/pax_root").unwrap().contents_utf8().unwrap();
    Tera::one_off(template.into(), &tera::Context::from_serialize(args).unwrap(), false).unwrap()
}