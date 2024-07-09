//! # Building Module
//!
//! The `building` module provides structures and functions for building the complete chassis.
//! The `build_chassis_with_cartridge` function is the main entrypoint

use flate2::read::GzDecoder;
use libc::EXIT_FAILURE;
use pax_manifest::{HostCrateInfo, PaxManifest};
use std::{
    collections::{BTreeMap, HashMap},
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
mod design;

/// Runs `cargo build` (or `wasm-pack build`) with appropriate env in the directory
/// of the generated chassis project inside the specified .pax dir
/// Returns an output object containing bytestreams of stdout/stderr as well as an exit code
pub fn build_chassis_with_cartridge(
    pax_dir: &PathBuf,
    ctx: &RunContext,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
) -> Result<Option<PathBuf>, eyre::Report> {
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
            Ok(None)
        }
        RunTarget::Web => {
            let fs = build_web_chassis_with_cartridge(ctx, &pax_dir, process_child_ids)?;
            Ok(Some(fs))
        }
    }
}

pub fn update_type_id_prefixes_in_place(
    manifest: &mut PaxManifest,
    host_crate_info: &HostCrateInfo,
) {
    manifest
        .main_component_type_id
        .fully_qualify_type_id(host_crate_info);
    let mut updated_type_table = HashMap::new();
    manifest.type_table.iter_mut().for_each(|t| {
        t.1.type_id.fully_qualify_type_id(host_crate_info);
        if let Some(inner) = &mut t.1.inner_iterable_type_id {
            inner.fully_qualify_type_id(host_crate_info);
        }
        t.1.property_definitions.iter_mut().for_each(|pd| {
            pd.type_id.fully_qualify_type_id(host_crate_info);
        });
        let mut key = t.0.clone();
        updated_type_table.insert(
            key.fully_qualify_type_id(host_crate_info).clone(),
            t.1.clone(),
        );
    });
    std::mem::swap(&mut manifest.type_table, &mut updated_type_table);

    let mut updated_component_table = BTreeMap::new();
    manifest.components.iter_mut().for_each(|c| {
        c.1.type_id.fully_qualify_type_id(host_crate_info);

        if let Some(template) = c.1.template.as_mut() {
            template.fully_qualify_template_type_ids(host_crate_info);
        }

        let mut key = c.0.clone();
        updated_component_table.insert(
            key.fully_qualify_type_id(host_crate_info).clone(),
            c.1.clone(),
        );
    });
    std::mem::swap(&mut manifest.components, &mut updated_component_table);
}

