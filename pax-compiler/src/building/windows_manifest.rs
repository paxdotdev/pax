//! Executable declarations belong to the application, not the host DLL.
use super::desktop::checked;
use crate::RunContext;
use color_eyre::eyre::{eyre, Result, WrapErr};
use std::{
    ffi::c_void,
    fs,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    process::Command,
};

#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryExW(path: *const u16, file: *mut c_void, flags: u32) -> *mut c_void;
    fn FindResourceW(module: *mut c_void, name: *const u16, kind: *const u16) -> *mut c_void;
    fn FreeLibrary(module: *mut c_void) -> i32;
}

fn has_manifest(executable: &Path) -> Result<bool> {
    let path: Vec<u16> = executable
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // A data-file mapping never executes the application's entry point or imports.
    let module = unsafe { LoadLibraryExW(path.as_ptr(), std::ptr::null_mut(), 0x2) };
    if module.is_null() {
        return Err(std::io::Error::last_os_error()).wrap_err("Cannot inspect launcher resources");
    }
    let resource = unsafe { FindResourceW(module, 1usize as *const u16, 24usize as *const u16) };
    let error = std::io::Error::last_os_error();
    unsafe { FreeLibrary(module) };
    if !resource.is_null() {
        Ok(true)
    } else if matches!(error.raw_os_error(), Some(1812 | 1813 | 1814)) {
        Ok(false)
    } else {
        Err(error).wrap_err("Cannot inspect launcher manifest")
    }
}

fn manifest_tool() -> Result<PathBuf> {
    let mut candidates: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path| {
            std::env::split_paths(&path)
                .map(|dir| dir.join("mt.exe"))
                .collect()
        })
        .unwrap_or_default();
    let architectures: &[&str] = if cfg!(target_arch = "aarch64") {
        &["arm64", "x64"]
    } else {
        &["x64"]
    };
    if let Some(directory) = std::env::var_os("WindowsSdkVerBinPath") {
        candidates.extend(
            architectures
                .iter()
                .map(|arch| PathBuf::from(&directory).join(arch).join("mt.exe")),
        );
    }
    if let Some(directory) = std::env::var_os("ProgramFiles(x86)") {
        let sdk_bin = PathBuf::from(directory).join("Windows Kits/10/bin");
        if let Ok(entries) = fs::read_dir(sdk_bin) {
            let mut versions: Vec<_> = entries
                .flatten()
                .filter_map(|entry| {
                    let version = entry
                        .file_name()
                        .to_str()?
                        .split('.')
                        .map(str::parse::<u32>)
                        .collect::<Result<Vec<_>, _>>()
                        .ok()?;
                    (version.len() == 4).then_some((version, entry.path()))
                })
                .collect();
            versions.sort_by(|left, right| right.0.cmp(&left.0));
            for (_, directory) in versions {
                candidates.extend(
                    architectures
                        .iter()
                        .map(|arch| directory.join(arch).join("mt.exe")),
                );
            }
        }
    }
    candidates.into_iter().find(|path| path.is_file()).ok_or_else(|| eyre!(
        "Windows launcher packaging requires Windows SDK mt.exe. Install the Windows SDK or run pax-cli from a Visual Studio Native Tools shell."
    ))
}

pub(super) fn embed(
    ctx: &RunContext,
    pax_dir: &Path,
    executable: &Path,
    launcher: &str,
    version: [u64; 3],
) -> Result<()> {
    let tool = manifest_tool()?;
    let manifest = pax_dir.join("windows-launcher.manifest");
    fs::write(
        &manifest,
        include_str!("../../files/windows-launcher.manifest"),
    )?;
    let mut command = Command::new(&tool);
    command.arg("-nologo");
    if has_manifest(executable)? {
        // mt.exe does not reject every conflicting DPI value during a merge.
        // Inspect the original before it can be normalized or combined.
        let authored = pax_dir.join("windows-launcher-authored.manifest");
        checked(
            Command::new(&tool)
                .arg("-nologo")
                .arg(format!("-inputresource:{};#1", executable.display()))
                .arg(format!("-out:{}", authored.display())),
            ctx,
        )?;
        super::windows_manifest_identity::validate_dpi(&fs::read_to_string(&authored)?, true)?;
        // Preserve app-authored UAC, dependency, and other manifest declarations.
        // Conflicting declarations fail the SDK merge rather than being replaced.
        command.arg(format!("-inputresource:{};#1", executable.display()));
    }
    let merged = pax_dir.join("windows-launcher-merged.manifest");
    command
        .arg("-manifest")
        .arg(&manifest)
        .arg(format!("-out:{}", merged.display()));
    checked(&mut command, ctx).wrap_err_with(|| format!(
        "Could not merge Windows 10/11 compatibility and PerMonitorV2 declarations with {}. Check the application's existing manifest for conflicting declarations.", tool.display()
    ))?;
    // A compatibility fragment is not a complete assembly. Validate only after
    // merging and, if needed, supplying the application's definition identity.
    let complete = pax_dir.join("windows-launcher-complete.manifest");
    let xml = fs::read_to_string(&merged)?;
    super::windows_manifest_identity::validate_dpi(&xml, false)?;
    if let Some(identity) =
        super::windows_manifest_identity::fallback_identity(&xml, launcher, version)?
    {
        checked(
            Command::new(&tool)
                .arg("-nologo")
                .arg("-manifest")
                .arg(&merged)
                .arg(format!("-identity:{identity}"))
                .arg(format!("-out:{}", complete.display())),
            ctx,
        )?;
    } else {
        fs::copy(&merged, &complete)?;
    }
    checked(
        Command::new(&tool)
            .args(["-nologo", "-manifest"])
            .arg(&complete)
            .arg("-validate_manifest"),
        ctx,
    )
    .wrap_err("The complete Windows launcher manifest failed SDK validation")?;
    checked(
        Command::new(&tool)
            .args(["-nologo", "-manifest"])
            .arg(&complete)
            .arg(format!("-outputresource:{};#1", executable.display())),
        ctx,
    )
    .wrap_err("Could not embed the validated Windows launcher manifest")
}
