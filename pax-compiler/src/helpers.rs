use colored::{ColoredString, Colorize};
use include_dir::{include_dir, Dir};
use lazy_static::lazy_static;
use pax_manifest::HostCrateInfo;
use pax_runtime::api::serde::Deserialize;
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use toml_edit;
use toml_edit::Document;

lazy_static! {
    #[allow(non_snake_case)]
    pub static ref PAX_BADGE: ColoredString = "[Pax]".bold().on_black().white();
    pub static ref DIR_IGNORE_LIST_MACOS : Vec<&'static str> = vec!["target", ".build", ".git", "tests"];
    pub static ref DIR_IGNORE_LIST_WEB : Vec<&'static str> = vec![".git"];
}

pub static PAX_CREATE_TEMPLATE: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/files/new-project/new-project-template");
pub static PAX_WEB_INTERFACE_TEMPLATE: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/files/interfaces/web/public/");
pub static PAX_MACOS_INTERFACE_TEMPLATE: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/files/interfaces/macos/");
pub static PAX_IOS_INTERFACE_TEMPLATE: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/files/interfaces/ios/");

pub static PAX_SWIFT_CARTRIDGE_TEMPLATE: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/files/swift/pax-swift-cartridge/");
pub static PAX_SWIFT_COMMON_TEMPLATE: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/files/swift/pax-swift-common/");

pub const PAX_CREATE_LIBDEV_TEMPLATE_DIR_NAME: &str = "new-libdev-project-template";
pub const INTERFACE_DIR_NAME: &str = "interface";
pub const BUILD_DIR_NAME: &str = "build";
pub const PUBLIC_DIR_NAME: &str = "public";
pub const ASSETS_DIR_NAME: &str = "assets";

pub const ERR_SPAWN: &str = "failed to spawn child";

//whitelist of package ids that are relevant to the compiler, e.g. for cloning & patching, for assembling FS paths,
//or for looking up package IDs from a userland Cargo.lock.
pub const ALL_PKGS: &[&str] = &[
    "pax-chassis-common",
    "pax-chassis-ios",
    "pax-chassis-macos",
    "pax-chassis-web",
    "pax-cli",
    "pax-compiler",
    "pax-designtime",
    "pax-kit",
    "pax-runtime",
    "pax-runtime-api",
    "pax-engine",
    "pax-macro",
    "pax-message",
    "pax-std",
    "pax-manifest",
    "pax-language",
];

#[derive(Default)]
struct ProjectFeatureConfig {
    local_features: HashSet<String>,
    dependencies: HashSet<String>,
}

impl ProjectFeatureConfig {
    fn from_project_path(project_path: &Path) -> Option<Self> {
        let manifest_path = project_path.join("Cargo.toml");
        let document = fs::read_to_string(&manifest_path)
            .ok()?
            .parse::<Document>()
            .ok()?;

        Some(Self {
            local_features: table_keys(&document, "features"),
            dependencies: table_keys(&document, "dependencies"),
        })
    }

    fn has_local_feature(&self, feature: &str) -> bool {
        self.local_features.contains(feature)
    }

    fn has_dependency(&self, dependency: &str) -> bool {
        self.dependencies.contains(dependency)
    }
}

fn table_keys(document: &Document, table_name: &str) -> HashSet<String> {
    document
        .get(table_name)
        .and_then(|item| item.as_table())
        .map(|table| table.iter().map(|(key, _)| key.to_string()).collect())
        .unwrap_or_default()
}

fn push_unique(features: &mut Vec<String>, feature: impl Into<String>) {
    let feature = feature.into();
    if !features.iter().any(|existing| existing == &feature) {
        features.push(feature);
    }
}

fn pax_dependency_feature_selectors(config: &ProjectFeatureConfig, feature: &str) -> Vec<String> {
    if config.has_dependency("pax-kit") {
        return vec![format!("pax-kit/{feature}")];
    }

    match feature {
        "parser" if config.has_dependency("pax-std") => vec!["pax-std/parser".to_string()],
        "designtime" if config.has_dependency("pax-std") => {
            vec!["pax-std/designtime".to_string()]
        }
        "designtime" if config.has_dependency("pax-engine") => {
            vec!["pax-engine/designtime".to_string()]
        }
        "designer" if config.has_dependency("pax-designer") => {
            vec!["pax-designer/designtime".to_string()]
        }
        "web" | "webgl" | "macos" | "ios" if config.has_dependency("pax-engine") => {
            vec![format!("pax-engine/{feature}")]
        }
        _ => vec![],
    }
}

