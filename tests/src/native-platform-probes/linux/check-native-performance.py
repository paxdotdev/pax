#!/usr/bin/env python3
"""Bounded native animation/idle memory samples, with real pause-button input."""
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
    parser.add_argument("--seconds", type=int, choices=range(10, 301), default=120)
    parser.add_argument("--paused-seconds", type=int, choices=range(10, 301),
                        help="Pause duration per cycle; defaults to --seconds")
    parser.add_argument("--cycles", type=int, choices=range(1, 4), default=1,
                        help="Animation/pause cycles in the same process")
    parser.add_argument("--no-screenshots", action="store_true",
                        help="Keep state snapshots but exclude GPU readback and PNG capture overhead")
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    report, capture = out / "state.json", out / "capture.png"
    env = dict(os.environ, GSK_RENDERER="ngl", PAX_GPU_MEMORY_DIAGNOSTICS="1",
               PAX_NATIVE_TEST_REPORT=str(report), PAX_NATIVE_TEST_SCREENSHOT=str(capture))
    checks, samples = [], []
    completed, failure = False, None
    log = (out / "app.log").open("w")
    app = NativeProcess(args, env, log, out)

    def state():
        if app.poll() is not None:
            raise RuntimeError(f"application exited {app.returncode}")
        try:
            return json.loads(report.read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            return {}

    def check(ok, label):
        checks.append({"check": label, "passed": bool(ok)})
        if not ok:
            raise RuntimeError(label)

    def wait(predicate, label, seconds=30):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            value = state()
            if predicate(value):
                return value
            time.sleep(.1)
        raise RuntimeError("Timeout: " + label)

    def xdo(*values):
        return subprocess.check_output(["xdotool", *map(str, values)], text=True, timeout=10).strip()

    def sample(stage):
        value = state()
        stat = Path(f"/proc/{app.pid}/stat").read_text().rsplit(")", 1)[1].split()
        memory = {}
        for line in Path(f"/proc/{app.pid}/smaps_rollup").read_text().splitlines()[1:]:
            fields = line.split()
            if len(fields) == 3 and fields[2] == "kB":
                memory[fields[0].rstrip(":")] = int(fields[1]) * 1024
        item = {"stage": stage, "monotonic": time.monotonic(), "frames": value.get("frames"),
                "ticks": value.get("ticks"), "changedFrames": value.get("changedFrames"),
                "cleanFrames": value.get("cleanFrames"), "nodeCount": len(value.get("nodes", [])),
                "cpuSeconds": (int(stat[11]) + int(stat[12])) / os.sysconf("SC_CLK_TCK"),
                "rssBytes": int(stat[21]) * os.sysconf("SC_PAGE_SIZE"), "memory": memory,
                "gpu": value.get("runtime", {}).get("gpu", {}),
                "glLiveImages": value.get("glLiveImages"), "glLiveBytes": value.get("glLiveBytes")}
        samples.append(item)
        if item["rssBytes"] >= 4 * 1024**3 or item["gpu"].get("liveAllocations", 49) > 48 or \
                item["gpu"].get("bytes", 2**30) > 256 * 1024**2:
            raise RuntimeError("resource safety bound exceeded")
        return value

    def snapshot(label):
        value = state()
        (out / f"{label}.json").write_text(json.dumps(value, indent=2) + "\n")
        if args.no_screenshots:
            return value
        Path(str(capture) + ".request").touch()
        wait(lambda _: capture.exists() and not Path(str(capture) + ".request").exists(), "screenshot", 15)
        capture.rename(out / f"{label}.png")
        return value

    def phase(name, seconds):
        start = sample(name)
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            time.sleep(min(1, max(0, deadline - time.monotonic())))
            sample(name)
        end = snapshot(name)
        check(end["ticks"] > start["ticks"], name + " keeps native event delivery alive")
        if name.startswith("animated"):
            check(end["changedFrames"] > start["changedFrames"], name + " produces changed frames")

    def toggle_animation(label, expected):
        button = next(n for n in state()["nodes"] if n.get("text") == label)
        x, y, width, height = button["bounds"]
        xdo("mousemove", "--window", window, round(x + width/2), round(y + height/2))
        xdo("click", 1)
        wait(lambda s: any(n.get("text") == expected for n in s.get("nodes", [])),
             "real input: " + label)
        time.sleep(2)

    try:
        wait(lambda s: s.get("frames", 0) > 5, "startup", 90)
        window = app.x11_window("Neon Opacity")
        xdo("windowsize", "--sync", window, 1280, 960)
        xdo("windowfocus", "--sync", window)
        time.sleep(5)
        maps = Path(f"/proc/{app.pid}/maps").read_text()
        names = sorted({line.split()[-1] for line in maps.splitlines()
                        if "libpax_host.so" in line or "libneon_opacity.so" in line})
        mounted = Path(f"/proc/{app.pid}/root")
        (out / "identity.json").write_text(json.dumps({"pid": app.pid, "window": window,
            "executableSHA256": hashlib.sha256(Path(f"/proc/{app.pid}/exe").read_bytes()).hexdigest(),
            "libraries": {n: hashlib.sha256((mounted / n.lstrip("/")).read_bytes()).hexdigest() for n in names}}, indent=2) + "\n")
        initial = snapshot("initial")
        check({"Oxanium", "Space Mono"}.issubset({n.get("resolvedFont") for n in initial["nodes"]}), "canonical fonts resolve")
        for cycle in range(args.cycles):
            suffix = "" if cycle == 0 else f"-{cycle + 1}"
            if cycle:
                toggle_animation("RESUME ANIMATION", "PAUSE ANIMATION")
            phase("animated" + suffix, args.seconds)
            toggle_animation("PAUSE ANIMATION", "RESUME ANIMATION")
            phase("paused" + suffix, args.paused_seconds or args.seconds)
        check(True, "animated and paused memory samples remain within safety bounds")
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
            (out / "samples.json").write_text(json.dumps(samples, indent=2) + "\n")
            (out / "checks.json").write_text(json.dumps({"completed": completed,
                "passed": completed and all(c["passed"] for c in checks), "error": failure,
                "pid": app.pid, "exitCode": app.poll(), "checks": checks}, indent=2) + "\n")
            (out / "timing-notes.json").write_text(json.dumps({"secondsPerPhase": args.seconds,
                "pausedSecondsPerCycle": args.paused_seconds or args.seconds,
                "cycles": args.cycles,
                "screenshots": not args.no_screenshots,
                "histograms": "lifetime/startup-inclusive", "displayRateQualified": False,
                "animationPhase": "ticks/60; equal wall time is not equal animation work",
                "pause": "actual canonical Pause Animation button; native event delivery continues"}, indent=2) + "\n")

if __name__ == "__main__":
    main()
