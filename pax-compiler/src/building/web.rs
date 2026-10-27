use crate::building::web_public::{
    existing_project_public_dir, materialize_public_dir, project_public_dir, validate_public_dir,
};
use crate::dev_session::{
    self, now_ms, project_designtime_manifest_file, project_dev_dir, remove_project_active_session,
    write_project_active_session, DevSession,
};
use crate::helpers::{
    configure_pax_build_env, pax_project_feature_args, wait_with_output, ASSETS_DIR_NAME,
    BUILD_DIR_NAME, DIR_IGNORE_LIST_WEB, INTERFACE_DIR_NAME, PAX_BADGE,
};
use crate::{
    copy_dir_recursively, prepare_cartridge_sources, BuildTimings, RunContext,
    RunLifecycleNotifier, RunTarget,
};

use color_eyre::eyre;
use flate2::{write::GzEncoder, Compression};
use std::fs;
use std::io::Write;
use std::path::Path;
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
    gzip_size: Option<u64>,
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
    include_gzip: bool,
) -> Result<Option<BundleStat>, eyre::Report> {
    let path = build_dest.join(filename);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path)?;
    let gzip_size = if include_gzip {
        Some(gzip_size(&bytes)?)
    } else {
        None
    };
    Ok(Some(BundleStat {
        label,
        raw_size: bytes.len() as u64,
        gzip_size,
    }))
}

fn print_web_bundle_stats(build_dest: &std::path::Path, build_mode_name: &str) {
    let include_gzip = build_mode_name != "debug";
    let stats_result = [
        "pax-cartridge_bg.wasm",
        "pax-cartridge.js",
        "pax-interface-web.js",
    ]
    .into_iter()
    .zip(["wasm", "cartridge js", "interface js"])
    .filter_map(|(filename, label)| {
        collect_bundle_stat(build_dest, label, filename, include_gzip).transpose()
    })
    .collect::<Result<Vec<_>, _>>();

    let stats = match stats_result {
        Ok(stats) => stats,
        Err(err) => {
            eprintln!(
                "{} 📦 Failed to calculate web bundle stats: {}",
                *PAX_BADGE, err
            );
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
        if build_mode_name == "profiling" {
            "PROFILING MODE"
        } else if build_mode_name == "release" {
            "RELEASE MODE"
        } else {
            "DEBUG MODE"
        }
    );
    let mut total_raw = 0u64;
    let mut total_gzip = 0u64;
    for stat in &stats {
        total_raw += stat.raw_size;
        if let Some(gzip_size) = stat.gzip_size {
            total_gzip += gzip_size;
            println!(
                "{}    {:<12} {:>9} emitted  {:>9} gzip",
                *PAX_BADGE,
                stat.label,
                format_bytes(stat.raw_size),
                format_bytes(gzip_size),
            );
        } else {
            println!(
                "{}    {:<12} {:>9} emitted",
                *PAX_BADGE,
                stat.label,
                format_bytes(stat.raw_size),
            );
        }
    }
    if include_gzip {
        println!(
            "{}    {:<12} {:>9} emitted  {:>9} gzip",
            *PAX_BADGE,
            "total",
            format_bytes(total_raw),
            format_bytes(total_gzip),
        );
    } else {
        println!(
            "{}    {:<12} {:>9} emitted",
            *PAX_BADGE,
            "total",
            format_bytes(total_raw),
        );
    }
}

fn web_cargo_features(ctx: &RunContext) -> String {
    let mut features = vec!["web"];
    if ctx.should_run_designtime {
        features.push("designtime");
    }
    pax_project_feature_args(&ctx.project_path, &features).join(",")
}

fn apply_release_size_profile(cmd: &mut Command, preserve_wasm_names: bool) {
    cmd.env("CARGO_PROFILE_RELEASE_OPT_LEVEL", "z")
        .env("CARGO_PROFILE_RELEASE_LTO", "true")
        .env("CARGO_PROFILE_RELEASE_CODEGEN_UNITS", "1")
        .env("CARGO_PROFILE_RELEASE_PANIC", "abort");

    if preserve_wasm_names {
        cmd.env("CARGO_PROFILE_RELEASE_DEBUG", "true")
            .env("CARGO_PROFILE_RELEASE_SPLIT_DEBUGINFO", "off");
    } else if std::env::var_os("PAX_RELEASE_KEEP_SYMBOLS").is_none() {
        cmd.env("CARGO_PROFILE_RELEASE_STRIP", "symbols");
    }
}

fn configure_web_rasterization(cmd: &mut Command) {
    // wasm-pack supplies --target, so these flags affect Wasm, never host build
    // scripts. Cargo merges existing build.rustflags; explicit target flags or
    // RUSTFLAGS retain their normal precedence over this default.
    cmd.arg("--config")
        .arg("build.rustflags=[\"-Ctarget-feature=+simd128\"]");
    // Software rasterization needs vectorization and loop optimization even in
    // development. Keep the rest of the release cartridge optimized for size.
    for profile in ["dev", "release"] {
        for package in ["pax-runtime", "vello_cpu", "vello_common", "fearless_simd"] {
            cmd.arg("--config")
                .arg(format!("profile.{profile}.package.{package}.opt-level=3"));
        }
    }
}

fn wasm_opt_binary_name() -> &'static str {
    if cfg!(windows) {
        "wasm-opt.exe"
    } else {
        "wasm-opt"
    }
}

