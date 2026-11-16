//! Keep SDK restore caches and build products out of embedded Rust artifacts.
//! Add authored build inputs here when a desktop host gains another source file.
use std::{fs, path::Path};

macro_rules! source {
    ($platform:literal, $file:literal) => {
        (
            $file,
            include_bytes!(concat!("../native/", $platform, "/", $file)) as &[u8],
        )
    };
}
const WINDOWS: &[(&str, &[u8])] = &[
    source!("windows", "build.ps1"),
    source!("windows", "PaxHost.vcxproj"),
    source!("windows", "packages.config"),
    source!("windows", "host.cpp"),
    source!("windows", "assets.h"),
    source!("windows", "cartridge.h"),
    source!("windows", "composition_presenter.h"),
    source!("windows", "json_helpers.h"),
    source!("windows", "keyboard.h"),
    source!("windows", "native_scene.h"),
    source!("windows", "native_images.h"),
    source!("windows", "navigation.h"),
    source!("windows", "snap_canvas.h"),
    source!("windows", "wheel_input.h"),
    source!("windows", "native_mask.h"),
    source!("windows", "mask_path.h"),
];
const LINUX: &[(&str, &[u8])] = &[
    source!("linux", "build.sh"),
    source!("linux", "host.c"),
    source!("linux", "assets.c"),
    source!("linux", "assets.h"),
    source!("linux", "gpu-import.c"),
    source!("linux", "gpu-import.h"),
    source!("linux", "native-scene.c"),
    source!("linux", "native-scene.h"),
    source!("linux", "navigation.h"),
];
pub const HEADER: &[u8] = include_bytes!("../native/common/pax-desktop.h");

pub fn extract(target: &str, destination: &Path) -> std::io::Result<()> {
    let sources = match target {
        "windows" => WINDOWS,
        "linux" => LINUX,
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "unknown desktop host",
            ))
        }
    };
    fs::create_dir_all(destination)?;
    for (name, bytes) in sources {
        let path = destination.join(name);
        if fs::read(&path).ok().as_deref() != Some(*bytes) {
            fs::write(path, bytes)?;
        }
    }
    let common = destination
        .parent()
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "desktop SDK needs a parent directory",
            )
        })?
        .join("common");
    fs::create_dir_all(&common)?;
    let header = common.join("pax-desktop.h");
    if fs::read(&header).ok().as_deref() != Some(HEADER) {
        fs::write(header, HEADER)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extracted_hosts_include_build_inputs_without_native_dependency_caches() {
        let root = tempfile::tempdir().unwrap();
        for (platform, entry) in [("windows", "PaxHost.vcxproj"), ("linux", "build.sh")] {
            let output = root.path().join(platform);
            extract(platform, &output).unwrap();
            assert!(output.join(entry).is_file());
            assert!(root.path().join("common/pax-desktop.h").is_file());
            assert!(!output.join("native-packages").exists());
            assert!(!output.join("out").exists());
            assert!(!output.join("tools").exists());
            for entry in fs::read_dir(&output).unwrap() {
                let path = entry.unwrap().path();
                let source = fs::read_to_string(&path).unwrap();
                for line in source.lines() {
                    if let Some(include) = line.trim().strip_prefix("#include \"") {
                        let include = include.split('"').next().unwrap();
                        assert!(
                            output.join(include).is_file(),
                            "{} requires missing embedded input {include}",
                            path.display()
                        );
                    }
                }
            }
        }
    }
}
