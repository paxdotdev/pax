#!/usr/bin/env python3
"""Real X11 input through the Linux host, shared Wheel dispatcher and Rust fixture.

Runs examples/fixtures/native-wheel. Counters come from the app's own Snapshot
button, and bound positions are compared with GTK's actual adjustments. This is
a functional mouse-wheel check, not a physical touchpad or framerate benchmark.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import time

from native_process import NativeProcess, add_launch_arguments, _x11_client_pid

# Reuse the exact root-child ownership query used by website acceptance.
_spec = importlib.util.spec_from_file_location("website_probe", Path(__file__).with_name("check-native-website.py"))
_website = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_website)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("app", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--screenshots", action="store_true")
    add_launch_arguments(parser)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report, capture = output / "state.json", output / "capture.png"
    checks, receipts = [], []
    log = (output / "app.log").open("w")
    app, failure, completed = None, None, False

    def save(name, value):
        (output / name).write_text(json.dumps(value, indent=2) + "\n")

    def check(condition, name, **details):
        checks.append({"check": name, "passed": bool(condition), **details})
        print("PASS" if condition else "FAIL", name, flush=True)
        if not condition:
            raise RuntimeError(name)

    def state():
        if app.poll() is not None:
            raise RuntimeError(f"Application exited {app.returncode}")
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
            time.sleep(.05)
        raise RuntimeError("Timeout: " + name)

    def fresh():
        before = state().get("frames", 0)
        return wait(lambda s: s.get("frames", 0) > before, "fresh native report")

    def xdo(*values):
        return subprocess.check_output(["xdotool", *map(str, values)], text=True, timeout=10).strip()

    def point(x, y):
        value = state()
        sx, sy = value["sceneBoundsInSurface"][:2]
        xdo("mousemove", "--window", window, round(sx+x), round(sy+y))
        pointer = _website.pointer_window()
        if not pointer["sameScreen"] or not pointer["window"] or _x11_client_pid(pointer["window"]) != app.pid:
            raise RuntimeError("Pointer left this fixture's owned X11 surface")

    def click(x, y):
        point(x, y)
        xdo("click", 1)

    def scrollers(value):
        result = {node["id"]: node for node in value.get("nodes", []) if node["kind"] == "Scroller"}
        return {"inner": result[inner_id], "outer": result[outer_id]}

    def snapshot(name):
        before = len((output / "app.log").read_text().split("pax-native-wheel:"))
        click(894, 123)
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            lines = (output / "app.log").read_text().split("pax-native-wheel:")
            if len(lines) > before:
                line = lines[-1].splitlines()[0]
                break
            state()
            time.sleep(.05)
        else:
            raise RuntimeError("Snapshot button did not invoke its Rust handler")
        fields = dict(re.findall(r"(\w+)=([^\s]+)", line.split(" last=[", 1)[0]))
        result = {key: (value == "true" if key == "prevent_native_default" else float(value))
                  for key, value in fields.items()}
        result["last"] = line.split(" last=[", 1)[1].rsplit("]", 1)[0]
        value = fresh()
        result["native"] = {key: {field: node.get(field) for field in
                                  ("id", "bounds", "scrollX", "scrollY", "scrollUpper", "scrollPage")}
                            for key, node in scrollers(value).items()}
        result["name"] = name
        receipts.append(result)
        save("snapshots.json", receipts)
        save(name + "-state.json", value)
        for owner, node in result["native"].items():
            for axis in ("x", "y"):
                check(abs(result[f"{owner}_{axis}"] - node["scroll"+axis.upper()]) < .11,
                      f"{name}: {owner} {axis} binding matches native offset")
        return result

    def reset(name):
        click(695, 123)
        wait(lambda s: all(abs(n.get("scrollX", 0)) < .1 and abs(n.get("scrollY", 0)) < .1
                           for n in s.get("nodes", []) if n["kind"] == "Scroller"), "authored Reset offsets")
        time.sleep(.5)
        value = snapshot(name + "-reset")
        check(all(value[k+"_events"] == 0 for k in ("inner", "outer", "root", "overlay")), name+": counters reset")
        return value

    def wheel(x, y, button):
        point(x, y)
        xdo("click", button)
        # Native GTK smooth adjustment movement must settle before cancellation.
        time.sleep(.8)
        fresh()

    def path(before, after, expected, name):
        increments = {key: after[key+"_events"] - before[key+"_events"]
                      for key in ("inner", "outer", "root", "overlay")}
        root = increments["root"]
        # Each call injects one discrete X11 button notch, not a touchpad gesture.
        check(root == 1 and all(v == (1 if key in expected else 0) for key, v in increments.items()),
              name + ": each Rust handler receives one event per bubble", increments=increments)

    def unchanged(before, after, owners, name):
        check(all(abs(after[f"{owner}_{axis}"] - before[f"{owner}_{axis}"]) < .1
                  for owner in owners for axis in ("x", "y")), name)

    try:
        app = NativeProcess(args, dict(os.environ, GSK_RENDERER="ngl", PAX_NATIVE_TEST_REPORT=str(report),
                                      PAX_NATIVE_TEST_SCREENSHOT=str(capture)), log, output)
        value = wait(lambda s: len([n for n in s.get("nodes", []) if n["kind"] == "Scroller"]) == 2,
                     "two native fixture scrollers", 120)
        window = app.x11_window(title="Pax native wheel input")
        xdo("windowraise", window)
        xdo("windowfocus", "--sync", window)
        time.sleep(1)
        value = fresh()
        nodes = sorted((n for n in value["nodes"] if n["kind"] == "Scroller"), key=lambda n: n["bounds"][2])
        inner_id, outer_id = nodes[0]["id"], nodes[1]["id"]
        check(abs(nodes[0]["bounds"][2]-440) < 1 and abs(nodes[1]["bounds"][2]-760) < 1,
              "native bounds match authored inner and outer viewports")
        maps = Path(f"/proc/{app.pid}/maps").read_text()
        paths = sorted({line.split()[-1] for line in maps.splitlines()
                        if "libpax_host.so" in line or "libnative_wheel.so" in line})
        check({Path(p).name for p in paths} == {"libpax_host.so", "libnative_wheel.so"}, "both exact native libraries are mapped")
        root = Path(f"/proc/{app.pid}/root")
        save("identity.json", {"pid": app.pid, "window": window,
                              "libraries": {p: hashlib.sha256((root/p.lstrip('/')).read_bytes()).hexdigest() for p in paths}})
        if args.screenshots:
            Path(str(capture)+".request").touch()
            wait(lambda _: capture.exists() and not Path(str(capture)+".request").exists(), "fixture screenshot")
            capture.rename(output/"initial.png")
        for owner, xy, expected in [("inner", (250,500), {"inner","outer","root"}),
                                     ("outer", (720,580), {"outer","root"})]:
            before = reset(owner)
            wheel(*xy, 5)
            after = snapshot(owner+"-down")
            path(before, after, expected, owner)
            check(after[owner+"_y"] > before[owner+"_y"]+1, owner+": vertical native movement")
            unchanged(before, after, ["outer" if owner == "inner" else "inner"], owner+": unrelated viewport stays still")
            check("dy=1.000" in after["last"] and "modifiers=none" in after["last"], owner+": raw delta and no modifiers reach Rust")
            check(f"at ({xy[0]:.1f}, {xy[1]:.1f})" in after["last"], owner+": window-space logical point reaches Rust")
            wheel(*xy, 4)
            back = snapshot(owner+"-up")
            path(after, back, expected, owner+" reverse")
            check(back[owner+"_y"] < after[owner+"_y"]-1, owner+": reverse movement")
            wheel(*xy, 7)
            horizontal = snapshot(owner+"-right")
            path(back, horizontal, expected, owner+" horizontal")
            check(horizontal[owner+"_x"] > back[owner+"_x"]+1, owner+": horizontal movement")
            click(38, 124)
            time.sleep(.3)
            before = snapshot(owner+"-cancel-before")
            check(before["prevent_native_default"], owner+": authored cancellation is enabled")
            wheel(*xy, 5)
            after = snapshot(owner+"-cancel-after")
            path(before, after, expected, owner+" cancelled")
            unchanged(before, after, ["inner","outer"], owner+": root prevent_default stops native movement")
        for name, xy, expected, moving in [
                ("overlay-inner", (460,450), {"overlay","root"}, None),
                ("overlay-outer", (600,450), {"overlay","root"}, None),
                ("overlay-vector-inner", (418,475), {"overlay","root"}, None),
                ("overlay-vector-outer", (622,475), {"overlay","root"}, None),
                ("inner-rounded-exterior", (108,364), {"outer","root"}, "outer"),
                ("outer-rounded-exterior", (28,264), {"root"}, None)]:
            before = reset(name)
            if name.startswith("overlay-vector-"):
                labels = [n for n in state().get("nodes", [])
                          if n.get("text", "").startswith("ROOT OVERLAY")]
                check(len(labels) == 1 and len(labels[0].get("bounds", [])) == 4,
                      name + ": overlay label bounds are observed")
                x, y, width, height = labels[0]["bounds"]
                check(not (x <= xy[0] < x + width and y <= xy[1] < y + height),
                      name + ": input point is outside native overlay text", point=xy,
                      labelBounds=labels[0]["bounds"])
                check(not before["prevent_native_default"],
                      name + ": cancellation is disabled before vector-only input")
            wheel(*xy, 5)
            after = snapshot(name)
            path(before, after, expected, name)
            unchanged(before, after, [k for k in ("inner","outer") if k != moving], name+": unrelated viewport defaults stay still")
            if moving:
                check(after[moving+"_y"] > before[moving+"_y"]+1, name+": owning native viewport moves")
        before = reset("modifiers")
        point(250,500)
        if _x11_client_pid(int(xdo("getwindowfocus", "-f"))) != app.pid:
            raise RuntimeError("Fixture does not own keyboard focus")
        try:
            xdo("keydown", "ctrl", "shift")
            wheel(250,500,5)
        finally:
            xdo("keyup", "shift", "ctrl")
        after = snapshot("modifiers")
        path(before, after, {"inner","outer","root"}, "modifiers")
        check("Control" in after["last"] and "Shift" in after["last"], "event-time modifiers reach Rust")
        app.terminate()
        check(app.wait(timeout=30) == 0, "fixture closes cleanly")
        log.flush()
        check("gl_refs=0 gl_bytes=0" in (output/"app.log").read_text(), "published GPU images retire on close")
        completed = True
    except Exception as error:
        failure = str(error)
    finally:
        if app and app.poll() is None:
            app.terminate()
            try:
                app.wait(timeout=30)
            except subprocess.TimeoutExpired:
                app.kill()
                app.wait(timeout=10)
                failure = (failure or "") + "; forced cleanup"
        log.close()
        save("checks.json", {"completed": completed, "error": failure, "checks": checks,
                             "probeSHA256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                             "physicalTouchpadQualified": False, "displayFPSQualified": False})
    if not completed:
        raise SystemExit(failure)


if __name__ == "__main__":
    main()
