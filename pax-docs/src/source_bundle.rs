//! Select live monorepo sources or the crate-owned release snapshot.
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Component, Path, PathBuf};

pub fn example_workspace(manifest: &Path, out: &Path) -> io::Result<PathBuf> {
    let workspace = manifest
        .parent()
        .ok_or_else(|| invalid("Missing crate parent"))?;
    if workspace.join("examples/src").is_dir() {
        return Ok(workspace.to_path_buf());
    }
    let bytes = fs::read(manifest.join("bundled-example-sources.json"))?;
    materialize(&bytes, out)
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub fn materialize(bytes: &[u8], out: &Path) -> io::Result<PathBuf> {
    let bundle: serde_json::Value = serde_json::from_slice(bytes)?;
    if bundle["schema"].as_u64() != Some(1) {
        return Err(invalid("Unsupported docs source bundle schema"));
    }
    let files = bundle["files"]
        .as_object()
        .filter(|files| !files.is_empty())
        .ok_or_else(|| invalid("Empty or missing docs example sources"))?;
    // Validate the entire archive before writing. A version/content-specific
    // directory prevents removed source files leaking from an earlier snapshot.
    for (name, contents) in files {
        let path = Path::new(name);
        if name.contains('\\')
            || !name.starts_with("examples/src/")
            || !path
                .components()
                .all(|part| matches!(part, Component::Normal(_)))
            || contents.as_str().is_none()
        {
            return Err(invalid("Invalid docs example source entry"));
        }
    }
    let mut hash = DefaultHasher::new();
    bytes.hash(&mut hash);
    let workspace = out.join(format!("example-sources-{:016x}", hash.finish()));
    for (name, contents) in files {
        let path = workspace.join(name);
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, contents.as_str().unwrap())?;
    }
    Ok(workspace)
}
