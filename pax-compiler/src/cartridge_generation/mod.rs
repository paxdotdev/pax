//! # Code Generation Module
//!
//! The `code_generation` module provides structures and functions for generating Pax Cartridges
//! from Pax Manifests. The `generate_and_overwrite_cartridge` function is the main entrypoint.

use std::fs;
use std::io;

use pax_manifest::{cartridge_generation::CommonProperty, PaxManifest};

use std::path::{Path, PathBuf};

pub mod templating;

pub const CARTRIDGE_PARTIAL_PATH: &str = "cartridge.partial.rs";

// Generates (codegens) the PaxCartridge definition, abiding by the PaxCartridge trait.
// Side-effect: writes the generated string to disk as .pax/cartridge.partial.rs,
// so that it may be `include!`d by the  #[pax] #[main] macro
pub fn generate_cartridge_partial_rs(
    pax_dir: &PathBuf,
    merged_manifest: &PaxManifest,
    userland_manifest: &PaxManifest,
    designer_manifest: Option<PaxManifest>,
    use_rust_manifest: bool,
) -> PathBuf {
    //press template into String
    let generated_lib_rs = templating::press_template_codegen_cartridge_snippet(
        templating::TemplateArgsCodegenCartridgeSnippet {
            cartridge_struct_id: merged_manifest.get_main_cartridge_struct_id(),
            definition_to_instance_traverser_struct_id: userland_manifest
                .get_main_definition_to_instance_traverser_struct_id(),
            components: merged_manifest.generate_codegen_component_info(),
            common_properties: CommonProperty::get_as_common_property(),
            type_table: userland_manifest.type_table.clone(),
            is_designtime: cfg!(feature = "designtime"),
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
            designer_manifest_json: if let Some(designer_manifest) = designer_manifest {
                serde_json::to_string(&designer_manifest).unwrap()
            } else {
                "{}".to_string()
            },
            use_rust_manifest,
            engine_import_path: userland_manifest.engine_import_path.clone(),
        },
    );

    let path = pax_dir.join(CARTRIDGE_PARTIAL_PATH);
    write_if_changed(&path, generated_lib_rs.as_bytes()).unwrap();
    path
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
