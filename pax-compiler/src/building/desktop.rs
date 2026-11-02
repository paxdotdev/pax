//! Native Windows/Linux launchers keep the application library behind its C ABI.
use crate::{helpers, project_layout, BuildTimings, RunContext, RunTarget};
use cargo_metadata::MetadataCommand;
use color_eyre::eyre::{eyre, Result};
use pax_manifest::PaxManifest;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

#[path = "desktop_artifacts.rs"]
mod artifacts;

pub(crate) fn validate_host_target(target: &RunTarget) -> Result<()> {
    if matches!(target, RunTarget::Windows) && !cfg!(target_os = "windows") {
        return Err(eyre!(
            "Windows native builds require a Windows host with MSVC and the Windows SDK"
        ));
    }
    if matches!(target, RunTarget::Linux) && !cfg!(target_os = "linux") {
        return Err(eyre!(
            "Linux native builds require a Linux host with GTK 4.14+, EGL and Vulkan"
        ));
    }
    Ok(())
}

pub(crate) fn target_triple(target: &RunTarget) -> Result<&'static str> {
    validate_host_target(target)?;
    match (target, std::env::consts::ARCH) {
        (RunTarget::Windows, "x86_64") => Ok("x86_64-pc-windows-msvc"),
        (RunTarget::Windows, "aarch64") => Ok("aarch64-pc-windows-msvc"),
        (RunTarget::Linux, "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        (RunTarget::Linux, "aarch64") => Ok("aarch64-unknown-linux-gnu"),
        _ => Err(eyre!("Unsupported native desktop host architecture")),
    }
}

fn cartridge_name(library: &str, windows: bool) -> String {
    let library = library.replace('-', "_");
    if windows {
        format!("{library}.dll")
    } else {
        format!("lib{library}.so")
    }
}

fn sdk_directory(ctx: &RunContext, pax_dir: &Path) -> Result<PathBuf> {
    if let Some(directory) = std::env::var_os("PAX_HOST_SDK_DIR") {
        let sdk = PathBuf::from(directory).canonicalize()?;
        let library = if ctx.target == RunTarget::Windows {
            "PaxHost.dll"
        } else {
            "libpax_host.so"
        };
        if !sdk.join(library).is_file() {
            return Err(eyre!(
                "PAX_HOST_SDK_DIR must contain the built native host library {}",
                library
            ));
        }
        return Ok(sdk);
    }
    Ok(pax_dir
        .join("native-sdk")
        .join(if ctx.target == RunTarget::Windows {
            "windows"
        } else {
            "linux"
        })
        .join(if ctx.is_release { "release" } else { "debug" }))
}

fn validate_package_destination(sdk: &Path, destination: &Path) -> Result<()> {
    let sdk = sdk.canonicalize()?;
    // Resolve the existing ancestor even on a first build, when the output
    // does not exist yet. Its parent may itself be reached through a symlink.
    let mut ancestor = destination;
    let mut suffix = Vec::new();
    let mut destination = loop {
        match ancestor.canonicalize() {
            Ok(path) => break path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                suffix.push(
                    ancestor
                        .file_name()
                        .ok_or_else(|| eyre!("Invalid package output directory"))?
                        .to_os_string(),
                );
                ancestor = ancestor
                    .parent()
                    .ok_or_else(|| eyre!("Package output needs a parent directory"))?;
            }
            Err(error) => return Err(error.into()),
        }
    };
    for component in suffix.into_iter().rev() {
        destination.push(component);
    }
    if sdk.starts_with(&destination) || destination.starts_with(&sdk) {
        return Err(eyre!(
            "Native SDK and package output directories must not overlap: SDK {}, output {}",
            sdk.display(),
            destination.display()
        ));
    }
    Ok(())
}

fn copy_sdk_runtime(source: &Path, destination: &Path, windows_release: bool) -> Result<()> {
    if source.is_dir() {
        fs::create_dir_all(destination)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_sdk_runtime(
                &entry.path(),
                &destination.join(entry.file_name()),
                windows_release,
            )?;
        }
    } else {
        // Keep symbols and linker inputs in the SDK for debugging/relinking;
        // runtime DLLs, PRI files and other resources still travel together.
        let build_artifact = source
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| {
                ["pdb", "ilk", "exp", "lib", "obj"]
                    .iter()
                    .any(|excluded| extension.eq_ignore_ascii_case(excluded))
            });
        if !windows_release || !build_artifact {
            fs::copy(source, destination)?;
        }
    }
    Ok(())
}

