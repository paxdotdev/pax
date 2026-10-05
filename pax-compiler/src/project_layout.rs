//! Selects the UI library independently of the package owning the process entry.
use cargo_metadata::MetadataCommand;
use color_eyre::eyre::{eyre, Result};
use std::path::{Path, PathBuf};

/// Cargo paths used for a Pax build. Generated files and host metadata remain
/// under the requested application package, even when its UI is a sibling.
#[derive(Clone, Debug)]
pub(crate) struct ProjectLayout {
    pub cartridge_dir: PathBuf,
    pub target_dir: PathBuf,
    pub library_name: String,
    pub exports_cartridge: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_workspace_packages_and_mismatched_library_targets() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("host/src")).unwrap();
        std::fs::create_dir_all(root.path().join("ui/src")).unwrap();
        std::fs::write(
            root.path().join("Cargo.toml"),
            "[workspace]\nmembers=['host','ui']\nresolver='2'\n",
        )
        .unwrap();
        std::fs::write(root.path().join("host/src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(root.path().join("ui/src/lib.rs"), "").unwrap();
        std::fs::write(root.path().join("ui/Cargo.toml"), "[package]\nname='layout-ui'\nversion='0.1.0'\n[lib]\nname='chosen_ui'\ncrate-type=['rlib','cdylib']\n").unwrap();
        for (selection, error) in [
            ("package='missing'", "not a member"),
            ("package='layout-ui'\nlib='wrong'", "does not match"),
            ("package='layout-ui'\nlibs='chosen_ui'", "Unknown"),
            ("package=12", "must name"),
        ] {
            std::fs::write(root.path().join("host/Cargo.toml"), format!("[package]\nname='layout-host'\nversion='0.1.0'\n[package.metadata.pax.cartridge]\n{selection}\n")).unwrap();
            assert!(ProjectLayout::resolve(&root.path().join("host"))
                .unwrap_err()
                .to_string()
                .contains(error));
        }
    }
}

impl ProjectLayout {
    pub fn resolve(project_dir: &Path) -> Result<Self> {
        let manifest = project_dir.join("Cargo.toml").canonicalize()?;
        let metadata = MetadataCommand::new()
            .manifest_path(&manifest)
            .no_deps()
            .exec()?;
        let host = metadata
            .packages
            .iter()
            .find(|package| package.manifest_path.as_std_path() == manifest)
            .ok_or_else(|| {
                eyre!(
                    "Select an application package, rather than a virtual workspace: {}",
                    manifest.display()
                )
            })?;
        let selection = host
            .metadata
            .get("pax")
            .and_then(|pax| pax.get("cartridge"));
        let cartridge = if let Some(selection) = selection {
            let fields = selection.as_object().ok_or_else(|| eyre!("package.metadata.pax.cartridge must be a table with `package` and optional `lib`"))?;
            if let Some(unknown) = fields
                .keys()
                .find(|key| !matches!(key.as_str(), "package" | "lib"))
            {
                return Err(eyre!("Unknown package.metadata.pax.cartridge key `{unknown}`; expected `package` or `lib`"));
            }
            let name = selection
                .get("package")
                .and_then(|value| value.as_str())
                .ok_or_else(|| {
                    eyre!("package.metadata.pax.cartridge.package must name a workspace package")
                })?;
            metadata.packages.iter().find(|package| package.name == name && metadata.workspace_members.contains(&package.id))
                .ok_or_else(|| eyre!("Selected cartridge package `{name}` is not a member of this Cargo workspace"))?
        } else {
            host
        };
        let library = cartridge.targets.iter().find(|target| target.kind.iter().any(|kind| matches!(kind.as_str(), "lib" | "rlib" | "cdylib" | "dylib" | "staticlib")))
            .ok_or_else(|| eyre!("Cartridge package `{}` needs a library with crate-type = [\"rlib\", \"cdylib\"]", cartridge.name))?;
        if let Some(value) = selection.and_then(|selection| selection.get("lib")) {
            let name = value.as_str().ok_or_else(|| {
                eyre!("package.metadata.pax.cartridge.lib must be a library target name")
            })?;
            if name != library.name {
                return Err(eyre!(
                    "Selected library `{name}` does not match `{}` in cartridge package `{}`",
                    library.name,
                    cartridge.name
                ));
            }
        }
        Ok(Self {
            cartridge_dir: cartridge
                .manifest_path
                .parent()
                .unwrap()
                .as_std_path()
                .to_path_buf(),
            target_dir: metadata.target_directory.into_std_path_buf(),
            library_name: library.name.clone(),
            exports_cartridge: library.crate_types.iter().any(|kind| kind == "cdylib"),
        })
    }

    pub fn configure_cartridge(&self, command: &mut std::process::Command) {
        command
            .env("PAX_CARTRIDGE_MANIFEST_DIR", &self.cartridge_dir)
            .env("PAX_BUILD_CARTRIDGE", "true");
    }
}
