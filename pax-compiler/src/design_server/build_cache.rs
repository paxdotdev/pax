//! Validate an optimized development product before reusing it on a new server.
use crate::RunContext;
use cargo_metadata::{DependencyKind, MetadataCommand, Package};
use color_eyre::eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

#[derive(Serialize, Deserialize)]
pub(super) struct Receipt {
    pub inputs: String,
    pub artifact: String,
    pub resources: String,
    pub manifest: String,
}

fn field(hash: &mut Sha256, bytes: &[u8]) {
    hash.update(bytes.len().to_le_bytes());
    hash.update(bytes);
}

fn hash_file(hash: &mut Sha256, path: &Path) -> Result<()> {
    let mut source = io::BufReader::new(fs::File::open(path)?);
    hash.update(source.get_ref().metadata()?.len().to_le_bytes());
    io::copy(&mut source, hash)?;
    Ok(())
}

fn tree(hash: &mut Sha256, root: &Path, source: bool) -> Result<()> {
    let entries = WalkDir::new(root)
        .sort_by_file_name()
        .follow_links(source)
        .into_iter()
        .filter_entry(|e| {
            !source
                || !matches!(
                    e.file_name().to_str(),
                    Some(
                        ".git"
                            | "target"
                            | ".pax"
                            | "node_modules"
                            | ".build"
                            | ".swiftpm"
                            | "DerivedData"
                    )
                )
        });
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        field(
            hash,
            path.strip_prefix(root)?.as_os_str().as_encoded_bytes(),
        );
        if entry.file_type().is_symlink() {
            field(hash, b"link");
            field(hash, fs::read_link(path)?.as_os_str().as_encoded_bytes());
        } else if entry.file_type().is_file() {
            field(hash, b"file");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                hash.update((entry.metadata()?.permissions().mode() & 0o111).to_le_bytes());
            }
            hash_file(hash, path)?;
        } else {
            field(hash, b"directory");
        }
    }
    Ok(())
}

pub(super) fn artifact(path: &Path) -> Result<String> {
    let mut hash = Sha256::new();
    tree(&mut hash, path, false)?;
    Ok(format!("{:x}", hash.finalize()))
}

fn version(hash: &mut Sha256, program: &str, args: &[&str]) -> Result<()> {
    let out = Command::new(program).args(args).output()?;
    if !out.status.success() {
        return Err(eyre!("Could not identify {program}"));
    }
    field(hash, &out.stdout);
    Ok(())
}