pub(super) fn checked(command: &mut Command, ctx: &RunContext) -> Result<()> {
    command
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        command.pre_exec(crate::pre_exec_hook);
    }
    let output = helpers::wait_with_output(&ctx.process_child_ids, command.spawn()?);
    if !output.status.success() {
        return Err(eyre!("Native desktop command failed: {}", output.status));
    }
    Ok(())
}

pub(crate) fn build_sdk(
    ctx: &RunContext,
    pax_dir: &Path,
    timings: &mut BuildTimings,
) -> Result<PathBuf> {
    validate_host_target(&ctx.target)?;
    let windows = ctx.target == RunTarget::Windows;
    let platform = if windows { "windows" } else { "linux" };
    let configuration = if ctx.is_release { "Release" } else { "Debug" };
    let sdk = sdk_directory(ctx, pax_dir)?;
    if std::env::var_os("PAX_HOST_SDK_DIR").is_some() {
        // An explicit SDK override is already built. Use that same directory
        // for cartridge compilation, launcher compilation, and packaging.
        return Ok(sdk);
    }
    fs::create_dir_all(&sdk)?;
    let interface = pax_dir.join(helpers::INTERFACE_DIR_NAME).join(platform);
    timings.record("native host SDK", || -> Result<()> {
        let mut command = if windows {
            let mut command = Command::new("powershell.exe");
            command
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                ])
                .arg(interface.join("build.ps1"))
                .arg("-Architecture")
                .arg(if std::env::consts::ARCH == "aarch64" {
                    "ARM64"
                } else {
                    "x64"
                })
                .args(["-Configuration", configuration, "-OutputDirectory"])
                .arg(&sdk);
            command
        } else {
            let mut command = Command::new("bash");
            command
                .arg(interface.join("build.sh"))
                .args(["--configuration", configuration, "--output-directory"])
                .arg(&sdk);
            command
        };
        checked(&mut command, ctx)
    })?;
    Ok(sdk)
}

