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

#[derive(TemplateOnce)]
#[template(path = "../templates/cartridge.stpl", escape = false)]
pub struct TemplateArgsCartridge {
    pub catalog: bool,
    pub loader: Option<String>,
    pub loader_snippet: String,
    pub web_entrypoint: String,
    pub native_exports: pax_codegen::project_layout::NativeExports,
    pub root_path: String,
    pub initializer: String,
    pub cartridge_snippet: String,
    pub engine_import_path: String,
    pub build_config: TemplateBuildConfig,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct TemplateBuildConfig {
    pub web: bool,
    pub macos: bool,
    pub ios: bool,
    pub ipados: bool,
    pub windows: bool,
    pub linux: bool,
    pub designtime: bool,
}

#[derive(TemplateOnce)]
#[template(path = "../templates/derive_pax.stpl", escape = false)]
pub struct TemplateArgsDerivePax {
    /// Modal properties
    pub cartridge: String,

    /// Shared properties
    pub internal_definitions: InternalDefinitions,
    pub pascal_identifier: String,
    pub is_custom_interpolatable: bool,
    pub is_custom_coercion_rules: bool,
    pub can_derive_identity_roundtrip: bool,
    pub _is_enum: bool,

    /// Used to specify a custom import prefix for codegen when importing
    /// `pax-engine` directly rather than through `pax-kit`.
    pub engine_import_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_main(root: bool, target: &str, designtime: bool) -> String {
        render_main_catalog(root, target, designtime, false)
    }

    fn render_main_catalog(root: bool, target: &str, designtime: bool, catalog: bool) -> String {
        TemplateArgsDerivePax {
            cartridge: if root {
                TemplateArgsCartridge {
                    catalog,
                    loader: None,
                    loader_snippet: String::new(),
                    web_entrypoint: "init_web".into(),
                    native_exports: pax_codegen::project_layout::NativeExports::new(
                        "fixture@1",
                        "ui",
                    ),
                    root_path: "EmbeddedExample".into(),
                    initializer: "example_init".into(),
                    cartridge_snippet: "const CARTRIDGE_OWNER: bool = true;".into(),
                    engine_import_path: "pax_engine".into(),
                    build_config: TemplateBuildConfig {
                        web: target == "web",
                        macos: target == "macos",
                        ios: target == "ios",
                        ipados: target == "ipados",
                        windows: target == "windows",
                        linux: target == "linux",
                        designtime,
                    },
                }
                .render_once()
                .unwrap()
            } else {
                String::new()
            },
            internal_definitions: InternalDefinitions::Struct(vec![]),
            pascal_identifier: "EmbeddedExample".into(),
            is_custom_interpolatable: false,
            is_custom_coercion_rules: false,
            can_derive_identity_roundtrip: false,
            _is_enum: false,
            engine_import_path: "pax_engine".into(),
        }
        .render_once()
        .unwrap()
    }

    #[test]
    fn dependency_main_remains_a_component_without_platform_entrypoints() {
        for target in ["web", "macos", "ios", "ipados", "windows", "linux"] {
            for designtime in [true, false] {
                let generated = render_main(false, target, designtime);
                syn2::parse_file(&generated).unwrap();
                assert!(generated.contains("Interpolatable for EmbeddedExample"));
                assert!(!generated.contains("CARTRIDGE_OWNER"));
                assert!(!generated.contains("fn pax_init"));
                assert!(!generated.contains("fn init_web"));
                assert!(!generated.contains("init_manifest"));
                assert!(!generated.contains("example_init"));
                assert!(!generated.contains("pub use pax_engine::pax_chassis"));
            }
        }
    }

    #[test]
    fn native_adapters_export_only_the_selected_library_names() {
        let exports = pax_codegen::project_layout::NativeExports::new("fixture@1", "ui");
        for target in ["macos", "ios", "ipados", "windows", "linux"] {
            for designtime in [true, false] {
                let generated = render_main(true, target, designtime);
                syn2::parse_file(&generated).unwrap();
                assert!(!generated.contains("#[no_mangle]"));
                for name in [
                    &exports.abi_version,
                    &exports.init,
                    &exports.init_with_data,
                    &exports.init_entry,
                ] {
                    assert_eq!(
                        generated
                            .matches(&format!("#[unsafe(export_name = {name:?})]"))
                            .count(),
                        1
                    );
                }
                assert!(!generated.contains(&exports.program_id)); // Emitted with program contents.
            }
        }
        assert!(!render_main(true, "web", false).contains("export_name"));
    }

    #[test]
    fn desktop_target_facts_do_not_select_an_apple_adapter() {
        for target in ["windows", "linux"] {
            for designtime in [true, false] {
                let generated = render_main(true, target, designtime);
                syn2::parse_file(&generated).unwrap();
                assert!(generated.contains("CARTRIDGE_OWNER"));
                assert!(!generated.contains("core_graphics_c_bridge"));
                assert!(!generated.contains("OS::Mac"));
                assert!(!generated.contains("OS::IPhone"));
                assert!(!generated.contains("OS::IPad"));
                assert!(!generated.contains("fn init_web"));
                assert!(generated.contains("fn pax_cartridge_abi_version() -> u32 { 4 }"));
                assert!(generated.contains("DesktopEngineContainer"));
                assert!(generated.contains(&format!("pax_chassis_{target}::create_backend")));
                assert_eq!(
                    generated.contains("DesktopEngineContainer::new_designtime"),
                    designtime
                );
            }
        }
    }

