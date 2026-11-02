//! Native SDK assembly for build.rs; never executed by initialization.
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
static MACOS: include_dir::Dir<'_> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/native/macos");
static COMMON: include_dir::Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/native/common");
#[path = "desktop_sources.rs"]
mod desktop_sources;
const CARTRIDGE_PACKAGE: &str = "pax-swift-cartridge/Package.swift";
fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<()> {
    if fs::read(path).ok().as_deref() != Some(bytes) {
        fs::write(path, bytes)?;
    }
    Ok(())
}
fn extract_sources(source: &include_dir::Dir<'_>, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination.join(source.path()))?;
    for file in source.files() {
        if file.path() != Path::new(CARTRIDGE_PACKAGE) {
            write_if_changed(&destination.join(file.path()), file.contents())?;
        }
    }
    for directory in source.dirs() {
        extract_sources(directory, destination)?;
    }
    Ok(())
}
pub fn build(out: &Path, architecture: &str, debug: bool) -> Result<PathBuf> {
    let native = out.join("interface");
    fs::create_dir_all(&native)?;
    // CLI packaging and ordinary Cargo may alternate the framework override.
    // Preserve unchanged source mtimes so Xcode can reuse its generic SDK build.
    extract_sources(&MACOS, &native.join("macos"))?;
    extract_sources(&COMMON, &native.join("common"))?;
    // The generic host imports ABI types but does not link an application. The
    // CLI's copy retains its binary target for signed application packaging.
    let cartridge = native.join("common/pax-swift-cartridge");
    let package = cartridge.join("Package.swift");
    let source = COMMON
        .get_file(CARTRIDGE_PACKAGE)
        .unwrap()
        .contents_utf8()
        .unwrap();
    write_if_changed(&package, source.replace(
        ".binaryTarget(\n            name: \"PaxCartridge\",\n            path: \"PaxCartridge.xcframework\"\n        )",
        ".systemLibrary(name: \"PaxCartridge\")"
    ).replace("targets: [\"PaxCartridge\", \"PaxCartridgeAssets\"]", "targets: [\"PaxCartridge\"]").as_bytes())?;
    let headers = cartridge.join("Sources/PaxCartridge");
    fs::create_dir_all(&headers)?;
    for name in ["PaxCartridge.h", "PaxProgramExports.h"] {
        write_if_changed(&headers.join(name), &fs::read(cartridge.join(
            "PaxCartridge.xcframework/macos-arm64_x86_64/PaxCartridge.framework/Headers"
        ).join(name))?)?;
    }
    write_if_changed(
        &headers.join("module.modulemap"),
        b"module PaxCartridge { header \"PaxCartridge.h\" export * }\n",
    )?;
    let products = out.join("products");
    let log = out.join("xcodebuild.log");
    let file = fs::File::create(&log)?;
    let architecture = match architecture {
        "aarch64" => "arm64",
        "x86_64" => "x86_64",
        other => return Err(format!("unsupported macOS architecture {other}").into()),
    };
    let configuration = if debug { "Debug" } else { "Release" };
    let status = Command::new("xcodebuild")
        .arg("-project")
        .arg(native.join("macos/pax-app-macos/pax-app-macos.xcodeproj"))
        .args([
            "-scheme",
            "PaxHost",
            "-configuration",
            configuration,
            "-sdk",
            "macosx",
            "-destination",
            "generic/platform=macOS",
        ])
        .arg("-derivedDataPath")
        .arg(out.join("derived"))
        .arg(format!("CONFIGURATION_BUILD_DIR={}", products.display()))
        .arg(format!("ARCHS={architecture}"))
        .args(["ONLY_ACTIVE_ARCH=YES", "CODE_SIGNING_ALLOWED=NO", "-quiet"])
        .stdout(file.try_clone()?)
        .stderr(file)
        .status()
        .map_err(|error| {
            format!("Building the macOS host requires Xcode and xcodebuild: {error}")
        })?;
    if !status.success() {
        return Err(format!("Pax native SDK build failed; see {}", log.display()).into());
    }
    Ok(products.join("PaxHost.framework/PaxHost"))
}

/// Builds the native desktop host without linking any application cartridge.
pub fn build_desktop(out: &Path, target: &str, architecture: &str, debug: bool) -> Result<PathBuf> {
    let native = out.join("interface");
    desktop_sources::extract(target, &native.join(target))?;
    let products = out.join("products");
    let configuration = if debug { "Debug" } else { "Release" };
    let mut command = if target == "windows" {
        let mut command = Command::new("powershell.exe");
        command
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(native.join("windows/build.ps1"))
            .arg("-Architecture")
            .arg(match architecture {
                "aarch64" => "ARM64",
                "x86_64" => "x64",
                other => return Err(format!("unsupported Windows architecture {other}").into()),
            })
            .args(["-Configuration", configuration, "-OutputDirectory"])
            .arg(&products);
        command
    } else if target == "linux" {
        let mut command = Command::new("bash");
        command
            .arg(native.join("linux/build.sh"))
            .args(["--configuration", configuration, "--output-directory"])
            .arg(&products);
        command
    } else {
        return Err(format!("unsupported desktop target {target}").into());
    };
    let log = out.join("native-host-build.log");
    let file = fs::File::create(&log)?;
    let status = command.stdout(file.try_clone()?).stderr(file).status()?;
    if !status.success() {
        return Err(format!("Pax {target} host build failed; see {}", log.display()).into());
    }
    let library = products.join(if target == "windows" {
        "PaxHost.dll"
    } else {
        "libpax_host.so"
    });
    if !library.is_file() {
        return Err(format!("Native host build did not produce {}", library.display()).into());
    }
    Ok(library)
}