pub fn pax_project_feature_args(project_path: &Path, requested_features: &[&str]) -> Vec<String> {
    let Some(config) = ProjectFeatureConfig::from_project_path(project_path) else {
        return requested_features
            .iter()
            .map(|feature| feature.to_string())
            .collect();
    };

    let mut features = vec![];
    for requested_feature in requested_features {
        if config.has_local_feature(requested_feature) {
            push_unique(&mut features, *requested_feature);
        }

        for dependency_feature in pax_dependency_feature_selectors(&config, requested_feature) {
            push_unique(&mut features, dependency_feature);
        }

        if features.is_empty() || !config.has_local_feature(requested_feature) {
            let has_requested_dependency_feature = features
                .iter()
                .any(|feature| feature.ends_with(&format!("/{requested_feature}")));
            if !has_requested_dependency_feature {
                push_unique(&mut features, *requested_feature);
            }
        }
    }

    features
}

/// Resolves the exact Cargo graph built for the requested release features and
/// target triples, then reports any runtime path which contains Pax's
/// designtime protocol. `cargo metadata`'s node features are unified across
/// dev and build dependency contexts, so use Cargo's normal-edge tree here to
/// match what `cargo build --release` will actually compile.
pub fn pax_project_release_devtime_activators(
    project_path: &Path,
    requested_features: &[String],
    target_triples: &[&str],
) -> Result<Vec<String>, String> {
    let manifest_path = project_path.join("Cargo.toml");
    let mut activators = BTreeSet::new();

    for target_triple in target_triples {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let mut command = Command::new(cargo);
        command
            .arg("tree")
            .arg("--manifest-path")
            .arg(&manifest_path)
            .arg("--target")
            .arg(target_triple)
            .arg("--edges")
            .arg("normal,no-proc-macro")
            .arg("--prefix")
            .arg("depth")
            .arg("--format")
            .arg("{p}\t{f}")
            .arg("--charset")
            .arg("ascii")
            .arg("--color")
            .arg("never")
            .arg("--quiet");
        if !requested_features.is_empty() {
            command.arg("--features").arg(requested_features.join(","));
        }

        let output = command
            .output()
            .map_err(|err| format!("failed to run Cargo's release dependency resolver: {err}"))?;
        if !output.status.success() {
            return Err(format!(
                "failed to resolve release Cargo features for target {target_triple}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        collect_release_devtime_activators(
            &String::from_utf8_lossy(&output.stdout),
            &mut activators,
        )?;
    }

    Ok(activators.into_iter().collect())
}

fn collect_release_devtime_activators(
    cargo_tree: &str,
    activators: &mut BTreeSet<String>,
) -> Result<(), String> {
    let mut path = Vec::<String>::new();

    for line in cargo_tree.lines().filter(|line| !line.trim().is_empty()) {
        let depth_len = line
            .as_bytes()
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if depth_len == 0 {
            return Err(format!("unexpected Cargo dependency tree line: {line}"));
        }
        let depth = line[..depth_len]
            .parse::<usize>()
            .map_err(|err| format!("invalid Cargo dependency depth in `{line}`: {err}"))?;
        let (package, features) = line[depth_len..]
            .split_once('\t')
            .ok_or_else(|| format!("Cargo dependency tree omitted features in `{line}`"))?;
        let package_name = package
            .split_whitespace()
            .next()
            .ok_or_else(|| format!("Cargo dependency tree omitted a package name in `{line}`"))?;
        if depth > path.len() {
            return Err(format!("unexpected Cargo dependency depth in `{line}`"));
        }
        path.truncate(depth);
        path.push(package_name.to_string());

        if matches!(package_name, "pax-designtime" | "pax-designer") {
            activators.insert(format!("runtime dependency path `{}`", path.join(" -> ")));
        }
        if package_name.starts_with("pax-") {
            for feature in features.split(',') {
                if matches!(feature, "designtime" | "designer") {
                    activators.insert(format!(
                        "runtime dependency path `{}` enables `{feature}`",
                        path.join(" -> ")
                    ));
                }
            }
        }
    }

    Ok(())
}

pub fn configure_pax_build_env(
    cmd: &mut Command,
    target: &str,
    should_run_designtime: bool,
    should_run_designer: bool,
) {
    cmd.env("PAX_BUILD_TARGET", target)
        .env(
            "PAX_BUILD_DESIGNTIME",
            if should_run_designer || should_run_designtime {
                "1"
            } else {
                "0"
            },
        )
        .env(
            "PAX_BUILD_DESIGNER",
            if should_run_designer { "1" } else { "0" },
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_manifest(contents: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        fs::write(dir.path().join("Cargo.toml"), contents).expect("manifest should be written");
        dir
    }

    fn write_local_crate(path: &Path, name: &str, manifest_body: &str) {
        fs::create_dir_all(path.join("src")).unwrap();
        fs::write(
            path.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n{manifest_body}"
            ),
        )
        .unwrap();
        fs::write(path.join("src/lib.rs"), "pub fn fixture() {}\n").unwrap();
    }

    #[test]
    fn pax_kit_projects_get_local_and_dependency_features() {
        let dir = write_manifest(
            r#"
[package]
name = "demo"
version = "0.1.0"
edition = "2021"

[dependencies]
pax-kit = "0.38.3"

[features]
web = []
designtime = []
designer = []
parser = []
"#,
        );

        assert_eq!(
            pax_project_feature_args(
                dir.path(),
                &["web", "designtime", "designer", "parser", "webgl"]
            ),
            vec![
                "web",
                "pax-kit/web",
                "designtime",
                "pax-kit/designtime",
                "designer",
                "pax-kit/designer",
                "parser",
                "pax-kit/parser",
                "pax-kit/webgl",
            ]
        );
    }

    #[test]
    fn pax_kit_projects_do_not_need_local_feature_stubs() {
        let dir = write_manifest(
            r#"
[package]
name = "demo"
version = "0.1.0"
edition = "2021"

[dependencies]
pax-kit = "0.38.3"
"#,
        );

        assert_eq!(
            pax_project_feature_args(
                dir.path(),
                &["web", "designtime", "designer", "parser", "webgl"]
            ),
            vec![
                "pax-kit/web",
                "pax-kit/designtime",
                "pax-kit/designer",
                "pax-kit/parser",
                "pax-kit/webgl",
            ]
        );
    }

    #[test]
    fn direct_engine_projects_keep_existing_feature_names() {
        let dir = write_manifest(
            r#"
[package]
name = "demo"
version = "0.1.0"
edition = "2021"

[dependencies]
pax-engine = "0.38.3"
pax-std = "0.38.3"

[features]
web = ["pax-engine/web"]
designtime = ["pax-engine/designtime", "pax-std/designtime"]
"#,
        );

        assert_eq!(
            pax_project_feature_args(dir.path(), &["web", "designtime", "parser"]),
            vec![
                "web",
                "pax-engine/web",
                "designtime",
                "pax-std/designtime",
                "pax-std/parser",
            ]
        );
    }

    #[test]
    fn unreadable_manifest_falls_back_to_requested_features() {
        let dir = tempfile::tempdir().expect("tempdir should be created");

        assert_eq!(
            pax_project_feature_args(dir.path(), &["web", "designtime"]),
            vec!["web", "designtime"]
        );
    }

    #[test]
    fn release_guard_follows_transitive_workspace_runtime_features() {
        let workspace = tempfile::tempdir().unwrap();
        fs::write(
            workspace.path().join("Cargo.toml"),
            r#"
[workspace]
resolver = "2"
members = ["app", "widget", "pax-engine"]

[workspace.dependencies]
widget = { path = "widget" }
"#,
        )
        .unwrap();
        write_local_crate(
            &workspace.path().join("pax-engine"),
            "pax-engine",
            "[features]\ndesigntime = []\n",
        );
        write_local_crate(
            &workspace.path().join("widget"),
            "widget",
            "[dependencies]\npax-engine = { path = \"../pax-engine\", features = [\"designtime\"] }\n",
        );
        let app = workspace.path().join("app");
        write_local_crate(
            &app,
            "app",
            "[dependencies]\nwidget.workspace = true\n\n[features]\nweb = []\n",
        );

        let activators = pax_project_release_devtime_activators(
            &app,
            &["web".to_string()],
            &["wasm32-unknown-unknown"],
        )
        .unwrap();
        assert_eq!(
            activators,
            vec!["runtime dependency path `app -> widget -> pax-engine` enables `designtime`"]
        );
    }

    #[test]
    fn release_guard_ignores_inactive_optional_dev_and_off_target_dependencies() {
        let workspace = tempfile::tempdir().unwrap();
        fs::write(
            workspace.path().join("Cargo.toml"),
            "[workspace]\nresolver = \"2\"\nmembers = [\"app\", \"pax-designer\", \"pax-designtime\", \"pax-engine\"]\n",
        )
        .unwrap();
        write_local_crate(&workspace.path().join("pax-designer"), "pax-designer", "");
        write_local_crate(
            &workspace.path().join("pax-designtime"),
            "pax-designtime",
            "",
        );
        write_local_crate(
            &workspace.path().join("pax-engine"),
            "pax-engine",
            "[features]\ndesigntime = []\n",
        );
        let app = workspace.path().join("app");
        write_local_crate(
            &app,
            "app",
            r#"[dependencies]
optional-tools = { package = "pax-designer", path = "../pax-designer", optional = true }
pax-engine = { path = "../pax-engine" }

[dev-dependencies]
pax-engine = { path = "../pax-engine", features = ["designtime"] }

[target.'cfg(target_os = "windows")'.dependencies]
windows-tools = { package = "pax-designtime", path = "../pax-designtime" }

[features]
default = ["shipping-theme"]
shipping-theme = []
web = []
"#,
        );

        assert!(pax_project_release_devtime_activators(
            &app,
            &["web".to_string()],
            &["wasm32-unknown-unknown"],
        )
        .unwrap()
        .is_empty());
    }

    #[test]
    fn release_guard_reads_deduplicated_cargo_tree_entries() {
        let mut activators = BTreeSet::new();
        collect_release_devtime_activators(
            "0app v0.1.0\t\n1wrapper v0.1.0\t\n2pax-engine v0.1.0 (*)\tdesigntime,web\n",
            &mut activators,
        )
        .unwrap();

        assert_eq!(
            activators.into_iter().collect::<Vec<_>>(),
            vec!["runtime dependency path `app -> wrapper -> pax-engine` enables `designtime`"]
        );
    }
}

#[derive(Debug, Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    version: String,
}

pub fn set_path_on_pax_dependencies(full_path: &Path) {
    // Read the Cargo.toml
    let mut doc = fs::read_to_string(&full_path.join("Cargo.toml"))
        .expect("Failed to read Cargo.toml")
        .parse::<toml_edit::Document>()
        .expect("Failed to parse Cargo.toml");

    // Update the `dependencies` section to set path
    if let Some(deps) = doc
        .as_table_mut()
        .entry("dependencies")
        .or_insert_with(toml_edit::table)
        .as_table_mut()
    {
        let keys: Vec<String> = deps
            .iter()
            .filter_map(|(key, _)| {
                if key.starts_with("pax-") {
                    Some(key.to_string())
                } else {
                    None
                }
            })
            .collect();

        for key in keys {
            let dep_entry = deps.get_mut(&key).unwrap();

            if let toml_edit::Item::Value(toml_edit::Value::InlineTable(ref mut dep_table)) =
                dep_entry
            {
                dep_table.insert(
                    "path",
                    toml_edit::Value::String(toml_edit::Formatted::new(
                        ".pax/pkg/".to_string() + &key,
                    )),
                );
            }
        }
    }

    // Write the modified Cargo.toml back to disk
    fs::write(&full_path.join("Cargo.toml"), doc.to_string())
        .expect("Failed to write modified Cargo.toml");
}

pub fn update_pax_dependency_versions(doc: &mut Document, ctx_version: &str) {
    if let Some(deps) = doc
        .as_table_mut()
        .entry("dependencies")
        .or_insert_with(toml_edit::table)
        .as_table_mut()
    {
        let keys: Vec<String> = deps
            .iter()
            .filter_map(|(key, _)| {
                if key.starts_with("pax-") {
                    Some(key.to_string())
                } else {
                    None
                }
            })
            .collect();

        for key in keys {
            let dep_entry = deps.get_mut(&key).unwrap();

            if let toml_edit::Item::Value(toml_edit::Value::InlineTable(ref mut dep_table)) =
                dep_entry
            {
                dep_table.insert(
                    "version",
                    toml_edit::Value::String(toml_edit::Formatted::new(ctx_version.to_string())),
                );
            } else {
                let dep_string = format!("version = \"{}\"", ctx_version);
                *dep_entry = toml_edit::Item::from_str(&dep_string).unwrap_or_default();
            }
        }
    }
}

const ERR_LOCK: &str = "Failed to lock process_child_ids mutex";

pub fn wait_with_output(
    process_child_ids: &Arc<Mutex<Vec<u64>>>,
    child: std::process::Child,
) -> std::process::Output {
    let child_id: u64 = child.id().into();

    // Push the child_id to the shared process_child_ids vector
    process_child_ids.lock().expect(ERR_LOCK).push(child_id);

    // Wait for the child process to complete
    let output = child
        .wait_with_output()
        .expect("Failed to wait for child process");

    // Ensure the child ID is removed after completion
    process_child_ids
        .lock()
        .expect(ERR_LOCK)
        .retain(|&id| id != child_id);

    output
}

pub fn get_or_create_pax_directory(project_path: &PathBuf) -> PathBuf {
    let working_path = std::path::Path::new(project_path).join(".pax");
    std::fs::create_dir_all(&working_path).unwrap();
    fs::canonicalize(working_path).unwrap()
}

pub fn get_version_of_whitelisted_packages(path: &str) -> Result<String, &'static str> {
    let mut cmd = Command::new("cargo");
    let output = cmd
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(path)
        .output()
        .expect("Failed to execute `cargo metadata`");

    if !output.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        panic!("Failed to get metadata from Cargo");
    }

    let metadata: Metadata =
        serde_json::from_slice(&output.stdout).expect("Failed to parse JSON from `cargo metadata`");

    let mut tracked_version: Option<String> = None;

    for package in &metadata.packages {
        if ALL_PKGS.contains(&package.name.as_str()) {
            if let Some(ref version) = tracked_version {
                if package.version != *version {
                    panic!(
                        "Version mismatch for {}: expected {}, found {}",
                        package.name, version, package.version
                    );
                }
            } else {
                tracked_version = Some(package.version.clone());
            }
        }
    }

    tracked_version.ok_or("Cannot build a Pax project without a `pax-*` dependency somewhere in your project's dependency graph.  Add e.g. `pax-engine` to your Cargo.toml to resolve this error.")
}

