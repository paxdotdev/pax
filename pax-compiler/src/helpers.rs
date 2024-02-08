use colored::{ColoredString, Colorize};
use include_dir::{include_dir, Dir};
use lazy_static::lazy_static;
use pax_manifest::HostCrateInfo;
use pax_runtime_api::serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{parse_file, ItemStruct};
use toml_edit;
use toml_edit::Document;

lazy_static! {
    #[allow(non_snake_case)]
    pub static ref PAX_BADGE: ColoredString = "[Pax]".bold().on_black().white();
    pub static ref DIR_IGNORE_LIST_MACOS : Vec<&'static str> = vec!["target", ".build", ".git", "tests"];
    pub static ref DIR_IGNORE_LIST_WEB : Vec<&'static str> = vec![".git"];
}

pub static PAX_CREATE_TEMPLATE: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/new-project-template");

pub const PAX_CREATE_LIBDEV_TEMPLATE_DIR_NAME: &str = "new-libdev-project-template";
pub const PKG_DIR_NAME: &str = "pkg";
pub const BUILD_DIR_NAME: &str = "build";
pub const PUBLIC_DIR_NAME: &str = "public";
pub const ASSETS_DIR_NAME: &str = "assets";

pub const ERR_SPAWN: &str = "failed to spawn child";

//whitelist of package ids that are relevant to the compiler, e.g. for cloning & patching, for assembling FS paths,
//or for looking up package IDs from a userland Cargo.lock.
pub const ALL_PKGS: [&'static str; 14] = [
    "pax-cartridge",
    "pax-chassis-common",
    "pax-chassis-ios",
    "pax-chassis-macos",
    "pax-chassis-web",
    "pax-cli",
    "pax-compiler",
    "pax-runtime",
    "pax-engine",
    "pax-macro",
    "pax-message",
    "pax-runtime-api",
    "pax-std",
    "pax-manifest",
];

#[derive(Debug, Deserialize)]
#[serde(crate = "pax_runtime_api::serde")]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "pax_runtime_api::serde")]
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

pub fn remove_path_from_pax_dependencies(full_path: &Path) {
    // Read the Cargo.toml
    let mut doc = fs::read_to_string(&full_path.join("Cargo.toml"))
        .expect("Failed to read Cargo.toml")
        .parse::<toml_edit::Document>()
        .expect("Failed to parse Cargo.toml");

    // Update the `dependencies` section
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
                dep_table.remove("path");
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

pub fn get_or_create_pax_directory(working_dir: &str) -> PathBuf {
    let working_path = std::path::Path::new(working_dir).join(".pax");
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

#[derive(Debug)]
pub struct InlinedTemplate {
    pub struct_name: String,
    pub start: (usize, usize),
    pub end: (usize, usize),
    pub template: String,
}

#[derive(Debug)]
pub struct InlinedTemplateFinder {
    pub file_contents: String,
    pub templates: Vec<InlinedTemplate>,
}

impl InlinedTemplateFinder {
    pub fn new(file_contents: String) -> Self {
        InlinedTemplateFinder {
            file_contents,
            templates: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for InlinedTemplateFinder {
    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        let mut has_pax = false;
        let struct_name = i.ident.to_string();
        for attr in &i.attrs {
            // Check for #[pax]
            if attr.path.is_ident("pax") {
                has_pax = true;
            }

            if attr.path.is_ident("inlined") {
                let start = attr.tokens.span().start();
                let start_tuple = (start.line, start.column + 1);
                let end = attr.tokens.span().end();
                let end_tuple = (end.line, end.column + 1);
                let content =
                    get_substring_by_line_column(&self.file_contents, start_tuple, end_tuple)
                        .unwrap()
                        .trim_start_matches("(")
                        .trim_end_matches(")")
                        .to_string();
                if has_pax {
                    let found_template = InlinedTemplate {
                        struct_name: struct_name.clone(),
                        start: start_tuple,
                        end: end_tuple,
                        template: content,
                    };
                    self.templates.insert(0, found_template);
                }
            }
        }
    }
}

fn find_start_end_bytes(
    input: &str,
    start: (usize, usize),
    end: (usize, usize),
) -> (Option<usize>, Option<usize>) {
    let mut current_line = 1;
    let mut current_column = 1;
    let mut start_byte = None;
    let mut end_byte = None;

    for (i, c) in input.char_indices() {
        if current_line == start.0 && current_column == start.1 {
            start_byte = Some(i);
        }
        if current_line == end.0 && current_column == end.1 {
            end_byte = Some(i);
            break;
        }

        if c == '\n' {
            current_line += 1;
            current_column = 1;
        } else {
            current_column += 1;
        }
    }

    (start_byte, end_byte)
}

pub fn replace_by_line_column(
    input: &str,
    start: (usize, usize),
    end: (usize, usize),
    replacement: String,
) -> Option<String> {
    let (start_byte, end_byte) = find_start_end_bytes(input, start, end);

    match (start_byte, end_byte) {
        (Some(start_byte), Some(end_byte)) => {
            let mut result = String::new();
            result.push_str(&input[..start_byte]);
            result.push_str(&replacement);
            result.push_str(&input[end_byte..]);
            Some(result)
        }
        _ => None,
    }
}

pub fn get_substring_by_line_column(
    input: &str,
    start: (usize, usize),
    end: (usize, usize),
) -> Option<String> {
    let (start_byte, end_byte) = find_start_end_bytes(input, start, end);

    match (start_byte, end_byte) {
        (Some(start_byte), Some(end_byte)) => Some(input[start_byte..end_byte].to_string()),
        _ => None,
    }
}

pub fn clear_inlined_template(file_path: &str, pascal_identifier: &str) {
    let path = Path::new(file_path);
    let content = fs::read_to_string(path).expect("Failed to read file");
    let ast = parse_file(&content).expect("Failed to parse file");

    let mut finder = InlinedTemplateFinder::new(content.clone());
    finder.visit_file(&ast);

    let mut modified_content = content;
    for template in finder.templates {
        if template.struct_name == pascal_identifier {
            let blank_template = format!("()");
            modified_content = replace_by_line_column(
                &modified_content,
                template.start,
                template.end,
                blank_template,
            )
            .unwrap();
        }
    }
    fs::write(path, modified_content).expect("Failed to write to file");
}
