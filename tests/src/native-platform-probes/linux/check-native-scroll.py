#!/usr/bin/env python3
"""Bounded real X11 scrolling and startup-inclusive native timing diagnostics."""
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
    parser.add_argument("--title", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--seconds", type=int, default=20, choices=range(5, 61))
    parser.add_argument("--batches", type=int, choices=range(1, 201),
                        help="Run an exact input batch count instead of a timed interval")
    parser.add_argument("--scrollers", type=int, required=True)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    report, capture = out / "state.json", out / "capture.png"
    env = dict(os.environ, GSK_RENDERER="ngl", PAX_NATIVE_TEST_REPORT=str(report),
               PAX_NATIVE_TEST_SCREENSHOT=str(capture))
    checks, samples = [], []
    completed, failure = False, None
    log = (out / "app.log").open("w")
    app = NativeProcess(args, env, log, out)

    def check(ok, name):
        checks.append({"check": name, "passed": bool(ok)})
        if not ok:
            raise RuntimeError(name)

    def state():
        if app.poll() is not None:
            raise RuntimeError(f"Application exited {app.returncode}")
        try:
            return json.loads(report.read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            return {}

    def wait(predicate, name, timeout=30):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            value = state()
            if predicate(value):
                return value
            time.sleep(.1)
        raise RuntimeError("Timeout: " + name)

    def xdo(*values):
        return subprocess.check_output(["xdotool", *map(str, values)], text=True, timeout=10).strip()

    def sample(stage):
        value = state()
        stat = Path(f"/proc/{app.pid}/stat").read_text().rsplit(")", 1)[1].split()
        gpu = value.get("runtime", {}).get("gpu", {})
        item = {"stage": stage, "monotonic": time.monotonic(), "frames": value.get("frames"),
                "ticks": value.get("ticks"), "changedFrames": value.get("changedFrames"),
                "backpressure": value.get("backpressure"), "gpu": gpu,
                "glLiveImages": value.get("glLiveImages"), "glLiveBytes": value.get("glLiveBytes"),
                "cpuSeconds": (int(stat[11]) + int(stat[12])) / os.sysconf("SC_CLK_TCK"),
                "rssBytes": int(stat[21]) * os.sysconf("SC_PAGE_SIZE"),
                "scrollers": [{"id": n["id"], "scrollX": n.get("scrollX"),
                               "scrollY": n.get("scrollY"), "bounds": n.get("bounds")}
                              for n in value.get("nodes", []) if n["kind"] == "Scroller"]}
        samples.append(item)
        check(item["rssBytes"] < 4 * 1024**3, "fixture RSS remains below four GiB safety bound")
        check(gpu.get("liveAllocations", 49) <= 48 and gpu.get("bytes", 2**30) <= 256 * 1024**2,
              "Vulkan live and retiring exports remain bounded")
        check(item["glLiveImages"] <= 48 and item["glLiveBytes"] <= 256 * 1024**2,
              "owned GL publications remain bounded")
        return value

    def screenshot(name):
        Path(str(capture) + ".request").touch()
        wait(lambda _: capture.exists() and not Path(str(capture) + ".request").exists(),
             "one-shot capture", 15)
        capture.rename(out / f"{name}.png")

    def settled_scrollers():
        deadline, stable_since, previous = time.monotonic() + 8, None, None
        while time.monotonic() < deadline:
            value = state()
            current = {n["id"]: (n.get("scrollX", 0), n.get("scrollY", 0))
                       for n in value.get("nodes", []) if n["kind"] == "Scroller"}
            if current == previous:
                stable_since = stable_since or time.monotonic()
                if time.monotonic() - stable_since >= .5:
                    return value
            else:
                stable_since = None
            previous = current
            time.sleep(.1)
        raise RuntimeError("Native scrollers did not reach stable offsets within eight seconds")

    try:
        wait(lambda s: sum(n["kind"] == "Scroller" for n in s.get("nodes", [])) == args.scrollers,
             "expected native scrollers", 90)
        window = app.x11_window(args.title)
        xdo("windowsize", "--sync", window, 1280, 960)
        xdo("windowfocus", "--sync", window)
        maps = Path(f"/proc/{app.pid}/maps").read_text()
        names = sorted({line.split()[-1] for line in maps.splitlines()
                        if "libpax_host.so" in line or "lib" + args.app.name.replace("-", "_") + ".so" in line})
        mounted = Path(f"/proc/{app.pid}/root")
        identity = {"pid": app.pid, "app": str(args.app), "window": window,
                    "executableSHA256": hashlib.sha256(Path(f"/proc/{app.pid}/exe").read_bytes()).hexdigest(),
                    "libraries": {n: hashlib.sha256((mounted / n.lstrip("/")).read_bytes()).hexdigest() for n in names}}
        (out / "identity.json").write_text(json.dumps(identity, indent=2) + "\n")
        time.sleep(5)
        initial = sample("after-five-second-warmup")
        (out / "initial-state.json").write_text(json.dumps(initial, indent=2) + "\n")
        screenshot("initial")
        scrollers = sorted([n for n in settled_scrollers()["nodes"] if n["kind"] == "Scroller"],
                           key=lambda n: (n["bounds"][1], n["bounds"][0]))
        initial_offsets = {n["id"]: n["scrollY"] for n in scrollers}
        for target in scrollers:
            x, y, width, height = target["bounds"]
            xdo("mousemove", "--window", window, round(x + width * .6), round(y + height * .6))
            xdo("click", "--repeat", 4, "--delay", 30, 5)
            value = wait(lambda s: any(n["id"] == target["id"] and n.get("scrollY", 0) >
                                      initial_offsets[target["id"]] for n in s.get("nodes", [])),
                         f"real wheel moves scroller {target['id']}")
            value = settled_scrollers()
            (out / f"settled-{target['id']}.json").write_text(json.dumps(value, indent=2) + "\n")
            for node in value["nodes"]:
                if node["kind"] == "Scroller" and node["id"] != target["id"]:
                    check(node["scrollY"] == initial_offsets[node["id"]],
                          "wheel leaves other native scrollers at their previous offsets")
            initial_offsets[target["id"]] = next(n["scrollY"] for n in value["nodes"] if n["id"] == target["id"])
        before = sample("measurement-start")
        (out / "measurement-start.json").write_text(json.dumps({
            "pid": app.pid, "monotonic": samples[-1]["monotonic"],
        }, indent=2) + "\n")
        start = time.monotonic()
        index = 0
        while (index < args.batches if args.batches is not None
               else time.monotonic() - start < args.seconds):
            target = scrollers[index % len(scrollers)]
            x, y, width, height = target["bounds"]
            xdo("mousemove", "--window", window, round(x + width * .6), round(y + height * .6))
            xdo("click", "--repeat", 4, "--delay", 20, 5 if (index // len(scrollers)) % 6 < 4 else 4)
            time.sleep(.15)
            sample("scrolling")
            index += 1
        # The host publishes diagnostics every 250 ms; a single input batch can
        # finish sooner. Include the wait in CPU/wall time so the final frame
        # counters come from a report produced after the input sequence.
        last_reported_frame = state().get("frames", 0)
        wait(lambda s: s.get("frames", 0) > last_reported_frame,
             "fresh post-input frame report")
        final = sample("measurement-end")
        elapsed = samples[-1]["monotonic"] - samples[-index-2]["monotonic"]
        (out / "final-state.json").write_text(json.dumps(final, indent=2) + "\n")
        screenshot("final")
        check(final["frames"] > before["frames"] and final.get("runtime", {}).get("gpu", {}).get("reused", 0) > 0,
              "native delivery advances while synchronized exports are reused")
        (out / "timing-notes.json").write_text(json.dumps({
            "requestedScrollSeconds": None if args.batches is not None else args.seconds,
            "requestedInputBatches": args.batches, "inputBatches": index, "observedSeconds": elapsed,
            "committedFramesDelta": final["frames"] - before["frames"],
            "measurementIncludesFinalReportWait": True,
            "timingDistributionScope": "startup-inclusive; lifetime histogram percentiles cannot be warmup-subtracted",
            "displayRateQualified": False,
            "caveat": "Diagnostic-enabled headless Xorg; callback throughput is not physical display FPS.",
        }, indent=2) + "\n")
        app.terminate()
        check(app.wait(timeout=30) == 0, "scroll fixture closes normally")
        log.flush()
        check("PAX_LINUX_CLOSED" in (out / "app.log").read_text() and
              "gl_refs=0 gl_bytes=0" in (out / "app.log").read_text(), "owned GL publications retired")
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
            (out / "checks.json").write_text(json.dumps({
                "completed": completed, "passed": completed and all(c["passed"] for c in checks),
                "error": failure, "pid": app.pid, "exitCode": app.poll(), "checks": checks,
            }, indent=2) + "\n")


if __name__ == "__main__":
    main()
