use crate::errors::source_map::SourceMap;
use crate::helpers::{
    wait_with_output, ASSETS_DIR_NAME, BUILD_DIR_NAME, DIR_IGNORE_LIST_WEB, ERR_SPAWN, PAX_BADGE,
    PKG_DIR_NAME, PUBLIC_DIR_NAME,
};
use crate::{copy_dir_recursively, errors, pre_exec_hook, RunContext, RunTarget};

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

const IS_DESIGN_TIME_BUILD: bool = cfg!(feature = "designtime");

pub fn build_web_chassis_with_cartridge(
    ctx: &RunContext,
    pax_dir: &PathBuf,
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

    if ctx.is_libdev_mode {
        let mut cmd = Command::new("./build-interface.sh");
        cmd.current_dir(&chassis_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        cmd.output().expect("failed to build web interface");
    }

    let is_release: bool = ctx.is_release;

    let build_mode_name: &str = if is_release { "release" } else { "debug" };

    let interface_path = pax_dir
        .join(PKG_DIR_NAME)
        .join(format!("pax-chassis-{}", target_str_lower))
        .join("interface");

    // First pass cargo build to catch errors in template with source map
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&chassis_path)
        .arg("build")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .arg("--message-format=json")
        .env("PAX_DIR", &pax_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    if is_release {
        cmd.arg("--release");
    }

    if IS_DESIGN_TIME_BUILD {
        cmd.arg("--features").arg("designtime");
    }

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(pre_exec_hook);
    }

    let child = cmd.spawn().expect(ERR_SPAWN);
    let output = wait_with_output(&process_child_ids, child);
    if !output.status.success() {
        let result = errors::process_messages(output, source_map, ctx.verbose);
        if ctx.verbose {
            // Print and continue to wasm-pack to get full error stack trace
            if let Err(e) = result {
                eprintln!("Error encountered: {:?}", e);
            }
        } else {
            result?;
        }
    }

    // wasm-pack build
    let mut cmd = Command::new("wasm-pack");
    cmd.current_dir(&chassis_path)
        .arg("build")
        .arg("--target")
        .arg("web")
        .arg("--out-name")
        .arg("pax-chassis-web")
        .arg("--out-dir")
        .arg(
            chassis_path
                .join("interface")
                .join("public")
                .to_str()
                .unwrap(),
        )
        .env("PAX_DIR", &pax_dir)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    if is_release {
        cmd.arg("--release");
    } else {
        cmd.arg("--dev");
    }
    if IS_DESIGN_TIME_BUILD {
        cmd.arg("--features").arg("designtime");
    }

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(pre_exec_hook);
    }

    let child = cmd.spawn().expect(ERR_SPAWN);

    // Execute wasm-pack build
    let output = wait_with_output(&process_child_ids, child);
    if !output.status.success() {
        return Err(eyre!("Failed to build project with wasm-pack. Aborting."));
    }

    // Copy assets
    let asset_src = pax_dir.join("..").join(ASSETS_DIR_NAME);
    let asset_dest = interface_path.join(PUBLIC_DIR_NAME).join(ASSETS_DIR_NAME);

    // Create target assets directory
    if let Err(e) = fs::create_dir_all(&asset_dest) {
        return Err(eyre!("Error creating directory {:?}: {}", asset_dest, e));
    }

    // Check if the asset_src directory exists before attempting the copy
    if asset_src.exists() {
        // Perform recursive copy from userland `assets/` to built `assets/`
        if let Err(e) = copy_dir_recursively(&asset_src, &asset_dest, &vec![]) {
            return Err(eyre!("Error copying assets: {}", e));
        }
    }

    //Copy fully built project into .pax/build/web, ready for e.g. publishing
    let build_src = interface_path.join(PUBLIC_DIR_NAME);
    let build_dest = pax_dir
        .join(BUILD_DIR_NAME)
        .join(build_mode_name)
        .join(target_str_lower);
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
        println!("{} 🐇 Running Pax Web...", *PAX_BADGE);
        let _ = start_static_http_server(interface_path.join(PUBLIC_DIR_NAME));
    } else {
        println!(
            "{} 🗂️ Done: {} build available at {}",
            *PAX_BADGE,
            build_mode_name,
            build_dest.to_str().unwrap()
        );
    }
    Ok(())
}

fn start_static_http_server(fs_path: PathBuf) -> std::io::Result<()> {
    // Initialize logging

    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::Builder::from_env(env_logger::Env::default())
        .format(|buf, record| writeln!(buf, "{} 🍱 Served {}", *PAX_BADGE, record.args()))
        .init();

    // Create a Runtime
    let runtime = actix_rt::System::new().block_on(async {
        let mut port = 8080;
        let server = loop {
            // Check if the port is available
            if TcpListener::bind(("127.0.0.1", port)).is_ok() {
                // Log the server details
                println!(
                    "{} 🗂️  Serving static files from {}",
                    *PAX_BADGE,
                    &fs_path.to_str().unwrap()
                );
                let address_msg = format!("http://127.0.0.1:{}", port).blue();
                let server_running_at_msg = format!("Server running at {}", address_msg).bold();
                println!("{} 📠 {}", *PAX_BADGE, server_running_at_msg);
                break HttpServer::new(move || {
                    App::new().wrap(Logger::new("| %s | %U")).service(
                        actix_files::Files::new("/*", fs_path.clone()).index_file("index.html"),
                    )
                })
                .bind(("127.0.0.1", port))
                .expect("Error binding to address")
                .workers(2);
            } else {
                port += 1; // Try the next port
            }
        };

        server.run().await
    });

    runtime
}
