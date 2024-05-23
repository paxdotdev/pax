use std::path::PathBuf;

use sailfish::TemplateOnce;

use serde_derive::Serialize;

#[derive(Serialize, Debug)]
pub struct StaticPropertyDefinition {
    pub scoped_resolvable_types: Vec<String>,
    pub root_scoped_resolvable_type: String,
    pub field_name: String,
    pub original_type: String,
    pub pascal_identifier: String,
    pub is_property_wrapped: bool,
    pub is_enum: bool,
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
    pub associated_pax_file_path: Option<PathBuf>,
    pub error_message: Option<String>,
}

#[derive(TemplateOnce)]
#[template(path = "../templates/derive_pax.stpl", escape = false)]
pub struct TemplateArgsDerivePax {
    /// Modal properties
    pub args_primitive: Option<ArgsPrimitive>,
    pub args_struct_only_component: Option<ArgsStructOnlyComponent>,
    pub args_full_component: Option<ArgsFullComponent>,

    /// Shared properties
    pub static_property_definitions: Vec<StaticPropertyDefinition>,
    pub pascal_identifier: String,
    pub is_custom_interpolatable: bool,
}
