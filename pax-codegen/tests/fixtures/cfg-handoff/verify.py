#!/usr/bin/env python3
"""Run the compiler-handoff acceptance fixture without an application build.rs."""
import argparse
import sys
import json
import os
import shutil
from pathlib import Path
import subprocess
import tempfile


def positive_jobs(value):
    jobs = int(value)
    if jobs < 1:
        raise argparse.ArgumentTypeError("jobs must be at least 1")
    return jobs


parser = argparse.ArgumentParser()
parser.add_argument("--target-dir", required=True, type=Path)
parser.add_argument("--logs", type=Path)
parser.add_argument("--macos-host-sdk", type=Path,
                    help="also exercise an already-built macOS host SDK without mounting UI")
export_modes = parser.add_mutually_exclusive_group()
export_modes.add_argument("--native-exports-only", action="store_true",
                    help="only build and probe native debug, designtime, and release cartridges")
export_modes.add_argument("--legacy-exports-only", action="store_true",
                    help="only probe a modern debug cartridge and legacy dependency debug/release builds")
parser.add_argument("--jobs", type=positive_jobs, default=2,
                    help="maximum concurrent Cargo jobs (default: 2)")
args = parser.parse_args()
root = Path(__file__).resolve().parent
logs = args.logs or Path(tempfile.mkdtemp(prefix="pax-cfg-handoff-"))
logs.mkdir(parents=True, exist_ok=True)
environment = dict(os.environ, CARGO_NET_OFFLINE="true")
assert not (root / "app/build.rs").exists()
assert not (root / "widgets/src/missing.rs").exists()

# Macro execution needs source/codegen types, never a host rendering engine.
# A runtime edge here makes cross compilation build a second GPU stack.
repository = root.parents[3]
for package in ["pax-macro", "pax-compiler"]:
    graph = subprocess.check_output(
        ["cargo", "tree", "--offline", "--manifest-path", str(repository / "Cargo.toml"),
         "-p", package, "--features", "designtime", "--edges", "normal,build",
         "--prefix", "none", "--format", "{p}"],
        env=environment, text=True,
    )
    names = {line.split()[0] for line in graph.splitlines() if line.strip()}
    assert not (names & {"pax-runtime", "pax-gpu", "wgpu", "naga"}), (package, names)
print("PASS build-tools-without-renderer", flush=True)

# Even a native app with development capabilities must not link its packager.
# Inspect build edges too: tools must not sneak in through an application macro.
graph = subprocess.check_output(
    ["cargo", "tree", "--offline", "--manifest-path", str(repository / "Cargo.toml"),
     "-p", "pax-kit", "--features", "host,designtime", "--edges", "normal,build",
     "--prefix", "none", "--format", "{p}"], env=environment, text=True,
)
names = {line.split()[0] for line in graph.splitlines() if line.strip()}
assert not (names & {"pax-compiler", "pax-cli", "actix-web"}), names
print("PASS application-without-packager", flush=True)


