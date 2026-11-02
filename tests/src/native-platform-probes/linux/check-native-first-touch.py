#!/usr/bin/env python3
"""Qualify the unchanged CLI-created default PaxLogo on a native Linux host."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time
from native_process import NativeProcess, add_launch_arguments


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("app", type=Path); add_launch_arguments(parser)
    parser.add_argument("--title", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(); out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    report, capture = out / "state.json", out / "capture.png"
    env = dict(os.environ, GSK_RENDERER="ngl", PAX_NATIVE_TEST_REPORT=str(report),
               PAX_NATIVE_TEST_SCREENSHOT=str(capture))
    log = (out / "app.log").open("w"); app = NativeProcess(args, env, log, out)
    checks = []; completed = False; failure = None
    def check(ok, label):
        checks.append({"check": label, "passed": bool(ok)}); print("PASS" if ok else "FAIL", label, flush=True)
        if not ok: raise RuntimeError(label)
    def state():
        if app.poll() is not None: raise RuntimeError(f"application exited {app.returncode}")
        try: return json.loads(report.read_text())
        except (FileNotFoundError, json.JSONDecodeError): return {}
    def wait(predicate, label, seconds=20):
        until = time.monotonic() + seconds
        while time.monotonic() < until:
            value = state()
            if predicate(value): return value
            time.sleep(.05)
        raise RuntimeError("Timeout: " + label)
    def xdo(*values):
        return subprocess.check_output(["xdotool", *map(str, values)], text=True, timeout=10).strip()
    def click(node):
        x,y,w,h = node["bounds"]
        xdo("mousemove", "--window", window, round(x+w/2), round(y+h/2)); xdo("click", 1)
    def find(value, text): return next(n for n in value["nodes"] if n.get("text") == text)
    def slider(value): return next(n for n in value["nodes"] if n["kind"] == "Slider")
    def snapshot(label):
        value = state(); (out / (label + ".json")).write_text(json.dumps(value, indent=2) + "\n")
        Path(str(capture) + ".request").touch()
        wait(lambda _: capture.exists() and not Path(str(capture) + ".request").exists(), "screenshot")
        capture.rename(out / (label + ".png")); return value
    try:
        wait(lambda s: any(n.get("text") == "Replay" for n in s.get("nodes", [])), "fresh scaffold startup", 90)
        window = app.x11_window(args.title); xdo("windowsize", "--sync", window, 1024, 768); xdo("windowfocus", "--sync", window)
        value = wait(lambda s: slider(s)["value"] >= .99, "initial logo animation completes")
        maps = Path(f"/proc/{app.pid}/maps").read_text(); mount = Path(f"/proc/{app.pid}/root")
        names = sorted({line.split()[-1] for line in maps.splitlines() if "libpax_host.so" in line or "liblinux_alpha_first_touch.so" in line})
        (out / "identity.json").write_text(json.dumps({"pid": app.pid, "window": window,
            "executableSHA256": hashlib.sha256(Path(f"/proc/{app.pid}/exe").read_bytes()).hexdigest(),
            "libraries": {n: hashlib.sha256((mount / n.lstrip("/")).read_bytes()).hexdigest() for n in names}}, indent=2) + "\n")
        check(slider(value)["controlType"] == "GtkScale", "fresh default scaffold creates genuine native slider")
        replay = find(value, "Replay")
        check(replay["resolvedFont"] == "Roboto" and replay["textLayout"]["unknownGlyphs"] == 0,
              "unchanged default text resolves bundled Roboto glyphs")
        check(Path(replay["resolvedFontFile"]).name == "Roboto.ttf",
              "default Replay glyphs use the packaged normal face file")
        actual_font = mount / replay["resolvedFontFile"].lstrip("/")
        packaged_font = next((args.app.parent / "assets").rglob("Roboto.ttf"))
        check(hashlib.sha256(actual_font.read_bytes()).hexdigest() == hashlib.sha256(packaged_font.read_bytes()).hexdigest(),
              "loaded default font bytes match the relocated package asset")
        snapshot("light")
        click(find(value, "Use dark background"))
        wait(lambda s: any(n.get("text") == "Use light background" for n in s["nodes"]), "native theme button event")
        check(True, "native theme button updates the unchanged scaffold")
        click(slider(state())); xdo("key", "Home")
        wait(lambda s: abs(slider(s)["value"]) < .0001, "native slider Home")
        time.sleep(.5)
        check(abs(slider(state())["value"]) < .0001, "native slider input cancels animation and preserves zero")
        xdo("key", "End"); wait(lambda s: slider(s)["value"] >= .99, "native slider End")
        check(True, "native slider supports full authored range")
        click(find(state(), "Replay")); wait(lambda s: slider(s)["value"] < .5, "vector-group replay click restarts animation")
        wait(lambda s: slider(s)["value"] >= .99, "replay animation completes")
        check(True, "vector-group click and reactive animation remain connected to native slider")
        xdo("mousemove", "--window", window, 8, 8); snapshot("dark-replayed")
        app.terminate(); check(app.wait(timeout=30) == 0, "fresh scaffold closes normally")
        log.flush(); text = (out / "app.log").read_text()
        check("PAX_LINUX_CLOSED" in text and "gl_refs=0 gl_bytes=0" in text, "owned GL publications retired")
        check("Too many gtk_snapshot_" not in text, "fresh scaffold snapshots preserve native stack balance")
        completed = True
    except Exception as error: failure = str(error); raise
    finally:
        try:
            if app.poll() is None: app.terminate(); app.wait(timeout=30)
        finally:
            log.close(); (out / "checks.json").write_text(json.dumps({"completed": completed,
                "passed": completed and all(c["passed"] for c in checks), "error": failure,
                "pid": app.pid, "exitCode": app.poll(), "checks": checks}, indent=2) + "\n")


if __name__ == "__main__": main()
