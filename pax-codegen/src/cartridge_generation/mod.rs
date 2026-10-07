//! Cartridge rendering shared by in-memory and file-based build frontends.

use std::fs;
use std::io;

use pax_manifest::{cartridge_generation::CommonProperty, PaxManifest};

use std::path::{Path, PathBuf};

pub mod templating;

pub const CARTRIDGE_PARTIAL_PATH: &str = "cartridge.partial.rs";

/// Write the generated cartridge to the caller's prepared output directory.
/// The CLI includes this file from `runnable!` or the legacy `#[main]` shorthand.
pub fn generate_cartridge_partial_rs(
    pax_dir: &PathBuf,
    merged_manifest: &PaxManifest,
    userland_manifest: &PaxManifest,
    is_designtime: bool,
    use_rust_manifest: bool,
) -> PathBuf {
    let generated = render_cartridge(
        merged_manifest,
        userland_manifest,
        is_designtime,
        use_rust_manifest,
    );
    let path = pax_dir.join(CARTRIDGE_PARTIAL_PATH);
    write_if_changed(&path, generated.as_bytes()).unwrap();
    path
}

/// Render the program without writing files or invoking another toolchain.
pub fn render_cartridge(
    merged_manifest: &PaxManifest,
    userland_manifest: &PaxManifest,
    is_designtime: bool,
    use_rust_manifest: bool,
) -> String {
    templating::press_template_codegen_cartridge_snippet(
        templating::TemplateArgsCodegenCartridgeSnippet {
            cartridge_struct_id: merged_manifest.get_main_cartridge_struct_id(),
            definition_to_instance_traverser_struct_id: userland_manifest
                .get_main_definition_to_instance_traverser_struct_id(),
            components: merged_manifest.generate_codegen_component_info(),
            common_properties: CommonProperty::get_as_common_property(),
            type_table: userland_manifest.type_table.clone(),
            is_designtime,
            userland_manifest_json: if use_rust_manifest {
                String::new()
            } else {
                serde_json::to_string(userland_manifest).unwrap()
            },
            userland_manifest_rust: if use_rust_manifest {
                pax_manifest::rust_manifest::to_rust_expression(userland_manifest)
            } else {
                String::new()
            },
            use_rust_manifest,
            engine_import_path: userland_manifest.engine_import_path.clone(),
        },
    )
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Ok(existing) = fs::read(path) {
        if existing == bytes {
            return Ok(());
        }
    }

    fs::write(path, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    #[test]
    fn write_if_changed_preserves_mtime_when_contents_match() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let path = dir.path().join("cartridge.partial.rs");
        fs::write(&path, b"same").expect("initial write should succeed");
        let old_mtime = SystemTime::now() - Duration::from_secs(60);
        filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(old_mtime))
            .expect("mtime should be settable");
        let expected_mtime = fs::metadata(&path)
            .expect("metadata should be readable")
            .modified()
            .expect("mtime should be readable");

        write_if_changed(&path, b"same").expect("matching write should succeed");

        let actual_mtime = fs::metadata(&path)
            .expect("metadata should be readable")
            .modified()
            .expect("mtime should be readable");
        assert_eq!(actual_mtime, expected_mtime);
    }

    #[test]
    fn write_if_changed_updates_contents_when_they_differ() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let path = dir.path().join("cartridge.partial.rs");
        fs::write(&path, b"old").expect("initial write should succeed");

        write_if_changed(&path, b"new").expect("changed write should succeed");

        assert_eq!(fs::read(&path).expect("file should be readable"), b"new");
    }
}