/// Helper recursive fs copy method, like fs::copy, but suited for our purposes.
/// Includes ability to ignore directories by name during recursion.
pub fn copy_dir_recursively(
    src: &Path,
    dest: &Path,
    ignore_list: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    if src.is_dir() {
        // If the directory name is in the ignore list, we skip this directory
        if ignore_list.contains(&src.file_name().unwrap().to_str().unwrap()) {
            return Ok(());
        }

        // Create the corresponding directory in the destination,
        // and copy its contents recursively
        fs::create_dir_all(dest)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            let dest_child = dest.join(path.file_name().ok_or("Invalid file name")?);
            copy_dir_recursively(&path, &dest_child, ignore_list)?;
        }
    } else {
        // If source is a file, just copy it to the destination
        fs::copy(src, dest)?;
    }
    Ok(())
}

pub fn get_host_crate_info(cargo_toml_path: &Path) -> HostCrateInfo {
    let existing_cargo_toml = toml_edit::Document::from_str(
        &fs::read_to_string(fs::canonicalize(cargo_toml_path).unwrap()).unwrap(),
    )
    .expect("Error loading host Cargo.toml");

    let name = existing_cargo_toml["package"]["name"]
        .as_str()
        .unwrap()
        .to_string();
    let identifier = name.replace("-", "_"); //NOTE: perhaps this could be less naive?
    let import_prefix = format!("{}::pax_reexports::", &identifier);

    HostCrateInfo {
        name,
        identifier,
        import_prefix,
    }
}
