use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};

use color_eyre::eyre;

use crate::{
    errors::source_map::SourceMap, helpers::HostCrateInfo, manifest::PaxManifest, RunContext,
    RunTarget, ALL_PKGS, PKG_DIR_NAME,
};

use self::{apple::build_apple_chassis_with_cartridge, web::build_web_chassis_with_cartridge};

pub mod apple;
pub mod web;

/// Runs `cargo build` (or `wasm-pack build`) with appropriate env in the directory
/// of the generated chassis project inside the specified .pax dir
/// Returns an output object containing bytestreams of stdout/stderr as well as an exit code
pub fn build_chassis_with_cartridge(
    pax_dir: &PathBuf,
    ctx: &RunContext,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
    source_map: &SourceMap,
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
            build_web_chassis_with_cartridge(ctx, &pax_dir, process_child_ids, source_map)?;
        }
    }
    Ok(())
}

pub fn update_property_prefixes_in_place(
    manifest: &mut PaxManifest,
    host_crate_info: &HostCrateInfo,
) {
    let mut updated_type_table = HashMap::new();
    manifest.type_table.iter_mut().for_each(|t| {
        t.1.type_id_escaped = t.1.type_id_escaped.replace("{PREFIX}", "");
        t.1.type_id =
            t.1.type_id
                .replace("{PREFIX}", &host_crate_info.import_prefix);
        t.1.property_definitions.iter_mut().for_each(|pd| {
            pd.type_id = pd
                .type_id
                .replace("{PREFIX}", &host_crate_info.import_prefix);
        });
        updated_type_table.insert(
            t.0.replace("{PREFIX}", &host_crate_info.import_prefix),
            t.1.clone(),
        );
    });
    std::mem::swap(&mut manifest.type_table, &mut updated_type_table);
}
