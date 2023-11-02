extern crate core;

mod building;
mod code_generation;
pub mod errors;
pub mod expressions;
mod helpers;
pub mod manifest;
pub mod parsing;
mod reexports;
pub mod templating;

use color_eyre::eyre;
use helpers::{copy_dir_recursively, wait_with_output};
use manifest::PaxManifest;
use std::fs;
use std::io::Write;
use std::sync::{Arc, Mutex};

use flate2::read::GzDecoder;
use fs_extra::dir::{self, CopyOptions};
use tar::Archive;

use color_eyre::eyre::Report;
use eyre::eyre;
use include_dir::{include_dir, Dir};
use lazy_static::lazy_static;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use crate::building::{build_chassis_with_cartridge, update_property_prefixes_in_place};

use crate::code_generation::{
    generate_and_overwrite_cartridge, generate_and_overwrite_properties_coproduct,
};
use crate::errors::source_map::SourceMap;
use crate::reexports::generate_reexports_partial_rs;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use colored::{ColoredString, Colorize};

use crate::helpers::{
    get_host_crate_info, get_or_create_pax_directory, get_version_of_whitelisted_packages,
    remove_path_from_pax_dependencies, set_path_on_pax_dependencies,
    update_pax_dependency_versions,
};

lazy_static! {
    #[allow(non_snake_case)]
    static ref PAX_BADGE: ColoredString = "[Pax]".bold().on_black().white();
    static ref DIR_IGNORE_LIST_MACOS : Vec<&'static str> = vec!["target", ".build", ".git"];
    static ref DIR_IGNORE_LIST_WEB : Vec<&'static str> = vec![".git"];
}

static PAX_CREATE_TEMPLATE: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/new-project-template");

const PAX_CREATE_LIBDEV_TEMPLATE_DIR_NAME: &str = "new-libdev-project-template";
const PKG_DIR_NAME: &str = "pkg";
const BUILD_DIR_NAME: &str = "build";
const PUBLIC_DIR_NAME: &str = "public";
const ASSETS_DIR_NAME: &str = "assets";

const ERR_SPAWN: &str = "failed to spawn child";

//whitelist of package ids that are relevant to the compiler, e.g. for cloning & patching, for assembling FS paths,
//or for looking up package IDs from a userland Cargo.lock.
const ALL_PKGS: [&'static str; 14] = [
    "pax-cartridge",
    "pax-chassis-common",
    "pax-chassis-ios",
    "pax-chassis-macos",
    "pax-chassis-web",
    "pax-cli",
    "pax-compiler",
    "pax-core",
    "pax-lang",
    "pax-macro",
    "pax-message",
    "pax-properties-coproduct",
    "pax-runtime-api",
    "pax-std",
];

/// For the specified file path or current working directory, first compile Pax project,
/// then run it with a patched build of the `chassis` appropriate for the specified platform
/// See: pax-compiler-sequence-diagram.png
pub fn perform_build(ctx: &RunContext) -> eyre::Result<(), Report> {
    //First we clone dependencies into the .pax/pkg directory.  We must do this before running
    //the parser binary specifical for libdev in pax-example — see pax-example/Cargo.toml where
    //dependency paths are `.pax/pkg/*`.
    let pax_dir = get_or_create_pax_directory(&ctx.path);

    //Inspect Cargo.lock to find declared pax lib versions.  Note that this is moot for
    //libdev, where we don't care about a crates.io version (and where `cargo metadata` won't work
    //on a cold-start monorepo clone.)
    let pax_version = if ctx.is_libdev_mode {
        None
    } else {
        Some(get_version_of_whitelisted_packages(&ctx.path).unwrap())
    };
    clone_all_to_pkg_dir(&pax_dir, &pax_version, &ctx);

    if ctx.is_libdev_mode {
        let full_path = Path::new(&ctx.path);
        set_path_on_pax_dependencies(&full_path);
    }

    println!("{} 🛠️  Building parser binary with `cargo`...", *PAX_BADGE);
    // Run parser bin from host project with `--features parser`
    let output = run_parser_binary(&ctx.path, Arc::clone(&ctx.process_child_ids));

    // Forward stderr only
    std::io::stderr()
        .write_all(output.stderr.as_slice())
        .unwrap();

    if !output.status.success() {
        return Err(eyre!(
            "Parsing failed — there is likely a syntax error in the provided pax"
        ));
    }

    let out = String::from_utf8(output.stdout).unwrap();
    let mut manifest: PaxManifest =
        serde_json::from_str(&out).expect(&format!("Malformed JSON from parser: {}", &out));
    let host_cargo_toml_path = Path::new(&ctx.path).join("Cargo.toml");
    let host_crate_info = get_host_crate_info(&host_cargo_toml_path);
    update_property_prefixes_in_place(&mut manifest, &host_crate_info);

    let mut source_map = SourceMap::new();

    println!("{} 🧮 Compiling expressions", *PAX_BADGE);
    expressions::compile_all_expressions(&mut manifest, &mut source_map)?;

    println!("{} 🦀 Generating Rust", *PAX_BADGE);
    generate_reexports_partial_rs(&pax_dir, &manifest);
    generate_and_overwrite_properties_coproduct(&pax_dir, &manifest, &host_crate_info);
    let cartridge_path =
        generate_and_overwrite_cartridge(&pax_dir, &manifest, &host_crate_info, &mut source_map);
    source_map.extract_ranges_from_generated_code(cartridge_path.to_str().unwrap());

    //7. Build the appropriate `chassis` from source, with the patched `Cargo.toml`, Properties Coproduct, and Cartridge from above
    println!("{} 🧱 Building cartridge with `cargo`", *PAX_BADGE);
    build_chassis_with_cartridge(
        &pax_dir,
        &ctx,
        Arc::clone(&ctx.process_child_ids),
        &source_map,
    )?;
    Ok(())
}

