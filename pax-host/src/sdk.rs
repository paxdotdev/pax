//! Generic Apple SDK assembly for build.rs; never executed by initialization.
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
static NATIVE: include_dir::Dir<'_> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/native");
const CARTRIDGE_PACKAGE: &str = "common/pax-swift-cartridge/Package.swift";
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
    extract_sources(&NATIVE, &native)?;
    // The generic host imports ABI types but does not link an application. The
    // CLI's copy retains its binary target for signed application packaging.
    let cartridge = native.join("common/pax-swift-cartridge");
    let package = cartridge.join("Package.swift");
    let source = NATIVE
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
    write_if_changed(&headers.join("PaxCartridge.h"), &fs::read(cartridge.join("PaxCartridge.xcframework/macos-arm64_x86_64/PaxCartridge.framework/Headers/PaxCartridge.h"))?)?;
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
