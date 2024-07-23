use colored::Colorize;
use serde_json::Value;

use crate::helpers::{BUILD_DIR_NAME, DIR_IGNORE_LIST_MACOS, ERR_SPAWN, PAX_BADGE, PKG_DIR_NAME};
use crate::{copy_dir_recursively, wait_with_output, RunContext, RunTarget};

use color_eyre::eyre;
use eyre::eyre;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use std::thread;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

const RUST_IOS_DYLIB_FILE_NAME: &str = "libpaxchassisios.dylib";
const RUST_MACOS_DYLIB_FILE_NAME: &str = "libpaxchassismacos.dylib";
const PORTABLE_DYLIB_INSTALL_NAME: &str = "@rpath/PaxCartridge.framework/PaxCartridge";

const XCODE_MACOS_TARGET_DEBUG: &str = "Pax macOS (Development)";
const XCODE_MACOS_TARGET_RELEASE: &str = "Pax macOS (Release)";
const XCODE_IOS_TARGET_DEBUG: &str = "Pax iOS (Development)";
const XCODE_IOS_TARGET_RELEASE: &str = "Pax iOS (Release)";

// These package IDs represent the directory / package names inside the xcframework,
const MACOS_MULTIARCH_PACKAGE_ID: &str = "macos-arm64_x86_64";
const IOS_SIMULATOR_MULTIARCH_PACKAGE_ID: &str = "ios-arm64_x86_64-simulator";
const IOS_PACKAGE_ID: &str = "ios-arm64";