/// Clean all `.pax` temp files
pub fn perform_clean(path: &str) {
    let path = PathBuf::from(path);
    let pax_dir = path.join(".pax");

    remove_path_from_pax_dependencies(&path);

    fs::remove_dir_all(&pax_dir).ok();
}

pub fn perform_create(ctx: &CreateContext) {
    let full_path = Path::new(&ctx.path);

    // Abort if directory already exists
    if full_path.exists() {
        panic!("Error: destination `{:?}` already exists", full_path);
    }
    let _ = fs::create_dir_all(&full_path);

    // clone template into full_path
    if ctx.is_libdev_mode {
        //For is_libdev_mode, we copy our monorepo @/pax-compiler/new-project-template directory
        //to the target directly.  This enables iterating on new-project-template during libdev
        //without the sticky caches associated with `include_dir`
        let pax_compiler_cargo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let template_src = pax_compiler_cargo_root.join(PAX_CREATE_LIBDEV_TEMPLATE_DIR_NAME);

        let mut options = CopyOptions::new();
        options.overwrite = true;

        for entry in std::fs::read_dir(&template_src).expect("Failed to read template directory") {
            let entry_path = entry.expect("Failed to read entry").path();
            if entry_path.is_dir() {
                dir::copy(&entry_path, &full_path, &options).expect("Failed to copy directory");
            } else {
                fs::copy(&entry_path, full_path.join(entry_path.file_name().unwrap()))
                    .expect("Failed to copy file");
            }
        }
    } else {
        // File src is include_dir — recursively extract files from include_dir into full_path
        PAX_CREATE_TEMPLATE
            .extract(&full_path)
            .expect("Failed to extract files");
    }

    //Patch Cargo.toml
    let cargo_template_path = full_path.join("Cargo.toml.template");
    let extracted_cargo_toml_path = full_path.join("Cargo.toml");
    let _ = fs::copy(&cargo_template_path, &extracted_cargo_toml_path);
    let _ = fs::remove_file(&cargo_template_path);

    let crate_name = full_path.file_name().unwrap().to_str().unwrap().to_string();

    // Read the Cargo.toml
    let mut doc = fs::read_to_string(&full_path.join("Cargo.toml"))
        .expect("Failed to read Cargo.toml")
        .parse::<toml_edit::Document>()
        .expect("Failed to parse Cargo.toml");

    // Update the `dependencies` section
    update_pax_dependency_versions(&mut doc, &ctx.version);

    // Update the `package` section
    if let Some(package) = doc
        .as_table_mut()
        .entry("package")
        .or_insert_with(toml_edit::table)
        .as_table_mut()
    {
        if let Some(name_item) = package.get_mut("name") {
            *name_item = toml_edit::Item::Value(crate_name.into());
        }
        if let Some(version_item) = package.get_mut("version") {
            *version_item = toml_edit::Item::Value(ctx.version.clone().into());
        }
    }

    // Write the modified Cargo.toml back to disk
    fs::write(&full_path.join("Cargo.toml"), doc.to_string())
        .expect("Failed to write modified Cargo.toml");

    println!(
        "\nCreated new Pax project at {}.\nTo run:\n  `cd {} && pax-cli run --target=web`",
        full_path.to_str().unwrap(),
        full_path.to_str().unwrap()
    );
}

