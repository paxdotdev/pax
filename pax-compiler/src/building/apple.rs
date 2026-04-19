use cargo_metadata::MetadataCommand;
use colored::Colorize;
use pax_manifest::PaxManifest;

use crate::dev_session::{
    self, now_ms, project_designtime_manifest_file, project_dev_dir, write_project_active_session,
    write_registered_session, DevSession,
};
use crate::helpers::{
    BUILD_DIR_NAME, DIR_IGNORE_LIST_MACOS, ERR_SPAWN, INTERFACE_DIR_NAME, PAX_BADGE,
};
use crate::{copy_dir_recursively, wait_with_output, RunContext, RunTarget};

use color_eyre::eyre;
use eyre::eyre;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::fs as unix_fs;
#[cfg(unix)]
use std::os::unix::process::CommandExt;

const PORTABLE_DYLIB_INSTALL_NAME: &str = "@rpath/PaxCartridge.framework/PaxCartridge";

const XCODE_MACOS_TARGET_DEBUG: &str = "Pax macOS (Development)";
const XCODE_MACOS_TARGET_RELEASE: &str = "Pax macOS (Release)";
const XCODE_IOS_TARGET_DEBUG: &str = "Pax iOS (Development)";
const XCODE_IOS_TARGET_RELEASE: &str = "Pax iOS (Release)";

// These package IDs represent the directory / package names inside the xcframework,
const MACOS_MULTIARCH_PACKAGE_ID: &str = "macos-arm64_x86_64";
const IOS_SIMULATOR_MULTIARCH_PACKAGE_ID: &str = "ios-arm64_x86_64-simulator";
const IOS_PACKAGE_ID: &str = "ios-arm64";
const PAX_CARTRIDGE_FRAMEWORK_BINARY: &str = "PaxCartridge";

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppleTargetKind {
    MacOS,
    IosSimulator,
    IosDevice,
}

#[derive(Clone, Copy)]
struct AppleTargetMapping {
    rust_target: &'static str,
    packaged_arch: &'static str,
    xcode_arch: &'static str,
    kind: AppleTargetKind,
}

struct AppleBuildResult {
    packaged_arch: String,
    dylib_path: String,
    output: Output,
    kind: AppleTargetKind,
}

const MACOS_ARM64_TARGET: AppleTargetMapping = AppleTargetMapping {
    rust_target: "aarch64-apple-darwin",
    packaged_arch: "macos-arm64",
    xcode_arch: "arm64",
    kind: AppleTargetKind::MacOS,
};

const MACOS_X86_64_TARGET: AppleTargetMapping = AppleTargetMapping {
    rust_target: "x86_64-apple-darwin",
    packaged_arch: "macos-x86_64",
    xcode_arch: "x86_64",
    kind: AppleTargetKind::MacOS,
};

const IOS_DEVICE_ARM64_TARGET: AppleTargetMapping = AppleTargetMapping {
    rust_target: "aarch64-apple-ios",
    packaged_arch: "ios-arm64",
    xcode_arch: "arm64",
    kind: AppleTargetKind::IosDevice,
};

const IOS_SIMULATOR_X86_64_TARGET: AppleTargetMapping = AppleTargetMapping {
    rust_target: "x86_64-apple-ios",
    packaged_arch: "iossimulator-x86_64",
    xcode_arch: "x86_64",
    kind: AppleTargetKind::IosSimulator,
};

const IOS_SIMULATOR_ARM64_TARGET: AppleTargetMapping = AppleTargetMapping {
    rust_target: "aarch64-apple-ios-sim",
    packaged_arch: "iossimulator-arm64",
    xcode_arch: "arm64",
    kind: AppleTargetKind::IosSimulator,
};

fn select_apple_target_mappings(
    target: &RunTarget,
    is_release: bool,
    resolved_ios_device: Option<&ResolvedIosDevice>,
    host_arch: &str,
) -> Vec<AppleTargetMapping> {
    match target {
        RunTarget::macOS => {
            if is_release {
                vec![MACOS_ARM64_TARGET, MACOS_X86_64_TARGET]
            } else {
                match host_arch {
                    "x86_64" => vec![MACOS_X86_64_TARGET],
                    "aarch64" | "arm64" => vec![MACOS_ARM64_TARGET],
                    _ => vec![MACOS_ARM64_TARGET, MACOS_X86_64_TARGET],
                }
            }
        }
        RunTarget::iOS => match ios_build_destination(is_release, resolved_ios_device) {
            IosDeviceKind::Physical => vec![IOS_DEVICE_ARM64_TARGET],
            IosDeviceKind::Simulator => match host_arch {
                "x86_64" => vec![IOS_SIMULATOR_X86_64_TARGET],
                "aarch64" | "arm64" => vec![IOS_SIMULATOR_ARM64_TARGET],
                _ => vec![IOS_SIMULATOR_ARM64_TARGET, IOS_SIMULATOR_X86_64_TARGET],
            },
        },
        RunTarget::Web => vec![],
    }
}

fn ios_build_destination(
    is_release: bool,
    resolved_ios_device: Option<&ResolvedIosDevice>,
) -> IosDeviceKind {
    match resolved_ios_device.map(|device| device.kind) {
        Some(kind) => kind,
        None if is_release => IosDeviceKind::Physical,
        None => IosDeviceKind::Simulator,
    }
}

fn xcode_archs_for_target_mappings(target_mappings: &[AppleTargetMapping]) -> String {
    let mut archs = Vec::new();
    for target_mapping in target_mappings {
        if !archs.contains(&target_mapping.xcode_arch) {
            archs.push(target_mapping.xcode_arch);
        }
    }
    archs.join(" ")
}

fn copy_or_lipo_dylibs(
    process_child_ids: &Arc<Mutex<Vec<u64>>>,
    label: &str,
    input_paths: &[String],
    output_path: &Path,
) -> Result<(), eyre::Report> {
    if input_paths.is_empty() {
        return Err(eyre!(
            "No architecture-specific binaries were available for {} packaging.",
            label
        ));
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if input_paths.len() == 1 {
        fs::copy(&input_paths[0], output_path)?;
        return Ok(());
    }

    let mut lipo_command = Command::new("lipo");
    lipo_command
        .arg("-create")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    for path in input_paths {
        lipo_command.arg(path);
    }

    lipo_command.arg("-output").arg(output_path);

    #[cfg(unix)]
    unsafe {
        lipo_command.pre_exec(crate::pre_exec_hook);
    }
    let child = lipo_command.spawn().expect(ERR_SPAWN);
    let output = wait_with_output(process_child_ids, child);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(eyre!(
            "Failed to combine {} dylibs with lipo. {}",
            label,
            stderr.trim()
        ));
    }

    Ok(())
}

fn remove_path_if_exists(path: &Path) -> Result<(), eyre::Report> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_file(path)?;
            }
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

#[cfg(unix)]
fn replace_symlink(link_path: &Path, target: &Path) -> Result<(), eyre::Report> {
    remove_path_if_exists(link_path)?;
    unix_fs::symlink(target, link_path)?;
    Ok(())
}

fn move_into_versions_if_needed(src: &Path, dst: &Path) -> Result<(), eyre::Report> {
    match fs::symlink_metadata(src) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Ok(());
            }
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            if fs::symlink_metadata(dst).is_ok() {
                remove_path_if_exists(dst)?;
            }
            fs::rename(src, dst)?;
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

