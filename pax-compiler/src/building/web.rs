use crate::helpers::{
    wait_with_output, ASSETS_DIR_NAME, BUILD_DIR_NAME, DIR_IGNORE_LIST_WEB, INTERFACE_DIR_NAME,
    PAX_BADGE,
};
use crate::{copy_dir_recursively, RunContext, RunTarget};

use color_eyre::eyre;
use flate2::{write::GzEncoder, Compression};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};

use dotenv::dotenv;
use eyre::eyre;
use pax_manifest::PaxManifest;
#[cfg(unix)]
use std::os::unix::process::CommandExt;

struct BundleStat {
    label: &'static str,
    raw_size: u64,
    gzip_size: u64,
}

fn gzip_size(bytes: &[u8]) -> Result<u64, eyre::Report> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes)?;
    Ok(encoder.finish()?.len() as u64)
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn collect_bundle_stat(
    build_dest: &std::path::Path,
    label: &'static str,
    filename: &'static str,
) -> Result<Option<BundleStat>, eyre::Report> {
    let path = build_dest.join(filename);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path)?;
    Ok(Some(BundleStat {
        label,
        raw_size: bytes.len() as u64,
        gzip_size: gzip_size(&bytes)?,
    }))
}

fn print_web_bundle_stats(build_dest: &std::path::Path, is_release: bool) {
    let stats_result = ["pax-cartridge_bg.wasm", "pax-cartridge.js", "pax-interface-web.js"]
        .into_iter()
        .zip(["wasm", "cartridge js", "interface js"])
        .filter_map(|(filename, label)| collect_bundle_stat(build_dest, label, filename).transpose())
        .collect::<Result<Vec<_>, _>>();

    let stats = match stats_result {
        Ok(stats) => stats,
        Err(err) => {
            eprintln!("{} 📦 Failed to calculate web bundle stats: {}", *PAX_BADGE, err);
            return;
        }
    };

    if stats.is_empty() {
        return;
    }

    println!("{} 📦 Web bundle stats", *PAX_BADGE);
    println!(
        "{}    {}",
        *PAX_BADGE,
        if is_release {
            "RELEASE MODE"
        } else {
            "DEBUG MODE"
        }
    );
    let mut total_raw = 0u64;
    let mut total_gzip = 0u64;
    for stat in &stats {
        total_raw += stat.raw_size;
        total_gzip += stat.gzip_size;
        println!(
            "{}    {:<12} {:>9} emitted  {:>9} gzip",
            *PAX_BADGE,
            stat.label,
            format_bytes(stat.raw_size),
            format_bytes(stat.gzip_size),
        );
    }
    println!(
        "{}    {:<12} {:>9} emitted  {:>9} gzip",
        *PAX_BADGE,
        "total",
        format_bytes(total_raw),
        format_bytes(total_gzip),
    );
}

pub fn build_web_project_with_cartridge(
    ctx: &RunContext,
    pax_dir: &PathBuf,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
    assets_dirs: Vec<String>,
    manifest: PaxManifest, //used by designtime
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

    if is_release {
        cmd.arg("--release");
    } else {
        cmd.arg("--dev");
    }
    if ctx.should_run_designer {
        cmd.arg("--features").arg("designer");
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
    let asset_dest = interface_path.join(ASSETS_DIR_NAME);

    // Create target assets directory
    if let Err(e) = fs::create_dir_all(&asset_dest) {
        return Err(eyre!("Error creating directory {:?}: {}", asset_dest, e));
    }

    // `asset_dirs` gets collected by detecting #[pax]#[main] through compiletime
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

    print_web_bundle_stats(&build_dest, is_release);

    // Start local server if this is a `run` rather than a `build`
    if ctx.should_also_run {
        if ctx.should_run_designer {
            println!("{} 🐇🎨 Running Pax Web with Pax Designer...", *PAX_BADGE);
            dotenv().ok();
            let _ = crate::design_server::start_server(
                build_dest.to_str().unwrap(),
                pax_dir.parent().unwrap().to_str().unwrap(),
                manifest,
            );
        } else {
            println!("{} 🐇 Running Pax Web...", *PAX_BADGE);
            let _ = crate::design_server::static_server::start_server(build_dest);
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
