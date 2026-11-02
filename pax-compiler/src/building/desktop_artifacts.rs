//! Physical application artifacts kept apart from Cargo's shared PDB outputs.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub(super) struct FrozenCartridge {
    pub(super) image: PathBuf,
    pub(super) symbols: Option<PathBuf>,
}

pub(super) fn launcher_pdb_name(binary: &str) -> String {
    format!("{}.pdb", binary.replace('-', "_"))
}

fn copy_optional(source: &Path, destination: &Path) -> io::Result<Option<PathBuf>> {
    match fs::metadata(source) {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)?;
    Ok(Some(destination.to_owned()))
}

pub(super) fn freeze_cartridge(source: &Path, directory: &Path) -> io::Result<FrozenCartridge> {
    let name = source.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Cartridge must have a filename",
        )
    })?;
    fs::create_dir_all(directory)?;
    let image = directory.join(name);
    // Cargo can rebuild the library and overwrite its same-named PDB while
    // linking the launcher. A byte copy preserves the selected cartridge pair.
    fs::copy(source, &image)?;
    let symbols = copy_optional(&source.with_extension("pdb"), &image.with_extension("pdb"))?;
    Ok(FrozenCartridge { image, symbols })
}

pub(super) fn stage_symbols(
    directory: &Path,
    cartridge: Option<&Path>,
    launcher: &Path,
) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    if let Some(cartridge) = cartridge {
        let destination = directory
            .join("cartridge")
            .join(cartridge.file_name().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Cartridge PDB must have a filename",
                )
            })?);
        fs::create_dir_all(destination.parent().unwrap())?;
        // This optional PDB was already captured. Its disappearance is an error.
        fs::copy(cartridge, destination)?;
    }
    let launcher_name = launcher.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Launcher PDB must have a filename",
        )
    })?;
    copy_optional(launcher, &directory.join("launcher").join(launcher_name))?;
    Ok(())
}

pub(super) fn publish_symbols(staged: &Path, destination: &Path) -> io::Result<()> {
    // Publish only after packaging succeeds. Replacing the whole CLI-owned
    // directory also removes stale symbols when a later profile omits PDBs.
    if !fs::metadata(staged)?.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Staged symbols must be a directory",
        ));
    }
    if destination.try_exists()? {
        fs::remove_dir_all(destination)?;
    }
    fs::rename(staged, destination)
}
