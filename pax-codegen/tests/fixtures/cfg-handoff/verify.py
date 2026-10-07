#!/usr/bin/env python3
"""Run the compiler-handoff acceptance fixture without an application build.rs."""
import argparse
import ctypes
import sys
import json
import os
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
print(f"All configuration-handoff cases passed. Logs: {logs}")
