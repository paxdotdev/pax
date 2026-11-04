#!/usr/bin/env python3
"""Qualify orthographic spatial-workbench controls through real X11 input.

This is a functional/identity smoke test, not a perspective or frame-rate claim.
Screenshots retain the composed result for independent visual review.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

from native_process import NativeProcess, add_launch_arguments, _x11_client_pid


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("app", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    add_launch_arguments(parser)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    report, capture = out / "state.json", out / "capture.png"
    log = (out / "app.log").open("w")
    app = NativeProcess(args, dict(os.environ, GSK_RENDERER="ngl",
                        PAX_NATIVE_TEST_REPORT=str(report),
                        PAX_NATIVE_TEST_SCREENSHOT=str(capture)), log, out)
    checks, failure, completed = [], None, False

    def check(ok, label):
        checks.append({"check": label, "passed": bool(ok)})
        print("PASS" if ok else "FAIL", label, flush=True)
        if not ok:
            raise RuntimeError(label)

    def state():
        if app.poll() is not None:
            raise RuntimeError(f"Application exited {app.returncode}")
        try:
            return json.loads(report.read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            return {}

    def wait(predicate, label, seconds=20):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            value = state()
            if predicate(value):
                return value
            time.sleep(.05)
        raise RuntimeError("Timeout: " + label)

    def xdo(*args):
        return subprocess.check_output(["xdotool", *map(str, args)], text=True, timeout=15).strip()

    def node(node_id, value=None):
        return next(n for n in (value or state())["nodes"] if n["id"] == node_id)

    def click(target):
        x, y, w, h = target["bounds"]
        bounds = state()["sceneBoundsInSurface"]
        check(w > 2 and h > 2 and bounds[0] < x + w / 2 < bounds[0] + bounds[2]
              and bounds[1] < y + h / 2 < bounds[1] + bounds[3], "input target is inside the owned surface")
        xdo("mousemove", "--window", window, round(x + w / 2), round(y + h / 2))
        xdo("click", 1)
        time.sleep(.2)

    def button(label):
        current = wait(lambda s: any(n["kind"] == "Button" and n.get("text") == label
                                    for n in s.get("nodes", [])), "button: " + label)
        click(next(n for n in current["nodes"] if n["kind"] == "Button" and n.get("text") == label))

    def snapshot(name):
        request = Path(str(capture) + ".request")
        request.touch()
        value = wait(lambda _: capture.exists() and not request.exists(), "composed screenshot", 30)
        capture.rename(out / (name + ".png"))
        (out / (name + ".json")).write_text(json.dumps(value, indent=2))

    try:
        wait(lambda s: s.get("frames", 0) > 2, "first spatial frames", 120)
        candidates = xdo("search", "--onlyvisible", "--name", ".*").splitlines()
        owned = []
        for candidate in candidates:
            if _x11_client_pid(candidate) != app.pid:
                continue
            g = dict(row.split("=", 1) for row in xdo("getwindowgeometry", "--shell", candidate).splitlines())
            if int(g["WIDTH"]) > 1 and int(g["HEIGHT"]) > 1:
                owned.append(candidate)
        check(len(owned) == 1, "exactly one XRes-owned spatial window")
        window = owned[0]
        xdo("windowsize", "--sync", window, 1280, 1000)
        xdo("windowfocus", "--sync", window)
        wait(lambda s: abs(s.get("sceneBoundsInSurface", [0, 0, 0, 0])[2] - 1280) < 1,
             "wide spatial layout")
        maps = Path(f"/proc/{app.pid}/maps").read_text()
        libs = sorted({line.split()[-1] for line in maps.splitlines()
                       if "libpax_host.so" in line or "libspatial_workbench.so" in line})
        check({Path(n).name for n in libs} == {"libpax_host.so", "libspatial_workbench.so"},
              "mapped spatial cartridge and host identity")
        mount = Path(f"/proc/{app.pid}/root")
        (out / "identity.json").write_text(json.dumps({"pid": app.pid, "window": window,
            "executableSHA256": hashlib.sha256(Path(f"/proc/{app.pid}/exe").read_bytes()).hexdigest(),
            "libraries": {n: hashlib.sha256((mount / n.lstrip('/')).read_bytes()).hexdigest() for n in libs}}, indent=2))
        button("Reset pose")
        button("Hide amber")
        editor = next(n for n in state()["nodes"] if n["kind"] == "Textbox")
        editor_id = editor["id"]
        click(editor)
        xdo("key", "ctrl+a")
        typed = "PAX 1038 orthographic"
        xdo("type", "--clearmodifiers", "--delay", 20, typed)
        wait(lambda s: node(editor_id, s).get("text") == typed, "native edit reaches Rust binding")
        button("Press / 0")
        wait(lambda s: any(n.get("text") == "Press / 1" for n in s["nodes"]), "transformed native button")
        check(True, "orthographic transformed editor and button accept native input")
        snapshot("orthographic")
        button("Zoom out")
        wait(lambda s: node(editor_id, s)["bounds"][2] < editor["bounds"][2] * .9, "orthographic camera zoom")
        check(node(editor_id).get("text") == typed, "camera zoom retains editor identity and binding")
        snapshot("zoomed")
        button("Zoom reset")
        time.sleep(.6)
        before = node(editor_id)["bounds"]
        button("Switch to perspective")
        time.sleep(.6)
        check(max(abs(a - b) for a, b in zip(before, node(editor_id)["bounds"])) < 1,
              "unsupported perspective retains the default orthographic placement")
        button("Switch to orthographic")
        button("Aperture off")
        click(node(editor_id))
        xdo("key", "ctrl+a")
        wait(lambda s: node(editor_id, s).get("selectionStart") == 0
             and node(editor_id, s).get("selectionEnd") == len(typed), "native selection")
        # The Y-tilt slider is the only authored -180..180 range. Keyboard input
        # reaches exactly 90 degrees without assigning application properties.
        tilt = next(n for n in state()["nodes"] if n["kind"] == "Slider"
                    and n.get("min") == -180 and n.get("max") == 180)
        click(tilt)
        xdo("key", "End")
        xdo("key", "--repeat", 90, "--delay", 10, "Left")
        wait(lambda s: abs(node(tilt["id"], s).get("value", 0) - 90) < .01, "exact edge-on tilt")
        check(node(editor_id).get("text") == typed, "clipped edge-on pose retains editor state")
        check(node(editor_id).get("placementVisible") is False,
              "edge-on native placement is hidden without destroying its control")
        check((node(editor_id).get("selectionStart"), node(editor_id).get("selectionEnd")) == (0, len(typed)),
              "edge-on pose retains native selection")
        snapshot("edge-on")
        button("Reset pose")
        button("Hide amber")
        wait(lambda s: node(editor_id, s)["bounds"][2] > 300, "finite placement recovers")
        button("Press / 1")
        wait(lambda s: any(n.get("text") == "Press / 2" for n in s["nodes"]), "recovered native button")
        check(node(editor_id).get("text") == typed, "recovered control retains its edited value")
        check(node(editor_id).get("placementVisible") is True, "finite native placement is restored")
        check((node(editor_id).get("selectionStart"), node(editor_id).get("selectionEnd")) == (0, len(typed)),
              "recovery retains native selection")
        snapshot("recovered")
        app.terminate()
        check(app.wait(timeout=30) == 0, "spatial fixture closes normally")
        log.flush()
        contents = (out / "app.log").read_text()
        check("CRITICAL" not in contents, "no GTK or Pango critical diagnostics")
        check("Perspective @camera is unsupported" in contents, "unsupported capability is diagnosed")
        check("PAX_LINUX_CLOSED" in contents and "gl_refs=0 gl_bytes=0" in contents,
              "spatial GPU publications retire on shutdown")
        completed = True
    except Exception as error:
        failure = str(error)
        raise
    finally:
        try:
            if app.poll() is None:
                app.terminate()
                app.wait(timeout=30)
        finally:
            log.close()
            (out / "checks.json").write_text(json.dumps({"passed": completed,
                "error": failure, "exitCode": app.poll(), "checks": checks}, indent=2))


if __name__ == "__main__":
    main()
