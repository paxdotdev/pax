use std::path::Path;

/// The selected library may live outside the host's generated directory.
/// Exact package identity prevents dependency components from emitting adapters.
pub(crate) fn is_application_crate(
    cartridge_manifest_dir: Option<&Path>,
    pax_dir: Option<&Path>,
    manifest_dir: &Path,
    is_primary_package: bool,
) -> bool {
    match cartridge_manifest_dir.or_else(|| pax_dir.and_then(Path::parent)) {
        Some(root) => {
            let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
            let manifest = manifest_dir
                .canonicalize()
                .unwrap_or_else(|_| manifest_dir.to_path_buf());
            root == manifest
        }
        // Direct Cargo invocations have no Pax build directory. Preserve the
        // actionable missing-cartridge diagnostic for the selected application.
        None => is_primary_package,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_workspace_library_owns_exports_even_when_host_is_elsewhere() {
        let host = Path::new("/workspace/host");
        let ui = Path::new("/workspace/ui");
        assert!(is_application_crate(
            Some(ui),
            Some(&host.join(".pax")),
            ui,
            false
        ));
        assert!(!is_application_crate(
            Some(ui),
            Some(&host.join(".pax")),
            host,
            true
        ));
        assert!(!is_application_crate(
            Some(ui),
            Some(&host.join(".pax")),
            Path::new("/workspace/widgets"),
            true
        ));
    }

    #[test]
    fn only_the_active_app_owns_the_cartridge() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let pax_dir = root.join(".pax");
        assert!(is_application_crate(None, Some(&pax_dir), root, false));
        assert!(!is_application_crate(
            None,
            Some(&pax_dir),
            &root.join("dependency"),
            false
        ));
        assert!(!is_application_crate(
            None,
            Some(&pax_dir),
            root.parent().unwrap(),
            false
        ));
        assert!(!is_application_crate(
            None,
            Some(&pax_dir),
            &root.with_file_name("pax-kit"),
            true
        ));
    }

    #[test]
    fn canonical_directory_identity_ignores_relative_spelling() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let alternate = root.join("src/..");
        assert!(is_application_crate(
            None,
            Some(&alternate.join(".pax")),
            root,
            false
        ));
        assert!(is_application_crate(
            None,
            Some(&root.join(".pax")),
            &alternate,
            false
        ));
    }

    #[test]
    fn direct_cargo_builds_distinguish_primary_and_dependency_crates() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(is_application_crate(None, None, root, true));
        assert!(!is_application_crate(None, None, root, false));
    }
}
