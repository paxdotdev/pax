#!/usr/bin/env python3
"""Real X11 keyboard/drag, shared bindings, popup, and standalone clip checks."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import time

from native_process import NativeProcess, add_launch_arguments

spec = importlib.util.spec_from_file_location("pixels", Path(__file__).with_name("check-native-app.py"))
pixels = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pixels)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("app", type=Path)
    add_launch_arguments(parser)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    report, capture = out / "state.json", out / "capture.png"
    env = dict(os.environ, GSK_RENDERER="ngl", GTK_A11Y="atspi",
               PAX_NATIVE_TEST_REPORT=str(report), PAX_NATIVE_TEST_SCREENSHOT=str(capture))
    checks, snapshots = [], {}
    completed, failure = False, None
    log = (out / "app.log").open("w")
    app = NativeProcess(args, env, log, out)

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

    def values(value=None):
        value = state() if value is None else value
        text = next((n.get("text", "") for n in value.get("nodes", [])
                     if n.get("text", "").startswith("value=")), "")
        return dict(field.split("=", 1) for field in text.split() if "=" in field)

    def wait(predicate, label, seconds=15):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            value = state()
            if predicate(value):
                return value
            time.sleep(.05)
        raise RuntimeError("Timeout: " + label)

    def xdo(*arguments):
        return subprocess.check_output(["xdotool", *map(str, arguments)], text=True, timeout=10).strip()

    def move(x, y):
        xdo("mousemove", "--window", window, round(x), round(y))

    def click(x, y):
        move(x, y)
        xdo("click", 1)
        time.sleep(.25)

    def button(label):
        n = next(n for n in state()["nodes"] if n["kind"] == "Button" and n.get("text") == label)
        x, y, w, h = n["bounds"]
        click(x + w / 2, y + h / 2)

    def nodes(kind):
        return sorted([n for n in state()["nodes"] if n["kind"] == kind],
                      key=lambda n: (n["bounds"][1], n["bounds"][0]))

    def all_sliders(expected):
        return wait(lambda s: all(abs(n["value"] - expected) < .001
                                  for n in s.get("nodes", []) if n["kind"] == "Slider"),
                    "all shared slider peers agree")

    def snapshot(name, image=False):
        value = state()
        snapshots[name] = value
        (out / f"{name}.json").write_text(json.dumps(value, indent=2) + "\n")
        if image:
            Path(str(capture) + ".request").touch()
            wait(lambda _: capture.exists() and not Path(str(capture) + ".request").exists(), "composed capture")
            capture.rename(out / f"{name}.png")
        return value

    def accessibility(label):
        import pyatspi
        native = next((a for a in pyatspi.Registry.getDesktop(0) if app.matches_accessible(a)), None)
        check(native is not None, "exact native application exposed through AT-SPI")
        records = []
        def visit(a, depth=0, ancestors=()):
            if depth > 24 or len(records) >= 512:
                return
            record = {"role": a.getRoleName(), "name": a.name,
                      "showing": a.getState().contains(pyatspi.STATE_SHOWING),
                      "checked": a.getState().contains(pyatspi.STATE_CHECKED),
                      "selected": a.getState().contains(pyatspi.STATE_SELECTED),
                      "ancestors": list(ancestors)}
            try:
                r = a.queryComponent().getExtents(pyatspi.DESKTOP_COORDS)
                record["bounds"] = [r.x, r.y, r.width, r.height]
            except NotImplementedError:
                pass
            records.append(record)
            for child in a:
                visit(child, depth + 1, (*ancestors, record["role"]))
        visit(native)
        (out / f"{label}-accessibility.json").write_text(json.dumps(records, indent=2) + "\n")
        return records

    def popup(label):
        records = accessibility(label)
        check(any(r["showing"] and r["name"] == "Gamma" and
                  any(role in ("list item", "menu item", "option") for role in
                      [r["role"], *r["ancestors"]]) for r in records),
              label + ": actual popup choices are visible to accessibility")

    def corner_pixel(name, expected):
        snapshot(name, image=True)
        _, _, channels, rows = pixels.png_pixels(out / f"{name}.png")
        actual = list(rows[714][458 * channels:458 * channels + 3])
        check(all(abs(a - b) <= 3 for a, b in zip(actual, expected)),
              name + ": standalone lower-right corner has expected composed color")
        return actual

    try:
        wait(lambda s: values(s).get("options") == "3", "fixture startup")
        window = app.x11_window("Pax native controls")
        xdo("windowsize", "--sync", window, 1000, 800)
        xdo("windowfocus", "--sync", window)
        time.sleep(.5)
        mapped = Path(f"/proc/{app.pid}/maps").read_text()
        libs = sorted({line.split()[-1] for line in mapped.splitlines()
                       if "libpax_host.so" in line or "libnative_controls.so" in line})
        mount = Path(f"/proc/{app.pid}/root")
        (out / "identity.json").write_text(json.dumps({
            "pid": app.pid, "window": window,
            "executableSHA256": hashlib.sha256(Path(f"/proc/{app.pid}/exe").read_bytes()).hexdigest(),
            "libraries": {n: hashlib.sha256((mount / n.lstrip("/")).read_bytes()).hexdigest() for n in libs},
        }, indent=2) + "\n")
        check(len(nodes("Slider")) == 5 and len(nodes("Dropdown")) == 3 and len(nodes("RadioList")) == 2,
              "real primary, transformed, repeated, and scrolled controls are present")
        all_sliders(.5)
        initial = snapshot("initial", image=True)
        primary = nodes("Slider")[0]
        x, y, w, h = primary["bounds"]
        click(x + w / 2, y + h / 2)
        before = values()
        move(990, 790)
        xdo("key", "Right")
        wait(lambda s: int(values(s).get("events", 0)) == int(before["events"]) + 1, "one arrow input event")
        expected = float(before["value"]) + .25
        all_sliders(expected)
        check(abs(float(values()["value"]) - expected) < .001, "Right increments by the authored step and reaches Rust once")
        xdo("key", "Home")
        wait(lambda s: values(s).get("value") == "-2.00", "Home reaches minimum")
        all_sliders(-2)
        xdo("key", "End")
        wait(lambda s: values(s).get("value") == "2.00", "End reaches maximum")
        all_sliders(2)
        before_drag = int(values()["events"])
        move(x + 15, y + h / 2)
        xdo("mousedown", 1)
        for fraction in [.2, .4, .6, .8]:
            move(x + w * fraction, y + h / 2)
            time.sleep(.15)
        check(int(values()["events"]) >= before_drag + 3, "drag publishes continuous values before pointer release")
        move(x + w + 40, y + h + 40)
        xdo("mouseup", 1)
        time.sleep(.3)
        all_sliders(float(values()["value"]))
        snapshot("after-drag")
        repeated = sorted([n for n in nodes("Slider") if n["bounds"][0] > 500 and n["bounds"][1] > 650],
                          key=lambda n: n["bounds"][1])
        check(len(repeated) == 2 and repeated[0]["id"] != repeated[1]["id"], "repeated sliders have independent expanded identities")
        rx, ry, rw, rh = repeated[1]["bounds"]
        click(rx + rw * .4, ry + rh / 2)
        all_sliders(float(values()["value"]))
        before_repeat = int(values()["events"])
        move(990, 790)
        xdo("key", "Right")
        wait(lambda s: int(values(s).get("events", 0)) == before_repeat + 1, "repeated peer emits exactly one event")
        all_sliders(float(values()["value"]))
        check(True, "input from repeated peer updates every other slider")

        button("Reset")
        before_program = int(values()["events"])
        button("Move range")
        wait(lambda s: values(s).get("range") == "10..20" and values(s).get("value") == "14.00", "atomic range move")
        all_sliders(14)
        check(int(values()["events"]) == before_program, "programmatic range move creates no slider input events")
        button("Reset")
        wait(lambda s: values(s).get("range") == "-2..2", "range reset")
        check(int(values()["events"]) == before_program, "reverse range move creates no slider input events")

        dropdown = next(n for n in nodes("Dropdown") if n["bounds"][0] < 100 and n["bounds"][1] < 400)
        dx, dy, dw, dh = dropdown["bounds"]
        click(dx + dw / 2, dy + dh / 2)
        popup("primary-popup")
        xdo("key", "End", "Return")
        wait(lambda s: values(s).get("dropdown") == "2", "popup selection reaches Rust")
        check(all(n["selectedId"] == 2 for n in nodes("Dropdown")), "dropdown selection updates transformed and scrolled peers")
        transformed = next(n for n in nodes("Dropdown") if n["bounds"][0] > 500)
        dx, dy, dw, dh = transformed["bounds"]
        click(dx + dw / 2, dy + dh / 2)
        popup("transformed-popup")
        xdo("key", "Escape")
        check(values()["dropdown"] == "2", "Escape dismisses transformed popup without selection")
        radio = nodes("RadioList")[0]
        rx, ry, rw, rh = radio["bounds"]
        click(rx + 30, ry + rh / 2)
        wait(lambda s: values(s).get("radio") == "1", "radio pointer selection")
        move(990, 790)
        xdo("key", "Down")
        wait(lambda s: values(s).get("radio") == "2", "radio arrow selection")
        check(values()["other"] == "2", "radio keyboard navigation leaves independent group unchanged")
        other = nodes("RadioList")[1]
        ox, oy, ow, oh = other["bounds"]
        click(ox + 30, oy + oh / 6)
        wait(lambda s: values(s).get("other") == "0", "independent radio selection")
        check(values()["radio"] == "2", "independent group leaves first group unchanged")
        accessible = accessibility("selected-controls")
        checked = {r["name"] for r in accessible
                   if r["role"] == "radio button" and r["checked"] and r["showing"]}
        check(checked == {"Gamma", "One"}, "AT-SPI exposes the two independent checked radio names")
        check(any(r["name"] == "Gamma" and r["showing"] and
                  "combo box" in r["ancestors"] for r in accessible),
              "AT-SPI exposes the selected dropdown label")
        button("Replace options")
        wait(lambda s: values(s).get("options") == "2", "options replacement")
        check(all(n["selectedId"] == 1 for n in nodes("Dropdown")) and values()["other"] == "0",
              "new options select Epsilon and preserve independent group")
        button("Empty options")
        wait(lambda s: values(s).get("options") == "0", "empty options")
        check(all(n["selectedId"] is None and n["options"] == [] for n in nodes("Dropdown")),
              "empty dropdowns have no stale native selection")
        button("Reset")
        wait(lambda s: values(s).get("options") == "3", "restore options")
        button("Toggle controls")
        wait(lambda s: sum(n["kind"] == "Slider" for n in s["nodes"]) == 1, "remove conditional controls")
        button("Toggle controls")
        wait(lambda s: sum(n["kind"] == "Slider" for n in s["nodes"]) == 5, "recreate conditional controls")
        all_sliders(.5)
        corner_pixel("radius48", (220, 30, 180))
        before = values()
        click(30, 476)
        xdo("click", "--repeat", 3, 5)
        time.sleep(.4)
        check(values()["corner"] == before["corner"] and values()["scroll"] == before["scroll"],
              "rounded cutout rejects both corner button and wheel input")
        click(70, 500)
        wait(lambda s: int(values(s).get("corner", 0)) == int(before["corner"]) + 1, "inside corner button")
        button("Cycle radius")
        wait(lambda s: values(s).get("radius") == "8", "radius eight")
        corner_pixel("radius8", (40, 210, 225))
        button("Cycle radius")
        wait(lambda s: values(s).get("radius") == "24", "radius twenty-four")
        corner_pixel("radius24", (220, 30, 180))
        move(300, 650)
        xdo("click", "--repeat", 4, 5)
        wait(lambda s: float(values(s).get("scroll", 0)) > 0, "native rounded scroller updates Rust")
        snapshot("scrolled", image=True)
        app.terminate()
        check(app.wait(timeout=30) == 0, "control fixture closes normally")
        log.flush()
        check("PAX_LINUX_CLOSED" in (out / "app.log").read_text() and
              "gl_refs=0 gl_bytes=0" in (out / "app.log").read_text(), "owned GL publications retired")
        completed = True
    except Exception as error:
        failure = str(error)
        raise
    finally:
        try:
            # Release synthetic button state even if a drag assertion failed.
            subprocess.run(["xdotool", "mouseup", "1"], capture_output=True, timeout=5)
            if app.poll() is None:
                app.terminate()
                app.wait(timeout=30)
        finally:
            log.close()
            (out / "checks.json").write_text(json.dumps({
                "completed": completed, "passed": completed and all(c["passed"] for c in checks),
                "error": failure, "pid": app.pid, "exitCode": app.poll(), "checks": checks,
            }, indent=2) + "\n")


if __name__ == "__main__":
    main()
