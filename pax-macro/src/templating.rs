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

#[derive(Serialize, Debug)]
pub struct EnumVariantDefinition {
    pub variant_name: String,
    pub variant_fields: Vec<StaticPropertyDefinition>,
}

pub enum InternalDefinitions {
    Struct(Vec<StaticPropertyDefinition>),
    Enum(Vec<EnumVariantDefinition>),
}

#[derive(Serialize)]
pub struct ArgsFullComponent {
    pub is_main_component: bool,
    pub cartridge_snippet: String,
}

#[derive(TemplateOnce)]
#[template(path = "../templates/derive_pax.stpl", escape = false)]
pub struct TemplateArgsDerivePax {
    /// Modal properties
    pub args_full_component: Option<ArgsFullComponent>,

    /// Shared properties
    pub internal_definitions: InternalDefinitions,
    pub pascal_identifier: String,
    pub is_custom_interpolatable: bool,
    pub is_root_crate: bool,
    pub _is_enum: bool,

    /// Used to specify a custom import prefix for codegen, if importing pax_engine
    /// via anything other than pax_kit::pax_engine (e.g. for pax-std and pax-designer, which
    /// import pax_engine directly)
    pub engine_import_path: String,
}
