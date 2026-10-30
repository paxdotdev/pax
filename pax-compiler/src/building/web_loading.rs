//! Same-build two-payload packaging. Split before wasm-bindgen rewrites imports.
use super::*;
use pax_codegen::project_layout::ProjectLayout;

fn executable_in(directory: &Path, name: &str, suffix: &str) -> Option<PathBuf> {
    let path = directory.join(format!("{name}{suffix}"));
    path.is_file().then_some(path)
}

fn tool(variable: &str, name: &str) -> eyre::Result<PathBuf> {
    if let Some(path) = std::env::var_os(variable) {
        return Ok(path.into());
    }
    if let Some(path) = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .find_map(|path| executable_in(&path, name, std::env::consts::EXE_SUFFIX))
    }) {
        return Ok(path);
    }
    if name == "wasm-bindgen" {
        if let Some(home) = std::env::var_os("HOME") {
            for cache in [
                "Library/Caches/.wasm-pack",
                ".cache/.wasm-pack",
                ".wasm-pack",
            ] {
                if let Ok(entries) = fs::read_dir(Path::new(&home).join(cache)) {
                    let mut candidates: Vec<_> = entries
                        .flatten()
                        .filter_map(|e| executable_in(&e.path(), name, std::env::consts::EXE_SUFFIX))
                        .collect();
                    candidates.sort();
                    if let Some(path) = candidates.pop() {
                        return Ok(path);
                    }
                }
            }
        }
    }
    Err(eyre!("Loader builds require {name}; install the qualified tool or set {variable} to its executable. See the loading-screen documentation."))
}

fn run(cmd: &mut Command, children: &Arc<Mutex<Vec<u64>>>) -> eyre::Result<()> {
    cmd.stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());
    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(crate::pre_exec_hook);
    }
    let child = cmd
        .spawn()
        .map_err(|e| eyre!("failed to start {cmd:?}: {e}"))?;
    if !wait_with_output(children, child).status.success() {
        return Err(eyre!("loading cartridge build step failed: {cmd:?}"));
    }
    Ok(())
}

pub(super) fn compile(
    ctx: &RunContext,
    pax_dir: &Path,
    layout: &ProjectLayout,
    output: &Path,
    children: &Arc<Mutex<Vec<u64>>>,
    timings: &mut BuildTimings,
) -> eyre::Result<()> {
    let splitter = tool("PAX_WASM_SPLIT", "wasm-split-cli")?;
    let bindgen = tool("PAX_WASM_BINDGEN", "wasm-bindgen")?;
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&layout.cartridge_dir)
        .args(["build", "--lib", "--target", "wasm32-unknown-unknown"])
        .env("PAX_DIR", pax_dir)
        .env(
            "PAX_MANIFEST_OUTPUT",
            pax_dir.join("manifest-wasm32-unknown-unknown.json"),
        );
    layout.configure_cartridge(&mut cmd);
    if ctx.is_release || ctx.profile_wasm_size {
        cmd.arg("--release");
        apply_release_size_profile(&mut cmd, true);
    }
    cmd.env("CARGO_PROFILE_RELEASE_STRIP", "none")
        .env("CARGO_PROFILE_DEV_STRIP", "none");
    cmd.arg(format!("--features={}", web_cargo_features(ctx)));
    configure_web_rasterization(&mut cmd);
    configure_pax_build_env(&mut cmd, "web", ctx.should_run_designtime);
    let flags = std::env::var("RUSTFLAGS").unwrap_or_else(|_| "-C target-feature=+simd128".into());
    cmd.env("RUSTFLAGS", format!("{flags} -C link-arg=--emit-relocs"));
    timings.record("cargo/loader", || run(&mut cmd, children))?;
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                layout.cartridge_dir.join(path)
            }
        })
        .unwrap_or_else(|| layout.target_dir.clone());
    let input = target
        .join("wasm32-unknown-unknown")
        .join(if ctx.is_release || ctx.profile_wasm_size {
            "release"
        } else {
            "debug"
        })
        .join(format!("{}.wasm", layout.library_name.replace('-', "_")));
    fs::create_dir_all(output)?;
    timings.record("wasm-split", || {
        run(
            Command::new(splitter)
                .arg(&input)
                .arg(output)
                .args(["--out-name", "pax-cartridge"]),
            children,
        )
    })?;
    timings.record("wasm-bindgen", || {
        run(
            Command::new(bindgen)
                .arg(output.join("pax-cartridge.wasm"))
                .arg("--out-dir")
                .arg(output)
                .args([
                    "--out-name",
                    "pax-cartridge",
                    "--target",
                    "web",
                    "--keep-debug",
                    "--no-demangle",
                    "--keep-lld-exports",
                ]),
            children,
        )
    })?;
    for name in ["pax-cartridge_bg.wasm", "split_application.wasm"] {
        let path = output.join(name);
        strip_link_metadata(&path)?;
        if ctx.is_release {
            optimize_web_wasm(&path, children, ctx.profile_wasm_size)?;
        }
    }
    install_adapter(output)?;
    expose_selected_web_entrypoint(output, &layout.web_entrypoint)?;
    for extension in ["js", "d.ts"] {
        let path = output.join(format!("pax-cartridge.{extension}"));
        let mut contents = fs::read_to_string(&path)?;
        contents.push_str(&format!(
            "\nexport {{ {}_loader as init_loading_screen }};\n",
            layout.web_entrypoint
        ));
        fs::write(path, contents)?;
    }
    // The unspecialized intermediate is build output, not a third download.
    fs::remove_file(output.join("pax-cartridge.wasm"))?;
    Ok(())
}