/// Clone all dependencies to `.pax/pkg`.  Similar in spirit to the Cargo package cache,
/// this temp directory enables Pax to codegen and building in the context of the larger monorepo,
/// working around various constraints with Cargo (for example, limits surrounding the `patch` directive.)
///
/// The packages in `.pax/pkg` are both where we write our codegen (into pax-cartridge and pax-properties-coproduct)
/// and where we build chassis and chassis-interfaces. (for example, running `wasm-pack` inside `.pax/pkg/pax-chassis-web`.
/// This assumes that you are in the examples/src directory in the monorepo
fn clone_all_to_pkg_dir(pax_dir: &PathBuf, pax_version: &Option<String>, ctx: &RunContext) {
    let dest_pkg_root = pax_dir.join(PKG_DIR_NAME);
    for pkg in ALL_PKGS {
        if ctx.is_libdev_mode {
            //Copy all packages from monorepo root on every build.  this allows us to propagate changes
            //to a libdev build without "sticky caches."
            let pax_workspace_root = pax_dir
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap();
            let src = pax_workspace_root.join(pkg);
            let dest = dest_pkg_root.join(pkg);

            copy_dir_recursively(&src, &dest, &DIR_IGNORE_LIST_MACOS)
                .expect(&format!("Failed to copy from {:?} to {:?}", src, dest));
        } else {
            let dest = dest_pkg_root.join(pkg);
            if !dest.exists() {
                let pax_version = pax_version
                    .as_ref()
                    .expect("Pax version required but not found");
                let tarball_url = format!(
                    "https://crates.io/api/v1/crates/{}/{}/download",
                    pkg, pax_version
                );
                let resp = reqwest::blocking::get(&tarball_url).expect(&format!(
                    "Failed to fetch tarball for {} at version {}",
                    pkg, pax_version
                ));

                let tarball_bytes = resp.bytes().expect("Failed to read tarball bytes");

                // Wrap the byte slice in a Cursor, so it can be used as a Read trait object.
                let cursor = std::io::Cursor::new(&tarball_bytes[..]);

                // Create a GzDecoder to handle the gzip layer.
                let gz = GzDecoder::new(cursor);

                // Pass the GzDecoder to tar::Archive.
                let mut archive = Archive::new(gz);
                // Iterate over the entries in the archive and modify the paths before extracting.
                for entry_result in archive.entries().expect("Failed to read entries") {
                    let mut entry = entry_result.expect("Failed to read entry");
                    let path = match entry
                        .path()
                        .expect("Failed to get path")
                        .components()
                        .skip(1)
                        .collect::<PathBuf>()
                        .as_path()
                        .to_owned()
                    {
                        path if path.to_string_lossy() == "" => continue, // Skip the root folder
                        path => dest.join(path),
                    };
                    if entry.header().entry_type().is_dir() {
                        fs::create_dir_all(&path).expect("Failed to create directory");
                    } else {
                        if let Some(parent) = path.parent() {
                            fs::create_dir_all(&parent).expect("Failed to create parent directory");
                        }
                        entry.unpack(&path).expect("Failed to unpack file");
                    }
                }
            }
        }
    }
}

#[allow(unused)]
static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

/// Executes a shell command to run the feature-flagged parser at the specified path
/// Returns an output object containing bytestreams of stdout/stderr as well as an exit code
pub fn run_parser_binary(path: &str, process_child_ids: Arc<Mutex<Vec<u64>>>) -> Output {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(path)
        .arg("run")
        .arg("--bin")
        .arg("parser")
        .arg("--features")
        .arg("parser")
        .arg("--color")
        .arg("always")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(pre_exec_hook);
    }

    let child = cmd.spawn().expect(ERR_SPAWN);

    // child.stdin.take().map(drop);
    let output = wait_with_output(&process_child_ids, child);
    output
}

pub struct CreateContext {
    pub path: String,
    pub is_libdev_mode: bool,
    pub version: String,
}

pub struct RunContext {
    pub target: RunTarget,
    pub path: String,
    pub verbose: bool,
    pub should_also_run: bool,
    pub is_libdev_mode: bool,
    pub process_child_ids: Arc<Mutex<Vec<u64>>>,
    pub is_release: bool,
}

pub enum RunTarget {
    #[allow(non_camel_case_types)]
    macOS,
    Web,
    #[allow(non_camel_case_types)]
    iOS,
}

impl From<&str> for RunTarget {
    fn from(input: &str) -> Self {
        match input.to_lowercase().as_str() {
            "macos" => RunTarget::macOS,
            "web" => RunTarget::Web,
            "ios" => RunTarget::iOS,
            _ => {
                unreachable!()
            }
        }
    }
}

impl<'a> Into<&'a str> for &'a RunTarget {
    fn into(self) -> &'a str {
        match self {
            RunTarget::Web => "Web",
            RunTarget::macOS => "macOS",
            RunTarget::iOS => "iOS",
        }
    }
}

#[cfg(unix)]
fn pre_exec_hook() -> Result<(), std::io::Error> {
    // Set a new process group for this command
    unsafe {
        libc::setpgid(0, 0);
    }
    Ok(())
}
