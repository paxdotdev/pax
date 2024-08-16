use crate::helpers::{
    wait_with_output, ASSETS_DIR_NAME, BUILD_DIR_NAME, DIR_IGNORE_LIST_WEB, INTERFACE_DIR_NAME,
    PAX_BADGE,
};
use crate::{copy_dir_recursively, RunContext, RunTarget};

use color_eyre::eyre;
use colored::Colorize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};

use actix_web::middleware::Logger;
use actix_web::{App, HttpServer};
use env_logger;
use eyre::eyre;
use std::io::Write;
use std::net::TcpListener;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use dotenv::dotenv;

pub fn build_web_project_with_cartridge(
    ctx: &RunContext,
    pax_dir: &PathBuf,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
    assets_dirs: Vec<String>,
) -> Result<PathBuf, eyre::Report> {
    let target: &RunTarget = &ctx.target;
    let target_str: &str = target.into();
    let target_str_lower = &target_str.to_lowercase();

    let is_release: bool = ctx.is_release;

    let build_mode_name: &str = if is_release { "release" } else { "debug" };

    let interface_path = pax_dir.join(INTERFACE_DIR_NAME).join("web");

    // wasm-pack build
    let mut cmd = Command::new("wasm-pack");
    cmd.current_dir(&ctx.project_path)
        .arg("build")
        .arg("--target")
        .arg("web")
        .arg("--out-name")
        .arg("pax-cartridge")
        .arg("--out-dir")
        .arg(
            pax_dir
                .join(INTERFACE_DIR_NAME)
                .join("web")
                .to_str()
                .unwrap(),
        )
        .arg("--features=web")
        .env("PAX_DIR", &pax_dir)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    if is_release || cfg!(not(debug_assertions)) {
        cmd.arg("--release");
    } else {
        cmd.arg("--dev");
    }
    if ctx.should_run_designer {
        cmd.arg("--features").arg("designtime");
    }

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(crate::pre_exec_hook);
    }

    let child = cmd.spawn().expect(
        r#"failed to run wasm-pack, is it:
- installed?
- present in PATH?"#,
    );

    // Execute wasm-pack build
    let output = wait_with_output(&process_child_ids, child);
    if !output.status.success() {
        return Err(eyre!("failed to compile project with wasm-pack"));
    }

    // Copy assets
    // let asset_src = pax_dir.join("..").join(ASSETS_DIR_NAME);
    let asset_dest = interface_path.join(ASSETS_DIR_NAME);

    // Create target assets directory
    if let Err(e) = fs::create_dir_all(&asset_dest) {
        return Err(eyre!("Error creating directory {:?}: {}", asset_dest, e));
    }

    for asset_src in assets_dirs {
        let asset_src = PathBuf::from(asset_src);
        // Check if the asset_src directory exists before attempting the copy
        if asset_src.exists() {
            // Perform recursive copy from userland `assets/` to built `assets/`
            if let Err(e) = copy_dir_recursively(&asset_src, &asset_dest, &vec![]) {
                return Err(eyre!("Error copying assets: {}", e));
            }
        }
    }

    //Copy fully built project into .pax/build/web, ready for e.g. publishing
    let build_src = interface_path.clone();
    let build_dest = pax_dir
        .join(BUILD_DIR_NAME)
        .join(build_mode_name)
        .join(target_str_lower);

    // Clean build dir
    let _ = fs::remove_dir_all(&build_dest);

    // Copy files to build dir
    let res = copy_dir_recursively(&build_src, &build_dest, &DIR_IGNORE_LIST_WEB);
    if let Err(e) = res {
        eprintln!(
            "Failed to copy built files from {} to {}.  {:?}",
            &build_src.to_str().unwrap(),
            &build_dest.to_str().unwrap(),
            e
        );
    }

    // Start local server if this is a `run` rather than a `build`
    if ctx.should_also_run {
        if ctx.should_run_designer {
            println!("{} 🐇🎨 Running with Pax Designer...", *PAX_BADGE);
            dotenv().ok();
            let _ = crate::design_server::start_server(build_dest.to_str().unwrap(),true, ctx.is_libdev_mode);
        } else {
            println!("{} 🐇 Running Pax Web...", *PAX_BADGE);
            let _ = crate::design_server::start_server(build_dest.to_str().unwrap(),false, ctx.is_libdev_mode);
        }
    } else {
        println!(
            "{} 🗂️ Done: {} build available at {}",
            *PAX_BADGE,
            build_mode_name,
            build_dest.to_str().unwrap()
        );
    }
    Ok(build_src)
}

