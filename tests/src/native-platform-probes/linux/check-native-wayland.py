#!/usr/bin/env python3
"""Exercise the actual GTK/Pax app as a native Wayland client.

Requires a real Wayland socket and shared AT-SPI session. AT-SPI actions exercise
native controls; this does not qualify physical keyboard/pointer input or KMS.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import time
from native_process import NativeProcess, add_launch_arguments


pixel_spec = importlib.util.spec_from_file_location("native_app_pixels", Path(__file__).with_name("check-native-app.py"))
pixel_helpers = importlib.util.module_from_spec(pixel_spec)
pixel_spec.loader.exec_module(pixel_helpers)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("app", type=Path)
    add_launch_arguments(parser)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report, capture = output / "state.json", output / "wayland.png"
    env = dict(os.environ, GDK_BACKEND="wayland", GSK_RENDERER="ngl", GTK_A11Y="atspi",
               PAX_NATIVE_TEST_REPORT=str(report), PAX_NATIVE_TEST_SCREENSHOT=str(capture))
    env.pop("DISPLAY", None)
    checks = []
    log = (output / "app.log").open("w")
    app = NativeProcess(args, env, log, output)

    def check(condition, name):
        checks.append({"check": name, "passed": bool(condition)})
        if not condition:
            raise RuntimeError(name)
        print("PASS", name, flush=True)

    def state():
        if app.poll() is not None:
            raise RuntimeError(f"App exited {app.returncode}")
        try:
            return json.loads(report.read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            return {}

    def wait(predicate, name, seconds=20):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            value = state()
            if predicate(value):
                return value
            time.sleep(.1)
        raise RuntimeError("Timeout: " + name)

    def text(value, expected):
        return any(n.get("text") == expected for n in value.get("nodes", []))

    try:
        import pyatspi
        value = wait(lambda s: text(s, "Initialized by user-owned main"), "startup")
        check(value.get("displayBackend") == "GdkWaylandDisplay", "actual native GDK Wayland display, no X11 fallback")
        desktop = pyatspi.Registry.getDesktop(0)
        native = None
        deadline = time.monotonic() + 10
        while native is None and time.monotonic() < deadline:
            native = next((a for a in desktop if app.matches_accessible(a)), None)
            time.sleep(.1)
        check(native is not None, "Wayland app exposed over AT-SPI")
        controls = []

        def visit(a, depth=0):
            if depth > 24 or len(controls) > 512:
                return
            controls.append(a)
            for child in a:
                visit(child, depth+1)

        visit(native)
        button = next(a for a in controls if a.getRoleName() == "push button" and a.name == "Increment")
        check(button.queryAction().doAction(0), "native Wayland button action accepted")
        wait(lambda s: text(s, "Count: 1"), "Rust handler")
        check(True, "native Wayland action reaches Rust handler")
        editor = next(a for a in controls if a.getRoleName() == "text")
        check(editor.queryEditableText().setTextContents("Wayland native text"), "AT-SPI edits genuine GTK editor")
        wait(lambda s: any(n["kind"] == "Textbox" and n.get("text") == "Wayland native text" for n in s.get("nodes", [])), "native editor binding")
        button.queryAction().doAction(0)
        value = wait(lambda s: s.get("frames", 0) >= 240, "continuous native Wayland delivery", seconds=30)
        gpu = value.get("runtime", {}).get("gpu", {})
        check(gpu.get("reused", 0) > 50 and not gpu.get("cpuPresentationReadback", True),
              "Wayland reuses synchronized exports without CPU presentation readback")
        Path(str(capture)+".request").touch()
        wait(lambda s: capture.exists() and not Path(str(capture)+".request").exists(), "Wayland capture")
        check(True, "one-shot Wayland composed capture completed")
        color_samples = pixel_helpers.image_color_samples(pixel_helpers.png_pixels(capture))
        (output / "image-colors.json").write_text(json.dumps(color_samples, indent=2)+"\n")
        for sample in color_samples:
            check(sample["passed"], sample["check"])
        (output / "final-state.json").write_text(json.dumps(value, indent=2)+"\n")
        app.terminate()
        check(app.wait(timeout=20) == 0, "Wayland standalone close succeeds")
        log.flush()
        check("gl_refs=0 gl_bytes=0" in (output / "app.log").read_text(), "Wayland close releases owned GL publications")
    finally:
        if app.poll() is None:
            app.terminate()
            try:
                app.wait(timeout=20)
            except subprocess.TimeoutExpired:
                app.kill()
                app.wait()
        log.close()
        (output / "checks.json").write_text(json.dumps({"checks": checks}, indent=2)+"\n")


if __name__ == "__main__":
    main()