fn install_adapter(output: &Path) -> eyre::Result<()> {
    let path = output.join("__wasm_split.js");
    let original = fs::read_to_string(&path)?;
    let prefix = original
        .split_once("function wrapAsyncCb(")
        .ok_or_else(|| eyre!("Unexpected splitter glue; use wasm_split_cli_support 0.2.4"))?
        .0;
    if !prefix.contains("function getSharedImports()") {
        return Err(eyre!(
            "Splitter did not expose the qualified shared-import contract"
        ));
    }
    let size = fs::metadata(output.join("split_application.wasm"))?.len();
    fs::write(
        output.join("pax-split-loader.js"),
        include_str!("../../files/interfaces/web/src/utils/split-loader.js"),
    )?;
    fs::write(
        path,
        format!(
            r#"{prefix}
import {{createSplitLoader}} from './pax-split-loader.js';
export const loader = createSplitLoader({{
    url: new URL('./split_application.wasm', import.meta.url),
    decodedSize: {size}, getImports: getSharedImports,
}});
let owners = 0;
export function acquire() {{
    owners++; let released = false;
    return () => {{ if (released) return; released = true; if (--owners === 0) loader.cancel(); }};
}}
export const __wasm_split_load_application = loader.callback(() => getSharedImports().__wasm_split.__indirect_function_table);
"#
        ),
    )?;
    Ok(())
}

fn strip_link_metadata(path: &Path) -> eyre::Result<()> {
    fn leb(bytes: &[u8], cursor: &mut usize) -> eyre::Result<usize> {
        let mut value = 0usize;
        for shift in (0..35).step_by(7) {
            let byte = *bytes
                .get(*cursor)
                .ok_or_else(|| eyre!("truncated Wasm section"))?;
            *cursor += 1;
            value |= usize::from(byte & 127) << shift;
            if byte < 128 {
                return Ok(value);
            }
        }
        Err(eyre!("invalid Wasm section length"))
    }
    let bytes = fs::read(path)?;
    if !bytes.starts_with(b"\0asm\x01\0\0\0") {
        return Err(eyre!("invalid Wasm module"));
    }
    let mut output = bytes[..8].to_vec();
    let mut cursor = 8;
    while cursor < bytes.len() {
        let start = cursor;
        let kind = bytes[cursor];
        cursor += 1;
        let size = leb(&bytes, &mut cursor)?;
        let end = cursor
            .checked_add(size)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| eyre!("invalid Wasm section"))?;
        let drop = if kind == 0 {
            let len = leb(&bytes, &mut cursor)?;
            let name = bytes
                .get(cursor..cursor + len)
                .filter(|_| cursor + len <= end)
                .ok_or_else(|| eyre!("invalid custom section"))?;
            name == b"linking" || name.starts_with(b"reloc.") || name == b"__wasm_split_unstable"
        } else {
            false
        };
        if !drop {
            output.extend_from_slice(&bytes[start..end]);
        }
        cursor = end;
    }
    fs::write(path, output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tool_discovery_uses_the_platform_executable_suffix() {
        let directory = tempfile::tempdir().unwrap();
        for name in ["wasm-split-cli", "wasm-bindgen"] {
            let windows = directory.path().join(format!("{name}.exe"));
            fs::write(&windows, []).unwrap();
            assert_eq!(executable_in(directory.path(), name, ".exe"), Some(windows));
            assert_eq!(executable_in(directory.path(), name, ""), None);
            let unix = directory.path().join(name);
            fs::write(&unix, []).unwrap();
            assert_eq!(executable_in(directory.path(), name, ""), Some(unix));
        }
    }

    #[test]
    fn strips_relocations_without_changing_executable_or_other_custom_sections() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("module.wasm");
        let header = b"\0asm\x01\0\0\0";
        fn custom(name: &str) -> Vec<u8> {
            let mut bytes = vec![0, (name.len() + 1) as u8, name.len() as u8];
            bytes.extend_from_slice(name.as_bytes());
            bytes
        }
        let code = [1, 4, 1, 0x60, 0, 0];
        let mut input = header.to_vec();
        for name in ["linking", "reloc.CODE", "__wasm_split_unstable", "name"] {
            input.extend(custom(name));
        }
        input.extend(code);
        fs::write(&path, input).unwrap();
        strip_link_metadata(&path).unwrap();
        let mut expected = header.to_vec();
        expected.extend(custom("name"));
        expected.extend(code);
        assert_eq!(fs::read(&path).unwrap(), expected);
        fs::write(&path, [header.as_slice(), &[0, 3, 100]].concat()).unwrap();
        assert!(strip_link_metadata(&path).is_err());
    }
}
