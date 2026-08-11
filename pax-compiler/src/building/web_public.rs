use crate::helpers::PUBLIC_DIR_NAME;
use color_eyre::eyre::{eyre, Report, WrapErr};
use std::fs;
use std::path::{Component, Path, PathBuf};

const RESERVED_TOP_LEVEL_PATHS: &[&str] = &["assets", "__reloads__", "snippets"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PublicEntryKind {
    Directory,
    File,
}

#[derive(Debug)]
struct PublicEntry {
    source: PathBuf,
    relative: PathBuf,
    kind: PublicEntryKind,
}

pub(crate) fn project_public_dir(project_root: &Path) -> PathBuf {
    project_root.join(PUBLIC_DIR_NAME)
}

pub(crate) fn existing_project_public_dir(project_root: &Path) -> Option<PathBuf> {
    let public_dir = project_public_dir(project_root);
    public_dir.is_dir().then_some(public_dir)
}

pub(crate) fn validate_public_dir(public_dir: &Path, output_root: &Path) -> Result<(), Report> {
    build_copy_plan(public_dir, output_root).map(|_| ())
}

pub(crate) fn materialize_public_dir(public_dir: &Path, output_root: &Path) -> Result<(), Report> {
    let entries = build_copy_plan(public_dir, output_root)?;
    for entry in entries {
        let destination = output_root.join(&entry.relative);
        match entry.kind {
            PublicEntryKind::Directory => fs::create_dir_all(&destination).wrap_err_with(|| {
                format!(
                    "Failed to create public output directory `{}`",
                    destination.display()
                )
            })?,
            PublicEntryKind::File => {
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent).wrap_err_with(|| {
                        format!(
                            "Failed to create public output directory `{}`",
                            parent.display()
                        )
                    })?;
                }
                fs::copy(&entry.source, &destination).wrap_err_with(|| {
                    format!(
                        "Failed to copy public file `{}` to `{}`",
                        entry.source.display(),
                        destination.display()
                    )
                })?;
            }
        }
    }
    Ok(())
}

pub(crate) fn is_servable_public_path(public_dir: &Path, relative: &Path) -> bool {
    let Ok(public_metadata) = fs::symlink_metadata(public_dir) else {
        return false;
    };
    if !public_metadata.is_dir()
        || public_metadata.file_type().is_symlink()
        || relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || is_reserved_public_path(relative)
    {
        return false;
    }

    let mut candidate = public_dir.to_path_buf();
    for component in relative.components() {
        let Component::Normal(component) = component else {
            return false;
        };
        candidate.push(component);
        let Ok(metadata) = fs::symlink_metadata(&candidate) else {
            return false;
        };
        if metadata.file_type().is_symlink() {
            return false;
        }
    }

    let Ok(metadata) = fs::metadata(&candidate) else {
        return false;
    };
    if metadata.is_file() {
        return true;
    }
    if !metadata.is_dir() {
        return false;
    }

    let index_path = candidate.join("index.html");
    match fs::symlink_metadata(index_path) {
        Ok(index_metadata) => index_metadata.is_file() && !index_metadata.file_type().is_symlink(),
        Err(_) => false,
    }
}

fn build_copy_plan(public_dir: &Path, output_root: &Path) -> Result<Vec<PublicEntry>, Report> {
    let public_metadata = match fs::symlink_metadata(public_dir) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(err).wrap_err_with(|| {
                format!(
                    "Failed to inspect public directory `{}`",
                    public_dir.display()
                )
            })
        }
    };
    if public_metadata.file_type().is_symlink() {
        return Err(eyre!(
            "Pax web public directory `{}` must not be a symbolic link",
            public_dir.display()
        ));
    }
    if !public_metadata.is_dir() {
        return Err(eyre!(
            "Pax web public path `{}` must be a directory",
            public_dir.display()
        ));
    }

    let mut entries = Vec::new();
    collect_entries(public_dir, public_dir, &mut entries)?;
    entries.sort_by(|left, right| left.relative.cmp(&right.relative));

    for entry in &entries {
        if is_reserved_public_path(&entry.relative) {
            return Err(eyre!(
                "Public path `{}` is reserved by the Pax web runtime",
                entry.relative.display()
            ));
        }

        let destination = output_root.join(&entry.relative);
        let destination_metadata = match fs::symlink_metadata(&destination) {
            Ok(metadata) => Some(metadata),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
            Err(err) => {
                return Err(err).wrap_err_with(|| {
                    format!(
                        "Failed to inspect generated web output `{}`",
                        destination.display()
                    )
                })
            }
        };

        match (entry.kind, destination_metadata) {
            (PublicEntryKind::Directory, Some(metadata)) if metadata.is_dir() => {}
            (_, Some(_)) => {
                return Err(eyre!(
                    "Public path `{}` conflicts with generated or custom web output at `{}`",
                    entry.relative.display(),
                    destination.display()
                ));
            }
            (_, None) => {}
        }
    }

    Ok(entries)
}

fn collect_entries(
    public_root: &Path,
    directory: &Path,
    entries: &mut Vec<PublicEntry>,
) -> Result<(), Report> {
    let read_dir = fs::read_dir(directory).wrap_err_with(|| {
        format!(
            "Failed to read Pax web public directory `{}`",
            directory.display()
        )
    })?;

    for child in read_dir {
        let child = child.wrap_err_with(|| {
            format!(
                "Failed to read an entry in Pax web public directory `{}`",
                directory.display()
            )
        })?;
        let source = child.path();
        let relative = source
            .strip_prefix(public_root)
            .map_err(|_| {
                eyre!(
                    "Public path escaped its source root: `{}`",
                    source.display()
                )
            })?
            .to_path_buf();
        let metadata = fs::symlink_metadata(&source)
            .wrap_err_with(|| format!("Failed to inspect public path `{}`", source.display()))?;

        if metadata.file_type().is_symlink() {
            return Err(eyre!(
                "Public path `{}` must not be a symbolic link",
                relative.display()
            ));
        }
        if metadata.is_dir() {
            entries.push(PublicEntry {
                source: source.clone(),
                relative,
                kind: PublicEntryKind::Directory,
            });
            collect_entries(public_root, &source, entries)?;
        } else if metadata.is_file() {
            entries.push(PublicEntry {
                source,
                relative,
                kind: PublicEntryKind::File,
            });
        } else {
            return Err(eyre!(
                "Public path `{}` is not a regular file or directory",
                relative.display()
            ));
        }
    }

    Ok(())
}