fn find_executable_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|entry| entry.join(name))
        .find(|candidate| candidate.is_file())
}

fn wasm_pack_cache_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        roots.push(home.join("Library/Caches/.wasm-pack"));
        roots.push(home.join(".cache/.wasm-pack"));
        roots.push(home.join(".wasm-pack"));
    }
    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        roots.push(local_app_data.join(".wasm-pack"));
    }
    roots
}

fn find_cached_wasm_pack_wasm_opt(name: &str) -> Option<PathBuf> {
    for root in wasm_pack_cache_roots() {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !file_name.starts_with("wasm-opt-") {
                continue;
            }
            let candidate = path.join("bin").join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn find_wasm_opt() -> Option<PathBuf> {
    let name = wasm_opt_binary_name();
    std::env::var_os("PAX_WASM_OPT")
        .or_else(|| std::env::var_os("WASM_OPT"))
        .map(PathBuf::from)
        .filter(|candidate| candidate.is_file())
        .or_else(|| find_executable_on_path(name))
        .or_else(|| find_cached_wasm_pack_wasm_opt(name))
}

fn optimize_web_wasm(
    wasm_path: &Path,
    process_child_ids: &Arc<Mutex<Vec<u64>>>,
    preserve_wasm_names: bool,
) -> Result<(), eyre::Report> {
    let Some(wasm_opt) = find_wasm_opt() else {
        println!(
            "{} 🗜️  Skipping wasm-opt transfer-size candidate; install Binaryen or set PAX_WASM_OPT=/path/to/wasm-opt to enable it",
            *PAX_BADGE
        );
        return Ok(());
    };

    let original_bytes = fs::read(wasm_path)?;
    let original_raw_size = original_bytes.len() as u64;
    let original_gzip_size = gzip_size(&original_bytes)?;

    let optimized_path = wasm_path.with_extension("wasm-opt.wasm");
    let mut cmd = Command::new(&wasm_opt);
    cmd.arg(wasm_path)
        .arg("-o")
        .arg(&optimized_path)
        .arg("-Oz")
        .arg("--enable-simd")
        .arg("--enable-bulk-memory")
        .arg("--enable-nontrapping-float-to-int")
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());
    if preserve_wasm_names {
        cmd.arg("-g").arg("--strip-dwarf");
    }

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(crate::pre_exec_hook);
    }

    println!(
        "{} 🗜️  Optimizing wasm with `{}`",
        *PAX_BADGE,
        wasm_opt.display()
    );
    let child = cmd
        .spawn()
        .map_err(|err| eyre!("failed to run wasm-opt at {}: {}", wasm_opt.display(), err))?;
    let output = wait_with_output(process_child_ids, child);
    if !output.status.success() {
        let _ = fs::remove_file(&optimized_path);
        return Err(eyre!("failed to optimize wasm with wasm-opt"));
    }

    let optimized_bytes = fs::read(&optimized_path)?;
    let optimized_raw_size = optimized_bytes.len() as u64;
    let optimized_gzip_size = gzip_size(&optimized_bytes)?;
    let keep_optimized = optimized_gzip_size < original_gzip_size
        || (optimized_gzip_size == original_gzip_size && optimized_raw_size < original_raw_size);

    if keep_optimized {
        fs::copy(&optimized_path, wasm_path)?;
        println!(
            "{} 🗜️  Keeping wasm-opt output: gzip {} -> {}, raw {} -> {}",
            *PAX_BADGE,
            format_bytes(original_gzip_size),
            format_bytes(optimized_gzip_size),
            format_bytes(original_raw_size),
            format_bytes(optimized_raw_size),
        );
    } else {
        println!(
            "{} 🗜️  Keeping pre-wasm-opt wasm for transfer size: gzip {} -> {}, raw {} -> {}",
            *PAX_BADGE,
            format_bytes(original_gzip_size),
            format_bytes(optimized_gzip_size),
            format_bytes(original_raw_size),
            format_bytes(optimized_raw_size),
        );
    }
    fs::remove_file(&optimized_path)?;
    Ok(())
}