pub(crate) fn build(
    ctx: &RunContext,
    pax_dir: &Path,
    assets: Vec<String>,
    manifest: PaxManifest,
    timings: &mut BuildTimings,
) -> Result<PathBuf> {
    let triple = target_triple(&ctx.target)?;
    let windows = ctx.target == RunTarget::Windows;
    let platform = if windows { "windows" } else { "linux" };
    let mode = if ctx.is_release { "release" } else { "debug" };
    let layout = project_layout::ProjectLayout::resolve(&ctx.project_path)?;
    let sdk = sdk_directory(ctx, pax_dir)?;
    let metadata = MetadataCommand::new()
        .manifest_path(ctx.project_path.join("Cargo.toml"))
        .no_deps()
        .exec()?;
    let host_manifest = ctx.project_path.join("Cargo.toml").canonicalize()?;
    let package = metadata
        .packages
        .iter()
        .find(|package| {
            package
                .manifest_path
                .as_std_path()
                .canonicalize()
                .ok()
                .as_deref()
                == Some(host_manifest.as_path())
        })
        .ok_or_else(|| eyre!("Select a Pax application package"))?;
    let selected = crate::helpers::select_native_launcher(package)?;
    let mut requested = vec![platform];
    if ctx.should_run_designtime {
        requested.push("designtime");
    }
    let same_package = layout.cartridge_dir == ctx.project_path.canonicalize()?;
    let mut features = if same_package {
        helpers::pax_project_feature_args(&ctx.project_path, &requested)
    } else {
        let mut features =
            helpers::retained_default_features(&ctx.project_path, ctx.should_run_designtime);
        features.extend(
            requested
                .iter()
                .filter(|feature| package.features.contains_key(**feature))
                .map(|feature| (*feature).to_owned()),
        );
        features
    };
    features.push("standalone".into());
    features.sort();
    features.dedup();
    crate::validate_release_cargo_features(ctx, &ctx.project_path, &features, &[triple])?;
    let destination = pax_dir
        .join(helpers::BUILD_DIR_NAME)
        .join(mode)
        .join(platform);
    let symbols_destination = destination.with_file_name("windows-symbols");
    let cartridge = cartridge_name(&layout.library_name, windows);
    let cartridge_source = layout.target_dir.join(triple).join(mode).join(&cartridge);
    let frozen = if windows && same_package {
        validate_package_destination(&sdk, &destination)?;
        validate_package_destination(&sdk, &symbols_destination)?;
        let temporary = tempfile::Builder::new()
            .prefix("desktop-cartridge-")
            .tempdir_in(pax_dir)?;
        let pair = artifacts::freeze_cartridge(&cartridge_source, temporary.path())?;
        Some((temporary, pair))
    } else {
        None
    };
    timings.record("cargo/native launcher", || -> Result<()> {
        let mut command = Command::new("cargo");
        command
            .current_dir(&ctx.project_path)
            .args([
                "build",
                "--bin",
                &selected.name,
                "--target",
                triple,
                "--features",
            ])
            .arg(features.join(","))
            .env("PAX_DIR", pax_dir)
            .env("PAX_BUILD_CARTRIDGE", "false")
            .env("PAX_HOST_SDK_DIR", &sdk)
            .env("PAX_CARTRIDGE_MANIFEST_DIR", &layout.cartridge_dir);
        helpers::configure_pax_build_env(&mut command, platform, ctx.should_run_designtime);
        if ctx.is_release {
            command.arg("--release");
        }
        checked(&mut command, ctx)
    })?;
    if let Some((temporary, pair)) = &frozen {
        let launcher_symbols = metadata
            .target_directory
            .join(triple)
            .join(mode)
            .join(artifacts::launcher_pdb_name(&selected.name));
        artifacts::stage_symbols(
            &temporary.path().join("symbols"),
            pair.symbols.as_deref(),
            launcher_symbols.as_std_path(),
        )?;
    }
    validate_package_destination(&sdk, &destination)?;
    // This directory is CLI-owned output; replace stale SDK resources as a unit.
    if destination.exists() {
        fs::remove_dir_all(&destination)?;
    }
    fs::create_dir_all(&destination)?;
    copy_sdk_runtime(&sdk, &destination, windows && ctx.is_release)?;
    let executable_name = format!("{}{}", selected.name, if windows { ".exe" } else { "" });
    let executable = destination.join(&executable_name);
    fs::copy(
        metadata
            .target_directory
            .join(triple)
            .join(mode)
            .join(&executable_name),
        &executable,
    )?;
    #[cfg(target_os = "windows")]
    if windows {
        super::windows_manifest::embed(
            ctx,
            pax_dir,
            &executable,
            &selected.name,
            [
                package.version.major,
                package.version.minor,
                package.version.patch,
            ],
        )?;
    }
    fs::copy(
        frozen
            .as_ref()
            .map_or(cartridge_source.as_path(), |(_, pair)| pair.image.as_path()),
        destination.join(&cartridge),
    )?;
    let mut roots = Vec::new();
    for (index, source) in assets.iter().enumerate() {
        let relative = format!("assets/{index}");
        helpers::copy_dir_recursively(Path::new(source), &destination.join(&relative), &[])
            .map_err(|error| eyre!(error.to_string()))?;
        roots.push(relative);
    }
    fs::write(
        destination.join("pax-app.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"cartridge":cartridge,"assets":roots,"nativeExports":layout.native_exports}),
        )?,
    )?;
    if let Some((temporary, _)) = &frozen {
        artifacts::publish_symbols(&temporary.path().join("symbols"), &symbols_destination)?;
    }
    drop(frozen);
    if ctx.should_also_run {
        run(ctx, pax_dir, &executable, &manifest)?;
    }
    Ok(destination)
}

