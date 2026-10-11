#!/usr/bin/env python3
"""Run the compiler-handoff acceptance fixture without an application build.rs."""
import argparse
import ctypes
import sys
import json
import os
import shutil
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument("--target-dir", required=True, type=Path)
parser.add_argument("--logs", type=Path)
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
                  "-p", "cfg-app", "--offline", "--target-dir", str(args.target_dir), "-j4", *command[1:]]
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
                            ("rebuild_marker", marker)]:
        assert (field in widget) == expected, (name, field, widget)
    assert not ({"impossible", "dependency_test_only"} & widget), (name, widget)
    leaf_id = next(v["type_id"]["_type_id"] for v in manifest["type_table"].values()
                   if v["type_id"]["_type_id"].endswith("::Leaf"))
    assert properties(leaf_id) == {"selected"}, (name, properties(leaf_id))
    print(f"PASS {name}", flush=True)


run("native-app-feature", ["check", "--features", "fancy", "--all-targets"], True, False, test_units=(False, True))
run("native-rich-baked", ["test", "--features", "fancy", "--lib"], True, False, test_units=(True,))
run("native-dependency-feature", ["test", "--features", "widget-fancy", "--lib"], False, True, test_units=(True,))
run("wasm-dependency-feature", ["check", "--features", "widget-fancy", "--target", "wasm32-unknown-unknown", "--lib"], False, True, web=True)
run("release-dependency-feature", ["test", "--release", "--features", "widget-fancy", "--lib"], False, True, debug=False, test_units=(True,))
run("custom-rustc-cfg", ["rustc", "--features", "fancy", "--lib", "--", "--cfg", "proof_flag"], True, False, explicit_flag=True)
if sys.platform == "darwin":
    library = ctypes.CDLL(str(args.target_dir / "debug/libcfg_app.dylib"))
    library.pax_init_entry.argtypes = [ctypes.c_float, ctypes.c_float, ctypes.c_char_p,
                                      ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t]
    library.pax_init_entry.restype = ctypes.c_void_p
    library.pax_dealloc_engine.argtypes = [ctypes.c_void_p]
    library.pax_dealloc_engine.restype = None
    library.cfg_initializer_count.argtypes = [ctypes.c_bool]
    library.cfg_initializer_count.restype = ctypes.c_size_t
    def counts():
        return (library.cfg_initializer_count(False), library.cfg_initializer_count(True))
    def initialize(entry):
        return library.pax_init_entry(600, 400, entry, len(entry), None, 0)
    assert counts() == (0, 0), "Loading a cartridge must not initialize its roots"
    assert not initialize(b""), "Several roots require explicit selection"
    assert not initialize(b"crate::Missing")
    assert counts() == (0, 0), "Rejected selection must not run an initializer"
    app = initialize(b"crate::App")
    shell = initialize(b"crate::Shell")
    assert app and shell and app != shell, "Engines must be separate instances"
    assert counts() == (1, 1), "Nested App definitions must not invoke App's initializer"
    library.pax_dealloc_engine(app)
    library.pax_dealloc_engine(shell)
    print("PASS native-catalog-instance-boundary", flush=True)

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
             "--target-dir", str(args.target_dir), "--lib", "--features", "widget-fancy", "-j4"],
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
             "--target-dir", str(args.target_dir), "--workspace", "--all-targets", "-j4"],
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