pub struct WebLogicReloadBuild {
    pub manifest: PaxManifest,
    pub build_id: String,
    pub extensionless_url: String,
}

fn expose_selected_web_entrypoint(interface_path: &Path, entrypoint: &str) -> eyre::Result<()> {
    // wasm-bindgen retains exports from runnable dependencies too. Select by
    // Cargo package/target identity, never by export order or unified features.
    let alias = format!("\nexport {{ {entrypoint} as init_web }};\n");
    for extension in ["js", "d.ts"] {
        let path = interface_path.join(format!("pax-cartridge.{extension}"));
        let mut contents = fs::read_to_string(&path)?;
        if !contents.ends_with(&alias) {
            contents.push_str(&alias);
            fs::write(path, contents)?;
        }
    }
    Ok(())
}

fn web_build_mode(ctx: &RunContext) -> &'static str {
    if ctx.profile_wasm_size {
        "profiling"
    } else if ctx.is_release {
        "release"
    } else {
        "debug"
    }
}

pub(crate) fn compile_web_cartridge(
    ctx: &RunContext,
    pax_dir: &PathBuf,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
    timings: &mut BuildTimings,
) -> Result<(PathBuf, &'static str), eyre::Report> {
    let layout = crate::project_layout::ProjectLayout::resolve(&ctx.project_path)?;
    let mut cartridge_ctx = ctx.clone();
    cartridge_ctx.project_path = layout.cartridge_dir.clone();
    let is_release: bool = ctx.is_release;
    let is_profiling = ctx.profile_wasm_size;

    crate::validate_release_cargo_feature_boundary(&cartridge_ctx, &["wasm32-unknown-unknown"])?;

    let build_mode_name = web_build_mode(ctx);

    let interface_path = pax_dir.join(INTERFACE_DIR_NAME).join("web");

    // wasm-pack build
    let mut cmd = Command::new("wasm-pack");
    cmd.current_dir(&layout.cartridge_dir)
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
        .env("PAX_DIR", &pax_dir)
        .env(
            "PAX_MANIFEST_OUTPUT",
            pax_dir.join("manifest-wasm32-unknown-unknown.json"),
        )
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());
    layout.configure_cartridge(&mut cmd);

    if is_profiling {
        cmd.arg("--profiling");
        cmd.arg("--no-opt");
        apply_release_size_profile(&mut cmd, true);
    } else if is_release {
        cmd.arg("--release");
        cmd.arg("--no-opt");
        apply_release_size_profile(&mut cmd, false);
    } else {
        cmd.arg("--dev");
    }
    // wasm-pack forwards trailing Cargo switches verbatim. Put its own profile
    // flags first so --dev/--profiling never reach `cargo build` as Cargo flags.
    cmd.arg(format!("--features={}", web_cargo_features(&cartridge_ctx)));
    configure_web_rasterization(&mut cmd);
    configure_pax_build_env(&mut cmd, "web", ctx.should_run_designtime);
    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(crate::pre_exec_hook);
    }

    // Execute wasm-pack build
    let output = timings.record("cargo/wasm-pack", || {
        let child = cmd.spawn().expect(
            r#"failed to run wasm-pack, is it:
- installed?
- present in PATH?"#,
        );
        wait_with_output(&process_child_ids, child)
    });
    if !output.status.success() {
        return Err(eyre!("failed to compile project with wasm-pack"));
    }
    expose_selected_web_entrypoint(&interface_path, &layout.web_entrypoint)?;

    if is_release {
        timings.record("wasm-opt", || {
            optimize_web_wasm(
                &interface_path.join("pax-cartridge_bg.wasm"),
                &process_child_ids,
                is_profiling,
            )
        })?;
    }

    Ok((interface_path, build_mode_name))
}