def run(name, command, app_fancy, widget_fancy, web=False, debug=True,
        test_units=(False,), marker=False, explicit_flag=False):
    invocation = ["cargo", command[0], "--manifest-path", str(root / "Cargo.toml"),
                  "-p", "cfg-app", "--offline", "--target-dir", str(args.target_dir),
                  "--jobs", str(args.jobs), *command[1:]]
    sidecar = logs / f"{name}.json"
    case_environment = dict(environment, PAX_MANIFEST_OUTPUT=str(sidecar),
                            PAX_CARTRIDGE_MANIFEST_DIR=str(root / "app"),
                            PAX_BUILD_CARTRIDGE="true")
    result = subprocess.run(invocation, env=case_environment, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (logs / f"{name}.log").write_text(result.stdout)
    if result.returncode:
        raise AssertionError(f"{name} failed; see {logs / (name + '.log')}\n{result.stdout[-6000:]}")
    if command[0] == "test":
        assert "generated_descriptors_match_rust_configuration ... ok" in result.stdout, result.stdout[-3000:]
    manifest = json.loads(sidecar.read_text())
    def properties(name):
        record = next(v for v in manifest["type_table"].values()
                      if v["type_id"]["_type_id"] == name)
        return {p["name"] for p in record["property_definitions"]}
    app = properties("crate::App")
    assert ("app_fancy" in app) == app_fancy, (name, app)
    assert ("test_only" in app) in test_units, (name, app)
    assert ("explicit_flag" in app) == explicit_flag, (name, app)
    assert "wrongly_inherited" not in app, (name, app)
    widget = properties("renamed_widgets::selected::Widget")
    for field, expected in [("fancy", widget_fancy), ("plain", not widget_fancy),
                            ("web", web), ("native", not web), ("debug", debug),
                            ("macos", not web and sys.platform == "darwin"),
                            ("windows", not web and sys.platform == "win32"),
                            ("linux", not web and sys.platform.startswith("linux")),
                            ("rebuild_marker", marker)]:
        assert (field in widget) == expected, (name, field, widget)
    assert not ({"impossible", "dependency_test_only"} & widget), (name, widget)
    leaf_id = next(v["type_id"]["_type_id"] for v in manifest["type_table"].values()
                   if v["type_id"]["_type_id"].endswith("::Leaf"))
    assert properties(leaf_id) == {"selected"}, (name, properties(leaf_id))
    print(f"PASS {name}", flush=True)


def probe_native_exports(name, release=False):
    filename = ("libcfg_app.dylib" if sys.platform == "darwin" else
                "cfg_app.dll" if sys.platform == "win32" else "libcfg_app.so")
    # Separate processes prevent a previously loaded library or its statics
    # from hiding a wrong export selection in the next build/profile.
    result = subprocess.run(
        [sys.executable, str(root / "probe-native-exports.py"),
         str(args.target_dir / ("release" if release else "debug") / filename),
         *([str(args.macos_host_sdk)] if args.macos_host_sdk else [])],
        text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (logs / f"{name}.log").write_text(result.stdout)
    assert result.returncode == 0, result.stdout[-6000:]
    print(f"PASS {name}", flush=True)
    reports = [json.loads(line) for line in result.stdout.splitlines()
               if line.startswith('{"result":')]
    assert len(reports) == 1, result.stdout
    return reports[0]


def probe_legacy_dependency(expected_exports):
    # The website and its embedded examples use the compatibility declaration.
    # Exercise its real macro path, then restore the authored fixture exactly.
    source = root / "widgets/src/lib.rs"
    original = source.read_text()
    declaration = "pax_kit::runnable! { root: crate::Wrapper, init: crate::initialize_wrapper }"
    assert declaration in original
    legacy = original.replace(declaration, "").replace(
        "#[inlined(<Widget/>)]\npub struct Wrapper",
        "#[main]\n#[init(crate::initialize_wrapper)]\n#[inlined(<Widget/>)]\npub struct Wrapper")
    try:
        source.write_text(legacy)
        for release in [False, True]:
            name = "release-legacy-dependency" if release else "native-legacy-dependency"
            run(name, ["build", "--lib", *(["--release"] if release else [])], False, False, debug=not release)
            observed = probe_native_exports(f"{name}-probe", release)
            assert observed["nativeExports"] == expected_exports
    finally:
        source.write_text(original)


if args.legacy_exports_only:
    run("native-modern-dependency", ["build", "--lib"], False, False)
    baseline = probe_native_exports("native-modern-dependency-probe")
    probe_legacy_dependency(baseline["nativeExports"])
    sys.exit(0)


if args.native_exports_only:
    expected_exports = None
    for name, flags, release in [
        ("native-baked-exports", [], False),
        ("native-designtime-exports", ["--features", "pax-kit/designtime"], False),
        ("release-baked-exports", ["--release"], True),
    ]:
        run(name, ["build", "--lib", *flags], False, False, debug=not release)
        observed = probe_native_exports(f"{name}-probe", release)
        if expected_exports is None:
            expected_exports = observed["nativeExports"]
        assert observed["nativeExports"] == expected_exports, "Export identity must not depend on the profile or baked/designtime representation"
    probe_legacy_dependency(expected_exports)
    sys.exit(0)


run("native-app-feature", ["check", "--features", "fancy", "--all-targets"], True, False, test_units=(False, True))
run("native-rich-baked", ["test", "--features", "fancy", "--lib"], True, False, test_units=(True,))
run("native-designtime", ["test", "--features", "fancy,pax-kit/designtime", "--lib"], True, False, test_units=(True,))
run("native-dependency-feature", ["test", "--features", "widget-fancy", "--lib"], False, True, test_units=(True,))
run("wasm-dependency-feature", ["check", "--features", "widget-fancy", "--target", "wasm32-unknown-unknown", "--lib"], False, True, web=True)
run("release-dependency-feature", ["test", "--release", "--features", "widget-fancy", "--lib"], False, True, debug=False, test_units=(True,))
run("custom-rustc-cfg", ["rustc", "--features", "fancy", "--lib", "--", "--cfg", "proof_flag"], True, False, explicit_flag=True)
native_catalog = probe_native_exports("native-catalog-instance-boundary")
run("release-native-library", ["build", "--release", "--features", "widget-fancy", "--lib"], False, True, debug=False)
probe_native_exports("release-native-catalog-instance-boundary", release=True)
probe_legacy_dependency(native_catalog["nativeExports"])

source = root / "widgets/src/plain.rs"
original = source.read_text()
try:
    source.write_text(original.replace("pub struct Widget {", "pub struct Widget {\n    pub rebuild_marker: Property<bool>,"))
    run("dependency-source-edit", ["check", "--features", "fancy", "--lib"], True, False, marker=True)
finally:
    source.write_text(original)
run("restored-source", ["check", "--features", "fancy", "--lib"], True, False)
# Reproduce a dependency living inside a different, cold Cargo workspace. Its
# unrelated member cannot resolve at all, so even a fully warm developer cache
# cannot hide a macro that incorrectly requests that workspace's full graph.
with tempfile.TemporaryDirectory(prefix="pax-source-workspace-") as temporary:
    fixture = Path(temporary)
    workspace = fixture / "libraries"
    workspace.mkdir()
    (workspace / "Cargo.toml").write_text(
        '[workspace]\nmembers = ["leaf", "widgets", "unrelated"]\nresolver = "2"\n')
    unrelated = workspace / "unrelated"
    unrelated.mkdir()
    (unrelated / "Cargo.toml").write_text(
        '[package]\nname = "unrelated"\nversion = "0.0.0"\n'
        '[lib]\npath = "lib.rs"\n[dependencies]\n'
        'pax-handoff-unrelated-must-never-resolve = "=0.0.0"\n')
    (unrelated / "lib.rs").write_text("")
    consumer = fixture / "app"
    for name, destination in [("leaf", workspace / "leaf"),
                              ("widgets", workspace / "widgets"), ("app", consumer)]:
        shutil.copytree(root / name, destination, ignore=shutil.ignore_patterns("target", ".pax"))
        manifest = destination / "Cargo.toml"
        text = manifest.read_text().replace("../../../../../pax-kit", str(repository / "pax-kit"))
        if name == "app":
            text = text.replace('../widgets', '../libraries/widgets')
            text += '\n[workspace]\nresolver = "2"\n'
        manifest.write_text(text)
    shutil.copyfile(root / "Cargo.lock", consumer / "Cargo.lock")
    # Exercise custom library names, with a transitive Rust import that cannot
    # be inferred from the package name in --no-deps metadata.
    manifest = workspace / "leaf/Cargo.toml"
    manifest.write_text(manifest.read_text().replace('[lib]', '[lib]\nname = "leaf_components"'))
    source = workspace / "widgets/src/lib.rs"
    source.write_text(source.read_text().replace('cfg_leaf::', 'leaf_components::'))
    metadata = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--offline", "--manifest-path",
         str(workspace / "widgets/Cargo.toml")],
        env=environment, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    assert metadata.returncode and "pax-handoff-unrelated-must-never-resolve" in metadata.stdout, metadata.stdout
    (logs / "unrelated-workspace-metadata.log").write_text(metadata.stdout)
    plain_environment = {key: value for key, value in environment.items() if not key.startswith("PAX_")}
    def test_external(name):
        result = subprocess.run(
            ["cargo", "test", "--offline", "--manifest-path", str(consumer / "Cargo.toml"),
             "--target-dir", str(args.target_dir), "--lib", "--features", "widget-fancy",
             "--jobs", str(args.jobs)],
            env=plain_environment, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (logs / f"{name}.log").write_text(result.stdout)
        assert result.returncode == 0, result.stdout[-6000:]
        assert "generated_descriptors_match_rust_configuration ... ok" in result.stdout, result.stdout[-3000:]
        print(f"PASS {name}", flush=True)
    test_external("external-workspace-with-unresolvable-unrelated-member")
    manifest = workspace / "widgets/Cargo.toml"
    manifest.write_text(manifest.read_text().replace('cfg-leaf = {', 'renamed-leaf = { package = "cfg-leaf",'))
    source.write_text(source.read_text().replace('leaf_components::', 'renamed_leaf::'))
    test_external("external-workspace-transitive-rename")
    # Absolute paths in exported helpers must invalidate when a source checkout
    # moves, even when its file contents/mtimes and shared target cache survive.
    relocated = fixture / "relocated-libraries"
    workspace.rename(relocated)
    workspace = relocated
    manifest = consumer / "Cargo.toml"
    manifest.write_text(manifest.read_text().replace('../libraries/widgets', '../relocated-libraries/widgets'))
    test_external("relocated-component-workspace")
    # Exercise default selection, renamed UI dependencies, custom library
    # targets, explicit options, and the ordinary same-package executable.
    (fixture / "Cargo.toml").write_text('[workspace]\nmembers = ["app", "host"]\nresolver = "2"\n')
    manifest = consumer / "Cargo.toml"
    text = manifest.read_text().replace('\n[workspace]\nresolver = "2"\n', '')
    text = text.replace('[lib]', '[lib]\nname = "custom_ui"')
    text = text.replace('pax-kit = { path = ', 'pax-kit = { features = ["host"], path = ')
    manifest.write_text(text)
    (consumer / "src/main.rs").write_text(
        'fn main() -> Result<(), pax_kit::host::Error> { pax_kit::init!()?.run() }\n')
    host = fixture / "host"
    (host / "src").mkdir(parents=True)
    (host / "Cargo.toml").write_text(
        '[package]\nname = "cfg-native-host"\nversion = "0.0.0"\nedition = "2021"\n'
        '[package.metadata.pax.cartridge]\npackage = "cfg-app"\nlib = "custom_ui"\n'
        '[dependencies]\nrenamed-ui = { package = "cfg-app", path = "../app" }\n'
        f'pax-kit = {{ path = "{repository / "pax-kit"}", features = ["host"] }}\n')
    (host / "src/lib.rs").write_text(
        'pub fn implicit() -> Result<pax_kit::host::Application, pax_kit::host::Error> { pax_kit::init!() }\n'
        'pub fn explicit() -> Result<pax_kit::host::Application, pax_kit::host::Error> { pax_kit::init!(renamed_ui) }\n'
        'pub fn options() -> Result<pax_kit::host::Application, pax_kit::host::Error> { pax_kit::init_with!(renamed_ui, pax_kit::host::InitOptions::default()) }\n')
    shutil.copyfile(consumer / "Cargo.lock", fixture / "Cargo.lock")
    def check_host():
        return subprocess.run(
            ["cargo", "check", "--offline", "--manifest-path", str(fixture / "Cargo.toml"),
             "--target-dir", str(args.target_dir), "--workspace", "--all-targets",
             "--jobs", str(args.jobs)],
            env=plain_environment, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    result = check_host()
    (logs / "external-native-host.log").write_text(result.stdout)
    assert result.returncode == 0, result.stdout[-6000:]
    print("PASS external-native-host-and-same-package-main", flush=True)
    host_manifest = host / "Cargo.toml"
    host_manifest.write_text(host_manifest.read_text().replace(
        'renamed-ui = { package = "cfg-app",', 'cfg-app = {'))
    host_source = host / "src/lib.rs"
    host_source.write_text(host_source.read_text().replace('renamed_ui', 'custom_ui'))
    result = check_host()
    (logs / "custom-library-native-host.log").write_text(result.stdout)
    assert result.returncode == 0, result.stdout[-6000:]
    print("PASS native-init-custom-library-without-rename", flush=True)
    manifest.write_text(text.replace('["rlib", "cdylib"]', '["rlib"]'))
    result = check_host()
    (logs / "missing-cdylib.log").write_text(result.stdout)
    assert result.returncode and 'The selected UI library needs crate-type' in result.stdout, result.stdout[-6000:]
    print("PASS native-init-rejects-missing-cdylib", flush=True)
print(f"All configuration-handoff cases passed. Logs: {logs}")
