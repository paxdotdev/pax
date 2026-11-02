#!/usr/bin/env python3
"""Canonical scrolling/opacity fixture diagnostics; not a display-FPS benchmark."""
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
    parser.add_argument("--fixture", choices=("scroll-matrix", "neon-opacity"), required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--check-neon-surfaces", action="store_true",
                        help="Use real controls to pause at full opacity and check authored surface pixels")
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    report, capture = out / "state.json", out / "capture.png"
    env = dict(os.environ, GSK_RENDERER="ngl", PAX_NATIVE_TEST_REPORT=str(report),
               PAX_NATIVE_TEST_SCREENSHOT=str(capture))
    log = (out / "app.log").open("w")
    app = NativeProcess(args, env, log, out)
    checks, samples = [], []
    completed, error = False, None

    def check(ok, label):
        checks.append({"check": label, "passed": bool(ok)})
        print("PASS" if ok else "FAIL", label, flush=True)
        if not ok:
            raise RuntimeError(label)

    def state():
        if app.poll() is not None:
            raise RuntimeError(f"application exited {app.returncode}")
        try:
            return json.loads(report.read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            return {}

    def wait(predicate, label, seconds=15):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            value = state()
            if predicate(value):
                return value
            time.sleep(.1)
        raise RuntimeError("Timeout: " + label)

    def xdo(*values):
        return subprocess.check_output(["xdotool", *map(str, values)],
                                       text=True, timeout=10).strip()

    def snapshot(label):
        value = state()
        (out / f"{label}.json").write_text(json.dumps(value, indent=2) + "\n")
        Path(str(capture) + ".request").touch()
        wait(lambda _: capture.exists() and not Path(str(capture) + ".request").exists(),
             "one-shot screenshot")
        capture.rename(out / f"{label}.png")
        return value

    def node(value, identity):
        return next(n for n in value["nodes"] if n["id"] == identity)

    def wheel(target, direction):
        x, y, w, h = target["bounds"]
        xdo("mousemove", "--window", window, round(x + .7 * w), round(y + .65 * h))
        xdo("click", "--repeat", 4, "--delay", 30, direction)

    def sample():
        value = state()
        stat = Path(f"/proc/{app.pid}/stat").read_text().rsplit(")", 1)[1].split()
        gpu = value["runtime"]["gpu"]
        item = {"monotonic": time.monotonic(), "frames": value["frames"],
                "ticks": value["ticks"], "changedFrames": value.get("changedFrames"),
                "rssBytes": int(stat[21]) * os.sysconf("SC_PAGE_SIZE"),
                "cpuSeconds": (int(stat[11]) + int(stat[12])) / os.sysconf("SC_CLK_TCK"),
                "gpu": gpu, "glLiveImages": value["glLiveImages"],
                "glLiveBytes": value["glLiveBytes"], "backpressure": value["backpressure"]}
        samples.append(item)
        if item["rssBytes"] >= 4 * 1024**3 or gpu["liveAllocations"] > 48 or \
                gpu["bytes"] > 256 * 1024**2 or item["glLiveImages"] > 48 or \
                item["glLiveBytes"] > 256 * 1024**2:
            raise RuntimeError("resource safety bound exceeded")
        return value

    try:
        wait(lambda s: s.get("frames", 0) > 5, "canonical startup", 60)
        title = "Scroll Matrix" if args.fixture == "scroll-matrix" else "Neon Opacity"
        window = app.x11_window(title)
        xdo("windowsize", "--sync", window, 1280, 960)
        xdo("windowfocus", "--sync", window)
        time.sleep(3)
        maps = Path(f"/proc/{app.pid}/maps").read_text()
        libraries = sorted({r.split()[-1] for r in maps.splitlines()
                            if "libpax_host.so" in r or
                            "lib" + args.fixture.replace("-", "_") + ".so" in r})
        mount = Path(f"/proc/{app.pid}/root")
        (out / "identity.json").write_text(json.dumps({
            "pid": app.pid, "window": window,
            "executableSHA256": hashlib.sha256(Path(f"/proc/{app.pid}/exe").read_bytes()).hexdigest(),
            "libraries": {p: hashlib.sha256((mount / p.lstrip("/")).read_bytes()).hexdigest()
                          for p in libraries}}, indent=2) + "\n")
        initial = snapshot("initial")
        kinds = {n["kind"] for n in initial["nodes"]}
        check({"Slider", "Dropdown", "RadioList"}.issubset(kinds),
              "canonical fixture creates all genuine native form families")
        expected_fonts = ({"Fraunces", "Azeret Mono"} if args.fixture == "scroll-matrix"
                          else {"Oxanium", "Space Mono"})
        fonts = {n.get("resolvedFont") for n in initial["nodes"]}
        check(expected_fonts.issubset(fonts), "bundled canonical font families actually resolve in Pango")
        if args.fixture == "neon-opacity":
            check(any(n.get("maskEntries", 0) for n in initial["nodes"]),
                  "visible animated fixture carries native composition masks")
        if args.fixture == "scroll-matrix":
            scrollers = [n for n in initial["nodes"] if n["kind"] == "Scroller"]
            outer = min(scrollers, key=lambda n: n["bounds"][1])
            vertical = min((n for n in scrollers if n["id"] != outer["id"]),
                           key=lambda n: n["bounds"][1])
            horizontal = min((n for n in scrollers if n["bounds"][1] > vertical["bounds"][1] + 100),
                             key=lambda n: n["bounds"][1])
            wheel(vertical, 5)
            moved = wait(lambda s: node(s, vertical["id"])["scrollY"] > 0, "vertical axis scroll")
            check(node(moved, vertical["id"])["scrollX"] == 0 and node(moved, outer["id"])["scrollY"] == 0,
                  "vertical wheel moves the inner vertical pane without sideways or outer drift")
            wheel(horizontal, 7)
            moved = wait(lambda s: node(s, horizontal["id"])["scrollX"] > 0, "horizontal axis scroll")
            check(node(moved, horizontal["id"])["scrollY"] == 0 and node(moved, outer["id"])["scrollY"] == 0,
                  "horizontal wheel moves the horizontal rail without vertical or outer drift")
            snapshot("axis-locks")
            # The outer margin is outside every nested card viewport.
            xdo("mousemove", "--window", window, 8, 900)
            xdo("click", "--repeat", 12, "--delay", 40, 5)
            wait(lambda s: node(s, outer["id"])["scrollY"] > 0, "outer scroller input")
            snapshot("outer-scrolled")
        before = sample()
        for _ in range(20):
            time.sleep(.5)
            sample()
        final = snapshot("final")
        check(final["frames"] > before["frames"] and final["runtime"]["gpu"]["reused"] > 0,
              "frame delivery progresses and synchronized GPU exports are reused")
        check(True, "RSS and owned GPU publications stay within bounded safety limits")
        (out / "timing-notes.json").write_text(json.dumps({
            "distributionScope": "lifetime/startup-inclusive; no percentile warmup subtraction",
            "displayRateQualified": False,
            "animationPhase": "neon-opacity advances by ticks/60, not elapsed wall time",
            "limitations": "Headless Xorg, diagnostics enabled. Native mask presence is not a complete final-pixel oracle.",
        }, indent=2) + "\n")
        if args.check_neon_surfaces:
            if args.fixture != "neon-opacity":
                raise RuntimeError("--check-neon-surfaces requires neon-opacity")
            def click(target):
                x, y, w, h = target["bounds"]
                xdo("mousemove", "--window", window, round(x+w/2), round(y+h/2))
                xdo("click", 1)
            click(next(n for n in state()["nodes"] if n.get("text") == "PAUSE ANIMATION"))
            wait(lambda s: any(n.get("text") == "RESUME ANIMATION" for n in s["nodes"]), "pause actual animation")
            slider = next(n for n in state()["nodes"] if n["kind"] == "Slider")
            click(slider); xdo("key", "End")
            wait(lambda s: abs(node(s, slider["id"])["value"] - 1) < .0001 and
                 any(n.get("text") == "BEAM 100%" for n in s["nodes"]), "full authored group opacity")
            xdo("mousemove", "--window", window, 10, 10)
            time.sleep(.5)
            opaque = snapshot("opaque-surfaces")
            import gi
            gi.require_version("GdkPixbuf", "2.0")
            from gi.repository import GdkPixbuf
            pixbuf = GdkPixbuf.Pixbuf.new_from_file(str(out / "opaque-surfaces.png"))
            pixels, stride, channels = pixbuf.get_pixels(), pixbuf.get_rowstride(), pixbuf.get_n_channels()
            observed = []
            for target, expected in [
                    (next(n for n in opaque["nodes"] if n["kind"] == "Textbox"), (8, 12, 32)),
                    (next(n for n in opaque["nodes"] if n.get("text", "").startswith("ENGAGE ")), (17, 20, 58)),
                    (next(n for n in opaque["nodes"] if n.get("text") == "RESUME ANIMATION"), (17, 20, 58))]:
                x, y, w, h = target["bounds"]
                matching = 0
                for py in range(max(0, round(y)), min(pixbuf.get_height(), round(y+h))):
                    for px in range(max(0, round(x)), min(pixbuf.get_width(), round(x+w))):
                        offset = py * stride + px * channels
                        matching += all(abs(pixels[offset+c] - expected[c]) <= 1 for c in range(3))
                observed.append({"id": target["id"], "kind": target["kind"], "text": target.get("text"),
                                 "expectedRGB": expected, "matchingPixels": matching})
                check(matching >= 64, "actual Neon " + target["kind"] + " contains authored surface color")
            (out / "surface-pixels.json").write_text(json.dumps(observed, indent=2) + "\n")
        app.terminate()
        check(app.wait(timeout=30) == 0, "canonical fixture closes normally")
        log.flush()
        check("PAX_LINUX_CLOSED" in (out / "app.log").read_text() and
              "gl_refs=0 gl_bytes=0" in (out / "app.log").read_text(),
              "owned GL publications retired")
        check("Too many gtk_snapshot_" not in (out / "app.log").read_text(),
              "native snapshots keep balanced clip and transform states")
        completed = True
    except Exception as failure:
        error = str(failure)
        raise
    finally:
        try:
            if app.poll() is None:
                app.terminate()
                app.wait(timeout=30)
        finally:
            log.close()
            (out / "samples.json").write_text(json.dumps(samples, indent=2) + "\n")
            (out / "checks.json").write_text(json.dumps({
                "completed": completed, "passed": completed and all(c["passed"] for c in checks),
                "error": error, "checks": checks, "pid": app.pid, "exitCode": app.poll()
            }, indent=2) + "\n")


if __name__ == "__main__":
    main()