fn copy_web_assets(
    interface_path: &Path,
    assets_dirs: Vec<String>,
    timings: &mut BuildTimings,
) -> eyre::Result<()> {
    timings.record("copy assets", || {
        // Copy assets
        let asset_dest = interface_path.join(ASSETS_DIR_NAME);

        // Create target assets directory
        if let Err(e) = fs::create_dir_all(&asset_dest) {
            return Err(eyre!("Error creating directory {:?}: {}", asset_dest, e));
        }

        // Static discovery includes assets of the selected UI and its component packages.
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
        Ok(())
    })
}

fn materialize_web_build_dir(
    pax_dir: &PathBuf,
    interface_path: &Path,
    build_mode_name: &str,
    target_str_lower: &str,
    timings: &mut BuildTimings,
) -> Result<PathBuf, eyre::Report> {
    let build_src = interface_path.to_path_buf();
    let build_dest = pax_dir
        .join(BUILD_DIR_NAME)
        .join(build_mode_name)
        .join(target_str_lower);

    timings.record("copy build output", || {
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
    });

    timings.record("bundle stats", || {
        print_web_bundle_stats(&build_dest, build_mode_name)
    });
    Ok(build_dest)
}

fn copy_web_reload_artifacts(interface_path: &Path, staged_dir: &Path) -> Result<(), eyre::Report> {
    const REQUIRED_FILES: [&str; 2] = ["pax-cartridge.js", "pax-cartridge_bg.wasm"];
    const OPTIONAL_FILES: [&str; 3] = [
        "pax-cartridge.d.ts",
        "pax-cartridge_bg.wasm.d.ts",
        "package.json",
    ];

    let parent = staged_dir
        .parent()
        .ok_or_else(|| eyre!("web reload staging path has no parent"))?;
    let staged_name = staged_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| eyre!("web reload staging path has no file name"))?;
    let pending_dir = parent.join(format!(".{staged_name}.pending"));
    let _ = fs::remove_dir_all(&pending_dir);
    fs::create_dir_all(&pending_dir)?;

    let copy_result = (|| -> Result<(), eyre::Report> {
        for file_name in REQUIRED_FILES {
            let src = interface_path.join(file_name);
            let metadata = fs::metadata(&src)
                .map_err(|err| eyre!("missing required web reload artifact {file_name}: {err}"))?;
            if !metadata.is_file() || metadata.len() == 0 {
                return Err(eyre!(
                    "required web reload artifact {file_name} is empty or not a file"
                ));
            }
            fs::copy(&src, pending_dir.join(file_name))?;
        }
        for file_name in OPTIONAL_FILES {
            let src = interface_path.join(file_name);
            if src.is_file() {
                fs::copy(&src, pending_dir.join(file_name))?;
            }
        }
        let snippets_src = interface_path.join("snippets");
        if !snippets_src.is_dir() {
            return Err(eyre!(
                "missing required web reload artifact directory snippets"
            ));
        }
        copy_dir_recursively(&snippets_src, &pending_dir.join("snippets"), &[])
            .map_err(|err| eyre!("failed to copy web reload snippets: {err}"))?;
        Ok(())
    })();

    if let Err(err) = copy_result {
        let _ = fs::remove_dir_all(&pending_dir);
        return Err(err);
    }
    if staged_dir.exists() {
        let _ = fs::remove_dir_all(&pending_dir);
        return Err(eyre!(
            "web reload staging destination already exists: {}",
            staged_dir.display()
        ));
    }
    fs::rename(&pending_dir, staged_dir)?;
    Ok(())
}

pub fn rebuild_staged_web_cartridge(
    project_root: &PathBuf,
    serve_dir: &Path,
) -> Result<WebLogicReloadBuild, eyre::Report> {
    let process_child_ids = Arc::new(Mutex::new(vec![]));
    let ctx = RunContext {
        port: None,
        target: RunTarget::Web,
        project_path: project_root.clone(),
        verbose: false,
        should_also_run: false,
        is_libdev_mode: false,
        process_child_ids: process_child_ids.clone(),
        should_run_designtime: true,
        hot_reload: None,
        is_release: false,
        profile_wasm_size: false,
        ios_device: None,
        ios_development_team: None,
        lifecycle_observer: None,
    };

    let prepared = prepare_cartridge_sources(&ctx)?;
    let mut timings = BuildTimings::start();
    let interface_path = prepared.pax_dir.join(INTERFACE_DIR_NAME).join("web");
    copy_web_assets(&interface_path, prepared.assets_dirs, &mut timings)?;

    let build_id = now_ms().to_string();
    let staged_dir = serve_dir.join("__reloads__").join(&build_id);
    // Each reload gets a unique served path so browser module caches cannot
    // alias an older cartridge image. PAX-889 tracks bounded cleanup of prior
    // staged bundles for long-running designtime sessions.
    copy_web_reload_artifacts(&interface_path, &staged_dir)?;

    Ok(WebLogicReloadBuild {
        manifest: prepared.userland_manifest,
        build_id: build_id.clone(),
        extensionless_url: format!("/__reloads__/{build_id}/pax-cartridge"),
    })
}

