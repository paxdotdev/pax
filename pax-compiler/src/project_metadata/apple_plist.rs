use super::{string_field, PaxProjectMetadata};
use crate::{helpers::INTERFACE_DIR_NAME, RunTarget};
use color_eyre::eyre::{self, eyre, WrapErr};
use plist::{Dictionary, Value};
use std::path::Path;
use toml_edit::{Item, Table, TableLike};

// Validate each authored layer before merging so an overridden conflict is not hidden.
pub(super) fn plist_overlay(
    table: &Table,
    kind: &str,
    metadata_path: &str,
    manifest_dir: &Path,
) -> eyre::Result<Dictionary> {
    let file_key = format!("{kind}_file");
    let mut entries = Dictionary::new();
    if let Some(file) = string_field(table, &file_key, &format!("{metadata_path}.{file_key}"))? {
        let path = manifest_dir.join(file);
        entries = read_dictionary(&path)
            .wrap_err_with(|| format!("Invalid `{metadata_path}.{file_key}`"))?;
        validate_owned_keys(&entries, kind, &path.display().to_string())?;
    }
    if let Some(item) = table.get(kind) {
        let path = format!("{metadata_path}.{kind}");
        let table = item
            .as_table_like()
            .ok_or_else(|| eyre!("`{path}` must be a TOML table"))?;
        let inline = convert_table(table, &path)?;
        validate_owned_keys(&inline, kind, &path)?;
        // Arrays and dictionaries replace as complete top-level values; never union permissions.
        entries.extend(inline);
    }
    Ok(entries)
}

fn read_dictionary(path: &Path) -> eyre::Result<Dictionary> {
    Value::from_file(path)
        .wrap_err_with(|| format!("Failed to read plist `{}`", path.display()))?
        .into_dictionary()
        .ok_or_else(|| eyre!("Plist `{}` must contain a root dictionary", path.display()))
}

fn convert_table(table: &dyn TableLike, path: &str) -> eyre::Result<Dictionary> {
    table
        .iter()
        .map(|(key, item)| {
            Ok((
                key.to_string(),
                convert_item(item, &format!("{path}.{key}"))?,
            ))
        })
        .collect()
}

fn convert_item(item: &Item, path: &str) -> eyre::Result<Value> {
    match item {
        Item::Value(value) => convert_value(value, path),
        Item::Table(table) => Ok(Value::Dictionary(convert_table(table, path)?)),
        Item::ArrayOfTables(tables) => Ok(Value::Array(
            tables
                .iter()
                .enumerate()
                .map(|(index, table)| {
                    Ok(Value::Dictionary(convert_table(
                        table,
                        &format!("{path}[{index}]"),
                    )?))
                })
                .collect::<eyre::Result<_>>()?,
        )),
        Item::None => Err(eyre!("`{path}` has no plist value")),
    }
}

fn convert_value(value: &toml_edit::Value, path: &str) -> eyre::Result<Value> {
    use toml_edit::Value as Toml;
    Ok(match value {
        Toml::String(value) => Value::String(value.value().clone()),
        Toml::Integer(value) => Value::Integer((*value.value()).into()),
        Toml::Float(value) if value.value().is_finite() => Value::Real(*value.value()),
        Toml::Float(_) => return Err(eyre!("`{path}` must be a finite plist real number")),
        Toml::Boolean(value) => Value::Boolean(*value.value()),
        Toml::Array(values) => Value::Array(
            values.iter().enumerate().map(|(index, value)| {
                convert_value(value, &format!("{path}[{index}]"))
            }).collect::<eyre::Result<_>>()?,
        ),
        Toml::InlineTable(table) => Value::Dictionary(convert_table(table, path)?),
        Toml::Datetime(_) => return Err(eyre!(
            "`{path}` is a TOML date/time; use `info_plist_file` or `entitlements_file` for native plist dates and data"
        )),
    })
}

