#!/usr/bin/env python3
"""Validate native export ownership with small, offline Rust/C/Swift programs.

This exercises linkage mechanisms, not the Pax compiler, renderer or host.
Generated workspaces and build outputs must live outside the repository.
"""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time


SOURCE = Path(__file__).resolve().parent
EXPORTS = {"pax_cartridge_abi_version", "pax_cartridge_program_id", "pax_init_entry",
           "pax_tick", "pax_dealloc_engine", "probe_count", "probe_live"}


def check_library(path, identity, expected):
    library = ctypes.CDLL(str(path))
    library.pax_cartridge_abi_version.restype = ctypes.c_uint32
    library.pax_cartridge_program_id.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
    library.pax_cartridge_program_id.restype = ctypes.c_size_t
    library.pax_init_entry.argtypes = [ctypes.c_char_p, ctypes.c_size_t]
    library.pax_init_entry.restype = ctypes.c_void_p
    library.pax_tick.argtypes = [ctypes.c_void_p, ctypes.c_uint64]
    library.pax_tick.restype = ctypes.c_uint64
    library.pax_dealloc_engine.argtypes = [ctypes.c_void_p]
    library.probe_count.argtypes = [ctypes.c_size_t]
    library.probe_count.restype = ctypes.c_size_t
    library.probe_live.restype = ctypes.c_size_t
    counts = lambda: [library.probe_count(i) for i in range(4)]
    assert library.pax_cartridge_abi_version() == 1
    size = library.pax_cartridge_program_id(None, 0)
    buffer = ctypes.create_string_buffer(size)
    assert library.pax_cartridge_program_id(buffer, size) == size
    assert buffer.raw == identity.encode()
    assert counts() == [0, 0, 0, 0] and library.probe_live() == 0
    for entry, length in [(b"missing", 7), (None, 1), (b"x", 129)]:
        assert not library.pax_init_entry(entry, length)
    assert counts() == [0, 0, 0, 0]
    first = library.pax_init_entry(None, 0)
    second = library.pax_init_entry(identity.encode(), len(identity))
    assert first and second and first != second
    assert library.pax_tick(first, 3) == expected + 3
    assert library.pax_tick(second, 0) == expected
    assert counts() == [2, 0, 0, 0] and library.probe_live() == 2
    library.pax_dealloc_engine(first)
    library.pax_dealloc_engine(second)
    assert library.probe_live() == 0
    print("PASS dynamic C ABI: identity, rejected selection, isolated instances, selected init only, owner cleanup")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work-dir", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--apple-cross", action="store_true",
                        help="Also compile and link tiny iOS device/simulator static C/Swift clients")
    args = parser.parse_args()
    work, output = args.work_dir.resolve(), args.output.resolve()
    repository = SOURCE.parents[3]
    if work.is_relative_to(repository):
        parser.error("--work-dir must be outside the repository")
    work.mkdir(parents=True, exist_ok=False)
    output.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, CARGO_NET_OFFLINE="true", CARGO_BUILD_JOBS="1",
               CARGO_INCREMENTAL="0", CARGO_TARGET_DIR=str(work / "target"),
               CLANG_MODULE_CACHE_PATH=str(work / "clang-cache"),
               SWIFT_MODULECACHE_PATH=str(work / "swift-cache"))
    results = []

    def run(name, command, expected_failure=None, extra_env=None, timeout=120):
        started = time.monotonic()
        result = subprocess.run(list(map(str, command)), cwd=work,
                                env=dict(env, **(extra_env or {})),
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                text=True, timeout=timeout)
        (output / (name + ".log")).write_text(result.stdout)
        row = {"case": name, "command": list(map(str, command)),
               "exit": result.returncode, "seconds": time.monotonic() - started,
               "expectedFailure": expected_failure}
        results.append(row)
        if expected_failure:
            assert result.returncode != 0 and expected_failure in result.stdout, row
        else:
            assert result.returncode == 0, f"{name}: {result.stdout[-4000:]}"
        print("PASS " + name, flush=True)
        return result.stdout

    def write(path, body):
        path = work / path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(body)

    def package(folder, name, code, dependencies="", extra="", version="1.0.0"):
        write(folder + "/Cargo.toml", f'[package]\nname="{name}"\nversion="{version}"\nedition="2021"\n'
              + extra + '\n[dependencies]\n' + dependencies)
        write(folder + "/src/lib.rs", code)

    profiles = '\n[profile.dev]\ndebug=0\n[profile.release]\nlto=true\ncodegen-units=1\npanic="abort"\n'
    write("Cargo.toml", '[workspace]\nresolver="2"\nmembers=["api","website","movie","adapter","calculator-adapter","launcher"]\n'
          'exclude=["calc-v1","calc-v2","legacy","cycle"]\n' + profiles)
    package("api", "proof-api", (SOURCE / "api.rs").read_text())
    for version, value in [(1, 11), (2, 17)]:
        package(f"calc-v{version}", "calculator-ui",
                f'proof_api::declare_program!("calculator-v{version}", {value} + if cfg!(feature="accent") {{10}} else {{0}}, '
                '|index| if index == 0 { initializations() } else { 0 });\n',
                'proof-api={path="../api"}\n',
                '[workspace]\n[features]\ndefault=["standalone"]\nstandalone=[]\naccent=[]\n',
                f"{version}.0.0")
    package("movie", "movie-ui", 'proof_api::declare_program!("movie", 23, |index| if index == 0 { initializations() } else { 0 });\n',
            'proof-api={path="../api"}\n', '[features]\ndefault=["standalone"]\nstandalone=[]\n')
    package("website", "site-ui", '''
fn counts(index: usize) -> usize {
    match index { 0 => initializations(), 1 => old_calc::initializations(),
        2 => new_calc::initializations(), 3 => movie::initializations(), _ => 0 }
}
proof_api::declare_program!("website", 100 + old_calc::component() + new_calc::component() + movie::component(), counts);
#[cfg(test)]
mod tests {
    #[test]
    fn ordinary_rust_entrypoints_remain_distinct() {
        let expected_new = if cfg!(feature="accent") { 27 } else { 17 };
        assert_eq!(old_calc::pax_init_entry(), 11);
        assert_eq!(new_calc::pax_init_entry(), expected_new);
        assert_eq!(movie::pax_init_entry(), 23);
        assert_eq!(super::pax_init_entry(), 100 + 11 + expected_new + 23);
        assert_eq!((0..4).map(super::counts).collect::<Vec<_>>(), vec![1, 1, 1, 1]);
    }
}
''', 'proof-api={path="../api"}\nold_calc={package="calculator-ui",path="../calc-v1"}\n'
            'new_calc={package="calculator-ui",path="../calc-v2"}\nmovie={package="movie-ui",path="../movie"}\n',
            '[features]\ndefault=["standalone"]\nstandalone=[]\naccent=["new_calc/accent"]\n')
    dylib = "libowner_adapter.dylib" if sys.platform == "darwin" else "libowner_adapter.so"
    metadata = f'#[macro_export] macro_rules! cartridge_file {{ () => {{ "{dylib}" }}; }}\n'
    package("adapter", "app-adapter", 'proof_api::export_program!(chosen_ui::program);\n' + metadata,
            'proof-api={path="../api"}\nchosen_ui={package="site-ui",path="../website"}\n',
            '[lib]\nname="owner_adapter"\ncrate-type=["rlib","cdylib","staticlib"]\n'
            '[features]\naccent=["chosen_ui/accent"]\n')
    package("calculator-adapter", "calculator-adapter", 'proof_api::export_program!(selected_ui::program);\n',
            'proof-api={path="../api"}\nselected_ui={package="calculator-ui",path="../calc-v2"}\n',
            '[lib]\nname="alternate_adapter"\ncrate-type=["rlib","cdylib","staticlib"]\n')
    write("launcher/Cargo.toml", '[package]\nname="launcher"\nversion="1.0.0"\nedition="2021"\n'
          '[features]\naccent=["boundary/accent"]\n[dependencies]\nboundary={package="app-adapter",path="../adapter"}\n')
    write("launcher/src/main.rs", (SOURCE / "launcher.rs").read_text())
    cargo = ["cargo", "--offline"]

    def artifacts(log):
        records = []
        for line in log.splitlines():
            if line.startswith("{"):
                value = json.loads(line)
                if value.get("reason") == "compiler-artifact":
                    records.append(value)
        return records

    def symbols(name, path):
        flags = ["-gU"] if sys.platform == "darwin" else ["-g", "--defined-only"]
        raw = run(name, ["nm", *flags, path])
        return [line.split()[-1].removeprefix("_") for line in raw.splitlines() if line.strip()]

    try:
        run("toolchain", ["rustc", "-vV"])
        for mode, expected in [("debug", 151), ("release", 161)]:
            flags = ["--release", "--features", "accent"] if mode == "release" else []
            log = run("cargo-run-" + mode,
                      [*cargo, "run", "-p", "launcher", "--jobs", "1", "--message-format=json", *flags, "--", expected])
            record = next(row for row in artifacts(log) if row["target"]["name"] == "owner_adapter")
            library = next(Path(p) for p in record["filenames"] if Path(p).name == dylib)
            staticlib = next(Path(p) for p in record["filenames"] if p.endswith(".a"))
            binary = work / "target" / mode / "launcher"
            definitions = symbols("adapter-symbols-" + mode, library)
            assert all(definitions.count(name) == 1 for name in EXPORTS)
            assert not (set(symbols("launcher-symbols-" + mode, binary)) & EXPORTS), "Launcher pulled in the cartridge's C API"
            run("dynamic-contract-" + mode, [sys.executable, __file__, "--check-library", library, "website", expected])
            if sys.platform == "darwin":
                for language, compiler in [("c", "clang"), ("swift", "swiftc")]:
                    client = work / (language + "-host-" + mode)
                    command = ["xcrun", compiler]
                    if language == "swift":
                        command += ["-module-cache-path", work / "swift-cache", "-import-objc-header", SOURCE / "host.h"]
                    command += [SOURCE / ("host." + language), staticlib, "-o", client]
                    run("link-" + language + "-" + mode, command)
                    run("static-" + language + "-" + mode, [client, expected])
            if mode == "release":
                package_dir = work / "relocated"
                package_dir.mkdir()
                shutil.copy2(binary, package_dir / "launcher")
                shutil.copy2(library, package_dir / dylib)
                run("relocated", [package_dir / "launcher", expected])
        warm = run("cargo-run-warm", [*cargo, "run", "-p", "launcher", "--release", "--features", "accent",
                                     "--message-format=json", "--", "161"])
        assert artifacts(warm) and all(row["fresh"] for row in artifacts(warm))
        for mode in ["debug", "release"]:
            flags = ["--release", "--features", "accent"] if mode == "release" else []
            run("rust-namespaces-" + mode, [*cargo, "test", "-p", "site-ui", "--lib", *flags])
        alternate = run("alternate-adapter", [*cargo, "build", "-p", "calculator-adapter", "--release", "--message-format=json"])
        record = next(row for row in artifacts(alternate) if row["target"]["name"] == "alternate_adapter")
        alternate_path = next(Path(p) for p in record["filenames"] if p.endswith((".dylib", ".so")))
        run("alternate-contract", [sys.executable, __file__, "--check-library", alternate_path, "calculator-v2", "17"])

        # Negative control: the duplicate global can either be rejected by
        # the linker or silently bind both libraries to one initializer.
        write("legacy/Cargo.toml", '[package]\nname="legacy"\nversion="0.0.0"\nedition="2021"\n[workspace]\n'
              '[dependencies]\na={path="a"}\nb={path="b"}\n[profile.dev]\ncodegen-units=1\n')
        write("legacy/src/main.rs", 'fn main() { assert_eq!((a::component(), b::component()), (1, 2), '
              '"pax_init_entry collision: dependency initializers must remain distinct"); }')
        for name, number in [("a", 1), ("b", 2)]:
            package("legacy/" + name, name, f'#[no_mangle] pub extern "C" fn pax_init_entry() -> u32 {{ {number} }}\n'
                    '#[inline(never)] pub fn component() -> u32 { '
                    'std::hint::black_box(pax_init_entry as extern "C" fn() -> u32)() }\n')
        negative = run("negative-global-exports", [*cargo, "run", "--manifest-path", work / "legacy/Cargo.toml"],
                       expected_failure="pax_init_entry")
        assert any(message in negative for message in [
            "duplicate symbol", "multiple definition", "already defined",
            "pax_init_entry collision: dependency initializers must remain distinct",
        ]), "Negative control must demonstrate a collision, not an unrelated compilation error"

        # A default-features=false edge does not break a package dependency
        # cycle between a combined UI/launcher package and its adapter.
        write("cycle/Cargo.toml", '[package]\nname="combined-app"\nversion="0.0.0"\nedition="2021"\n[workspace]\n'
              '[features]\ndefault=["standalone"]\nstandalone=["dep:adapter"]\n'
              '[dependencies]\nadapter={path="adapter",optional=true}\n')
        write("cycle/src/lib.rs", 'pub fn component() {}')
        write("cycle/src/main.rs", 'fn main() {}')
        package("cycle/adapter", "adapter", 'pub fn entry() { combined_app::component(); }',
                'combined-app={path="..",default-features=false}\n')
        run("negative-combined-package-cycle", [*cargo, "build", "--manifest-path", work / "cycle/Cargo.toml"],
            expected_failure="cyclic package dependency")

        if args.apple_cross:
            assert sys.platform == "darwin", "Apple cross-link checks require Xcode"
            for triple, sdk_name, clang_target in [
                ("aarch64-apple-ios", "iphoneos", "arm64-apple-ios15.0"),
                ("aarch64-apple-ios-sim", "iphonesimulator", "arm64-apple-ios15.0-simulator"),
            ]:
                run("rust-" + triple, [*cargo, "rustc", "-p", "app-adapter", "--lib", "--release", "--features", "accent",
                                      "--target", triple, "--crate-type", "staticlib"],
                    extra_env={"IPHONEOS_DEPLOYMENT_TARGET": "15.0"})
                sdk = run("sdk-" + sdk_name, ["xcrun", "--sdk", sdk_name, "--show-sdk-path"]).strip().splitlines()[-1]
                archive = work / "target" / triple / "release/libowner_adapter.a"
                for language, compiler in [("c", "clang"), ("swift", "swiftc")]:
                    target = work / (language + "-host-" + triple)
                    command = ["xcrun", "--sdk", sdk_name, compiler, "-target", clang_target]
                    if language == "swift":
                        command += ["-sdk", sdk, "-module-cache-path", work / "swift-cache", "-import-objc-header", SOURCE / "host.h"]
                    else:
                        command += ["-isysroot", sdk]
                    command += [SOURCE / ("host." + language), archive, "-o", target]
                    run("cross-link-" + language + "-" + triple, command)
                    run("inspect-" + language + "-" + triple, ["xcrun", "vtool", "-show-build", target])
        summary = {"passed": True, "scope": "standalone linkage model; no production Pax migration or GPU/UI acceptance",
                   "results": results, "appleCrossLinkOnly": args.apple_cross,
                   "sourceSHA256": {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                                    for p in SOURCE.iterdir() if p.is_file()},
                   "productionFilesChanged": False, "ec2Started": False}
    except Exception as error:
        summary = {"passed": False, "error": str(error), "results": results}
        raise
    finally:
        (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--check-library":
        check_library(Path(sys.argv[2]), sys.argv[3], int(sys.argv[4]))
    else:
        main()
