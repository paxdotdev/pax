//! Canonical native SDK sources, shared with Cargo and the packaging frontend.
use include_dir::{include_dir, Dir};
pub static PAX_MACOS_INTERFACE_TEMPLATE: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/native/macos");
pub static PAX_IOS_INTERFACE_TEMPLATE: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/native/ios");
pub static PAX_SWIFT_CARTRIDGE_TEMPLATE: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/native/common/pax-swift-cartridge");
pub static PAX_SWIFT_COMMON_TEMPLATE: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/native/common/pax-swift-common");

pub const SOURCE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/native");

#[path = "desktop_sources.rs"]
mod desktop_sources;
/// Extracts a desktop SDK and its common cartridge ABI header for packaging.
pub fn extract_desktop(target: &str, destination: &std::path::Path) -> std::io::Result<()> {
    desktop_sources::extract(target, destination)
}

pub const DESKTOP_HEADER: &[u8] = desktop_sources::HEADER;

/// Extracts the authored Android host and its common cartridge header.
pub fn extract_android(destination: &std::path::Path) -> std::io::Result<()> {
    let files: &[(&str, &[u8])] = &[
        ("host.c", include_bytes!("../native/android/host.c")),
        ("build.py", include_bytes!("../native/android/build.py")),
        (
            "java/dev/pax/FrameStats.java",
            include_bytes!("../native/android/java/dev/pax/FrameStats.java"),
        ),
        (
            "java/dev/pax/AssetPaths.java",
            include_bytes!("../native/android/java/dev/pax/AssetPaths.java"),
        ),
        (
            "java/dev/pax/PaxAssets.java",
            include_bytes!("../native/android/java/dev/pax/PaxAssets.java"),
        ),
        (
            "java/dev/pax/SystemFonts.java",
            include_bytes!("../native/android/java/dev/pax/SystemFonts.java"),
        ),
        (
            "java/dev/pax/Affine.java",
            include_bytes!("../native/android/java/dev/pax/Affine.java"),
        ),
        (
            "java/dev/pax/PaxActivity.java",
            include_bytes!("../native/android/java/dev/pax/PaxActivity.java"),
        ),
        (
            "java/dev/pax/PaxScene.java",
            include_bytes!("../native/android/java/dev/pax/PaxScene.java"),
        ),
        (
            "java/dev/pax/RichText.java",
            include_bytes!("../native/android/java/dev/pax/RichText.java"),
        ),
        (
            "java/dev/pax/PendingUpdates.java",
            include_bytes!("../native/android/java/dev/pax/PendingUpdates.java"),
        ),
        (
            "java/dev/pax/NativePath.java",
            include_bytes!("../native/android/java/dev/pax/NativePath.java"),
        ),
        (
            "java/dev/pax/NativeRoutes.java",
            include_bytes!("../native/android/java/dev/pax/NativeRoutes.java"),
        ),
    ];
    for (name, bytes) in files {
        let path = destination.join(name);
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(path, bytes)?;
    }
    let common = destination
        .parent()
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Android SDK needs a parent directory",
            )
        })?
        .join("common");
    std::fs::create_dir_all(&common)?;
    std::fs::write(common.join("pax-desktop.h"), DESKTOP_HEADER)
}