fn validate_owned_keys(entries: &Dictionary, kind: &str, source: &str) -> eyre::Result<()> {
    for key in entries.keys() {
        let owner = if kind == "info_plist" {
            match key.as_str() {
                "CFBundleIdentifier" => Some("Pax `bundle_identifier` metadata"),
                "CFBundleDisplayName" | "CFBundleName" => Some("Pax `title` metadata"),
                "CFBundleShortVersionString" => Some("Pax `marketing_version` metadata"),
                "CFBundleVersion" => Some("Pax `build_number` metadata"),
                "CFBundleExecutable" | "CFBundlePackageType" => Some("the Xcode host target"),
                _ => None,
            }
        } else {
            match key.as_str() {
                "application-identifier"
                | "com.apple.application-identifier"
                | "com.apple.developer.team-identifier"
                | "get-task-allow"
                | "com.apple.security.get-task-allow" => Some("Xcode signing and provisioning"),
                _ => None,
            }
        };
        if let Some(owner) = owner {
            return Err(eyre!("`{source}` contains `{key}`, which is managed by {owner}; remove this conflicting plist entry"));
        }
    }
    Ok(())
}

pub(super) fn apply_apple_plists(
    target: &RunTarget,
    pax_dir: &Path,
    metadata: &PaxProjectMetadata,
) -> eyre::Result<()> {
    let platform = match target {
        RunTarget::macOS => "macos",
        RunTarget::iOS | RunTarget::iPadOS => "ios",
        RunTarget::Web => return Ok(()),
    };
    let host = pax_dir
        .join(INTERFACE_DIR_NAME)
        .join(platform)
        .join(format!("pax-app-{platform}"))
        .join(format!("pax-app-{platform}"));
    for (name, overlay) in [
        ("Info.plist".to_string(), metadata.apple_info_plist(target)),
        (
            format!("pax_app_{platform}.entitlements"),
            metadata.apple_entitlements(target),
        ),
    ] {
        let path = host.join(name);
        let mut entries = read_dictionary(&path)?;
        entries.extend(overlay);
        Value::Dictionary(entries)
            .to_file_xml(&path)
            .wrap_err_with(|| format!("Failed to write generated plist `{}`", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_metadata::load_project_metadata_from_toml;

    #[test]
    fn arbitrary_keys_and_recursive_types_roundtrip_through_xml() {
        let metadata = load_project_metadata_from_toml(
            Path::new("/tmp"),
            r#"
            [package.metadata.pax.ios.info_plist]
            CustomString = "A < B & \"quoted\""
            CustomBoolean = true
            CustomInteger = -42
            CustomReal = 0.125
            NSBonjourServices = ["_example._tcp"]
            CustomArray = [{ name = "first", nested = [false, 3] }]
            Empty = []
            [package.metadata.pax.ios.info_plist.NSAppTransportSecurity]
            NSAllowsLocalNetworking = true
            [[package.metadata.pax.ios.info_plist.CFBundleURLTypes]]
            CFBundleURLSchemes = ["example"]
            [package.metadata.pax.ios.entitlements]
            "com.apple.developer.associated-domains" = ["applinks:example.com"]
            "future.apple.key" = { arbitrary = [true, "value"] }
        "#,
        )
        .unwrap();
        let values = metadata.apple_info_plist(&RunTarget::iOS);
        assert_eq!(
            values["CustomString"].as_string(),
            Some("A < B & \"quoted\"")
        );
        assert_eq!(values["CustomBoolean"].as_boolean(), Some(true));
        assert_eq!(values["CustomInteger"].as_signed_integer(), Some(-42));
        assert_eq!(values["CustomReal"].as_real(), Some(0.125));
        assert_eq!(
            values["CustomArray"].as_array().unwrap()[0]
                .as_dictionary()
                .unwrap()["nested"],
            Value::Array(vec![false.into(), 3_i64.into()])
        );
        assert_eq!(
            values["NSAppTransportSecurity"].as_dictionary().unwrap()["NSAllowsLocalNetworking"]
                .as_boolean(),
            Some(true)
        );
        assert_eq!(values["CFBundleURLTypes"].as_array().unwrap().len(), 1);
        let mut xml = Vec::new();
        Value::Dictionary(values.clone())
            .to_writer_xml(&mut xml)
            .unwrap();
        assert_eq!(
            Value::from_reader_xml(xml.as_slice()).unwrap(),
            Value::Dictionary(values)
        );
        let entitlements = metadata.apple_entitlements(&RunTarget::iOS);
        assert!(entitlements.contains_key("future.apple.key"));
        assert!(!entitlements.contains_key("com"));
    }

    #[test]
    fn files_then_inline_then_target_replace_top_level_values() {
        let dir = tempfile::tempdir().unwrap();
        let file: Dictionary = [
            ("Source".to_string(), Value::String("file".into())),
            ("FileOnly".to_string(), Value::Data(vec![0, 128, 255])),
            (
                "Date".to_string(),
                Value::Date(plist::Date::from_xml_format("2026-10-01T00:00:00Z").unwrap()),
            ),
            ("Unsigned".to_string(), Value::Integer(u64::MAX.into())),
        ]
        .into_iter()
        .collect();
        Value::Dictionary(file.clone())
            .to_file_binary(dir.path().join("base.plist"))
            .unwrap();
        Value::Dictionary(
            [(
                "example.group".to_string(),
                Value::Array(vec!["file".into()]),
            )]
            .into_iter()
            .collect(),
        )
        .to_file_xml(dir.path().join("app.entitlements"))
        .unwrap();
        let metadata = load_project_metadata_from_toml(
            dir.path(),
            r#"
            [package.metadata.pax]
            info_plist_file = "base.plist"
            info_plist = { Source = "common", Nested = { common = true } }
            [package.metadata.pax.ios]
            entitlements_file = "app.entitlements"
            [package.metadata.pax.ios.info_plist]
            Source = "ios"
            Items = ["ios"]
            Nested = { ios = true }
            [package.metadata.pax.ios.entitlements]
            "example.group" = ["ios"]
            [package.metadata.pax.ipados.info_plist]
            Items = []
            Nested = { ipad = true }
            [package.metadata.pax.ipados.entitlements]
            "example.group" = []
        "#,
        )
        .unwrap();
        let ipad = metadata.apple_info_plist(&RunTarget::iPadOS);
        assert_eq!(ipad["Source"].as_string(), Some("ios"));
        assert!(ipad["Items"].as_array().unwrap().is_empty());
        assert_eq!(ipad["Nested"].as_dictionary().unwrap().len(), 1);
        assert_eq!(
            ipad["Nested"].as_dictionary().unwrap()["ipad"].as_boolean(),
            Some(true)
        );
        for key in ["Date", "FileOnly", "Unsigned"] {
            assert_eq!(ipad[key], file[key]);
        }
        let mut xml = Vec::new();
        Value::Dictionary(ipad.clone())
            .to_writer_xml(&mut xml)
            .unwrap();
        assert_eq!(
            Value::from_reader_xml(xml.as_slice()).unwrap(),
            Value::Dictionary(ipad)
        );
        assert_eq!(
            metadata.apple_info_plist(&RunTarget::macOS)["Source"].as_string(),
            Some("common")
        );
        assert_eq!(
            metadata.apple_entitlements(&RunTarget::iOS)["example.group"],
            Value::Array(vec!["ios".into()])
        );
        assert!(
            metadata.apple_entitlements(&RunTarget::iPadOS)["example.group"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(metadata.apple_entitlements(&RunTarget::macOS).is_empty());
    }

    #[test]
    fn invalid_values_and_managed_keys_have_actionable_errors() {
        for (source, expected) in [
            ("info_plist = false", "must be a TOML table"),
            ("info_plist = { Test = nan }", "finite plist real"),
            (
                "info_plist = { Test = 2026-10-01 }",
                "native plist dates and data",
            ),
            (
                "info_plist = { CFBundleIdentifier = 'x' }",
                "bundle_identifier",
            ),
            ("info_plist = { CFBundleName = 'x' }", "title"),
            (
                "entitlements = { 'get-task-allow' = true }",
                "Xcode signing",
            ),
            (
                "entitlements = { 'com.apple.developer.team-identifier' = 'x' }",
                "Xcode signing",
            ),
            ("info_plist_file = 'missing.plist'", "Failed to read plist"),
        ] {
            let error = load_project_metadata_from_toml(
                Path::new("/nonexistent"),
                &format!("[package.metadata.pax.ios]\n{source}"),
            )
            .unwrap_err();
            assert!(format!("{error:#}").contains(expected), "{error:#}");
        }
        let error = load_project_metadata_from_toml(
            Path::new("/tmp"),
            "[package.metadata.pax.entitlements]\n'example' = true",
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("Entitlements must be configured under"));
    }

    #[test]
    fn source_files_require_dictionaries_and_cannot_override_managed_keys() {
        let dir = tempfile::tempdir().unwrap();
        for (value, expected) in [
            (Value::Array(vec![]), "root dictionary"),
            (
                Value::Dictionary(
                    [("CFBundleVersion", Value::from("7"))]
                        .into_iter()
                        .collect(),
                ),
                "build_number",
            ),
        ] {
            value.to_file_xml(dir.path().join("source.plist")).unwrap();
            let error = load_project_metadata_from_toml(
                dir.path(),
                "[package.metadata.pax]\ninfo_plist_file = 'source.plist'",
            )
            .unwrap_err();
            assert!(format!("{error:#}").contains(expected), "{error:#}");
        }
        std::fs::write(dir.path().join("source.plist"), "not a plist").unwrap();
        assert!(load_project_metadata_from_toml(
            dir.path(),
            "[package.metadata.pax]\ninfo_plist_file = 'source.plist'"
        )
        .is_err());
    }

    #[test]
    fn generated_hosts_preserve_defaults_and_isolate_platform_entitlements() {
        let dir = tempfile::tempdir().unwrap();
        let metadata = load_project_metadata_from_toml(
            dir.path(),
            r#"
            [package.metadata.pax.info_plist]
            CustomTypes = [true, 23, { nested = "<&>" }]
            [package.metadata.pax.macos.entitlements]
            "com.apple.security.network.client" = true
            "com.apple.security.network.server" = true
            [package.metadata.pax.ios.entitlements]
            "com.apple.developer.associated-domains" = ["applinks:example.com"]
            [package.metadata.pax.ipados.entitlements]
            "com.apple.developer.associated-domains" = []
        "#,
        )
        .unwrap();
        for target in [RunTarget::macOS, RunTarget::iOS, RunTarget::iPadOS] {
            let (platform, template) = if target == RunTarget::macOS {
                ("macos", &crate::helpers::PAX_MACOS_INTERFACE_TEMPLATE)
            } else {
                ("ios", &crate::helpers::PAX_IOS_INTERFACE_TEMPLATE)
            };
            let interface = dir.path().join(INTERFACE_DIR_NAME).join(platform);
            std::fs::create_dir_all(&interface).unwrap();
            template.extract(&interface).unwrap();
            apply_apple_plists(&target, dir.path(), &metadata).unwrap();
            let host = interface.join(format!("pax-app-{platform}/pax-app-{platform}"));
            let info = read_dictionary(&host.join("Info.plist")).unwrap();
            assert_eq!(
                info["CustomTypes"],
                metadata.common.info_plist["CustomTypes"]
            );
            assert_eq!(
                info["CFBundleIdentifier"].as_string(),
                Some("$(PRODUCT_BUNDLE_IDENTIFIER)")
            );
            let entitlements =
                read_dictionary(&host.join(format!("pax_app_{platform}.entitlements"))).unwrap();
            if target == RunTarget::macOS {
                assert_eq!(
                    entitlements["com.apple.security.app-sandbox"].as_boolean(),
                    Some(true)
                );
                assert_eq!(
                    entitlements["com.apple.security.network.client"].as_boolean(),
                    Some(true)
                );
                assert_eq!(
                    entitlements["com.apple.security.network.server"].as_boolean(),
                    Some(true)
                );
                assert!(!entitlements.contains_key("com.apple.developer.associated-domains"));
            } else {
                assert!(!entitlements.contains_key("com.apple.security.network.client"));
                assert_eq!(
                    entitlements["com.apple.developer.associated-domains"]
                        .as_array()
                        .unwrap()
                        .len(),
                    if target == RunTarget::iPadOS { 0 } else { 1 }
                );
                assert!(info.contains_key("UIApplicationSceneManifest"));
            }
            // Builds recopy authored templates before applying metadata; removed settings cannot linger.
            template.extract(&interface).unwrap();
            apply_apple_plists(&target, dir.path(), &PaxProjectMetadata::default()).unwrap();
            assert!(!read_dictionary(&host.join("Info.plist"))
                .unwrap()
                .contains_key("CustomTypes"));
            assert!(
                !read_dictionary(&host.join(format!("pax_app_{platform}.entitlements")))
                    .unwrap()
                    .contains_key("com.apple.security.network.client")
            );
        }
    }
}
