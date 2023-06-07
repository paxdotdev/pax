use sailfish::TemplateOnce;

use serde_derive::{Serialize};
use serde_json;

use std::collections::HashSet;

#[derive(Serialize)]
pub struct StaticPropertyDefinition {
    pub scoped_resolvable_types: Vec<String>,
    pub root_scoped_resolvable_type: String,
    pub field_name: String,
    pub original_type: String,
    pub pascal_identifier: String,
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
    pub include_imports: bool,
}

#[derive(TemplateOnce)]
#[template(path = "../templates/pax_type.stpl", escape=false)]
pub struct TemplateArgsMacroPaxType {
    pub pascal_identifier: String,
    pub type_dependencies: Vec<String>,
    pub static_property_definitions: Vec<StaticPropertyDefinition>,
    pub include_imports: bool,
}

#[derive(TemplateOnce)]
#[template(path = "../templates/pax.stpl", escape=false)]
pub struct TemplateArgsMacroPax {
    pub raw_pax: String,
    pub pascal_identifier: String,
    pub is_main_component: bool,
    pub template_dependencies: Vec<String>,
    pub static_property_definitions: Vec<StaticPropertyDefinition>,
    pub reexports_snippet: String,
}