pub fn build_web_project_with_cartridge(
    ctx: &RunContext,
    pax_dir: &PathBuf,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
    assets_dirs: Vec<String>,
    manifest: PaxManifest, //used by designtime
    project_root: &Path,
    timings: &mut BuildTimings,
) -> Result<PathBuf, eyre::Report> {
    compile_web_cartridge(ctx, pax_dir, process_child_ids, timings)?;
    finish_web_project_with_cartridge(ctx, pax_dir, assets_dirs, manifest, project_root, timings)
}

pub(crate) fn finish_web_project_with_cartridge(
    ctx: &RunContext,
    pax_dir: &PathBuf,
    assets_dirs: Vec<String>,
    manifest: PaxManifest,
    project_root: &Path,
    timings: &mut BuildTimings,
) -> Result<PathBuf, eyre::Report> {
    let lifecycle_notifier = RunLifecycleNotifier::new(ctx);
    let target: &RunTarget = &ctx.target;
    let target_str: &str = target.into();
    let target_str_lower = &target_str.to_lowercase();

    let interface_path = pax_dir.join(INTERFACE_DIR_NAME).join("web");
    let build_mode_name = web_build_mode(ctx);
    copy_web_assets(&interface_path, assets_dirs, timings)?;
    let public_dir = project_public_dir(project_root);
    timings.record("public files", || {
        if ctx.should_also_run {
            validate_public_dir(&public_dir, &interface_path)
        } else {
            materialize_public_dir(&public_dir, &interface_path)
        }
    })?;
    let build_dest = materialize_web_build_dir(
        pax_dir,
        &interface_path,
        build_mode_name,
        target_str_lower,
        timings,
    )?;

    timings.print_summary();

    // Start local server if this is a `run` rather than a `build`
    if ctx.should_also_run {
        if ctx.should_run_designtime {
            println!("{} 🐇 Running Pax Web with designtime...", *PAX_BADGE);
            dotenv().ok();
            let dev_session = prepare_web_dev_session(project_root, pax_dir)?;
            write_project_active_session(pax_dir, &dev_session)?;
            let server_result = crate::design_server::start_server_with_ready_callback(
                build_dest.to_str().unwrap(),
                project_root.to_str().unwrap(),
                manifest,
                ctx.port,
                None,
                true,
                Some(dev_session.clone()),
                Some(crate::design_server::LogicReloadConfig::Web(
                    crate::design_server::WebLogicReloadConfig {
                        serve_dir: build_dest.clone(),
                    },
                )),
                ctx.hot_reload.unwrap_or_default(),
                None,
                Some(project_designtime_manifest_file(pax_dir)),
                lifecycle_notifier.clone().into_callback(),
            );
            let cleanup_result = cleanup_web_dev_session(pax_dir, &dev_session);
            server_result?;
            cleanup_result?;
        } else {
            println!("{} 🐇 Running Pax Web...", *PAX_BADGE);
            crate::design_server::static_server::start_server_with_ready_callback(
                build_dest.clone(),
                existing_project_public_dir(project_root),
                ctx.port,
                lifecycle_notifier.into_callback(),
            )?;
        }
    } else {
        println!(
            "{} 🗂️ Done: {} build available at {}",
            *PAX_BADGE,
            build_mode_name,
            build_dest.to_str().unwrap()
        );
    }
    Ok(build_dest)
}

fn prepare_web_dev_session(
    project_root: &Path,
    pax_dir: &PathBuf,
) -> Result<DevSession, eyre::Report> {
    let started_at_ms = now_ms();
    let session_id = format!("web-{started_at_ms}-{}", std::process::id());
    let session_dir = project_dev_dir(pax_dir).join("sessions").join(&session_id);
    fs::create_dir_all(session_dir.join("requests"))?;
    fs::create_dir_all(session_dir.join("responses"))?;
    fs::create_dir_all(session_dir.join("captures"))?;

    Ok(DevSession {
        session_id,
        platform: "web".to_string(),
        designtime: true,
        project_root: Some(
            fs::canonicalize(project_root).unwrap_or_else(|_| project_root.to_path_buf()),
        ),
        session_dir: Some(session_dir),
        app_pid: None,
        design_server_addr: None,
        control_kind: "filesystem".to_string(),
        location: None,
        started_at_ms,
        last_seen_ms: started_at_ms,
    })
}

