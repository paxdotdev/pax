//! WinUI loads fonts through application resource URIs. Cargo launchers extract
//! their embedded font inventory beside the executable; packaged apps already
//! carry those files. Font license notices supplied beside the fonts travel in
//! the same inventory. Cache files are content-addressed and never overwritten.
use crate::Error;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Component, Path},
    sync::atomic::{AtomicU64, Ordering},
};

pub(crate) type EmbeddedFont<'a> = (usize, &'a str, &'a str, &'a [u8]);
const MAX_FONT_BYTES: usize = 16 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 64 * 1024 * 1024;

fn safe_directory(path: &Path) -> Result<(), Error> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(Error(format!("Font cache path is not an ordinary directory: {}", path.display()))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => fs::create_dir(path).or_else(|error| if error.kind() == std::io::ErrorKind::AlreadyExists {
            let metadata = fs::symlink_metadata(path)?;
            if metadata.is_dir() && !metadata.file_type().is_symlink() { Ok(()) } else { Err(error) }
        } else { Err(error) }).map_err(|error| Error(format!("Cannot create font cache {}: {error}. Direct Windows Cargo launches with bundled fonts need a writable executable directory; use a packaged application otherwise.", path.display()))),
        Err(error) => Err(Error(error.to_string())),
    }
}

fn verify(path: &Path, hash: &str, size: usize) -> Result<bool, Error> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(Error(error.to_string())),
        Ok(metadata) => {
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || metadata.len() != size as u64
            {
                return Err(Error(format!(
                    "Font cache entry is invalid and was not overwritten: {}",
                    path.display()
                )));
            }
            let bytes = fs::read(path).map_err(|error| Error(error.to_string()))?;
            if format!("{:x}", Sha256::digest(bytes)) != hash {
                return Err(Error(format!("Font cache checksum differs; remove the damaged cache entry before retrying: {}", path.display())));
            }
            Ok(true)
        }
    }
}

pub(crate) fn stage(
    recipe: &mut serde_json::Value,
    executable_directory: &Path,
    fonts: &[EmbeddedFont<'_>],
) -> Result<(), Error> {
    if executable_directory.join("pax-app.json").is_file() {
        recipe["fontAssets"] = recipe["assets"].clone();
        return Ok(());
    }
    if fonts.is_empty() {
        return Ok(());
    }
    let roots = recipe["assets"]
        .as_array()
        .ok_or_else(|| Error("Missing asset roots for embedded fonts".into()))?
        .len();
    let mut total = 0usize;
    let mut inventory = Sha256::new();
    if fonts.len() > 256 {
        return Err(Error("Embedded font inventory exceeds 256 files".into()));
    }
    for (root, relative, hash, bytes) in fonts {
        total = total
            .checked_add(bytes.len())
            .ok_or_else(|| Error("Font inventory size overflow".into()))?;
        if *root >= roots
            || bytes.len() > MAX_FONT_BYTES
            || total > MAX_TOTAL_BYTES
            || hash.len() != 64
            || !hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(Error(
                "Invalid or oversized embedded font inventory (16 MiB per file, 64 MiB total)"
                    .into(),
            ));
        }
        if relative.is_empty()
            || Path::new(relative)
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(Error(
                "Embedded font paths must remain within their asset root".into(),
            ));
        }
        if format!("{:x}", Sha256::digest(bytes)) != *hash {
            return Err(Error("Embedded font checksum mismatch".into()));
        }
        inventory.update(root.to_string());
        inventory.update([0]);
        inventory.update(relative);
        inventory.update([0]);
        inventory.update(hash);
    }
    let cache = executable_directory.join("pax-fonts");
    safe_directory(&cache)?;
    let cache = cache.join(format!("{:x}", inventory.finalize()));
    safe_directory(&cache)?;
    let cache = cache.join("assets");
    safe_directory(&cache)?;
    let mut staged_roots = Vec::new();
    for index in 0..roots {
        let root = cache.join(index.to_string());
        safe_directory(&root)?;
        staged_roots.push(root);
    }
    static NEXT_FILE: AtomicU64 = AtomicU64::new(0);
    for (root, relative, hash, bytes) in fonts {
        let destination = staged_roots[*root].join(relative);
        let mut parent = staged_roots[*root].clone();
        for part in Path::new(relative).parent().unwrap().components() {
            parent.push(part);
            safe_directory(&parent)?;
        }
        if verify(&destination, hash, bytes.len())? {
            continue;
        }
        let temporary = parent.join(format!(
            ".pax-font-{}-{}",
            std::process::id(),
            NEXT_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut created = false;
        let publish = (|| -> std::io::Result<()> {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            created = true;
            file.write_all(bytes)?;
            file.sync_all()?;
            drop(file);
            // A hard link publishes the complete file atomically without replacing
            // an existing entry, including another process's concurrent extraction.
            fs::hard_link(&temporary, &destination)
        })();
        if created {
            let _ = fs::remove_file(&temporary);
        }
        if let Err(error) = publish {
            if !verify(&destination, hash, bytes.len())? {
                return Err(Error(format!("Cannot publish bundled font {}: {error}. Direct Windows Cargo font staging requires a writable filesystem with hard-link support.", destination.display())));
            }
        }
    }
    recipe["fontAssets"] =
        serde_json::to_value(staged_roots).map_err(|error| Error(error.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn font_cache_is_verified_reusable_and_keeps_root_order() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = b"font fixture";
        let hash = format!("{:x}", Sha256::digest(bytes));
        let fonts = [(1, "fonts/test.ttf", hash.as_str(), bytes.as_slice())];
        let mut recipe = json!({"assets":["first","second"]});
        stage(&mut recipe, dir.path(), &fonts).unwrap();
        let file = Path::new(recipe["fontAssets"][1].as_str().unwrap()).join("fonts/test.ttf");
        assert_eq!(fs::read(&file).unwrap(), bytes);
        stage(&mut recipe, dir.path(), &fonts).unwrap();
        fs::write(&file, b"wrong bytes!").unwrap();
        assert!(stage(&mut recipe, dir.path(), &fonts).is_err());
        assert_eq!(fs::read(&file).unwrap(), b"wrong bytes!");
    }
    #[test]
    fn font_cache_rejects_escaping_paths_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let hash = format!("{:x}", Sha256::digest(b"x"));
        assert!(stage(
            &mut json!({"assets":["source"]}),
            dir.path(),
            &[(0, "../font.ttf", &hash, b"x")]
        )
        .is_err());
        assert!(!dir.path().join("pax-fonts").exists());
    }
}
