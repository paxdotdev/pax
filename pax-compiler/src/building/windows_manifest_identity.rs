use color_eyre::eyre::{eyre, Result};
use sha2::{Digest, Sha256};

const ASSEMBLY_NAMESPACE: &str = "urn:schemas-microsoft-com:asm.v1";

/// The SDK can retain conflicting DPI elements while returning success. Check
/// the authored declaration before merging, then require the merged declaration.
pub(super) fn validate_dpi(manifest: &str, allow_missing: bool) -> Result<()> {
    let document = roxmltree::Document::parse(manifest)
        .map_err(|error| eyre!("Invalid Windows launcher manifest: {error}"))?;
    let declarations: Vec<_> = document
        .descendants()
        .filter(|node| {
            node.has_tag_name((
                "http://schemas.microsoft.com/SMI/2016/WindowsSettings",
                "dpiAwareness",
            ))
        })
        .collect();
    if declarations.is_empty() && allow_missing {
        return Ok(());
    }
    if declarations.len() != 1
        || declarations[0]
            .text()
            .unwrap_or_default()
            .split(',')
            .next()
            .map(str::trim)
            != Some("PerMonitorV2")
    {
        return Err(eyre!(
            "Windows launcher manifest must select PerMonitorV2 as its first dpiAwareness value; conflicting DPI declarations are not supported"
        ));
    }
    Ok(())
}

/// Select a fallback only for a missing definition identity. A dependency's
/// reference identity must not be mistaken for the application's own identity.
pub(super) fn fallback_identity(
    merged: &str,
    launcher: &str,
    version: [u64; 3],
) -> Result<Option<String>> {
    let document = roxmltree::Document::parse(merged)
        .map_err(|error| eyre!("Invalid merged Windows launcher manifest: {error}"))?;
    let root = document.root_element();
    if !root.has_tag_name((ASSEMBLY_NAMESPACE, "assembly")) {
        return Err(eyre!(
            "Windows launcher manifest needs an asm.v1 assembly root"
        ));
    }
    let identities = root
        .children()
        .filter(|node| node.has_tag_name((ASSEMBLY_NAMESPACE, "assemblyIdentity")))
        .count();
    match identities {
        1 => return Ok(None), // The SDK validates the authored identity unchanged.
        0 => {}
        _ => {
            return Err(eyre!(
                "Windows launcher manifest has conflicting definition identities"
            ))
        }
    }
    if version.iter().any(|part| *part > u16::MAX as u64) {
        return Err(eyre!("Cargo version exceeds Windows manifest version limits; provide an application manifest with an explicit assemblyIdentity"));
    }
    // The SDK's -identity syntax uses comma-delimited attributes. A digest gives
    // every launcher a stable name without interpreting characters in its name.
    let name = format!("Pax.Application.{:x}", Sha256::digest(launcher.as_bytes()));
    Ok(Some(format!(
        "{name}, version={}.{}.{}.0, type=win32",
        version[0], version[1], version[2]
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_identity_is_preserved_with_namespaces_and_dependencies() {
        let xml = r#"<s:assembly xmlns:s="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
          <s:assemblyIdentity name="Vendor.Authored" version="9.8.7.6" type="win32"/>
          <s:dependency><s:dependentAssembly><s:assemblyIdentity name="Dependency" version="1.0.0.0"/></s:dependentAssembly></s:dependency>
          <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3"><security><requestedPrivileges><requestedExecutionLevel level="asInvoker" uiAccess="false"/></requestedPrivileges></security></trustInfo>
        </s:assembly>"#;
        // Even an unrepresentable Cargo version must not override authored identity.
        assert_eq!(
            fallback_identity(xml, "launcher", [u64::MAX, 0, 0]).unwrap(),
            None
        );
    }

    #[test]
    fn dependency_identity_does_not_suppress_stable_application_fallback() {
        let xml = r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
          <dependency><dependentAssembly><assemblyIdentity name="Dependency" version="1.0.0.0"/></dependentAssembly></dependency>
        </assembly>"#;
        let fallback = fallback_identity(xml, "app, type=invalid", [1, 2, 3])
            .unwrap()
            .unwrap();
        assert!(fallback.starts_with("Pax.Application."));
        assert!(fallback.ends_with(", version=1.2.3.0, type=win32"));
        assert_eq!(fallback.matches(',').count(), 2);
        assert_eq!(
            fallback_identity(xml, "app, type=invalid", [1, 2, 3]).unwrap(),
            Some(fallback.clone())
        );
        assert_ne!(
            fallback_identity(xml, "another app", [1, 2, 3]).unwrap(),
            Some(fallback)
        );
        assert!(fallback_identity(xml, "app", [65536, 0, 0]).is_err());
    }

    #[test]
    fn malformed_or_conflicting_definition_identities_are_rejected() {
        assert!(fallback_identity("<assembly>", "app", [1, 0, 0]).is_err());
        assert!(fallback_identity("<assembly/>", "app", [1, 0, 0]).is_err());
        let conflict = r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1"><assemblyIdentity name="One"/><assemblyIdentity name="Two"/></assembly>"#;
        assert!(fallback_identity(conflict, "app", [1, 0, 0]).is_err());
    }

    #[test]
    fn effective_dpi_is_required_without_overwriting_authored_conflicts() {
        let xml = |value| {
            format!(
                r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1"><application xmlns="urn:schemas-microsoft-com:asm.v3"><windowsSettings><dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">{value}</dpiAwareness></windowsSettings></application></assembly>"#
            )
        };
        assert!(validate_dpi(&xml("PerMonitorV2"), false).is_ok());
        assert!(validate_dpi(&xml(" PerMonitorV2, PerMonitor "), false).is_ok());
        for value in ["", "System", "System, PerMonitorV2", "PerMonitorV2System"] {
            assert!(validate_dpi(&xml(value), true).is_err(), "{value}");
        }
        let missing = r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1"/>"#;
        assert!(validate_dpi(missing, true).is_ok());
        assert!(validate_dpi(missing, false).is_err());
        let duplicate = xml("PerMonitorV2").replace("</windowsSettings>", r#"<dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness></windowsSettings>"#);
        assert!(validate_dpi(&duplicate, false).is_err());
    }
}