fn cleanup_web_dev_session(
    pax_dir: &PathBuf,
    dev_session: &DevSession,
) -> Result<(), eyre::Report> {
    remove_project_active_session(pax_dir, &dev_session.session_id)?;
    dev_session::remove_registered_session(&dev_session.session_id)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::copy_web_reload_artifacts;
    use std::fs;

    #[test]
    fn selects_one_web_entrypoint_without_removing_dependency_exports() {
        let interface = tempfile::tempdir().unwrap();
        for extension in ["js", "d.ts"] {
            fs::write(
                interface.path().join(format!("pax-cartridge.{extension}")),
                "export function dependency() {}\nexport function selected() {}\n",
            )
            .unwrap();
        }
        for _ in 0..2 {
            super::expose_selected_web_entrypoint(interface.path(), "selected").unwrap();
        }
        for extension in ["js", "d.ts"] {
            let contents =
                fs::read_to_string(interface.path().join(format!("pax-cartridge.{extension}")))
                    .unwrap();
            assert!(contents.contains("export function dependency()"));
            assert_eq!(contents.matches("as init_web").count(), 1);
            assert!(contents.ends_with("export { selected as init_web };\n"));
        }
    }
    use tempfile::tempdir;

    #[test]
    fn web_reload_artifacts_publish_only_after_required_files_exist() {
        let dir = tempdir().unwrap();
        let interface_dir = dir.path().join("interface");
        let staged_dir = dir.path().join("served/__reloads__/build-1");
        fs::create_dir_all(&interface_dir).unwrap();
        fs::write(interface_dir.join("pax-cartridge.js"), b"javascript").unwrap();
        fs::write(interface_dir.join("pax-cartridge_bg.wasm"), b"wasm").unwrap();
        fs::create_dir_all(interface_dir.join("snippets/chassis")).unwrap();
        fs::write(
            interface_dir.join("snippets/chassis/inline0.js"),
            b"snippet",
        )
        .unwrap();

        copy_web_reload_artifacts(&interface_dir, &staged_dir).unwrap();

        assert_eq!(
            fs::read(staged_dir.join("pax-cartridge.js")).unwrap(),
            b"javascript"
        );
        assert_eq!(
            fs::read(staged_dir.join("pax-cartridge_bg.wasm")).unwrap(),
            b"wasm"
        );
        assert_eq!(
            fs::read(staged_dir.join("snippets/chassis/inline0.js")).unwrap(),
            b"snippet"
        );
        assert!(!staged_dir
            .parent()
            .unwrap()
            .join(".build-1.pending")
            .exists());
    }

    #[test]
    fn missing_required_web_reload_artifact_is_never_published() {
        let dir = tempdir().unwrap();
        let interface_dir = dir.path().join("interface");
        let staged_dir = dir.path().join("served/__reloads__/build-1");
        fs::create_dir_all(&interface_dir).unwrap();
        fs::write(interface_dir.join("pax-cartridge.js"), b"javascript").unwrap();

        let err = copy_web_reload_artifacts(&interface_dir, &staged_dir).unwrap_err();

        assert!(err
            .to_string()
            .contains("missing required web reload artifact pax-cartridge_bg.wasm"));
        assert!(!staged_dir.exists());
        assert!(!staged_dir
            .parent()
            .unwrap()
            .join(".build-1.pending")
            .exists());
    }

    #[test]
    fn missing_web_reload_snippets_are_never_published() {
        let dir = tempdir().unwrap();
        let interface_dir = dir.path().join("interface");
        let staged_dir = dir.path().join("served/__reloads__/build-1");
        fs::create_dir_all(&interface_dir).unwrap();
        fs::write(interface_dir.join("pax-cartridge.js"), b"javascript").unwrap();
        fs::write(interface_dir.join("pax-cartridge_bg.wasm"), b"wasm").unwrap();

        let err = copy_web_reload_artifacts(&interface_dir, &staged_dir).unwrap_err();

        assert!(err
            .to_string()
            .contains("missing required web reload artifact directory snippets"));
        assert!(!staged_dir.exists());
        assert!(!staged_dir
            .parent()
            .unwrap()
            .join(".build-1.pending")
            .exists());
    }
}
