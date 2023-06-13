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

#[derive(Serialize)]
pub struct ArgsPrimitive {
    /// For example: "pax_std_primitives::RectangleInstance" for Rectangle (pax_std::primitives::Rectangle)
    pub primitive_instance_import_path: String,
}

#[derive(Serialize)]
pub struct ArgsStructOnlyComponent {}

#[derive(Serialize)]
pub struct ArgsFullComponent {
    pub raw_pax: String,
    pub is_main_component: bool,
    pub template_dependencies: Vec<String>,
    pub reexports_snippet: String,
}

#[derive(TemplateOnce)]
#[template(path = "../templates/derive_pax.stpl", escape=false)]
pub struct TemplateArgsDerivePax {
    /// Modal properties
    pub args_primitive: Option<ArgsPrimitive>,
    pub args_struct_only_component: Option<ArgsStructOnlyComponent>,
    pub args_full_component: Option<ArgsFullComponent>,

    /// Shared properties
    pub static_property_definitions: Vec<StaticPropertyDefinition>,
    pub pascal_identifier: String,
    pub include_imports: bool,
    pub is_custom_interpolatable: bool,
}