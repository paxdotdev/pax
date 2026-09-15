use flate2::read::GzDecoder;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path};

const BUNDLE: &[u8] = include_bytes!("../files/new-project/bundled-examples.paxbundle");
const MAGIC: &[u8] = b"PAXBUNDLE1\n";

#[derive(Clone, Debug, Deserialize)]
pub struct BundledExample {
    pub name: String,
    pub description: String,
    pub caveats: String,
    pub sha256: String,
}

#[derive(Debug, Deserialize)]
struct BundleManifest {
    default: String,
    examples: Vec<BundledExample>,
}

struct BundleFile {
    path: String,
    data: Vec<u8>,
    mode: u32,
}

struct ParsedBundle {
    manifest: BundleManifest,
    files: Vec<BundleFile>,
}

pub fn selected_example_name(requested: Option<&str>) -> Result<String, String> {
    let bundle = parse_bundle()?;
    let selected = requested.unwrap_or(&bundle.manifest.default);
    if bundle
        .manifest
        .examples
        .iter()
        .any(|entry| entry.name == selected)
    {
        Ok(selected.to_string())
    } else {
        Err(unknown_example_message(selected, &bundle.manifest.examples))
    }
}

pub fn extract_example(name: Option<&str>, destination: &Path) -> Result<String, String> {
    let bundle = parse_bundle()?;
    let selected_name = name.unwrap_or(&bundle.manifest.default);
    let selected = bundle
        .manifest
        .examples
        .iter()
        .find(|entry| entry.name == selected_name)
        .ok_or_else(|| unknown_example_message(selected_name, &bundle.manifest.examples))?;

    let prefix = format!("{selected_name}/");
    let mut selected_files = Vec::new();
    for file in bundle.files {
        if let Some(relative) = file.path.strip_prefix(&prefix) {
            validate_relative_path(relative)?;
            selected_files.push((relative.to_string(), file.data, file.mode));
        }
    }
    if selected_files.is_empty() {
        return Err(format!(
            "Bundled example `{selected_name}` contains no files"
        ));
    }
    validate_digest(selected, &selected_files)?;

    for (relative, data, mode) in selected_files {
        let output = destination.join(relative);
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(&output, data).map_err(|error| error.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&output, fs::Permissions::from_mode(mode))
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(selected_name.to_string())
}

fn parse_bundle() -> Result<ParsedBundle, String> {
    let mut decoded = Vec::new();
    GzDecoder::new(BUNDLE)
        .read_to_end(&mut decoded)
        .map_err(|error| format!("Failed to decompress bundled examples: {error}"))?;
    let mut cursor = Cursor::new(decoded.as_slice());
    let mut magic = vec![0; MAGIC.len()];
    cursor
        .read_exact(&mut magic)
        .map_err(|error| error.to_string())?;
    if magic != MAGIC {
        return Err("Invalid bundled-example header".to_string());
    }
    let manifest_len = read_u32(&mut cursor)? as usize;
    let mut manifest_bytes = vec![0; manifest_len];
    cursor
        .read_exact(&mut manifest_bytes)
        .map_err(|error| error.to_string())?;
    let manifest: BundleManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("Invalid bundled-example registry: {error}"))?;
    validate_manifest(&manifest)?;

    let file_count = read_u32(&mut cursor)? as usize;
    let mut files = Vec::with_capacity(file_count);
    for _ in 0..file_count {
        let path_len = read_u32(&mut cursor)? as usize;
        let data_len = read_u64(&mut cursor)? as usize;
        let mode = read_u32(&mut cursor)?;
        let mut path = vec![0; path_len];
        cursor
            .read_exact(&mut path)
            .map_err(|error| error.to_string())?;
        let path = String::from_utf8(path).map_err(|error| error.to_string())?;
        validate_relative_path(&path)?;
        let mut data = vec![0; data_len];
        cursor
            .read_exact(&mut data)
            .map_err(|error| error.to_string())?;
        files.push(BundleFile { path, data, mode });
    }
    if cursor.position() != decoded.len() as u64 {
        return Err("Bundled examples contain trailing data".to_string());
    }
    Ok(ParsedBundle { manifest, files })
}

fn validate_manifest(manifest: &BundleManifest) -> Result<(), String> {
    if !manifest
        .examples
        .iter()
        .any(|entry| entry.name == manifest.default)
    {
        return Err("Bundled-example default is unavailable".to_string());
    }
    let mut names = std::collections::HashSet::new();
    for example in &manifest.examples {
        if example.name.is_empty()
            || !example
                .name
                .chars()
                .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == '-')
            || !names.insert(&example.name)
        {
            return Err(format!("Invalid bundled example name `{}`", example.name));
        }
    }
    Ok(())
}

