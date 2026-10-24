//! Native font asset registration and direct Windows launcher resources. The
//! generated include_bytes! expressions track embedded font content in Cargo.
use color_eyre::eyre::{eyre, Result};
use sha2::{Digest, Sha256};
use std::path::Path;

pub(crate) fn append_native_defaults(
    roots: &mut Vec<String>,
    packages: &[pax_codegen::configured_source::SourcePackage],
    native: bool,
) -> Result<()> {
    if !native {
        return Ok(());
    }
    let Some(standard) = packages.iter().find(|package| package.name == "pax-std") else {
        return Ok(());
    };
    let root = standard.manifest_dir.join("native-assets").canonicalize()?;
    let root = root.to_string_lossy().into_owned();
    if !roots.contains(&root) {
        roots.push(root);
    }
    // These roots enter the existing manifest/baked-program asset inventory
    // before hashing/code generation. Web and Apple never register these fonts.
    pax_codegen::static_analysis::validate_asset_paths(roots)
}

pub(crate) fn inventory(roots: &[String]) -> Result<String> {
    fn visit(
        root: &Path,
        current: &Path,
        index: usize,
        entries: &mut Vec<String>,
        total: &mut u64,
    ) -> Result<()> {
        let mut files = std::fs::read_dir(current)?.collect::<std::io::Result<Vec<_>>>()?;
        files.sort_by_key(|file| file.file_name());
        let font_file = |path: &Path| {
            path.extension()
                .and_then(|value| value.to_str())
                .is_some_and(|extension| {
                    matches!(
                        extension.to_ascii_lowercase().as_str(),
                        "ttf" | "otf" | "ttc"
                    )
                })
        };
        let has_fonts = files.iter().any(|file| {
            font_file(&file.path()) && file.file_type().is_ok_and(|kind| kind.is_file())
        });
        for file in files {
            let kind = file.file_type()?;
            let path = file.path();
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                visit(root, &path, index, entries, total)?;
            } else if font_file(&path) || (has_fonts && is_font_license(&path)) {
                let size = file.metadata()?.len();
                *total = total
                    .checked_add(size)
                    .ok_or_else(|| eyre!("Font inventory size overflow"))?;
                if size > 16 * 1024 * 1024 || *total > 64 * 1024 * 1024 || entries.len() >= 256 {
                    return Err(eyre!("Direct Windows Cargo font inventory exceeds 16 MiB per resource, 64 MiB total, or 256 resources including license notices"));
                }
                let relative = path
                    .strip_prefix(root)?
                    .to_string_lossy()
                    .replace('\\', "/");
                let hash = format!("{:x}", Sha256::digest(std::fs::read(&path)?));
                entries.push(format!(
                    "({index}usize, {relative:?}, {hash:?}, include_bytes!({path:?}).as_slice())"
                ));
            }
        }
        Ok(())
    }
    let mut entries = Vec::new();
    let mut total = 0;
    for (index, root) in roots.iter().enumerate() {
        visit(
            Path::new(root),
            Path::new(root),
            index,
            &mut entries,
            &mut total,
        )?;
    }
    Ok(format!("&[{}]", entries.join(",")))
}

fn is_font_license(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let name = name.to_ascii_lowercase();
    matches!(
        name.as_str(),
        "ofl.txt" | "license" | "license.txt" | "copying" | "copying.txt"
    ) || (name.starts_with("ofl-") && name.ends_with(".txt"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_defaults_are_target_scoped_and_survive_baking() {
        let standard = tempfile::tempdir().unwrap();
        let package = pax_codegen::configured_source::SourcePackage {
            entry: standard.path().join("src/lib.rs"),
            manifest_dir: standard.path().to_owned(),
            name: "pax-std".into(),
            crate_name: "pax_std".into(),
            dependencies: Default::default(),
            links: Default::default(),
        };
        let packages = [package];
        let mut roots = Vec::new();
        // Other targets do not even inspect the native resource directory.
        append_native_defaults(&mut roots, &packages, false).unwrap();
        assert!(roots.is_empty());
        let assets = standard.path().join("native-assets");
        std::fs::create_dir(&assets).unwrap();
        std::fs::write(assets.join("font.ttf"), b"font").unwrap();
        append_native_defaults(&mut roots, &packages, true).unwrap();
        append_native_defaults(&mut roots, &packages, true).unwrap();
        assert_eq!(roots.len(), 1);
        let manifest = pax_manifest::PaxManifest {
            components: Default::default(),
            main_component_type_id: pax_manifest::TypeId::build_singleton(
                "crate::Example",
                Some("Example"),
            ),
            type_table: Default::default(),
            assets_dirs: roots.clone(),
            engine_import_path: "pax_kit::pax_engine".into(),
        };
        let rich =
            pax_manifest::binary::from_slice(&pax_manifest::binary::to_vec(&manifest).unwrap())
                .unwrap();
        assert_eq!(rich.assets_dirs, roots);
        let ir = pax_manifest::program_ir::ProgramIR::from_manifest(&rich);
        let baked = pax_manifest::program_ir::binary::from_slice(
            &pax_manifest::program_ir::binary::to_vec(&ir).unwrap(),
        )
        .unwrap();
        assert_eq!(baked.assets_dirs, roots);
        assert!(pax_manifest::rust_manifest::to_rust_expression(&rich).contains("native-assets"));

        let application = tempfile::tempdir().unwrap();
        std::fs::write(application.path().join("font.ttf"), b"conflict").unwrap();
        let mut conflicting = vec![application.path().display().to_string()];
        assert!(append_native_defaults(&mut conflicting, &packages, true).is_err());
    }
    #[test]
    fn inventory_embeds_only_fonts_and_preserves_asset_root_indices() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        std::fs::write(first.path().join("ignored.png"), b"pixels").unwrap();
        std::fs::write(second.path().join("family.ttf"), b"font").unwrap();
        std::fs::write(second.path().join("OFL-family.txt"), b"font license").unwrap();
        std::fs::write(first.path().join("LICENSE.txt"), b"unrelated license").unwrap();
        let generated = inventory(&[
            first.path().display().to_string(),
            second.path().display().to_string(),
        ])
        .unwrap();
        assert!(generated.contains("(1usize, \"family.ttf\""));
        assert!(generated.contains("include_bytes!"));
        assert!(generated.contains("OFL-family.txt"));
        assert!(!generated.contains("LICENSE.txt"));
        assert!(!generated.contains("ignored.png"));
        syn::parse_str::<syn::Expr>(&generated).unwrap();
    }
    #[test]
    fn inventory_rejects_oversized_font_before_reading_it() {
        let root = tempfile::tempdir().unwrap();
        std::fs::File::create(root.path().join("too-big.otf"))
            .unwrap()
            .set_len(16 * 1024 * 1024 + 1)
            .unwrap();
        assert!(inventory(&[root.path().display().to_string()])
            .unwrap_err()
            .to_string()
            .contains("16 MiB"));
    }
}