fn normalize_macos_framework_bundle(framework_dir: &Path) -> Result<(), eyre::Report> {
    let versions_dir = framework_dir.join("Versions");
    let version_dir = versions_dir.join("A");
    let resources_dir = version_dir.join("Resources");
    fs::create_dir_all(resources_dir.as_path())?;

    move_into_versions_if_needed(
        framework_dir.join(PAX_CARTRIDGE_FRAMEWORK_BINARY).as_path(),
        version_dir.join(PAX_CARTRIDGE_FRAMEWORK_BINARY).as_path(),
    )?;
    move_into_versions_if_needed(
        framework_dir.join("Headers").as_path(),
        version_dir.join("Headers").as_path(),
    )?;
    move_into_versions_if_needed(
        framework_dir.join("Modules").as_path(),
        version_dir.join("Modules").as_path(),
    )?;
    move_into_versions_if_needed(
        framework_dir.join("Resources").as_path(),
        version_dir.join("Resources").as_path(),
    )?;
    move_into_versions_if_needed(
        framework_dir.join("Info.plist").as_path(),
        resources_dir.join("Info.plist").as_path(),
    )?;

    #[cfg(unix)]
    {
        replace_symlink(versions_dir.join("Current").as_path(), Path::new("A"))?;
        replace_symlink(
            framework_dir.join(PAX_CARTRIDGE_FRAMEWORK_BINARY).as_path(),
            Path::new("Versions/Current/PaxCartridge"),
        )?;
        replace_symlink(
            framework_dir.join("Headers").as_path(),
            Path::new("Versions/Current/Headers"),
        )?;
        replace_symlink(
            framework_dir.join("Modules").as_path(),
            Path::new("Versions/Current/Modules"),
        )?;
        replace_symlink(
            framework_dir.join("Resources").as_path(),
            Path::new("Versions/Current/Resources"),
        )?;
    }

    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SimulatorVariantRank {
    Other = 0,
    Base = 1,
    ProMax = 2,
    Pro = 3,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum IosDeviceKind {
    Simulator,
    Physical,
}

#[derive(Clone)]
struct ResolvedIosDevice {
    kind: IosDeviceKind,
    name: String,
    identifier: String,
}

struct SimulatorDevice {
    name: String,
    udid: String,
    state: String,
}

struct PhysicalDevice {
    name: String,
    identifier: String,
}

pub fn build_apple_project_with_cartridge(
    ctx: &RunContext,
    pax_dir: &PathBuf,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
    assets_dirs: Vec<String>,
    manifest: PaxManifest,
) -> Result<(), eyre::Report> {
    let target: &RunTarget = &ctx.target;
    let target_str: &str = target.into();
    let target_str_lower = &target_str.to_lowercase();
    let pax_dir = PathBuf::from(pax_dir.to_str().unwrap());
    let project_path = ctx.project_path.clone();

    let is_release: bool = ctx.is_release;
    let is_ios = if let RunTarget::iOS = target {
        true
    } else {
        false
    };

    let resolved_ios_device = if is_ios && (ctx.should_also_run || ctx.ios_device.is_some()) {
        Some(resolve_ios_device(
            ctx.ios_device.as_deref(),
            &process_child_ids,
        )?)
    } else {
        None
    };

    let build_mode_name: &str = if is_release { "release" } else { "debug" };
    let should_run_designtime = ctx.should_run_designtime;
    let should_run_designer = ctx.should_run_designer;

    let target_mappings = select_apple_target_mappings(
        target,
        is_release,
        resolved_ios_device.as_ref(),
        std::env::consts::ARCH,
    );

    let dylib_file_name = resolve_dylib_file_name(&project_path)?;

    let mut handles = Vec::new();

    //(arch id, single-platform .dylib path, stdout/stderr from build)
    let build_results: Arc<Mutex<HashMap<u32, AppleBuildResult>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let targets_single_string = target_mappings
        .iter()
        .map(|tm| tm.packaged_arch.to_string())
        .collect::<Vec<String>>()
        .join(", ")
        .bold();
    println!(
        "{} 🧶 Compiling targets {{{}}} in {} mode using {} threads...\n",
        *PAX_BADGE,
        &targets_single_string,
        &build_mode_name.to_string().bold(),
        target_mappings.len()
    );

    for (index, target_mapping) in target_mappings.iter().copied().enumerate() {
        let project_path = project_path.clone();
        let pax_dir = pax_dir.clone();
        let dylib_file_name = dylib_file_name.clone();

        let process_child_ids_threadsafe = process_child_ids.clone();
        let build_results_threadsafe = build_results.clone();

        let arg_features = if let RunTarget::macOS = &target {
            "--features=macos"
        } else {
            "--features=ios"
        };

        let handle = thread::spawn(move || {
            let mut cmd = Command::new("cargo");

            cmd.current_dir(&project_path)
                .arg("build")
                .arg("--color")
                .arg("always")
                .arg("--target")
                .arg(target_mapping.rust_target)
                .arg(arg_features)
                .env("PAX_DIR", &pax_dir)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            if should_run_designer {
                cmd.arg("--features").arg("designer");
            } else if should_run_designtime {
                cmd.arg("--features").arg("designtime");
            }

            if is_release {
                cmd.arg("--release");
            }

            #[cfg(unix)]
            unsafe {
                cmd.pre_exec(crate::pre_exec_hook);
            }

            let child = cmd.spawn().expect(ERR_SPAWN);

            //Execute `cargo build`, which generates our dylibs
            let output = wait_with_output(&process_child_ids_threadsafe, child);

            let dylib_src = project_path
                .join("target")
                .join(target_mapping.rust_target)
                .join(build_mode_name)
                .join(&dylib_file_name);

            let new_val = AppleBuildResult {
                packaged_arch: target_mapping.packaged_arch.to_string(),
                dylib_path: dylib_src.to_str().unwrap().to_string(),
                output,
                kind: target_mapping.kind,
            };
            build_results_threadsafe
                .lock()
                .unwrap()
                .insert(index as u32, new_val);
        });
        handles.push(handle);
    }

    // Wait for all threads to complete and print their outputs
    for handle in handles {
        handle.join().unwrap();
    }

    let results = build_results.lock().unwrap();

    let mut should_abort = false;
    //Print stdout/stderr
    for i in 0..target_mappings.len() {
        let result = results.get(&(i as u32)).unwrap();
        let target = &result.packaged_arch;
        let output = &result.output;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if stdout != "" || stderr != "" {
            println!("{} build finished with output:", &target);
        }
        if stdout != "" {
            println!("{}", &stdout);
        }
        if stderr != "" {
            eprintln!("{}", &stderr);
        }

        if !output.status.success() {
            should_abort = true;
        }
    }

    if should_abort {
        return Err(eyre!(
            "Failed to build one or more targets with Cargo. Aborting."
        ));
    }

    // Update the `install name` of each Rust-built .dylib, instead of the default-output absolute file paths
    // embedded in each .dylib.  This allows our .dylibs to be portably embedded into an SPM module.
    let result = results.iter().try_for_each(|res: (&u32, &AppleBuildResult)| {
        let dylib_path = &res.1.dylib_path;
        let mut cmd = Command::new("install_name_tool");
        cmd
            .arg("-id")
            .arg(PORTABLE_DYLIB_INSTALL_NAME)
            .arg(dylib_path);

        #[cfg(unix)]
        unsafe {
            cmd.pre_exec(crate::pre_exec_hook);
        }
        let child = cmd.spawn().unwrap();
        let output = wait_with_output(&process_child_ids, child);
        if !output.status.success() {
            return Err(eyre!("Failed to rewrite dynamic library (path:{}) install name with install_name_tool.  Aborting.", dylib_path));
        }

        Ok(())
    });

    match result {
        Err(r) => {
            return Err(r);
        }
        _ => {}
    };

    let macos_framework_dir = pax_dir
        .join(INTERFACE_DIR_NAME)
        .join("common")
        .join("pax-swift-cartridge")
        .join("PaxCartridge.xcframework")
        .join(MACOS_MULTIARCH_PACKAGE_ID)
        .join("PaxCartridge.framework");

    let macos_dylib_dest = macos_framework_dir
        .join("Versions")
        .join("A")
        .join(PAX_CARTRIDGE_FRAMEWORK_BINARY);

    let simulator_dylib_dest = pax_dir
        .join(INTERFACE_DIR_NAME)
        .join("common")
        .join("pax-swift-cartridge")
        .join("PaxCartridge.xcframework")
        .join(IOS_SIMULATOR_MULTIARCH_PACKAGE_ID)
        .join("PaxCartridge.framework")
        .join("PaxCartridge");

    let iphone_native_dylib_dest = pax_dir
        .join(INTERFACE_DIR_NAME)
        .join("common")
        .join("pax-swift-cartridge")
        .join("PaxCartridge.xcframework")
        .join(IOS_PACKAGE_ID)
        .join("PaxCartridge.framework")
        .join("PaxCartridge");

    if let RunTarget::macOS = target {
        normalize_macos_framework_bundle(&macos_framework_dir)?;
    }

    if is_release || is_ios {
        // Merge architecture-specific binaries with `lipo` (this is an undocumented requirement
        // of multi-arch builds + xcframeworks for the Apple toolchain; we cannot bundle two
        // macos arch .frameworks in an xcframework; they must lipo'd into a single .framework + dylib.
        // Similarly, iOS binaries require a particular bundling for simulator & device builds.)
        println!(
            "{} 🖇️  Packaging architecture-specific binaries...",
            *PAX_BADGE
        );

        if let RunTarget::macOS = target {
            // For macOS, we want to lipo both our arm64 and x86_64 dylibs into a single binary,
            // then bundle that single binary into a single framework within the xcframework.

            let lipo_input_paths = results
                .iter()
                .filter(|res| res.1.kind == AppleTargetKind::MacOS)
                .map(|res| res.1.dylib_path.clone())
                .collect::<Vec<String>>();

            copy_or_lipo_dylibs(
                &process_child_ids,
                "macOS",
                &lipo_input_paths,
                &macos_dylib_dest,
            )?;
        } else {
            // For iOS, we want to:
            // 1. copy or lipo together the selected simulator build architectures
            // 2. copy or lipo together the selected device build architectures
            let simulator_builds = results
                .iter()
                .filter(|res| res.1.kind == AppleTargetKind::IosSimulator)
                .collect::<Vec<_>>();
            let device_build = results
                .iter()
                .filter(|res| res.1.kind == AppleTargetKind::IosDevice)
                .collect::<Vec<_>>();

            let simulator_input_paths = simulator_builds
                .iter()
                .map(|res| res.1.dylib_path.clone())
                .collect::<Vec<String>>();
            let device_input_paths = device_build
                .iter()
                .map(|res| res.1.dylib_path.clone())
                .collect::<Vec<String>>();

            if !simulator_input_paths.is_empty() {
                copy_or_lipo_dylibs(
                    &process_child_ids,
                    "iOS simulator",
                    &simulator_input_paths,
                    &simulator_dylib_dest,
                )?;
            }

            if !device_input_paths.is_empty() {
                copy_or_lipo_dylibs(
                    &process_child_ids,
                    "iOS device",
                    &device_input_paths,
                    &iphone_native_dylib_dest,
                )?;
            }
        }
    } else {
        // For macos development builds, instead of lipoing, just drop the singular build into the appropriate output destination
        // This measure speeds up development builds substantially.
        let result = results.iter().next().unwrap();
        let src = &result.1.dylib_path;
        let dest = macos_dylib_dest;
        let _ = fs::copy(src, dest);
    }

    if let RunTarget::macOS = target {
        normalize_macos_framework_layout(
            &pax_dir
                .join(INTERFACE_DIR_NAME)
                .join("common")
                .join("pax-swift-cartridge")
                .join("PaxCartridge.xcframework")
                .join(MACOS_MULTIARCH_PACKAGE_ID)
                .join("PaxCartridge.framework"),
        )?;
    }

    if is_release && is_ios {
        unimplemented!("\n\n\
Release builds for Pax iOS are not yet supported because configuration has not been exposed for development teams or code-signing.\n
You can build a release build manually by configuring the generated xcodeproject in `.pax/pkg/pax-chassis-ios/interface` with your development team and codesigning configuration.\n
The relevant Framework binaries have been built in release mode at `.pax/pkg/pax-chassis-common/pax-swift-cartridge/` and should be loaded via the above xcodeproject.\n
You can also use the SPM package exposed at `.pax/pkg/pax-chassis-common/pax-swift-cartridge/` for manual inclusion in your own SwiftUI app.\n
Note that the temporary directories mentioned above are subject to overwriting.\n\n")
    }

    let (xcodeproj_path, scheme) = if let RunTarget::macOS = target {
        (
            pax_dir
                .join(INTERFACE_DIR_NAME)
                .join("macos")
                .join("pax-app-macos")
                .join("pax-app-macos.xcodeproj"),
            if is_release {
                XCODE_MACOS_TARGET_RELEASE
            } else {
                XCODE_MACOS_TARGET_DEBUG
            },
        )
    } else {
        (
            pax_dir
                .join(INTERFACE_DIR_NAME)
                .join("ios")
                .join("pax-app-ios")
                .join("pax-app-ios.xcodeproj"),
            if is_release {
                XCODE_IOS_TARGET_RELEASE
            } else {
                XCODE_IOS_TARGET_DEBUG
            },
        )
    };

    let configuration = if is_release { "Release" } else { "Debug" };

    normalize_apple_package_paths(&xcodeproj_path)?;

    let build_dest_base = pax_dir
        .join(BUILD_DIR_NAME)
        .join(build_mode_name)
        .join(target_str_lower);
    let executable_output_dir_path = build_dest_base.join("app");
    let executable_dot_app_path = executable_output_dir_path.join(&format!("{}.app", &scheme));
    let executable_resources_bundle_path =
        executable_output_dir_path.join("PaxSwiftCartridge_PaxCartridgeAssets.bundle");
    let derived_data_path = build_dest_base.join("derived-data");
    let source_packages_path = build_dest_base.join("source-packages");
    let _ = fs::create_dir_all(&executable_output_dir_path);
    let _ = fs::create_dir_all(&derived_data_path);
    let _ = fs::create_dir_all(&source_packages_path);
    let _ = remove_path_if_exists(&executable_resources_bundle_path);
    let _ = remove_path_if_exists(&executable_dot_app_path);

    let asset_dest = pax_dir
        .join(INTERFACE_DIR_NAME)
        .join("common")
        .join("pax-swift-cartridge")
        .join("Sources")
        .join("PaxCartridgeAssets")
        .join("Resources")
        .join("assets");
    fs::create_dir_all(&asset_dest)?;
    for asset_src in assets_dirs {
        let asset_src = PathBuf::from(asset_src);
        if asset_src.exists() {
            copy_dir_recursively(&asset_src, &asset_dest, &[])
                .map_err(|err| eyre!("Error copying assets: {}", err))?;
        }
    }

    let build_for_physical_device = matches!(
        resolved_ios_device.as_ref().map(|device| device.kind),
        Some(IosDeviceKind::Physical)
    );

    let sdk = if let RunTarget::iOS = target {
        if build_for_physical_device || (is_release && resolved_ios_device.is_none()) {
            "iphoneos"
        } else {
            "iphonesimulator"
        }
    } else {
        "macosx"
    };
    let xcode_archs = if let RunTarget::iOS = target {
        Some(xcode_archs_for_target_mappings(&target_mappings))
    } else {
        None
    };

    println!("{} 💻 Building xcodeproject...", *PAX_BADGE);
    let mut cmd = Command::new("xcodebuild");
    cmd.arg("-configuration")
        .arg(configuration)
        .arg("-project")
        .arg(&xcodeproj_path)
        .arg("-scheme")
        .arg(scheme)
        .arg("-sdk")
        .arg(sdk)
        .arg("-derivedDataPath")
        .arg(&derived_data_path)
        .arg("-clonedSourcePackagesDirPath")
        .arg(&source_packages_path)
        .arg(&format!(
            "CONFIGURATION_BUILD_DIR={}",
            executable_output_dir_path.to_str().unwrap()
        ))
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::piped());

    if let Some(xcode_archs) = xcode_archs.as_deref() {
        cmd.arg(format!("ARCHS={xcode_archs}"));
    }

    if let Some(device) = resolved_ios_device.as_ref() {
        cmd.arg("-destination")
            .arg(format!("id={}", device.identifier));
    }

    if !is_release && !build_for_physical_device {
        cmd.arg("CODE_SIGNING_REQUIRED=NO")
            .arg("CODE_SIGN_IDENTITY=");
    } else if build_for_physical_device {
        cmd.arg("-allowProvisioningUpdates")
            .arg("-allowProvisioningDeviceRegistration");
    }

    if let Some(team_id) = ctx.ios_development_team.as_deref() {
        cmd.arg(format!("DEVELOPMENT_TEAM={team_id}"));
    }

    if !ctx.verbose {
        cmd.arg("-quiet");
        cmd.arg("GCC_WARN_INHIBIT_ALL_WARNINGS=YES");
    }

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(crate::pre_exec_hook);
    }
    let child = cmd.spawn().expect(ERR_SPAWN);
    let output = wait_with_output(&process_child_ids, child);

    // Crudely prune out noisy xcodebuild warnings due to an apparent xcode-internal bug at time of authoring, spitting out:
    //   Details:  createItemModels creation requirements should not create capability item model for a capability item model that already exists.
    //       Function: createItemModels(for:itemModelSource:)
    //   Thread:   <_NSMainThread: 0x600000be02c0>{number = 1, name = main}
    //   Please file a bug at https://feedbackassistant.apple.com with this warning message and any useful information you can provide.
    // If we get to a point where xcodebuild isn't spitting these errors, we can drop this block of code and just `.inherit` stderr in
    // the command above.
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if ctx.verbose {
        println!("{}", stderr);
    } else {
        let mut skip_lines = 0;
        for line in stderr.lines() {
            // Check if this line starts a blacklisted message
            if line.starts_with("Details:  createItemModels") {
                skip_lines = 5; // There are 5 lines to skip, including this one
            }

            // If skip_lines is non-zero, skip printing and decrement the counter
            if skip_lines > 0 {
                skip_lines -= 1;
                continue;
            }

            println!("{}", line);
        }
    }

    if !output.status.success() {
        if build_for_physical_device && stderr.contains("Developer Mode disabled") {
            let device_name = resolved_ios_device
                .as_ref()
                .map(|device| device.name.as_str())
                .unwrap_or("the selected device");
            return Err(eyre!(
                "Xcode can see `{}`, but Developer Mode is disabled. Enable Developer Mode on the phone, restart it if prompted, reconnect/trust it, and run again.",
                device_name
            ));
        }
        return Err(eyre!("Failed to build project with xcodebuild. Aborting."));
    }

    //Copy build artifacts & packages into `build`
    let swift_cart_src = pax_dir
        .join(INTERFACE_DIR_NAME)
        .join("common")
        .join("pax-swift-cartridge");
    let swift_common_src = pax_dir
        .join(INTERFACE_DIR_NAME)
        .join("common")
        .join("pax-swift-common");

    let swift_cart_build_dest = build_dest_base.join("common").join("pax-swift-cartridge");
    let swift_common_build_dest = build_dest_base.join("common").join("pax-swift-common");

    let (app_xcodeproj_src, app_xcodeproj_build_dest) = if let RunTarget::macOS = target {
        (
            pax_dir
                .join(INTERFACE_DIR_NAME)
                .join("macos")
                .join("pax-app-macos"),
            build_dest_base.join("pax-app-macos"),
        )
    } else {
        (
            pax_dir
                .join(INTERFACE_DIR_NAME)
                .join("ios")
                .join("pax-app-ios"),
            build_dest_base.join("pax-app-ios"),
        )
    };

    let _ = fs::create_dir_all(&swift_cart_build_dest);
    let _ = fs::create_dir_all(&swift_common_build_dest);
    let _ = fs::create_dir_all(&app_xcodeproj_build_dest);

    let _ = copy_dir_recursively(
        &swift_cart_src,
        &swift_cart_build_dest,
        &DIR_IGNORE_LIST_MACOS,
    );
    let _ = copy_dir_recursively(
        &swift_common_src,
        &swift_common_build_dest,
        &DIR_IGNORE_LIST_MACOS,
    );
    let _ = copy_dir_recursively(
        &app_xcodeproj_src,
        &app_xcodeproj_build_dest,
        &DIR_IGNORE_LIST_MACOS,
    );

    // Start  `run` rather than a `build`
    let target_str: &str = target.into();
    if ctx.should_also_run {
        if ctx.should_run_designtime && matches!(target, RunTarget::macOS) {
            println!(
                "{} 🐇{} Running Pax {}{}...",
                *PAX_BADGE,
                if ctx.should_run_designer { "🎨" } else { "" },
                target_str,
                if ctx.should_run_designer {
                    " with Pax Designer"
                } else {
                    " with designtime"
                }
            );
        } else {
            println!("{} 🐇 Running Pax {}...", *PAX_BADGE, target_str);
        }

        if let RunTarget::macOS = target {
            //
            // Handle macOS `run`
            //

            let system_binary_path =
                executable_dot_app_path.join(&format!("Contents/MacOS/{}", scheme));
            let mut dev_session = if ctx.should_run_designtime {
                Some(prepare_dev_session(&project_path, &pax_dir)?)
            } else {
                None
            };
            let mut designtime_server = if let Some(session) = dev_session.as_mut() {
                let ready_file = session
                    .session_dir
                    .as_ref()
                    .unwrap()
                    .join("design-server-url.txt");
                let server = Some(spawn_designtime_server_process(
                    &pax_dir,
                    &project_path,
                    &manifest,
                    &ready_file,
                    process_child_ids.clone(),
                )?);
                session.design_server_addr = Some(wait_for_designtime_ready(
                    &ready_file,
                    Duration::from_secs(5),
                )?);
                server
            } else {
                None
            };

            let mut cmd = Command::new(system_binary_path);
            if let Some(session) = &dev_session {
                cmd.env(
                    "PAX_DEV_SESSION_DIR",
                    session.session_dir.as_ref().unwrap().to_str().unwrap(),
                )
                .env(
                    "PAX_DEV_REGISTRY_FILE",
                    dev_session::global_session_registry_file(&session.session_id)?
                        .to_str()
                        .unwrap(),
                )
                .env(
                    "PAX_DEV_PROJECT_ROOT",
                    session.project_root.as_ref().unwrap().to_str().unwrap(),
                )
                .env(
                    "PAX_DESIGN_SERVER_ADDR",
                    session.design_server_addr.as_ref().unwrap(),
                );
            }

            #[cfg(unix)]
            unsafe {
                cmd.pre_exec(crate::pre_exec_hook);
            }

            let mut child = cmd.spawn().expect("failed to execute the app");
            let app_pid = child.id() as u64;
            if let Some(session) = dev_session.as_mut() {
                finalize_dev_session(&pax_dir, session, child.id())?;
            }
            process_child_ids.lock().unwrap().push(app_pid);
            let status = child.wait().expect("failed to wait for the app");
            process_child_ids
                .lock()
                .unwrap()
                .retain(|&id| id != app_pid);
            if let Some(session) = &dev_session {
                cleanup_dev_session(&pax_dir, session)?;
            }
            if let Some(server) = designtime_server.as_mut() {
                let server_pid = server.id() as u64;
                let _ = server.kill();
                let _ = server.wait();
                process_child_ids
                    .lock()
                    .unwrap()
                    .retain(|&id| id != server_pid);
            }

            println!("App exited with: {:?}", status);
        } else {
            //
            // Handle iOS `run`
            //
            let device = resolved_ios_device
                .as_ref()
                .ok_or_else(|| eyre!("Missing resolved iOS target device."))?;

            match device.kind {
                IosDeviceKind::Simulator => run_on_simulator(
                    &device.identifier,
                    &device.name,
                    &executable_output_dir_path,
                    &executable_dot_app_path,
                    &process_child_ids,
                )?,
                IosDeviceKind::Physical => run_on_physical_device(
                    &device.identifier,
                    &device.name,
                    &executable_output_dir_path,
                    &executable_dot_app_path,
                    &process_child_ids,
                )?,
            }
        }
    } else {
        let build_path = executable_output_dir_path.to_str().unwrap().bold();
        println!(
            "{} 🗂️  Done: {} {} build available at {}",
            *PAX_BADGE, target_str, build_mode_name, build_path
        );
    }
    Ok(())
}

fn resolve_ios_device(
    selector: Option<&str>,
    process_child_ids: &Arc<Mutex<Vec<u64>>>,
) -> Result<ResolvedIosDevice, eyre::Report> {
    let selector = selector.map(str::trim).filter(|value| !value.is_empty());
    let (kind_hint, query) = parse_ios_device_selector(selector);

    match query {
        None => match kind_hint.unwrap_or(IosDeviceKind::Simulator) {
            IosDeviceKind::Simulator => choose_best_simulator(process_child_ids),
            IosDeviceKind::Physical => choose_single_physical_device(process_child_ids),
        },
        Some(query) => resolve_named_ios_device(kind_hint, &query, process_child_ids),
    }
}

fn parse_ios_device_selector(selector: Option<&str>) -> (Option<IosDeviceKind>, Option<String>) {
    match selector {
        None => (Some(IosDeviceKind::Simulator), None),
        Some("simulator") => (Some(IosDeviceKind::Simulator), None),
        Some("device") => (Some(IosDeviceKind::Physical), None),
        Some(value) => {
            if let Some(query) = value.strip_prefix("simulator:") {
                (
                    Some(IosDeviceKind::Simulator),
                    Some(query.trim().to_string()),
                )
            } else if let Some(query) = value.strip_prefix("device:") {
                (
                    Some(IosDeviceKind::Physical),
                    Some(query.trim().to_string()),
                )
            } else {
                (None, Some(value.to_string()))
            }
        }
    }
}

fn choose_best_simulator(
    process_child_ids: &Arc<Mutex<Vec<u64>>>,
) -> Result<ResolvedIosDevice, eyre::Report> {
    let simulators = list_available_ios_simulators(process_child_ids)?;
    let mut best_choice: Option<(i32, SimulatorVariantRank, bool, &SimulatorDevice)> = None;

    for simulator in &simulators {
        let Some((generation, variant_rank)) = parse_iphone_simulator_preference(&simulator.name)
        else {
            continue;
        };

        let candidate = (
            generation,
            variant_rank,
            simulator.state == "Booted",
            simulator,
        );
        if best_choice
            .as_ref()
            .map(|best| {
                candidate.0 > best.0
                    || (candidate.0 == best.0
                        && (candidate.1 > best.1
                            || (candidate.1 == best.1 && candidate.2 && !best.2)))
            })
            .unwrap_or(true)
        {
            best_choice = Some(candidate);
        }
    }

    let Some((_, _, _, simulator)) = best_choice else {
        return Err(eyre!(
            "No installed iOS simulators found on this system. Install at least one iPhone simulator through Xcode and try again."
        ));
    };

    Ok(ResolvedIosDevice {
        kind: IosDeviceKind::Simulator,
        name: simulator.name.clone(),
        identifier: simulator.udid.clone(),
    })
}

fn choose_single_physical_device(
    process_child_ids: &Arc<Mutex<Vec<u64>>>,
) -> Result<ResolvedIosDevice, eyre::Report> {
    let devices = list_connected_physical_devices(process_child_ids)?;
    match devices.as_slice() {
        [] => Err(eyre!(
            "No connected iOS devices found. Attach and trust a phone, or pass --ios-device=simulator."
        )),
        [device] => Ok(ResolvedIosDevice {
            kind: IosDeviceKind::Physical,
            name: device.name.clone(),
            identifier: device.identifier.clone(),
        }),
        _ => Err(eyre!(
            "Multiple physical iOS devices are connected: {}. Pass --ios-device=device:<name-or-udid> to choose one.",
            devices
                .iter()
                .map(|device| format!("{} ({})", device.name, device.identifier))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn resolve_named_ios_device(
    kind_hint: Option<IosDeviceKind>,
    query: &str,
    process_child_ids: &Arc<Mutex<Vec<u64>>>,
) -> Result<ResolvedIosDevice, eyre::Report> {
    let mut candidates = Vec::new();

    if kind_hint != Some(IosDeviceKind::Physical) {
        candidates.extend(
            list_available_ios_simulators(process_child_ids)?
                .into_iter()
                .map(|simulator| ResolvedIosDevice {
                    kind: IosDeviceKind::Simulator,
                    name: simulator.name,
                    identifier: simulator.udid,
                }),
        );
    }

    if kind_hint != Some(IosDeviceKind::Simulator) {
        candidates.extend(
            list_connected_physical_devices(process_child_ids)?
                .into_iter()
                .map(|device| ResolvedIosDevice {
                    kind: IosDeviceKind::Physical,
                    name: device.name,
                    identifier: device.identifier,
                }),
        );
    }

    match_ios_device_query(query, candidates)
}

fn match_ios_device_query(
    query: &str,
    candidates: Vec<ResolvedIosDevice>,
) -> Result<ResolvedIosDevice, eyre::Report> {
    if candidates.is_empty() {
        return Err(eyre!("No matching iOS destinations are available."));
    }

    let query_lower = query.to_ascii_lowercase();
    for match_mode in [
        MatchMode::IdentifierExact,
        MatchMode::NameExact,
        MatchMode::NameContains,
    ] {
        let matches = candidates
            .iter()
            .filter(|candidate| match_mode.matches(candidate, &query_lower))
            .cloned()
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => continue,
            [device] => return Ok(device.clone()),
            _ => {
                return Err(eyre!(
                    "Ambiguous iOS destination `{}`. Matches: {}",
                    query,
                    matches
                        .iter()
                        .map(|device| format!("{} ({})", device.name, device.identifier))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
    }

    Err(eyre!(
        "No iOS destination matched `{}`. Available destinations: {}",
        query,
        candidates
            .iter()
            .map(|device| format!("{} ({})", device.name, device.identifier))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn list_available_ios_simulators(
    process_child_ids: &Arc<Mutex<Vec<u64>>>,
) -> Result<Vec<SimulatorDevice>, eyre::Report> {
    let mut cmd = Command::new("xcrun");
    cmd.arg("simctl")
        .arg("list")
        .arg("-j")
        .arg("devices")
        .arg("available")
        .stdout(std::process::Stdio::piped());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(crate::pre_exec_hook);
    }
    let child = cmd.spawn().expect(ERR_SPAWN);
    let output = wait_with_output(process_child_ids, child);
    let output_str = std::str::from_utf8(&output.stdout)
        .map_err(|_| eyre!("Failed to parse stdout for xcrun simctl list devices"))?;
    let parsed: Value = serde_json::from_str(output_str)
        .map_err(|_| eyre!("Failed to deserialize xcrun simctl list devices JSON."))?;

    let devices = parsed["devices"]
        .as_object()
        .ok_or_else(|| eyre!("Invalid JSON format for simulator devices."))?;

    let mut simulators = Vec::new();
    for device_list in devices.values() {
        let Some(device_array) = device_list.as_array() else {
            continue;
        };
        for device in device_array {
            let Some(name) = device["name"].as_str() else {
                continue;
            };
            let Some(udid) = device["udid"].as_str() else {
                continue;
            };
            let Some(state) = device["state"].as_str() else {
                continue;
            };
            simulators.push(SimulatorDevice {
                name: name.to_string(),
                udid: udid.to_string(),
                state: state.to_string(),
            });
        }
    }

    Ok(simulators)
}

fn list_connected_physical_devices(
    process_child_ids: &Arc<Mutex<Vec<u64>>>,
) -> Result<Vec<PhysicalDevice>, eyre::Report> {
    let mut cmd = Command::new("xcrun");
    cmd.arg("xctrace")
        .arg("list")
        .arg("devices")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(crate::pre_exec_hook);
    }
    let child = cmd.spawn().expect(ERR_SPAWN);
    let output = wait_with_output(process_child_ids, child);
    if !output.status.success() {
        return Err(eyre!(
            "Failed to list connected iOS devices with xcrun xctrace."
        ));
    }

    let output_str = std::str::from_utf8(&output.stdout)
        .map_err(|_| eyre!("Failed to parse stdout for xcrun xctrace list devices"))?;

    let mut in_devices_section = false;
    let mut devices = Vec::new();

    for line in output_str.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("== ") && line.ends_with(" ==") {
            in_devices_section = line == "== Devices ==";
            continue;
        }
        if !in_devices_section {
            continue;
        }
        let Some((name, identifier)) = parse_named_identifier_line(line) else {
            continue;
        };
        devices.push(PhysicalDevice { name, identifier });
    }

    Ok(devices)
}

fn parse_named_identifier_line(line: &str) -> Option<(String, String)> {
    let id_start = line.rfind(" (")?;
    let id_end = line.strip_suffix(')')?;
    let identifier = &id_end[id_start + 2..];
    let prefix = &id_end[..id_start];
    if !prefix.ends_with(')') {
        return None;
    }
    let version_start = prefix.rfind(" (")?;
    let version = &prefix[version_start + 2..prefix.len() - 1];
    if !version.chars().any(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let name = prefix[..version_start].trim();
    if name.is_empty() {
        return None;
    }
    Some((name.to_string(), identifier.to_string()))
}

fn run_on_simulator(
    device_udid: &str,
    device_name: &str,
    executable_output_dir_path: &PathBuf,
    executable_dot_app_path: &PathBuf,
    process_child_ids: &Arc<Mutex<Vec<u64>>>,
) -> Result<(), eyre::Report> {
    let simulators = list_available_ios_simulators(process_child_ids)?;
    for simulator in simulators {
        if simulator.state == "Booted" && simulator.udid != device_udid {
            let mut cmd = Command::new("xcrun");
            cmd.arg("simctl")
                .arg("shutdown")
                .arg(&simulator.udid)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());

            #[cfg(unix)]
            unsafe {
                cmd.pre_exec(crate::pre_exec_hook);
            }

            let child = cmd.spawn().expect(ERR_SPAWN);
            let _ = wait_with_output(process_child_ids, child);
        }
    }

    let mut cmd = Command::new("open");
    cmd.arg("-a")
        .arg("Simulator")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(crate::pre_exec_hook);
    }
    let child = cmd.spawn().expect(ERR_SPAWN);
    let output = wait_with_output(process_child_ids, child);
    if !output.status.success() {
        return Err(eyre!("Error opening iOS simulator. Aborting."));
    }

    let mut cmd = Command::new("xcrun");
    cmd.arg("simctl")
        .arg("boot")
        .arg(device_udid)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(crate::pre_exec_hook);
    }
    let child = cmd.spawn().expect(ERR_SPAWN);
    let _ = wait_with_output(process_child_ids, child);

    let mut cmd = Command::new("xcrun");
    cmd.arg("simctl")
        .arg("spawn")
        .arg(device_udid)
        .arg("launchctl")
        .arg("print")
        .arg("system")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(crate::pre_exec_hook);
    }
    let child = cmd.spawn().expect(ERR_SPAWN);
    let output = wait_with_output(process_child_ids, child);
    if !output.status.success() {
        return Err(eyre!("Error spawning iOS simulator. Aborting."));
    }

    let max_retries = 5;
    let retry_period_secs = 5;
    let mut retries = 0;

    while !is_simulator_booted(device_udid, process_child_ids) && retries < max_retries {
        println!("{} 💤 Waiting for simulator to boot...", *PAX_BADGE);
        std::thread::sleep(std::time::Duration::from_secs(retry_period_secs));
        retries += 1;
    }

    if retries == max_retries {
        return Err(eyre!(
            "Failed to boot the simulator within the expected time. Aborting."
        ));
    }

    println!(
        "{} 📤 Installing and running app from {} on simulator {}...",
        *PAX_BADGE,
        executable_output_dir_path.to_str().unwrap(),
        device_name
    );

    let mut cmd = Command::new("xcrun");
    cmd.arg("simctl")
        .arg("install")
        .arg(device_udid)
        .arg(executable_dot_app_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(crate::pre_exec_hook);
    }
    let child = cmd.spawn().expect(ERR_SPAWN);
    let output = wait_with_output(process_child_ids, child);
    if !output.status.success() {
        return Err(eyre!("Error installing app on iOS simulator. Aborting."));
    }

    let mut cmd = Command::new("xcrun");
    cmd.arg("simctl")
        .arg("launch")
        .arg(device_udid)
        .arg("dev.pax.pax-app-ios")
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(crate::pre_exec_hook);
    }
    let child = cmd.spawn().expect(ERR_SPAWN);
    let output = wait_with_output(process_child_ids, child);
    if !output.status.success() {
        return Err(eyre!("Error launching app on iOS simulator. Aborting."));
    }

    println!(
        "{} 🚀 App launched on simulator {}.",
        *PAX_BADGE, device_name
    );
    Ok(())
}

fn run_on_physical_device(
    device_identifier: &str,
    device_name: &str,
    executable_output_dir_path: &PathBuf,
    executable_dot_app_path: &PathBuf,
    process_child_ids: &Arc<Mutex<Vec<u64>>>,
) -> Result<(), eyre::Report> {
    println!(
        "{} 📤 Installing and running app from {} on device {}...",
        *PAX_BADGE,
        executable_output_dir_path.to_str().unwrap(),
        device_name
    );

    let mut cmd = Command::new("xcrun");
    cmd.arg("devicectl")
        .arg("device")
        .arg("install")
        .arg("app")
        .arg("--device")
        .arg(device_identifier)
        .arg(executable_dot_app_path)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(crate::pre_exec_hook);
    }
    let child = cmd.spawn().expect(ERR_SPAWN);
    let output = wait_with_output(process_child_ids, child);
    if !output.status.success() {
        return Err(eyre!(
            "Error installing app on physical iOS device. Aborting."
        ));
    }

    let mut cmd = Command::new("xcrun");
    cmd.arg("devicectl")
        .arg("device")
        .arg("process")
        .arg("launch")
        .arg("--device")
        .arg(device_identifier)
        .arg("--console")
        .arg("--terminate-existing")
        .arg("dev.pax.pax-app-ios")
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(crate::pre_exec_hook);
    }
    let child = cmd.spawn().expect(ERR_SPAWN);
    let output = wait_with_output(process_child_ids, child);
    if !output.status.success() {
        return Err(eyre!(
            "Error launching app on physical iOS device. Aborting."
        ));
    }

    println!("{} 🚀 App launched on device {}.", *PAX_BADGE, device_name);
    Ok(())
}

enum MatchMode {
    IdentifierExact,
    NameExact,
    NameContains,
}

impl MatchMode {
    fn matches(&self, candidate: &ResolvedIosDevice, query: &str) -> bool {
        let identifier = candidate.identifier.to_ascii_lowercase();
        let name = candidate.name.to_ascii_lowercase();
        match self {
            MatchMode::IdentifierExact => identifier == query,
            MatchMode::NameExact => name == query,
            MatchMode::NameContains => name.contains(query),
        }
    }
}

fn resolve_dylib_file_name(project_path: &PathBuf) -> Result<String, eyre::Report> {
    let manifest_path = project_path.join("Cargo.toml");
    let metadata = MetadataCommand::new()
        .manifest_path(&manifest_path)
        .no_deps()
        .exec()
        .map_err(|err| {
            eyre!(
                "Failed to read cargo metadata for {:?}: {}",
                manifest_path,
                err
            )
        })?;

    let root_package = metadata.root_package().ok_or_else(|| {
        eyre!(
            "Failed to determine root package for Apple build at {:?}",
            manifest_path
        )
    })?;

    let dylib_target = root_package
        .targets
        .iter()
        .find(|target| {
            target.kind.iter().any(|kind| kind == "cdylib")
                || target.crate_types.iter().any(|kind| kind == "cdylib")
        })
        .ok_or_else(|| {
            eyre!(
                "No cdylib target found for Apple build in {:?}",
                manifest_path
            )
        })?;

    Ok(format!("lib{}.dylib", dylib_target.name.replace('-', "_")))
}

fn parse_iphone_simulator_preference(name: &str) -> Option<(i32, SimulatorVariantRank)> {
    let rest = name.strip_prefix("iPhone ")?;
    let generation_end = rest.find(|ch: char| !ch.is_ascii_digit())?;
    let generation = rest[..generation_end].parse::<i32>().ok()?;
    let variant = rest[generation_end..].trim();
    let rank = match variant {
        "Pro" => SimulatorVariantRank::Pro,
        "Pro Max" => SimulatorVariantRank::ProMax,
        "" => SimulatorVariantRank::Base,
        _ => SimulatorVariantRank::Other,
    };
    Some((generation, rank))
}

fn normalize_apple_package_paths(xcodeproj_path: &PathBuf) -> Result<(), eyre::Report> {
    let project_file = xcodeproj_path.join("project.pbxproj");
    let contents = fs::read_to_string(&project_file)?;
    let normalized = contents
        .replace(
            "../../interface/common/pax-swift-cartridge",
            "../../common/pax-swift-cartridge",
        )
        .replace(
            "../../interface/common/pax-swift-common",
            "../../common/pax-swift-common",
        );

    if normalized != contents {
        fs::write(project_file, normalized)?;
    }

    Ok(())
}

fn normalize_macos_framework_layout(framework_dir: &PathBuf) -> Result<(), eyre::Report> {
    let versions_dir = framework_dir.join("Versions");
    let current_link = versions_dir.join("Current");
    if current_link.exists() {
        return Ok(());
    }

    let version_dir = versions_dir.join("A");
    let resources_dir = version_dir.join("Resources");
    fs::create_dir_all(&resources_dir)?;

    move_if_exists(&framework_dir.join("Headers"), &version_dir.join("Headers"))?;
    move_if_exists(&framework_dir.join("Modules"), &version_dir.join("Modules"))?;
    move_if_exists(
        &framework_dir.join("Info.plist"),
        &resources_dir.join("Info.plist"),
    )?;
    move_if_exists(
        &framework_dir.join("PaxCartridge"),
        &version_dir.join("PaxCartridge"),
    )?;

    create_symlink(&PathBuf::from("A"), &current_link)?;
    create_symlink(
        &PathBuf::from("Versions/Current/Headers"),
        &framework_dir.join("Headers"),
    )?;
    create_symlink(
        &PathBuf::from("Versions/Current/Modules"),
        &framework_dir.join("Modules"),
    )?;
    create_symlink(
        &PathBuf::from("Versions/Current/Resources"),
        &framework_dir.join("Resources"),
    )?;
    create_symlink(
        &PathBuf::from("Versions/Current/PaxCartridge"),
        &framework_dir.join("PaxCartridge"),
    )?;

    Ok(())
}

fn move_if_exists(src: &PathBuf, dest: &PathBuf) -> Result<(), eyre::Report> {
    if !src.exists() {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    if dest.exists() {
        if dest.is_dir() {
            fs::remove_dir_all(dest)?;
        } else {
            fs::remove_file(dest)?;
        }
    }
    fs::rename(src, dest)?;
    Ok(())
}

fn create_symlink(target: &PathBuf, link: &PathBuf) -> Result<(), eyre::Report> {
    if link.exists() || link.symlink_metadata().is_ok() {
        if link.is_dir() && !link.is_symlink() {
            fs::remove_dir_all(link)?;
        } else {
            fs::remove_file(link)?;
        }
    }
    #[cfg(unix)]
    unix_fs::symlink(target, link)?;
    Ok(())
}

fn prepare_dev_session(
    project_root: &PathBuf,
    pax_dir: &PathBuf,
) -> Result<DevSession, eyre::Report> {
    let started_at_ms = now_ms();
    let session_id = format!("macos-{started_at_ms}-{}", std::process::id());
    let session_dir = project_dev_dir(pax_dir).join("sessions").join(&session_id);
    fs::create_dir_all(session_dir.join("requests"))?;
    fs::create_dir_all(session_dir.join("responses"))?;
    fs::create_dir_all(session_dir.join("captures"))?;

    Ok(DevSession {
        session_id,
        platform: "macos".to_string(),
        designtime: true,
        project_root: Some(fs::canonicalize(project_root).unwrap_or_else(|_| project_root.clone())),
        session_dir: Some(session_dir),
        app_pid: None,
        design_server_addr: None,
        control_kind: "filesystem".to_string(),
        location: Some("local-window".to_string()),
        started_at_ms,
        last_seen_ms: started_at_ms,
    })
}

fn finalize_dev_session(
    pax_dir: &PathBuf,
    session: &mut DevSession,
    app_pid: u32,
) -> Result<(), eyre::Report> {
    session.app_pid = Some(app_pid);
    session.location = Some(format!("local-window pid:{app_pid}"));
    session.last_seen_ms = now_ms();
    write_project_active_session(pax_dir, session)?;
    write_registered_session(session)?;
    Ok(())
}

fn cleanup_dev_session(pax_dir: &PathBuf, session: &DevSession) -> Result<(), eyre::Report> {
    dev_session::remove_project_active_session(pax_dir, &session.session_id)?;
    dev_session::remove_registered_session(&session.session_id)?;
    Ok(())
}

fn spawn_designtime_server_process(
    pax_dir: &PathBuf,
    project_root: &PathBuf,
    manifest: &PaxManifest,
    ready_file: &PathBuf,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
) -> Result<Child, eyre::Report> {
    let manifest_path = project_designtime_manifest_file(pax_dir);
    fs::create_dir_all(manifest_path.parent().unwrap())?;
    fs::write(&manifest_path, serde_json::to_vec(manifest)?)?;

    let current_exe = std::env::current_exe()?;
    let mut cmd = Command::new(current_exe);
    cmd.arg("designtime-server")
        .arg("--serve-dir")
        .arg(pax_dir)
        .arg("--watch-dir")
        .arg(project_root)
        .arg("--manifest-path")
        .arg(manifest_path)
        .arg("--port")
        .arg("0")
        .arg("--ready-file")
        .arg(ready_file)
        .arg("--suppress-address-log")
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(crate::pre_exec_hook);
    }

    let child = cmd.spawn().expect(ERR_SPAWN);
    process_child_ids.lock().unwrap().push(child.id() as u64);
    Ok(child)
}

fn wait_for_designtime_ready(
    ready_file: &PathBuf,
    timeout: Duration,
) -> Result<String, eyre::Report> {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if let Ok(contents) = fs::read_to_string(ready_file) {
            let trimmed = contents.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err(eyre!(
        "timed out waiting for the local designtime server to become ready"
    ))
}
// This function checks if the simulator with the given UDID is booted
fn is_simulator_booted(device_udid: &str, process_child_ids: &Arc<Mutex<Vec<u64>>>) -> bool {
    let mut cmd = Command::new("xcrun");
    cmd.arg("simctl")
        .arg("list")
        .arg("devices")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(crate::pre_exec_hook);
    }
    let child = cmd.spawn().expect(ERR_SPAWN);
    let output = wait_with_output(&process_child_ids, child);
    if !output.status.success() {
        panic!("Error checking simulator status. This is an unhandled error and may leave orphaned processes.");
    }

    let output_str = String::from_utf8(output.stdout).expect("Failed to convert to string");
    output_str
        .lines()
        .any(|line| line.contains(device_udid) && line.contains("Booted"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved_ios_device(kind: IosDeviceKind) -> ResolvedIosDevice {
        ResolvedIosDevice {
            kind,
            name: "Test Device".to_string(),
            identifier: "TEST-DEVICE".to_string(),
        }
    }

    fn rust_targets(target_mappings: &[AppleTargetMapping]) -> Vec<&'static str> {
        target_mappings
            .iter()
            .map(|target_mapping| target_mapping.rust_target)
            .collect()
    }

    #[test]
    fn ios_debug_without_destination_builds_host_simulator_arch() {
        let target_mappings = select_apple_target_mappings(&RunTarget::iOS, false, None, "aarch64");

        assert_eq!(
            rust_targets(&target_mappings),
            vec!["aarch64-apple-ios-sim"]
        );
        assert_eq!(xcode_archs_for_target_mappings(&target_mappings), "arm64");
    }

    #[test]
    fn ios_debug_for_physical_device_builds_device_arch_only() {
        let device = resolved_ios_device(IosDeviceKind::Physical);
        let target_mappings =
            select_apple_target_mappings(&RunTarget::iOS, false, Some(&device), "aarch64");

        assert_eq!(rust_targets(&target_mappings), vec!["aarch64-apple-ios"]);
        assert_eq!(xcode_archs_for_target_mappings(&target_mappings), "arm64");
    }

    #[test]
    fn ios_release_without_destination_builds_device_arch_only() {
        let target_mappings = select_apple_target_mappings(&RunTarget::iOS, true, None, "aarch64");

        assert_eq!(rust_targets(&target_mappings), vec!["aarch64-apple-ios"]);
        assert_eq!(xcode_archs_for_target_mappings(&target_mappings), "arm64");
    }

    #[test]
    fn ios_simulator_on_unknown_host_keeps_multiarch_fallback() {
        let target_mappings = select_apple_target_mappings(&RunTarget::iOS, false, None, "mystery");

        assert_eq!(
            rust_targets(&target_mappings),
            vec!["aarch64-apple-ios-sim", "x86_64-apple-ios"]
        );
        assert_eq!(
            xcode_archs_for_target_mappings(&target_mappings),
            "arm64 x86_64"
        );
    }

    #[test]
    fn macos_release_keeps_multiarch_build() {
        let target_mappings =
            select_apple_target_mappings(&RunTarget::macOS, true, None, "aarch64");

        assert_eq!(
            rust_targets(&target_mappings),
            vec!["aarch64-apple-darwin", "x86_64-apple-darwin"]
        );
    }
}
