#!/usr/bin/env python3
"""Native text alignment, reactive styling, and edit preservation fixture."""
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
    parser.add_argument("app", type=Path)
    add_launch_arguments(parser)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    report, capture = out / "state.json", out / "capture.png"
    env = dict(os.environ, GSK_RENDERER="ngl", PAX_NATIVE_TEST_REPORT=str(report),
               PAX_NATIVE_TEST_SCREENSHOT=str(capture))
    checks = []
    completed, failure = False, None
    log = (out / "app.log").open("w")
    app = NativeProcess(args, env, log, out)

    def check(ok, name):
        checks.append({"check": name, "passed": bool(ok)})
        print("PASS" if ok else "FAIL", name, flush=True)
        if not ok:
            raise RuntimeError(name)

    def state():
        if app.poll() is not None:
            raise RuntimeError(f"application exited {app.returncode}")
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

    def xdo(*values):
        return subprocess.check_output(["xdotool", *map(str, values)], text=True, timeout=10).strip()

    def click(node):
        x, y, w, h = node["bounds"]
        xdo("mousemove", "--window", window, round(x+w/2), round(y+h/2))
        xdo("click", 1)
        time.sleep(.25)

    def snapshot(label):
        value = state()
        (out / f"{label}.json").write_text(json.dumps(value, indent=2) + "\n")
        Path(str(capture) + ".request").touch()
        wait(lambda _: capture.exists() and not Path(str(capture) + ".request").exists(), "screenshot")
        capture.rename(out / f"{label}.png")
        return value

    def alignment(value, label):
        singles = sorted([n for n in value["nodes"] if n.get("text") == "Native text"], key=lambda n:n["bounds"][0])
        multis = sorted([n for n in value["nodes"] if n.get("text", "").startswith("A longer first line")], key=lambda n:n["bounds"][0])
        check(len(singles) == 3 and len(multis) == 3, label + " has six native swatches")
        for index, node in enumerate(singles):
            layout = node["textLayout"]
            x, y, w, h = layout["logicalBounds"]
            width, height = node["bounds"][2:]
            check(abs(x - (width-w)*index/2) <= 2 and abs(y - (height-h)*index/2) <= 2,
                  f"{label}: single swatch{index} applies authored block alignment")
            check(layout["unknownGlyphs"] == 0 and not layout["ellipsized"], label + " single glyphs are present and not ellipsized")
        for index, node in enumerate(multis):
            layout = node["textLayout"]
            x, y, w, h = layout["logicalBounds"]
            width, height = node["bounds"][2:]
            check(abs(x - (width-w)*(2-index)/2) <= 2 and abs(y - (height-h)/2) <= 2,
                  f"{label}: multiline swatch{index} independently applies block alignment")
            lines = layout["lineLogicalBounds"]
            check(len(lines) == 2, label + " multiline swatch has two actual Pango lines")
            anchors = [line[0] + line[2]*index/2 for line in lines]
            check(abs(anchors[0] - anchors[1]) <= 2,
                  f"{label}: multiline swatch{index} independently applies line alignment")

    try:
        wait(lambda s: any(n.get("text") == "Change styles" for n in s.get("nodes", [])), "startup", 90)
        window = app.x11_window("Pax native text styling")
        xdo("windowsize", "--sync", window, 1024, 840)
        xdo("windowfocus", "--sync", window)
        time.sleep(1)
        mapped = Path(f"/proc/{app.pid}/maps").read_text()
        names = sorted({line.split()[-1] for line in mapped.splitlines() if "libpax_host.so" in line or "libnative_text_style.so" in line})
        mounted = Path(f"/proc/{app.pid}/root")
        (out / "identity.json").write_text(json.dumps({"pid": app.pid, "window": window,
            "executableSHA256": hashlib.sha256(Path(f"/proc/{app.pid}/exe").read_bytes()).hexdigest(),
            "libraries": {n: hashlib.sha256((mounted / n.lstrip("/")).read_bytes()).hexdigest() for n in names}}, indent=2) + "\n")
        initial = snapshot("initial")
        alignment(initial, "initial")
        editors = sorted([n for n in initial["nodes"] if n["kind"] == "Textbox"], key=lambda n:n["bounds"][1])
        check([n["controlType"] for n in editors] == ["GtkEntry", "GtkTextView"], "editors remain genuine single/multiline native widgets")
        ids = [n["id"] for n in editors]
        for index, editor in enumerate(editors):
            click(editor)
            xdo("key", "ctrl+a")
            xdo("type", "--clearmodifiers", "--delay", 8, f"Edited native field {index}")
            wait(lambda s: any(n["id"] == ids[index] and n.get("text") == f"Edited native field {index}" for n in s["nodes"]), "native edits reach Rust binding")
        click(next(n for n in state()["nodes"] if n.get("text") == "Change styles"))
        wait(lambda s: any(n.get("text") == "Restore styles" for n in s["nodes"]), "reactive style toggle")
        alternate = snapshot("alternate")
        alignment(alternate, "alternate")
        for identity in ids:
            editor = next(n for n in alternate["nodes"] if n["id"] == identity)
            style = editor["textStyle"]
            check(style["weight"] == 700 and style["slant"] == 2 and style["underline"] == 1,
                  "reactive editor font attributes reach the native text engine")
            check(abs(style["sizePoints"] - 15) < .01 and
                  all(abs(a-b/255) < .001 for a,b in zip(style["fill"], [145,36,74,255])),
                  "reactive editor point size and native foreground match authored style")
        check(all(any(n["id"] == identity and n.get("text") == f"Edited native field {index}" for n in alternate["nodes"]) for index,identity in enumerate(ids)), "style updates preserve edited content and native identity")
        click(next(n for n in state()["nodes"] if n.get("text") == "Restore styles"))
        wait(lambda s: any(n.get("text") == "Change styles" for n in s["nodes"]), "style reset")
        restored = snapshot("restored")
        alignment(restored, "restored")
        for identity in ids:
            style = next(n for n in restored["nodes"] if n["id"] == identity)["textStyle"]
            check(style["weight"] == 400 and style["slant"] == 0 and style["underline"] == 0,
                  "restoring styles clears native weight/slant/underline")
        app.terminate()
        check(app.wait(timeout=30) == 0, "fixture closes normally")
        log.flush()
        check("PAX_LINUX_CLOSED" in (out / "app.log").read_text() and "gl_refs=0 gl_bytes=0" in (out / "app.log").read_text(), "owned GL publications retired")
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
            (out / "checks.json").write_text(json.dumps({"completed": completed,
                "passed": completed and all(c["passed"] for c in checks), "error": failure,
                "pid": app.pid, "exitCode": app.poll(), "checks": checks}, indent=2) + "\n")

if __name__ == "__main__":
    main()
