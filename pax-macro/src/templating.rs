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

#[derive(Clone, Copy, Debug, Serialize)]
pub struct TemplateBuildConfig {
    pub web: bool,
    pub macos: bool,
    pub ios: bool,
    pub ipados: bool,
    pub designtime: bool,
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
    pub is_custom_coercion_rules: bool,
    pub can_derive_identity_roundtrip: bool,
    pub is_root_crate: bool,
    pub _is_enum: bool,
    pub build_config: TemplateBuildConfig,

    /// Used to specify a custom import prefix for codegen when importing
    /// `pax-engine` directly rather than through `pax-kit`.
    pub engine_import_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_main(root: bool, target: &str, designtime: bool) -> String {
        TemplateArgsDerivePax {
            args_full_component: Some(ArgsFullComponent {
                is_main_component: true,
                cartridge_snippet: "const CARTRIDGE_OWNER: bool = true;".into(),
            }),
            internal_definitions: InternalDefinitions::Struct(vec![]),
            pascal_identifier: "EmbeddedExample".into(),
            is_custom_interpolatable: false,
            is_custom_coercion_rules: false,
            can_derive_identity_roundtrip: false,
            is_root_crate: root,
            _is_enum: false,
            build_config: TemplateBuildConfig {
                web: target == "web",
                macos: target == "macos",
                ios: target == "ios",
                ipados: target == "ipados",
                designtime,
            },
            engine_import_path: "pax_engine".into(),
        }
        .render_once()
        .unwrap()
    }

    #[test]
    fn dependency_main_remains_a_component_without_platform_entrypoints() {
        for target in ["web", "macos", "ios", "ipados"] {
            for designtime in [true, false] {
                let generated = render_main(false, target, designtime);
                syn::parse_file(&generated).unwrap();
                assert!(generated.contains("Interpolatable for EmbeddedExample"));
                assert!(!generated.contains("CARTRIDGE_OWNER"));
                assert!(!generated.contains("fn pax_init"));
                assert!(!generated.contains("init_manifest"));
                assert!(!generated.contains("pub use pax_engine::pax_chassis"));
            }
        }
    }

    #[test]
    fn application_main_keeps_debug_and_release_entrypoints() {
        for target in ["web", "macos", "ios", "ipados"] {
            for designtime in [true, false] {
                let generated = render_main(true, target, designtime);
                syn::parse_file(&generated).unwrap();
                assert!(generated.contains("CARTRIDGE_OWNER"));
                assert_eq!(generated.matches("fn pax_init").count(), 1);
                assert!(generated.contains("init_manifest"));
                let debug_constructor = if target == "web" {
                    "new_designtime"
                } else {
                    "new_with_designtime"
                };
                assert_eq!(generated.contains(debug_constructor), designtime);
            }
        }
    }
}
