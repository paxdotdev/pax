use std::path::Path;

/// PAX_DIR is the active app's generated directory, not a dependency's directory.
/// Exact parent identity also avoids mistaking an ancestor crate for a nested app.
pub(crate) fn is_application_crate(
    pax_dir: Option<&Path>,
    manifest_dir: &Path,
    is_primary_package: bool,
) -> bool {
    match pax_dir {
        Some(pax_dir) => pax_dir.parent().is_some_and(|root| {
            let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
            let manifest = manifest_dir
                .canonicalize()
                .unwrap_or_else(|_| manifest_dir.to_path_buf());
            root == manifest
        }),
        // Direct Cargo invocations have no Pax build directory. Preserve the
        // actionable missing-cartridge diagnostic for the selected application.
        None => is_primary_package,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_active_app_owns_the_cartridge() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let pax_dir = root.join(".pax");
        assert!(is_application_crate(Some(&pax_dir), root, false));
        assert!(!is_application_crate(
            Some(&pax_dir),
            &root.join("dependency"),
            false
        ));
        assert!(!is_application_crate(
            Some(&pax_dir),
            root.parent().unwrap(),
            false
        ));
        assert!(!is_application_crate(
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
            Some(&alternate.join(".pax")),
            root,
            false
        ));
        assert!(is_application_crate(
            Some(&root.join(".pax")),
            &alternate,
            false
        ));
    }

    #[test]
    fn direct_cargo_builds_distinguish_primary_and_dependency_crates() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(is_application_crate(None, root, true));
        assert!(!is_application_crate(None, root, false));
    }
}