fn validate_relative_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || Path::new(path)
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("Unsafe bundled-example path `{path}`"));
    }
    Ok(())
}

fn validate_digest(
    example: &BundledExample,
    files: &[(String, Vec<u8>, u32)],
) -> Result<(), String> {
    let mut digest = Sha256::new();
    for (path, data, mode) in files {
        digest.update(path.as_bytes());
        digest.update([0]);
        digest.update(mode.to_string().as_bytes());
        digest.update([0]);
        digest.update(data);
    }
    if format!("{:x}", digest.finalize()) != example.sha256 {
        return Err(format!(
            "Bundled example `{}` failed validation",
            example.name
        ));
    }
    Ok(())
}

fn unknown_example_message(name: &str, examples: &[BundledExample]) -> String {
    let available = examples
        .iter()
        .map(|entry| {
            format!(
                "  {} — {}\n    Note: {}",
                entry.name, entry.description, entry.caveats
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("Unknown bundled example `{name}`. Available examples:\n{available}")
}

fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32, String> {
    let mut bytes = [0; 4];
    cursor
        .read_exact(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(u32::from_be_bytes(bytes))
}

fn read_u64(cursor: &mut Cursor<&[u8]>) -> Result<u64, String> {
    let mut bytes = [0; 8];
    cursor
        .read_exact(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(u64::from_be_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_valid_and_has_expected_default() {
        let bundle = parse_bundle().unwrap();
        assert_eq!(bundle.manifest.default, "living-quilt");
        assert!(bundle
            .manifest
            .examples
            .iter()
            .any(|entry| entry.name == "ink-and-light"));
        assert!(bundle
            .manifest
            .examples
            .iter()
            .any(|entry| entry.name == "increment"));
    }

    #[test]
    fn unsafe_paths_are_rejected() {
        assert!(validate_relative_path("src/lib.rs").is_ok());
        assert!(validate_relative_path("../Cargo.toml").is_err());
        assert!(validate_relative_path("/tmp/project").is_err());
    }

    #[test]
    fn filtered_build_outputs_are_absent() {
        let bundle = parse_bundle().unwrap();
        assert!(bundle.files.iter().all(|file| {
            !file.path.split('/').any(|part| {
                matches!(
                    part,
                    ".git" | ".pax" | "target" | "node_modules" | "__pycache__"
                )
            }) && !file.path.ends_with("Cargo.lock")
        }));
        assert!(bundle
            .files
            .iter()
            .any(|file| file.path == "living-quilt/src/animated_pax_logo_banner.rs"));
        assert!(bundle
            .files
            .iter()
            .any(|file| file.path == "ink-and-light/assets/moon-postage.png"));
    }

    #[test]
    fn canonical_examples_use_selective_debug_profiles() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let examples = workspace.join("examples/src");
        if !examples.is_dir() {
            return;
        }
        let mut checked = 0;
        for entry in fs::read_dir(examples).unwrap() {
            let manifest = entry.unwrap().path().join("Cargo.toml");
            if !manifest.is_file() {
                continue;
            }
            let doc = fs::read_to_string(&manifest)
                .unwrap()
                .parse::<toml_edit::Document>()
                .unwrap();
            let name = doc["package"]["name"].as_str().unwrap();
            assert_eq!(
                doc["profile"]["dev"]["opt-level"].as_integer(),
                Some(1),
                "{}",
                manifest.display()
            );
            assert_eq!(
                doc["profile"]["dev"]["package"][name]["opt-level"].as_integer(),
                Some(0),
                "{}",
                manifest.display()
            );
            assert!(
                !doc["profile"]["dev"]["package"]
                    .as_table()
                    .unwrap()
                    .contains_key("*"),
                "{} must not optimize build dependencies through a wildcard",
                manifest.display()
            );
            checked += 1;
        }
        assert!(checked > 0);
    }

    #[test]
    fn checked_in_bundle_matches_canonical_examples() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let sync = workspace.join("scripts/sync-cli-examples.py");
        if !sync.is_file() {
            return;
        }
        let status = std::process::Command::new("python3")
            .arg(sync)
            .arg("--check")
            .current_dir(workspace)
            .status()
            .expect("failed to run bundled-example drift check");
        assert!(status.success());
    }
}
