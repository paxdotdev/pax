//! Font resources needed by a direct Windows Cargo launcher. The generated
//! include_bytes! expressions make font content part of Cargo's dependency graph.
use color_eyre::eyre::{eyre, Result};
use sha2::{Digest, Sha256};
use std::path::Path;

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
        for file in files {
            let kind = file.file_type()?;
            let path = file.path();
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                visit(root, &path, index, entries, total)?;
            } else if path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|extension| {
                    matches!(
                        extension.to_ascii_lowercase().as_str(),
                        "ttf" | "otf" | "ttc"
                    )
                })
            {
                let size = file.metadata()?.len();
                *total = total
                    .checked_add(size)
                    .ok_or_else(|| eyre!("Font inventory size overflow"))?;
                if size > 16 * 1024 * 1024 || *total > 64 * 1024 * 1024 || entries.len() >= 256 {
                    return Err(eyre!("Direct Windows Cargo font inventory exceeds 16 MiB per font, 64 MiB total, or 256 fonts"));
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inventory_embeds_only_fonts_and_preserves_asset_root_indices() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        std::fs::write(first.path().join("ignored.png"), b"pixels").unwrap();
        std::fs::write(second.path().join("family.ttf"), b"font").unwrap();
        let generated = inventory(&[
            first.path().display().to_string(),
            second.path().display().to_string(),
        ])
        .unwrap();
        assert!(generated.contains("(1usize, \"family.ttf\""));
        assert!(generated.contains("include_bytes!"));
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