pub fn build_apple_chassis_with_cartridge(
    ctx: &RunContext,
    pax_dir: &PathBuf,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
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

    let build_mode_name: &str = if is_release { "release" } else { "debug" };

    //0: Rust arch string, for passing to cargo
    //1: Apple arch string, for addressing xcframework
    let target_mappings: &[(&str, &str)] = if let RunTarget::macOS = target {
        if is_release {
            &[
                ("aarch64-apple-darwin", "macos-arm64"),
                ("x86_64-apple-darwin", "macos-x86_64"),
            ]
        } else {
            // Build only relevant archs for dev
            if std::env::consts::ARCH == "x86_64" {
                &[("x86_64-apple-darwin", "macos-x86_64")]
            } else {
                &[("aarch64-apple-darwin", "macos-arm64")]
            }
        }
    } else {
        // Build all archs for iOS builds.  We could limit these like we do for macOS
        // dev builds, but at time of initial authoring, it was slowing zb down.
        &[
            ("aarch64-apple-ios", "ios-arm64"),
            ("x86_64-apple-ios", "iossimulator-x86_64"),
            ("aarch64-apple-ios-sim", "iossimulator-arm64"),
        ]
    };

    let dylib_file_name = if let RunTarget::macOS = target {
        RUST_MACOS_DYLIB_FILE_NAME
    } else {
        RUST_IOS_DYLIB_FILE_NAME
    };

    let mut handles = Vec::new();

    //(arch id, single-platform .dylib path, stdout/stderr from build)
    let build_results: Arc<Mutex<HashMap<u32, (String, String, Output)>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let targets_single_string = target_mappings
        .iter()
        .map(|tm| tm.1.to_string())
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

    let mut index = 0;
    for target_mapping in target_mappings {
        let project_path = project_path.clone();
        let pax_dir = pax_dir.clone();

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
                .arg(target_mapping.0)
                .arg(arg_features)
                .env("PAX_DIR", &pax_dir)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

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
                .join(target_mapping.0)
                .join(build_mode_name)
                .join(dylib_file_name);

            let new_val = (
                target_mapping.1.to_string(),
                dylib_src.to_str().unwrap().to_string(),
                output,
            );
            build_results_threadsafe
                .lock()
                .unwrap()
                .insert(index, new_val);
        });
        index = index + 1;
        handles.push(handle);
    }

    let mut index = 0;
    // Wait for all threads to complete and print their outputs
    for handle in handles {
        handle.join().unwrap();
    }

    let results = build_results.lock().unwrap();

    let mut should_abort = false;
    //Print stdout/stderr
    for i in 0..target_mappings.len() {
        let result = results.get(&(i as u32)).unwrap();
        let target = &result.0;
        let output = &result.2;

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

        index = index + 1;
    }

    if should_abort {
        return Err(eyre!(
            "Failed to build one or more targets with Cargo. Aborting."
        ));
    }

    // Update the `install name` of each Rust-built .dylib, instead of the default-output absolute file paths
    // embedded in each .dylib.  This allows our .dylibs to be portably embedded into an SPM module.
    let result = results.iter().try_for_each(|res: (&u32, &(String, String, Output))| {
        let dylib_path = &res.1.1;
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

    let macos_dylib_dest = pax_dir
        .join(PKG_DIR_NAME)
        .join("pax-chassis-common")
        .join("pax-swift-cartridge")
        .join("PaxCartridge.xcframework")
        .join(MACOS_MULTIARCH_PACKAGE_ID)
        .join("PaxCartridge.framework")
        .join("PaxCartridge");

    let simulator_dylib_dest = pax_dir
        .join(PKG_DIR_NAME)
        .join("pax-chassis-common")
        .join("pax-swift-cartridge")
        .join("PaxCartridge.xcframework")
        .join(IOS_SIMULATOR_MULTIARCH_PACKAGE_ID)
        .join("PaxCartridge.framework")
        .join("PaxCartridge");

    let iphone_native_dylib_dest = pax_dir
        .join(PKG_DIR_NAME)
        .join("pax-chassis-common")
        .join("pax-swift-cartridge")
        .join("PaxCartridge.xcframework")
        .join(IOS_PACKAGE_ID)
        .join("PaxCartridge.framework")
        .join("PaxCartridge");

    if is_release || is_ios {
        // Merge architecture-specific binaries with `lipo` (this is an undocumented requirement
        // of multi-arch builds + xcframeworks for the Apple toolchain; we cannot bundle two
        // macos arch .frameworks in an xcframework; they must lipo'd into a single .framework + dylib.
        // Similarly, iOS binaries require a particular bundling for simulator & device builds.)
        println!(
            "{} 🖇️  Combining architecture-specific binaries with `lipo`...",
            *PAX_BADGE
        );

        if let RunTarget::macOS = target {
            // For macOS, we want to lipo both our arm64 and x86_64 dylibs into a single binary,
            // then bundle that single binary into a single framework within the xcframework.

            let lipo_input_paths = results
                .iter()
                .map(|res| res.1 .1.clone())
                .collect::<Vec<String>>();

            // Construct the lipo command
            let mut lipo_command = Command::new("lipo");
            lipo_command
                .arg("-create")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            // Add each input path to the command
            for path in &lipo_input_paths {
                lipo_command.arg(path);
            }

            // Specify the output path
            lipo_command.arg("-output").arg(macos_dylib_dest);

            #[cfg(unix)]
            unsafe {
                lipo_command.pre_exec(crate::pre_exec_hook);
            }
            let child = lipo_command.spawn().expect(ERR_SPAWN);
            let output = wait_with_output(&process_child_ids, child);

            if !output.status.success() {
                return Err(eyre!("Failed to combine packages with lipo. Aborting."));
            }
        } else {
            // For iOS, we want to:
            // 1. lipo together both simulator build architectures
            // 2. copy (a) the lipo'd simulator binary, and (b) the vanilla arm64 iOS binary into the framework
            let simulator_builds = results
                .iter()
                .filter(|res| res.1 .0.starts_with("iossimulator-"))
                .collect::<Vec<_>>();
            let device_build = results
                .iter()
                .filter(|res| res.1 .0.starts_with("ios-"))
                .collect::<Vec<_>>();

            let lipo_input_paths = simulator_builds
                .iter()
                .map(|res| res.1 .1.clone())
                .collect::<Vec<String>>();

            // Construct the lipo command
            let mut lipo_command = Command::new("lipo");
            lipo_command
                .arg("-create")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            // Add each input path to the command
            for path in &lipo_input_paths {
                lipo_command.arg(path);
            }

            // Specify the output path
            lipo_command.arg("-output").arg(simulator_dylib_dest);

            #[cfg(unix)]
            unsafe {
                lipo_command.pre_exec(crate::pre_exec_hook);
            }
            let child = lipo_command.spawn().expect(ERR_SPAWN);
            let output = wait_with_output(&process_child_ids, child);
            if !output.status.success() {
                return Err(eyre!("Failed to combine dylibs with lipo. Aborting."));
            }

            //Copy singular device build (iOS, not simulator)
            let device_dylib_src = &device_build[0].1 .1;

            let _ = fs::copy(device_dylib_src, iphone_native_dylib_dest);
        }
    } else {
        // For macos development builds, instead of lipoing, just drop the singular build into the appropriate output destination
        // This measure speeds up development builds substantially.
        // Note that we could do something similar for iOS, but it wasn't immediately in reach at time of authoring (build failed when
        // providing non-lipo'd binaries in the framework for iOS)
        let result = results.iter().next().unwrap();
        let src = &result.1 .1;
        let dest = macos_dylib_dest;
        let _ = fs::copy(src, dest);
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
                .join(PKG_DIR_NAME)
                .join("pax-chassis-macos")
                .join("interface")
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
                .join(PKG_DIR_NAME)
                .join("pax-chassis-ios")
                .join("interface")
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

    let build_dest_base = pax_dir
        .join(BUILD_DIR_NAME)
        .join(build_mode_name)
        .join(target_str_lower);
    let executable_output_dir_path = build_dest_base.join("app");
    let executable_dot_app_path = executable_output_dir_path.join(&format!("{}.app", &scheme));
    let _ = fs::create_dir_all(&executable_output_dir_path);

    let sdk = if let RunTarget::iOS = target {
        if is_release {
            "iphoneos"
        } else {
            "iphonesimulator"
        }
    } else {
        "macosx"
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
        .arg(&format!(
            "CONFIGURATION_BUILD_DIR={}",
            executable_output_dir_path.to_str().unwrap()
        ))
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::piped());

    if !is_release {
        cmd.arg("CODE_SIGNING_REQUIRED=NO")
            .arg("CODE_SIGN_IDENTITY=");
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
        return Err(eyre!("Failed to build project with xcodebuild. Aborting."));
    }

    //Copy build artifacts & packages into `build`
    let swift_cart_src = pax_dir
        .join(PKG_DIR_NAME)
        .join("pax-chassis-common")
        .join("pax-swift-cartridge");
    let swift_common_src = pax_dir
        .join(PKG_DIR_NAME)
        .join("pax-chassis-common")
        .join("pax-swift-common");

    let swift_cart_build_dest = build_dest_base
        .join("pax-chassis-common")
        .join("pax-swift-cartridge");
    let swift_common_build_dest = build_dest_base
        .join("pax-chassis-common")
        .join("pax-swift-common");

    let (app_xcodeproj_src, app_xcodeproj_build_dest) = if let RunTarget::macOS = target {
        (
            pax_dir
                .join(PKG_DIR_NAME)
                .join("pax-chassis-macos")
                .join("interface")
                .join("pax-app-macos"),
            build_dest_base
                .join("pax-chassis-macos")
                .join("interface")
                .join("pax-app-macos"),
        )
    } else {
        (
            pax_dir
                .join(PKG_DIR_NAME)
                .join("pax-chassis-ios")
                .join("interface")
                .join("pax-app-ios"),
            build_dest_base
                .join("pax-chassis-ios")
                .join("interface")
                .join("pax-app-ios"),
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
        println!("{} 🐇 Running Pax {}...", *PAX_BADGE, target_str);

        if let RunTarget::macOS = target {
            //
            // Handle macOS `run`
            //

            let system_binary_path =
                executable_dot_app_path.join(&format!("Contents/MacOS/{}", scheme));

            let status = Command::new(system_binary_path)
                .status() // This will wait for the process to complete
                .expect("failed to execute the app");

            println!("App exited with: {:?}", status);
        } else {
            //
            // Handle iOS `run`
            //

            // Get list of devices
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
            let output = wait_with_output(&process_child_ids, child);
            let output_str = std::str::from_utf8(&output.stdout)
                .map_err(|_| eyre!("Failed to parse stdout for xcrun"))?;
            let parsed: Value = serde_json::from_str(&output_str)
                .map_err(|_| eyre!("Failed to deserialize xcrun."))?;

            // Extract devices
            let devices = parsed["devices"].as_object().ok_or_else(|| {
                return eyre!("Invalid JSON format for devices.");
            })?;

            let mut max_iphone_number = 0;
            let mut desired_udid = None;

            for (_, device_list) in devices {
                if let Some(device_array) = device_list.as_array() {
                    for device in device_array {
                        if let Some(device_type) = device["deviceTypeIdentifier"].as_str() {
                            if device_type
                                .starts_with("com.apple.CoreSimulator.SimDeviceType.iPhone-")
                            {
                                if let Some(number) = device_type.split('-').last() {
                                    if let Ok(number) = number.parse::<i32>() {
                                        if number > max_iphone_number {
                                            max_iphone_number = number;
                                            desired_udid = device["udid"].as_str();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let device_udid = match desired_udid {
                Some(udid) => udid,
                None => {
                    return Err(eyre!("No installed iOS simulators found on this system. Install at least one iPhone simulator through xcode and try again."));
                }
            };

            // Open the Simulator app
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
            let output = wait_with_output(&process_child_ids, child);
            if !output.status.success() {
                return Err(eyre!("Error opening iOS simulator. Aborting."));
            }

            // Boot current device
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
            let _output = wait_with_output(&process_child_ids, child);

            // Boot the relevant simulator
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
            let output = wait_with_output(&process_child_ids, child);
            if !output.status.success() {
                return Err(eyre!("Error spawning iOS simulator. Aborting."));
            }
            // ^ Note that we don't handle errors on this particular command; it will return an error by default
            // if the simulator isn't running, which isn't an "error" for us.  Instead, defer to the following
            // polling logic to decide whether the simulator failed to start, which would indeed be an error.

            // After opening the simulator, wait for the simulator to be booted
            let max_retries = 5;
            let retry_period_secs = 5;
            let mut retries = 0;

            while !is_simulator_booted(device_udid, &process_child_ids) && retries < max_retries {
                println!("{} 💤 Waiting for simulator to boot...", *PAX_BADGE);
                std::thread::sleep(std::time::Duration::from_secs(retry_period_secs));
                retries = retries + 1;
            }

            if retries == max_retries {
                return Err(eyre!(
                    "Failed to boot the simulator within the expected time. Aborting."
                ));
            }

            // Install and run app on simulator
            println!(
                "{} 📤 Installing and running app from {} on simulator...",
                *PAX_BADGE,
                executable_output_dir_path.to_str().unwrap()
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
            let output = wait_with_output(&process_child_ids, child);
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
            let output = wait_with_output(&process_child_ids, child);
            if !output.status.success() {
                return Err(eyre!("Error launching app on iOS simulator. Aborting."));
            }
            let status = output.status.code().unwrap();

            println!(
                "{} 🚀 App launched on simulator. Launch command exited with code: {:?}",
                *PAX_BADGE, status
            );
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