fn is_reserved_public_path(relative: &Path) -> bool {
    let Some(first) = relative.components().next() else {
        return false;
    };
    let Component::Normal(first) = first else {
        return true;
    };
    let Some(first) = first.to_str() else {
        return true;
    };
    RESERVED_TOP_LEVEL_PATHS.contains(&first)
}

#[cfg(test)]
mod tests {
    use super::{is_servable_public_path, materialize_public_dir, validate_public_dir};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn materializes_nested_public_files_without_transforming_bytes() {
        let dir = tempdir().unwrap();
        let public_dir = dir.path().join("public");
        let output_dir = dir.path().join("output");
        fs::create_dir_all(public_dir.join(".well-known")).unwrap();
        fs::create_dir_all(&output_dir).unwrap();
        fs::write(public_dir.join("ai.md"), b"# Pax\n\0raw").unwrap();
        fs::write(public_dir.join(".well-known/agent.txt"), b"pax").unwrap();

        materialize_public_dir(&public_dir, &output_dir).unwrap();

        assert_eq!(fs::read(output_dir.join("ai.md")).unwrap(), b"# Pax\n\0raw");
        assert_eq!(
            fs::read(output_dir.join(".well-known/agent.txt")).unwrap(),
            b"pax"
        );
    }

    #[test]
    fn absent_public_directory_is_a_no_op() {
        let dir = tempdir().unwrap();
        let output_dir = dir.path().join("output");
        fs::create_dir_all(&output_dir).unwrap();

        materialize_public_dir(&dir.path().join("public"), &output_dir).unwrap();

        assert!(fs::read_dir(output_dir).unwrap().next().is_none());
    }

    #[test]
    fn collision_fails_before_copying_any_public_file() {
        let dir = tempdir().unwrap();
        let public_dir = dir.path().join("public");
        let output_dir = dir.path().join("output");
        fs::create_dir_all(&public_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();
        fs::write(public_dir.join("a-safe.txt"), b"safe").unwrap();
        fs::write(public_dir.join("index.html"), b"public index").unwrap();
        fs::write(output_dir.join("index.html"), b"generated index").unwrap();

        let err = materialize_public_dir(&public_dir, &output_dir).unwrap_err();

        assert!(err.to_string().contains("index.html"));
        assert!(!output_dir.join("a-safe.txt").exists());
        assert_eq!(
            fs::read(output_dir.join("index.html")).unwrap(),
            b"generated index"
        );
    }

    #[test]
    fn public_directories_can_merge_without_overwriting_files() {
        let dir = tempdir().unwrap();
        let public_dir = dir.path().join("public");
        let output_dir = dir.path().join("output");
        fs::create_dir_all(public_dir.join("docs")).unwrap();
        fs::create_dir_all(output_dir.join("docs")).unwrap();
        fs::write(public_dir.join("docs/ai.md"), b"ai").unwrap();
        fs::write(output_dir.join("docs/index.html"), b"index").unwrap();

        materialize_public_dir(&public_dir, &output_dir).unwrap();

        assert_eq!(fs::read(output_dir.join("docs/ai.md")).unwrap(), b"ai");
        assert_eq!(
            fs::read(output_dir.join("docs/index.html")).unwrap(),
            b"index"
        );
    }

    #[test]
    fn reserved_runtime_directories_are_rejected() {
        let dir = tempdir().unwrap();
        let public_dir = dir.path().join("public");
        let output_dir = dir.path().join("output");
        fs::create_dir_all(public_dir.join("__reloads__")).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let err = validate_public_dir(&public_dir, &output_dir).unwrap_err();

        assert!(err.to_string().contains("reserved"));
        assert!(err.to_string().contains("__reloads__"));
    }

    #[test]
    fn live_public_filter_tracks_file_creation_edits_and_deletion() {
        let dir = tempdir().unwrap();
        let public_dir = dir.path().join("public");
        fs::create_dir_all(&public_dir).unwrap();
        let relative = std::path::Path::new("ai.md");

        assert!(!is_servable_public_path(&public_dir, relative));
        fs::write(public_dir.join(relative), b"one").unwrap();
        assert!(is_servable_public_path(&public_dir, relative));
        fs::write(public_dir.join(relative), b"two").unwrap();
        assert!(is_servable_public_path(&public_dir, relative));
        fs::remove_file(public_dir.join(relative)).unwrap();
        assert!(!is_servable_public_path(&public_dir, relative));
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_links_are_rejected_for_builds_and_live_serving() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        let public_dir = dir.path().join("public");
        let output_dir = dir.path().join("output");
        fs::create_dir_all(&public_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();
        fs::write(dir.path().join("secret.txt"), b"secret").unwrap();
        symlink(dir.path().join("secret.txt"), public_dir.join("ai.md")).unwrap();

        let err = validate_public_dir(&public_dir, &output_dir).unwrap_err();

        assert!(err.to_string().contains("symbolic link"));
        assert!(!is_servable_public_path(
            &public_dir,
            std::path::Path::new("ai.md")
        ));
    }
}