fn run(ctx: &RunContext, pax_dir: &Path, executable: &Path, manifest: &PaxManifest) -> Result<()> {
    let mut server = None;
    let mut _readiness = None;
    let result = (|| -> Result<()> {
        let mut command = Command::new(executable);
        command.current_dir(executable.parent().unwrap());
        if ctx.should_run_designtime {
            let ready = pax_dir.join("desktop-design-server-url.txt");
            if ready.exists() {
                fs::remove_file(&ready)?;
            }
            server = Some(super::apple::spawn_designtime_server_process(
                &pax_dir.to_path_buf(),
                &ctx.project_path,
                manifest,
                None,
                ctx.hot_reload.unwrap_or_default(),
                &ready,
                ctx.process_child_ids.clone(),
            )?);
            let address = super::apple::wait_for_designtime_ready(&ready, Duration::from_secs(10))?;
            let marker = crate::design_server::app_ready_marker_path(&ready);
            let notifier = crate::RunLifecycleNotifier::new(ctx);
            _readiness =
                super::apple::watch_app_ready_marker(&marker, Duration::from_secs(30), move || {
                    notifier.notify()
                });
            command.env("PAX_DESIGN_SERVER_ADDR", address);
        } else {
            command.env_remove("PAX_DESIGN_SERVER_ADDR");
        }
        checked(&mut command, ctx)
    })();
    if let Some(mut server) = server {
        super::apple::stop_designtime_server_process(&mut server, &ctx.process_child_ids);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cartridges_use_platform_names_without_changing_library_identity() {
        assert_eq!(cartridge_name("my-ui", true), "my_ui.dll");
        assert_eq!(cartridge_name("my-ui", false), "libmy_ui.so");
        assert_eq!(
            artifacts::launcher_pdb_name("other-launcher"),
            "other_launcher.pdb"
        );
    }
    #[test]
    fn native_builds_reject_other_host_operating_systems() {
        assert_eq!(
            validate_host_target(&RunTarget::Windows).is_ok(),
            cfg!(target_os = "windows")
        );
        assert_eq!(
            validate_host_target(&RunTarget::Linux).is_ok(),
            cfg!(target_os = "linux")
        );
        assert!(validate_host_target(&RunTarget::Web).is_ok());
    }
    #[test]
    fn packaging_rejects_sdk_overlap_before_replacing_output() {
        let root = tempfile::tempdir().unwrap();
        let sdk = root.path().join("sdk");
        fs::create_dir_all(&sdk).unwrap();
        let input = sdk.join("host-library");
        fs::write(&input, b"preserve SDK input").unwrap();
        assert!(validate_package_destination(&sdk, &sdk).is_err());
        assert!(validate_package_destination(&sdk, root.path()).is_err());
        assert!(validate_package_destination(&sdk, &sdk.join("package")).is_err());
        assert!(validate_package_destination(&sdk, &root.path().join("package")).is_ok());
        assert_eq!(fs::read(input).unwrap(), b"preserve SDK input");
    }
    #[test]
    fn windows_release_keeps_runtime_resources_and_leaves_build_artifacts_in_sdk() {
        let root = tempfile::tempdir().unwrap();
        let sdk = root.path().join("sdk");
        let runtime_files = [
            "PaxHost.dll",
            "resources/en-US/PaxHost.pri",
            "host.manifest",
        ];
        let build_files = [
            "PaxHost.PDB",
            "PaxHost.ilk",
            "PaxHost.exp",
            "PaxHost.lib",
            "obj/host.obj",
        ];
        for name in runtime_files.iter().chain(build_files.iter()) {
            let source = sdk.join(name);
            fs::create_dir_all(source.parent().unwrap()).unwrap();
            fs::write(source, name.as_bytes()).unwrap();
        }
        let release = root.path().join("release");
        copy_sdk_runtime(&sdk, &release, true).unwrap();
        for name in runtime_files {
            assert_eq!(fs::read(release.join(name)).unwrap(), name.as_bytes());
        }
        for name in build_files {
            assert!(!release.join(name).exists());
            assert_eq!(fs::read(sdk.join(name)).unwrap(), name.as_bytes());
        }
        // Debug Windows and Linux packages retain their existing copy behavior.
        let unfiltered = root.path().join("unfiltered");
        copy_sdk_runtime(&sdk, &unfiltered, false).unwrap();
        for name in runtime_files.iter().chain(build_files.iter()) {
            assert_eq!(fs::read(unfiltered.join(name)).unwrap(), name.as_bytes());
        }
    }

    #[test]
    fn windows_cartridge_and_symbols_survive_launcher_output_replacement() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("application.dll");
        let symbols = source.with_extension("pdb");
        fs::write(&source, b"selected cartridge").unwrap();
        fs::write(&symbols, b"cartridge symbols").unwrap();
        let temporary = tempfile::tempdir_in(root.path()).unwrap();
        let frozen = artifacts::freeze_cartridge(&source, temporary.path()).unwrap();
        fs::write(&source, b"launcher-phase library").unwrap();
        fs::write(&symbols, b"launcher symbols").unwrap();
        let staged = temporary.path().join("symbols");
        artifacts::stage_symbols(&staged, frozen.symbols.as_deref(), &symbols).unwrap();
        let published = root.path().join("windows-symbols");
        artifacts::publish_symbols(&staged, &published).unwrap();
        assert_eq!(fs::read(frozen.image).unwrap(), b"selected cartridge");
        assert_eq!(
            fs::read(published.join("cartridge/application.pdb")).unwrap(),
            b"cartridge symbols"
        );
        assert_eq!(
            fs::read(published.join("launcher/application.pdb")).unwrap(),
            b"launcher symbols"
        );
    }

    #[test]
    fn absent_optional_pdbs_remove_stale_published_symbols() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("application.dll");
        fs::write(&source, b"valid cartridge without symbols").unwrap();
        let temporary = tempfile::tempdir_in(root.path()).unwrap();
        let frozen = artifacts::freeze_cartridge(&source, temporary.path()).unwrap();
        assert!(frozen.symbols.is_none());
        let published = root.path().join("windows-symbols");
        fs::create_dir_all(published.join("cartridge")).unwrap();
        fs::write(published.join("cartridge/application.pdb"), b"stale").unwrap();
        let staged = temporary.path().join("symbols");
        artifacts::stage_symbols(&staged, None, &source.with_extension("pdb")).unwrap();
        artifacts::publish_symbols(&staged, &published).unwrap();
        assert_eq!(fs::read_dir(published).unwrap().count(), 0);
    }

    #[test]
    fn missing_images_and_unreadable_present_pdbs_fail() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("application.dll");
        let temporary = tempfile::tempdir_in(root.path()).unwrap();
        assert!(artifacts::freeze_cartridge(&source, temporary.path()).is_err());
        fs::write(&source, b"cartridge").unwrap();
        fs::create_dir(source.with_extension("pdb")).unwrap();
        assert!(artifacts::freeze_cartridge(&source, temporary.path()).is_err());
        assert!(artifacts::stage_symbols(
            &temporary.path().join("symbols"),
            None,
            &source.with_extension("pdb")
        )
        .is_err());
    }

    #[test]
    fn captured_pdb_disappearance_does_not_publish_a_partial_symbol_set() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("application.dll");
        fs::write(&source, b"cartridge").unwrap();
        fs::write(source.with_extension("pdb"), b"symbols").unwrap();
        let temporary = tempfile::tempdir_in(root.path()).unwrap();
        let frozen = artifacts::freeze_cartridge(&source, temporary.path()).unwrap();
        fs::remove_file(frozen.symbols.as_ref().unwrap()).unwrap();
        let staged = temporary.path().join("symbols");
        assert!(artifacts::stage_symbols(
            &staged,
            frozen.symbols.as_deref(),
            &source.with_extension("pdb")
        )
        .is_err());
        assert!(temporary.path().exists());
        let temporary_path = temporary.path().to_owned();
        drop(temporary);
        assert!(!temporary_path.exists());
    }

    #[test]
    fn missing_staged_symbols_preserve_the_previous_diagnostic_directory() {
        let root = tempfile::tempdir().unwrap();
        let published = root.path().join("windows-symbols");
        fs::create_dir(&published).unwrap();
        fs::write(published.join("previous.pdb"), b"previous build").unwrap();
        assert!(artifacts::publish_symbols(&root.path().join("missing"), &published).is_err());
        assert_eq!(
            fs::read(published.join("previous.pdb")).unwrap(),
            b"previous build"
        );
    }
}