    #[test]
    fn application_main_keeps_debug_and_release_entrypoints() {
        for target in ["web", "macos", "ios", "ipados", "windows", "linux"] {
            for designtime in [true, false] {
                let generated = render_main(true, target, designtime);
                syn2::parse_file(&generated).unwrap();
                assert!(generated.contains("CARTRIDGE_OWNER"));
                assert_eq!(
                    generated
                        .matches(if target == "web" {
                            "fn init_web"
                        } else {
                            "fn pax_init("
                        })
                        .count(),
                    1
                );
                assert!(generated.contains("init_manifest"));
                assert!(generated.contains("application.initialize(example_init)"));
                let debug_constructor = if target == "web" {
                    "new_designtime"
                } else {
                    "new_with_designtime"
                };
                assert_eq!(generated.contains(debug_constructor), designtime);
            }
        }
    }

    #[test]
    fn native_initialization_moves_selected_manifest_and_preserves_root() {
        // Compile the generated initialization statements with an observable
        // manifest clone. Parsing alone cannot catch use-after-move or a lost
        // catalog selection, and substring checks cannot establish ownership.
        let mut cases = String::new();
        for target in ["macos", "ios", "ipados", "windows", "linux"] {
            for designtime in [true, false] {
                for catalog in [false, true] {
                    let generated = render_main_catalog(true, target, designtime, catalog);
                    syn2::parse_file(&generated).unwrap();
                    let start = generated
                        .find(if designtime {
                            "let userland_manifest ="
                        } else {
                            "let manifest ="
                        })
                        .unwrap();
                    let end = start
                        + generated[start..]
                            .find(if designtime {
                                "let designtime_manager ="
                            } else {
                                "let start ="
                            })
                            .unwrap();
                    let statements = &generated[start..end];
                    syn2::parse_str::<syn2::Block>(&format!("{{ {statements} }}")).unwrap();
                    assert!(
                        generated
                            .find("application.initialize(example_init)")
                            .unwrap()
                            < start
                    );
                    if catalog {
                        assert!(generated.find("__pax_select_entry").unwrap() < start);
                    }
                    let result = if designtime {
                        "userland_main_component_instance"
                    } else {
                        "main_component_instance"
                    };
                    let selected = if catalog {
                        "catalog-selected"
                    } else {
                        "default-main"
                    };
                    let lookup = if designtime {
                        "USERLAND_COMPONENT_ROOT"
                    } else {
                        selected
                    };
                    cases.push_str(&format!(r#"
                        {{
                            CLONES.store(0, std::sync::atomic::Ordering::Relaxed);
                            let entry = "catalog-selected";
                            let root = 73_u32;
                            {statements}
                            assert_eq!({result}, ({selected:?}.to_string(), root, {lookup:?}.to_string()),
                                "{target} debug={designtime} catalog={catalog}");
                            assert_eq!(CLONES.load(std::sync::atomic::Ordering::Relaxed), 0,
                                "manifest copied: {target} debug={designtime} catalog={catalog}");
                        }}
                    "#));
                }
            }
        }
        let source = format!(
            r#"
            #![allow(unused_variables)]
            static CLONES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            struct Manifest {{ main_component_type_id: String }}
            impl Clone for Manifest {{
                fn clone(&self) -> Self {{
                    CLONES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Self {{ main_component_type_id: self.main_component_type_id.clone() }}
                }}
            }}
            struct Traverser(Manifest);
            fn init_manifest() -> Manifest {{ init_manifest_for("default-main") }}
            fn init_manifest_for(entry: &str) -> Manifest {{ Manifest {{ main_component_type_id: entry.into() }} }}
            fn init_definition_to_instance_traverser(manifest: Manifest) -> Traverser {{ Traverser(manifest) }}
            impl Traverser {{
                fn get_main_component_with_properties(&self, id: &str, root: u32) -> (String, u32, String) {{
                    assert!(id == self.0.main_component_type_id || id == "USERLAND_COMPONENT_ROOT");
                    (self.0.main_component_type_id.clone(), root, id.into())
                }}
            }}
            fn main() {{ {cases} }}
        "#
        );
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("native_initialization.rs");
        let binary = temp.path().join(format!(
            "native_initialization{}",
            std::env::consts::EXE_SUFFIX
        ));
        std::fs::write(&input, source).unwrap();
        let compiled = std::process::Command::new("rustc")
            .arg("--edition=2021")
            .arg(&input)
            .arg("-o")
            .arg(&binary)
            .output()
            .unwrap();
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        let result = std::process::Command::new(binary).output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}
