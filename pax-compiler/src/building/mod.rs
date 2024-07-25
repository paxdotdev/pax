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
    helpers::{copy_dir_recursively, ALL_PKGS, DIR_IGNORE_LIST_MACOS, INTERFACE_DIR_NAME},
    RunContext, RunTarget,
};

use self::{apple::build_apple_project_with_cartridge, web::build_web_project_with_cartridge};

pub mod apple;
pub mod web;

#[cfg(feature = "designtime")]
mod design;

/// Runs `cargo build` (or `wasm-pack build`) with appropriate env in the directory
/// of the generated chassis project inside the specified .pax dir
/// Returns an output object containing bytestreams of stdout/stderr as well as an exit code
pub fn build_project_with_cartridge(
    pax_dir: &PathBuf,
    ctx: &RunContext,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
) -> Result<Option<PathBuf>, eyre::Report> {
    let target: &RunTarget = &ctx.target;
    let pax_dir = PathBuf::from(pax_dir.to_str().unwrap());

    //string together a shell call to build the userland project, with cartridge injected via macro
    match target {
        RunTarget::macOS | RunTarget::iOS => {
            build_apple_project_with_cartridge(ctx, &pax_dir, process_child_ids)?;
            Ok(None)
        }
        RunTarget::Web => {
            let fs = build_web_project_with_cartridge(ctx, &pax_dir, process_child_ids)?;
            Ok(Some(fs))
        }
    }
}



