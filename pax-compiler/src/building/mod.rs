//! # Building Module
//!
//! The `building` module provides structures and functions for building the complete chassis.
//! The `build_chassis_with_cartridge` function is the main entrypoint

use flate2::read::GzDecoder;
use libc::EXIT_FAILURE;
use pax_manifest::{HostCrateInfo, PaxManifest};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};

use color_eyre::eyre;
use tar::Archive;

use crate::{
    helpers::{copy_dir_recursively, ALL_PKGS, DIR_IGNORE_LIST_MACOS, PKG_DIR_NAME},
    RunContext, RunTarget,
};

use self::{apple::build_apple_chassis_with_cartridge, web::build_web_chassis_with_cartridge};

pub mod apple;
pub mod web;

#[cfg(feature = "designtime")]
use pax_designtime;

/// Runs `cargo build` (or `wasm-pack build`) with appropriate env in the directory
/// of the generated chassis project inside the specified .pax dir
/// Returns an output object containing bytestreams of stdout/stderr as well as an exit code
pub fn build_chassis_with_cartridge(
    pax_dir: &PathBuf,
    ctx: &RunContext,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
) -> Result<(), eyre::Report> {
    let target: &RunTarget = &ctx.target;
    let target_str: &str = target.into();
    let target_str_lower = &target_str.to_lowercase();
    let pax_dir = PathBuf::from(pax_dir.to_str().unwrap());
    let chassis_path = pax_dir
        .join(PKG_DIR_NAME)
        .join(format!("pax-chassis-{}", target_str_lower));

    //Inject `patch` directive, which allows userland projects to refer to concrete versions like `0.4.0`, while we
    //swap them for our locally cloned filesystem versions during compilation.
    let existing_cargo_toml_path = chassis_path.join("Cargo.toml");
    let existing_cargo_toml_string = fs::read_to_string(&existing_cargo_toml_path).unwrap();
    let mut existing_cargo_toml =
        toml_edit::Document::from_str(&existing_cargo_toml_string).unwrap();

    //In builds where we don't wipe out the `pkg` directory (e.g. those installed from crates.io),
    //the Cargo.toml may already have been patched.  Injecting an additional patch would break cargo.
    if !existing_cargo_toml_string.contains("patch.crates-io") {
        let mut patch_table = toml_edit::table();
        for pkg in ALL_PKGS {
            patch_table[pkg]["path"] = toml_edit::value(format!("../{}", pkg));
        }

        existing_cargo_toml.insert("patch.crates-io", patch_table);
        fs::write(
            existing_cargo_toml_path,
            existing_cargo_toml
                .to_string()
                .replace("\"patch.crates-io\"", "patch.crates-io"),
        )
        .unwrap();
    }

    //string together a shell call to build our chassis, with cartridge inserted via `patch`
    match target {
        RunTarget::macOS | RunTarget::iOS => {
            build_apple_chassis_with_cartridge(ctx, &pax_dir, process_child_ids)?;
        }
        RunTarget::Web => {
            build_web_chassis_with_cartridge(ctx, &pax_dir, process_child_ids)?;
        }
    }
    Ok(())
}

pub fn update_type_id_prefixes_in_place(
    manifest: &mut PaxManifest,
    host_crate_info: &HostCrateInfo,
) {
    manifest.main_component_type_id.fully_qualify_type_id(host_crate_info);
    let mut updated_type_table = HashMap::new();
    manifest.type_table.iter_mut().for_each(|t| {
        t.1.type_id.fully_qualify_type_id(host_crate_info);
        if let Some(inner) = &t.1.inner_iterable_type_id {
            inner.fully_qualify_type_id(host_crate_info);
            t.1.inner_iterable_type_id = Some(*inner);
        }
        t.1.property_definitions.iter_mut().for_each(|pd| {
            pd.type_id.fully_qualify_type_id(host_crate_info);
        });
        updated_type_table.insert(t.0.fully_qualify_type_id(host_crate_info).clone(), t.1.clone());
    });
    std::mem::swap(&mut manifest.type_table, &mut updated_type_table);

    let mut updated_component_table = HashMap::new();
    manifest.components.iter_mut().for_each(|c| {
        c.1.type_id.fully_qualify_type_id(host_crate_info);

        if let Some(template) = c.1.template.as_mut() {
            template.fully_qualify_template_type_ids(host_crate_info);
        }

        updated_component_table.insert(c.0.fully_qualify_type_id(host_crate_info).clone(), c.1.clone());
    });
    std::mem::swap(&mut manifest.components, &mut updated_component_table);
}

/// Clone all dependencies to `.pax/pkg`.  Similar in spirit to the Cargo package cache,
/// this temp directory enables Pax to codegen and building in the context of the larger monorepo,
/// working around various constraints with Cargo (for example, limits surrounding the `patch` directive.)
///
/// The packages in `.pax/pkg` are both where we write our codegen (into pax-cartridge)
/// and where we build chassis and chassis-interfaces. (for example, running `wasm-pack` inside `.pax/pkg/pax-chassis-web`.
/// This assumes that you are in the examples/src directory in the monorepo
pub fn clone_all_to_pkg_dir(pax_dir: &PathBuf, pax_version: &Option<String>, ctx: &RunContext) {
    let dest_pkg_root = pax_dir.join(PKG_DIR_NAME);

    #[cfg(feature = "designtime")]
    {
        if ctx.is_libdev_mode {
            let pax_corp_root = if let Ok(specified_override) = std::env::var("PAX_CORP_ROOT") {
                PathBuf::from(&specified_override)
            } else {
                unreachable!("PAX_CORP_ROOT must be set in libdev design mode")
            };
            let src = pax_corp_root.join("pax-designtime");
            let dest = dest_pkg_root.join("pax-designtime");

            copy_dir_recursively(&src, &dest, &DIR_IGNORE_LIST_MACOS)
                .expect(&format!("Failed to copy from {:?} to {:?}", src, dest));

            let _ =
                pax_designtime::add_additional_dependencies_to_cargo_toml(&dest, "pax-designtime");
        }
    }

    for pkg in ALL_PKGS {
        if ctx.is_libdev_mode {
            //Copy all packages from monorepo root on every build.  this allows us to propagate changes
            //to a libdev build without "sticky caches."
            let pax_workspace_root =
                if let Ok(specified_override) = std::env::var("PAX_WORKSPACE_ROOT") {
                    PathBuf::from(&specified_override)
                } else {
                    eprintln!("ERROR: environment PAX_WORKSPACE_ROOT needs to be set");
                    std::process::exit(EXIT_FAILURE);
                };

            let src = pax_workspace_root.join(pkg);
            let dest = dest_pkg_root.join(pkg);

            copy_dir_recursively(&src, &dest, &DIR_IGNORE_LIST_MACOS)
                .expect(&format!("Failed to copy from {:?} to {:?}", src, dest));

            #[cfg(feature = "designtime")]
            {
                let _ = pax_designtime::add_additional_dependencies_to_cargo_toml(&dest, pkg);
            }
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

                if resp.status().is_success() {
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
                                fs::create_dir_all(&parent)
                                    .expect("Failed to create parent directory");
                            }
                            entry.unpack(&path).expect("Failed to unpack file");
                        }
                    }
                } else {
                    eprintln!(
                        "Failed to download tarball for {} at version {}. Status: {}",
                        pkg,
                        pax_version,
                        resp.status()
                    );
                }
            }
        }
    }
}