pub(super) fn inputs(ctx: &RunContext, variant: &str) -> Result<String> {
    let root = &ctx.project_path;
    let mut hash = Sha256::new();
    field(&mut hash, b"optimized-development-v1");
    field(&mut hash, variant.as_bytes());
    field(&mut hash, root.as_os_str().as_encoded_bytes());
    field(
        &mut hash,
        format!(
            "{}:{}:{:?}",
            ctx.is_libdev_mode, ctx.should_run_designtime, ctx.ios_development_team
        )
        .as_bytes(),
    );
    hash_file(&mut hash, &std::env::current_exe()?)?;
    version(&mut hash, "rustc", &["-vV"])?;
    version(&mut hash, "cargo", &["-V"])?;
    if variant == "web" {
        version(&mut hash, "wasm-pack", &["--version"])?;
        version(&mut hash, "node", &["--version"])?;
    } else {
        version(&mut hash, "xcodebuild", &["-version"])?;
        version(
            &mut hash,
            "xcrun",
            &[
                "--sdk",
                if variant == "macos" {
                    "macosx"
                } else {
                    "iphonesimulator"
                },
                "--show-sdk-build-version",
            ],
        )?;
    }
    // Environment input to Cargo, build scripts, Xcode and wasm-pack. Invocation
    // identities and working-directory bookkeeping change on each service start.
    for (k, v) in std::env::vars_os().collect::<BTreeMap<_, _>>() {
        if matches!(
            k.to_str(),
            Some(
                "PAX_DEV_SERVER_LAUNCH_ID"
                    | "PAX_DEV_INSTANCE_FILE"
                    | "PWD"
                    | "OLDPWD"
                    | "SHLVL"
                    | "XPC_SERVICE_NAME"
                    | "_"
            )
        ) {
            continue;
        }
        field(&mut hash, k.as_encoded_bytes());
        field(&mut hash, v.as_encoded_bytes());
    }
    let layout = crate::project_layout::ProjectLayout::resolve(root)?;
    let mut pending = vec![root.canonicalize()?, layout.cartridge_dir.canonicalize()?];
    let mut packages: BTreeMap<PathBuf, Package> = BTreeMap::new();
    let mut roots = BTreeSet::new();
    let mut configs = BTreeSet::new();
    while let Some(dir) = pending.pop() {
        if !roots.insert(dir.clone()) {
            continue;
        }
        if !packages.contains_key(&dir) {
            let meta = MetadataCommand::new()
                .manifest_path(dir.join("Cargo.toml"))
                .no_deps()
                .exec()?;
            configs.insert(meta.workspace_root.as_std_path().join("Cargo.toml"));
            configs.insert(meta.workspace_root.as_std_path().join("Cargo.lock"));
            for pkg in meta.packages {
                let p = pkg
                    .manifest_path
                    .as_std_path()
                    .parent()
                    .unwrap()
                    .canonicalize()?;
                packages.insert(p, pkg);
            }
        }
        let pkg = packages
            .get(&dir)
            .ok_or_else(|| eyre!("Unresolved local package"))?;
        let framework = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        if pkg
            .targets
            .iter()
            .any(|t| t.kind.iter().any(|k| k == "custom-build"))
            && !dir.starts_with(framework)
        {
            return Err(eyre!(
                "A local build script may read additional inputs; Cargo will validate this build"
            ));
        }
        for dep in &pkg.dependencies {
            if let Some(p) = dep
                .path
                .as_ref()
                .filter(|_| dep.kind != DependencyKind::Development)
            {
                pending.push(p.as_std_path().canonicalize()?);
            }
        }
        for a in dir.ancestors() {
            for name in [
                ".cargo/config.toml",
                ".cargo/config",
                "rust-toolchain",
                "rust-toolchain.toml",
            ] {
                configs.insert(a.join(name));
            }
        }
    }
    // The CLI's local Web interface and Apple host templates can be used even
    // when the project itself resolves its Pax crates through a package registry.
    let compiler = Path::new(env!("CARGO_MANIFEST_DIR"));
    for path in [compiler.join("files"), compiler.join("../pax-host/native")] {
        if path.is_dir() {
            roots.insert(path.canonicalize()?);
        }
    }
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|v| PathBuf::from(v).join(".cargo")));
    if let Some(ch) = cargo_home {
        configs.insert(ch.join("config.toml"));
        configs.insert(ch.join("config"));
    }
    for file in configs {
        if file.is_file() {
            field(&mut hash, file.as_os_str().as_encoded_bytes());
            hash_file(&mut hash, &file)?;
        }
    }
    for dir in &roots {
        if dir.ancestors().skip(1).any(|p| roots.contains(p)) {
            continue;
        }
        field(&mut hash, dir.as_os_str().as_encoded_bytes());
        tree(&mut hash, dir, true)?;
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub(super) fn resources(root: &Path, manifest: &pax_manifest::PaxManifest) -> Result<String> {
    let mut hash = Sha256::new();
    let paths: BTreeSet<_> = manifest
        .assets_dirs
        .iter()
        .map(PathBuf::from)
        .chain(
            manifest
                .components
                .values()
                .filter_map(|c| c.template.as_ref()?.get_file_path().map(PathBuf::from)),
        )
        .collect();
    for path in paths {
        let absolute = if path.is_absolute() {
            path
        } else {
            root.join(path)
        };
        field(&mut hash, absolute.as_os_str().as_encoded_bytes());
        if absolute.is_dir() {
            tree(&mut hash, &absolute, true)?
        } else if absolute.is_file() {
            hash_file(&mut hash, &absolute)?
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub(super) fn manifest_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn staged_product_digest_covers_contents_names_and_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("index.html"), "first").unwrap();
        let original = artifact(root).unwrap();
        fs::write(root.join("index.html"), "other").unwrap();
        assert_ne!(original, artifact(root).unwrap());
        fs::write(root.join("index.html"), "first").unwrap();
        assert_eq!(original, artifact(root).unwrap());
        fs::rename(root.join("index.html"), root.join("other.html")).unwrap();
        assert_ne!(original, artifact(root).unwrap());
        #[cfg(unix)]
        {
            let renamed = artifact(root).unwrap();
            std::os::unix::fs::symlink("other.html", root.join("index.html")).unwrap();
            assert_ne!(renamed, artifact(root).unwrap());
        }
    }
}
