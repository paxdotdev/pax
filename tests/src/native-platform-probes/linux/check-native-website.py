#!/usr/bin/env python3
"""Observe the packaged website using real X11 input and opt-in host reports.

Reports separate process startup, stationary animation, first traversal and
repeat traversal. They do not establish physical-display FPS or full-site parity.
"""
import argparse
import ctypes
import ctypes.util
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import time

from native_process import NativeProcess, add_launch_arguments, _x11_client_pid
from website_interactions import WebsiteInteractions


def pointer_window():
    """Read the actual X11 root child, without window-manager client heuristics."""
    x = ctypes.CDLL(ctypes.util.find_library("X11"))
    x.XOpenDisplay.argtypes = [ctypes.c_char_p]
    x.XOpenDisplay.restype = ctypes.c_void_p
    x.XCloseDisplay.argtypes = [ctypes.c_void_p]
    x.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
    x.XDefaultRootWindow.restype = ctypes.c_ulong
    x.XQueryPointer.argtypes = [ctypes.c_void_p, ctypes.c_ulong,
                               ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_ulong),
                               *([ctypes.POINTER(ctypes.c_int)] * 4),
                               ctypes.POINTER(ctypes.c_uint)]
    x.XQueryPointer.restype = ctypes.c_int
    display = x.XOpenDisplay(None)
    if not display:
        raise RuntimeError("Cannot open X11 display for pointer ownership")
    try:
        root, child = ctypes.c_ulong(), ctypes.c_ulong()
        rx, ry, wx, wy = (ctypes.c_int() for _ in range(4))
        mask = ctypes.c_uint()
        same_screen = x.XQueryPointer(display, x.XDefaultRootWindow(display),
                                     ctypes.byref(root), ctypes.byref(child),
                                     ctypes.byref(rx), ctypes.byref(ry),
                                     ctypes.byref(wx), ctypes.byref(wy), ctypes.byref(mask))
        return {"sameScreen": bool(same_screen), "root": root.value,
                "window": child.value, "x": rx.value, "y": ry.value}
    finally:
        x.XCloseDisplay(display)


def clipboard_window():
    """Inspect CLIPBOARD ownership without reading another client's contents."""
    x = ctypes.CDLL(ctypes.util.find_library("X11"))
    x.XOpenDisplay.argtypes = [ctypes.c_char_p]
    x.XOpenDisplay.restype = ctypes.c_void_p
    x.XCloseDisplay.argtypes = [ctypes.c_void_p]
    x.XInternAtom.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int]
    x.XInternAtom.restype = ctypes.c_ulong
    x.XGetSelectionOwner.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
    x.XGetSelectionOwner.restype = ctypes.c_ulong
    display = x.XOpenDisplay(None)
    if not display:
        raise RuntimeError("Cannot open X11 display for clipboard ownership")
    try:
        selection = x.XInternAtom(display, b"CLIPBOARD", 1)
        return x.XGetSelectionOwner(display, selection) if selection else 0
    finally:
        x.XCloseDisplay(display)


def read_owned_clipboard(pid):
    owner = clipboard_window()
    if not owner or _x11_client_pid(owner) != pid:
        raise RuntimeError("CLIPBOARD is not owned by the website; refusing to read it")
    # Bytes preserve Unicode, trailing newlines and CR/LF exactly. xclip handles
    # X11's incremental transfer protocol for source files larger than a reply.
    value = subprocess.check_output(
        ["xclip", "-selection", "clipboard", "-out", "-target", "UTF8_STRING"],
        timeout=10)
    if clipboard_window() != owner or _x11_client_pid(owner) != pid:
        raise RuntimeError("CLIPBOARD ownership changed during the source copy check")
    value.decode("utf-8", errors="strict")
    return value, owner


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("app", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--seconds", type=int, default=30, choices=range(10, 301),
                        help="Stationary observation and maximum duration of each traversal phase")
    parser.add_argument("--stationary-seconds", type=int, choices=range(10, 3901),
                        help="Override only stationary observation for a bounded memory soak")
    parser.add_argument("--stationary-only", action="store_true",
                        help="Observe the hero and close without traversal or application input")
    parser.add_argument("--post-eviction-seconds", type=int, choices=range(600, 901),
                        help="End a stationary-only soak this long after the first logged resource-cache eviction")
    parser.add_argument("--traversals", type=int, default=2, choices=(1, 2),
                        help="Use one for a first-traversal diagnostic; acceptance also needs repetition")
    parser.add_argument("--startup-only", action="store_true")
    parser.add_argument("--screenshots", action="store_true",
                        help="Capture phase boundaries; omit for memory comparisons")
    parser.add_argument("--open-source", action="store_true",
                        help="Check the first View Source drawer, including native copy (requires xclip)")
    parser.add_argument("--interaction-cases", nargs="+", default=[],
                        choices=("calculator", "paxflix", "gallery", "resize", "source-tabs", "route"),
                        help="Run specified real-input application checks after timing phases")
    parser.add_argument("--interactions-only", action="store_true",
                        help="Keep startup/identity/cleanup checks but skip stationary and traversal timing phases")
    add_launch_arguments(parser)
    args = parser.parse_args()
    if args.interactions_only and (not args.interaction_cases or args.startup_only):
        parser.error("--interactions-only requires --interaction-cases and cannot use --startup-only")
    if args.stationary_only and (args.startup_only or args.interactions_only or args.interaction_cases or args.open_source):
        parser.error("--stationary-only cannot be combined with startup-only or interaction checks")
    if args.post_eviction_seconds and (not args.stationary_only or args.screenshots or
                                      os.environ.get("PAX_GPU_MEMORY_DIAGNOSTICS") != "1"):
        parser.error("--post-eviction-seconds requires --stationary-only, PAX_GPU_MEMORY_DIAGNOSTICS=1, and no screenshots")
    if (args.open_source or "source-tabs" in args.interaction_cases) and not args.startup_only and not shutil.which("xclip"):
        parser.error("Source clipboard checks require host-side xclip")
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    report, capture = out / "state.json", out / "capture.png"
    env = dict(os.environ, GSK_RENDERER="ngl", PAX_NATIVE_TEST_REPORT=str(report),
               PAX_NATIVE_TEST_SCREENSHOT=str(capture))
    samples, phases, milestones, eviction_observation = [], [], {}, {}
    completed, failure, app = False, None, None
    log = (out / "app.log").open("w")
    started_ns = time.monotonic_ns()
    started_unix_ns = time.time_ns()
    anchor_read_end_ns = time.monotonic_ns()
    started = started_ns / 1e9
    probe_hash = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()

    def save(name, value):
        (out / name).write_text(json.dumps(value, indent=2) + "\n")

    def state():
        if app.poll() is not None:
            raise RuntimeError(f"Application exited {app.returncode}; see app.log")
        stat = Path(f"/proc/{app.pid}/stat").read_text().rsplit(")", 1)[1].split()
        if int(stat[21]) * os.sysconf("SC_PAGE_SIZE") > 6 * 1024**3:
            raise RuntimeError("Website process exceeded the six-GiB probe safety bound")
        try:
            return json.loads(report.read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            return {}

    def wait(predicate, description, timeout=30):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            value = state()
            if predicate(value):
                return value
            time.sleep(.1)
        raise RuntimeError("Timed out: " + description)

    def xdo(*values):
        return subprocess.check_output(["xdotool", *map(str, values)], text=True,
                                       timeout=10).strip()

    def sample(phase):
        value = state()
        stat = Path(f"/proc/{app.pid}/stat").read_text().rsplit(")", 1)[1].split()
        summary = {key: value.get(key) for key in
                   ("frames", "ticks", "changedFrames", "cleanFrames", "backpressure",
                    "glLiveImages", "glLiveBytes", "timings", "runtime",
                    "sceneReconciliations", "scenePlacementVisits",
                    "nativeImageSources", "nativeImageBytes",
                    "nativeImagePending", "nativeImageQueued")}
        summary["nativeNodeCount"] = len(value.get("nodes", []))
        summary["scrollers"] = [{key: node.get(key) for key in
                                 ("id", "scrollX", "scrollY", "scrollUpper", "scrollPage")}
                                for node in value.get("nodes", []) if node["kind"] == "Scroller"]
        item = {"phase": phase, "elapsedSeconds": time.monotonic() - started,
                "cpuSeconds": (int(stat[11]) + int(stat[12])) / os.sysconf("SC_CLK_TCK"),
                "rssBytes": int(stat[21]) * os.sysconf("SC_PAGE_SIZE"), "state": summary}
        samples.append(item)
        # This is a stop safeguard, not an alpha acceptance memory budget.
        if item["rssBytes"] > 6 * 1024**3:
            raise RuntimeError("Website process exceeded the six-GiB probe safety bound")
        return item

    def fresh():
        before = state().get("frames", 0)
        return wait(lambda s: s.get("frames", 0) > before, "fresh host report")

    def screenshot(name):
        if not args.screenshots:
            return
        Path(str(capture) + ".request").touch()
        wait(lambda _: capture.exists() and not Path(str(capture) + ".request").exists(),
             "explicit screenshot", 30)
        capture.rename(out / f"{name}.png")

    def phase(name, action=None, seconds=None):
        fresh()
        first = sample(name + "-start")
        deadline, index = time.monotonic() + (seconds or args.seconds), 0
        log_offset, log_pending, next_log_read = 0, "", 0
        while time.monotonic() < deadline:
            if action and action(index) is False:
                break
            time.sleep(.5)
            sample(name)
            index += 1
            if name == "top-stationary" and args.post_eviction_seconds:
                now = time.monotonic()
                if not eviction_observation and now >= next_log_read:
                    # Read only newly appended diagnostics. Re-reading the full
                    # log would add increasing observer work to a memory soak.
                    with (out / "app.log").open() as reader:
                        reader.seek(log_offset)
                        lines = (log_pending + reader.read(1024 * 1024)).split("\n")
                        log_offset, log_pending = reader.tell(), lines.pop()
                    next_log_read = now + 5
                    for line in lines:
                        match = re.search(r"resource_evictions_total=(\d+)", line)
                        if "[PaxGpuCacheMemory]" in line and match and int(match[1]) > 0:
                            eviction_observation.update(firstObservedElapsedSeconds=now - started,
                                                        sourceLine=line, pollSeconds=5,
                                                        requestedPostEvictionSeconds=args.post_eviction_seconds)
                            save("cache-eviction-observation.json", eviction_observation)
                            break
                if eviction_observation and now - started - eviction_observation["firstObservedElapsedSeconds"] >= args.post_eviction_seconds:
                    break
        fresh()
        last = sample(name + "-end")
        a, b = first["state"], last["state"]
        delta = {"phase": name, "seconds": last["elapsedSeconds"] - first["elapsedSeconds"],
                 "inputBatches": index if action else 0,
                 "cpuSeconds": last["cpuSeconds"] - first["cpuSeconds"],
                 "rssStart": first["rssBytes"], "rssEnd": last["rssBytes"],
                 "committedFrames": b.get("frames", 0) - a.get("frames", 0),
                 "changedFrames": b.get("changedFrames", 0) - a.get("changedFrames", 0)}
        for counter in ("sceneReconciliations", "scenePlacementVisits"):
            if a.get(counter) is not None and b.get(counter) is not None:
                delta[counter] = b[counter] - a[counter]
        for key in ("callbackWork", "callbackInterval"):
            left = a.get("timings", {}).get(key, {})
            right = b.get("timings", {}).get(key, {})
            count = right.get("count", 0) - left.get("count", 0)
            delta[key + "MeanMs"] = ((right.get("meanMs", 0) * right.get("count", 0)
                                       - left.get("meanMs", 0) * left.get("count", 0))
                                      / count if count else None)
        phases.append(delta)
        save(name + "-state.json", state())
        if name == "top-stationary" and args.post_eviction_seconds:
            eviction_observation["stationaryEndElapsedSeconds"] = last["elapsedSeconds"]
            eviction_observation["postEvictionObservedSeconds"] = (
                last["elapsedSeconds"] - eviction_observation["firstObservedElapsedSeconds"]
                if "firstObservedElapsedSeconds" in eviction_observation else 0)
            eviction_observation["postEvictionWindowComplete"] = eviction_observation["postEvictionObservedSeconds"] >= args.post_eviction_seconds
            save("cache-eviction-observation.json", eviction_observation)

    try:
        app = NativeProcess(args, env, log, out)
        milestones["pid"] = app.pid
        value = wait(lambda s: s.get("frames", 0) > 0, "first committed frame report", 120)
        milestones["firstCommittedReportSeconds"] = time.monotonic() - started
        if app.flatpak:
            # XRes identifies the server-side host PID across Flatpak namespaces.
            windows = xdo("search", "--onlyvisible", "--name", ".*").splitlines()
            windows = [w for w in windows if _x11_client_pid(w) == app.pid]
            candidates = []
            for candidate in windows:
                geometry = dict(line.split("=", 1) for line in
                                xdo("getwindowgeometry", "--shell", candidate).splitlines())
                candidates.append({"window": candidate, "geometry": geometry,
                                   "title": xdo("getwindowname", candidate)})
            save("window-candidates.json", candidates)
            # GTK can map a 1x1 auxiliary X11 window with the same client PID.
            # Only an owned application-size surface can receive our input.
            windows = [c["window"] for c in candidates
                       if int(c["geometry"]["WIDTH"]) > 1 and int(c["geometry"]["HEIGHT"]) > 1]
        else:
            windows = xdo("search", "--onlyvisible", "--pid", app.pid).splitlines()
        if len(windows) != 1:
            raise RuntimeError(f"Expected one visible application window, found {windows}")
        window = windows[0]
        milestones["mappedWindowObservedSeconds"] = time.monotonic() - started
        milestones["initialSceneBounds"] = value.get("sceneBoundsInSurface")
        milestones["initialNativeNodeCount"] = len(value.get("nodes", []))
        maps = Path(f"/proc/{app.pid}/maps").read_text()
        libraries = sorted({line.split()[-1] for line in maps.splitlines()
                            if "libpax_host.so" in line or "libpax_website.so" in line})
        if {Path(n).name for n in libraries} != {"libpax_host.so", "libpax_website.so"}:
            raise RuntimeError("Could not identify both loaded website and host libraries")
        mounted = Path(f"/proc/{app.pid}/root")
        save("identity.json", {"pid": app.pid, "app": str(args.app.resolve()), "window": window,
                              "executableSHA256": hashlib.sha256(Path(f"/proc/{app.pid}/exe").read_bytes()).hexdigest(),
                              "libraries": {n: hashlib.sha256((mounted / n.lstrip("/")).read_bytes()).hexdigest() for n in libraries}})
        save("initial-state.json", value)
        # A first-frame report alone missed a fatal third-frame layout/allocation
        # failure. Require sustained liveness before startup can be called complete.
        settle_until = time.monotonic() + 8
        while time.monotonic() < settle_until:
            sample("startup-settle")
            time.sleep(.25)
        fresh()
        milestones["sustainedStartupObservedSeconds"] = time.monotonic() - started
        # NativeImage loads independently of the canvas frame. A published GPU
        # frame can therefore coexist with blank device screenshots and fades.
        def native_images_ready(value):
            images = [node for node in value.get("nodes", [])
                      if node["kind"] == "NativeImage" and node.get("url")]
            if any(node.get("imageFailed") for node in images):
                raise RuntimeError("Website NativeImage decode failed")
            return bool(images) and all(node.get("imageReady") for node in images)

        image_state = wait(native_images_ready, "website native images decoded", 60)
        save("native-images.json", {"elapsedSeconds": time.monotonic() - started,
                                   "images": [node for node in image_state["nodes"]
                                              if node["kind"] == "NativeImage"],
                                   "decodedBytes": image_state.get("nativeImageBytes")})
        if not args.startup_only:
            xdo("windowsize", "--sync", window, 1280, 960)
            xdo("windowraise", window)
            xdo("windowfocus", "--sync", window)
            time.sleep(3)
            fresh()
            if not args.interactions_only:
                phase("top-stationary", seconds=args.stationary_seconds)
                screenshot("hero")
            scrollers = [n for n in state().get("nodes", [])
                         if n["kind"] == "Scroller" and len(n.get("bounds", [])) == 4]
            if not scrollers:
                raise RuntimeError("Website did not expose a native scroller")
            main = max(scrollers, key=lambda n: n["bounds"][2] * n["bounds"][3])
            main_id = main["id"]

            def outer():
                return next(n for n in state()["nodes"] if n["id"] == main_id)

            def wheel(button, count=8):
                x, y, width, height = outer()["bounds"]
                # Keep the same route through the website, including its full-
                # width horizontal gallery. Vertical input there must propagate
                # to the outer scroller; moving around it would hide a failure.
                xdo("mousemove", "--window", window, round(x + width * .02), round(y + height * .5))
                verify_pointer_owner()
                xdo("click", "--repeat", count, "--delay", 25, button)

            def verify_pointer_owner():
                pointer = pointer_window()
                save("pointer-owner.json", {**pointer, "expectedWindow": int(window),
                                            "expectedPID": app.pid})
                target = pointer["window"]
                if not pointer["sameScreen"] or not target:
                    raise RuntimeError(f"Pointer has no X11 client target: {pointer}")
                if target != int(window):
                    owner = _x11_client_pid(target) if app.flatpak else int(xdo("getwindowpid", target))
                    if owner != app.pid:
                        raise RuntimeError(f"Pointer target is not owned by the website process: {pointer}")

            def owned_key(key):
                state()
                focused = int(xdo("getwindowfocus", "-f"))
                if focused <= 1 or _x11_client_pid(focused) != app.pid:
                    raise RuntimeError("Keyboard focus is not owned by the website")
                xdo("key", key)
                return fresh()

            for name in ("first-traversal", "repeat-traversal")[:0 if args.interactions_only or args.stationary_only else args.traversals]:
                before = outer().get("scrollY", 0)

                def at_end(direction):
                    node = outer()
                    target = max(0, node.get("scrollUpper", 0) - node.get("scrollPage", 0)) if direction == 5 else 0
                    return abs(node.get("scrollY", 0) - target) <= 1

                def step(direction):
                    if at_end(direction):
                        # Lazy content may extend the range after entering view.
                        # Observe a settled endpoint, not just a momentary bound.
                        time.sleep(1)
                        fresh()
                        if at_end(direction):
                            return False
                    wheel(direction)
                    return True

                phase(name + "-down", lambda _: step(5))
                after = outer()
                if after.get("scrollY", 0) <= before:
                    raise RuntimeError("Real wheel input did not move the outer website scroller")
                if not at_end(5):
                    raise RuntimeError("Outer traversal did not reach the bottom within its time budget")
                screenshot(name + "-bottom")
                phase(name + "-up", lambda _: step(4))
                if outer().get("scrollY", 0) >= after.get("scrollY", 0):
                    raise RuntimeError("Real reverse wheel did not return toward the hero")
                if not at_end(4):
                    raise RuntimeError("Outer traversal did not return to the top within its time budget")
            if args.open_source:
                wheel(4, 40)
                time.sleep(1)
                labels = [n for n in state()["nodes"] if n.get("text") == "View Source"
                          and len(n.get("bounds", [])) == 4 and 0 <= n["bounds"][1] < 960]
                if not labels:
                    raise RuntimeError("No visible View Source label to exercise")
                label = min(labels, key=lambda n: n["bounds"][1])
                old_scrollers = {n["id"] for n in state()["nodes"] if n["kind"] == "Scroller"}
                x, y, width, height = label["bounds"]
                xdo("mousemove", "--window", window, round(x + width / 2), round(y + height / 2))
                verify_pointer_owner()
                xdo("click", 1)
                time.sleep(2)
                wait(lambda s: len({n["id"] for n in s.get("nodes", []) if n["kind"] == "Scroller"}
                                   - old_scrollers) >= 2,
                     "source drawer adds its tab and code scrollers")
                value = wait(lambda s: any(n.get("richTextRunCount", 0) > 1
                                             and n.get("selectable") and "\n" in n.get("text", "")
                                             for n in s.get("nodes", [])),
                             "source drawer renders selectable, styled native text")
                rich = [n for n in value["nodes"] if n.get("richTextRunCount", 0) > 1]
                if any("<pre data-pax-code-markup" in n.get("text", "") for n in rich):
                    raise RuntimeError("Source drawer displayed markup instead of native styled text")
                save("source-drawer-state.json", value)
                screenshot("source-drawer")
                added = [n for n in value["nodes"] if n["kind"] == "Scroller"
                         and n["id"] not in old_scrollers]
                code = max(added, key=lambda n: n["bounds"][3])
                tabs = min(added, key=lambda n: n["bounds"][3])
                label = next(n for n in rich if n.get("selectable"))
                code_id, text_id = code["id"], label["id"]
                original_text = label["text"]

                def node(value, node_id):
                    return next(n for n in value.get("nodes", []) if n["id"] == node_id)

                def point(x, y):
                    xdo("mousemove", "--window", window, round(x), round(y))
                    verify_pointer_owner()

                # Drag across actual first-line glyphs, inside the clipped code
                # viewport. This checks native input, not a selection setter.
                cx, cy, cw, ch = code["bounds"]
                tx, ty, _, _ = label["bounds"]
                line = label["textLayout"]["lineLogicalBounds"][0]
                start_x = max(cx + 4, tx + line[0] + 3)
                end_x = min(cx + cw - 8, tx + line[0] + line[2] - 3)
                line_y = ty + line[1] + line[3] / 2
                if end_x <= start_x + 20 or not cy < line_y < cy + ch:
                    raise RuntimeError("Source drawer has no visible first-line selection target")
                point(start_x, line_y)
                xdo("mousedown", 1)
                try:
                    time.sleep(.1)
                    point(end_x, line_y)
                    time.sleep(.1)
                finally:
                    xdo("mouseup", 1)
                selected = wait(lambda s: node(s, text_id).get("selectionEnd", 0)
                                > node(s, text_id).get("selectionStart", 0),
                                "real source text selection")
                save("source-selection-state.json", selected)

                def source_key(key):
                    # Use the actual X11 focus rather than xdotool's default
                    # _NET_WM_PID ancestor heuristic, which misses Flatpak.
                    state()
                    focused = int(xdo("getwindowfocus", "-f"))
                    if focused <= 1 or _x11_client_pid(focused) != app.pid:
                        raise RuntimeError("Keyboard focus is not owned by the website")
                    xdo("key", key)
                    return fresh()

                clipboard_checks = []

                def check_copy(name, expected):
                    source_key("ctrl+c")
                    copied, owner = read_owned_clipboard(app.pid)
                    encoded = expected.encode("utf-8")
                    evidence = {"case": name, "ownerWindow": owner, "ownerPID": app.pid,
                                "expectedBytes": len(encoded), "copiedBytes": len(copied),
                                "expectedSHA256": hashlib.sha256(encoded).hexdigest(),
                                "copiedSHA256": hashlib.sha256(copied).hexdigest(),
                                "nonASCIICharacters": sum(ord(c) > 127 for c in expected),
                                "exactUTF8Match": copied == encoded}
                    clipboard_checks.append(evidence)
                    save("source-clipboard.json", clipboard_checks)
                    if copied != encoded:
                        raise RuntimeError(f"Native source clipboard mismatch: {name}; see source-clipboard.json")

                selected_text = node(selected, text_id)
                a, b = selected_text["selectionStart"], selected_text["selectionEnd"]
                # GtkLabel selection offsets count Unicode characters, whereas
                # rich-text style ranges use UTF-8 byte offsets.
                if not 0 <= a < b <= len(original_text):
                    raise RuntimeError("Source selection has invalid Unicode character bounds")
                check_copy("pointer-selection", original_text[a:b])
                source_key("ctrl+a")
                all_selected = wait(lambda s: node(s, text_id).get("selectionStart") == 0
                                    and node(s, text_id).get("selectionEnd") == len(original_text),
                                    "Ctrl+A selects the full native source document")
                save("source-select-all-state.json", all_selected)
                check_copy("full-document", original_text)

                time.sleep(.75)
                fresh()
                outer_before = outer().get("scrollY", 0)
                point(cx + cw / 2, cy + ch / 2)
                # One detent tests interior axis ownership without reaching an
                # edge, where native chaining to the parent is intentional.
                xdo("click", 5)
                vertical = wait(lambda s: node(s, code_id).get("scrollY", 0) > 20,
                                "source code vertical wheel")
                save("source-vertical-state.json", vertical)
                time.sleep(.5)
                xdo("click", 7)
                horizontal = wait(lambda s: node(s, code_id).get("scrollX", 0) > 20,
                                  "source code horizontal wheel")
                save("source-scrolled-state.json", horizontal)
                if abs(outer().get("scrollY", 0) - outer_before) > 1:
                    raise RuntimeError(f"Source code wheel moved the outer website scroller: "
                                       f"{outer_before} -> {outer().get('scrollY', 0)}")

                # Switch to another source through the visible part of its tab;
                # labels can extend beyond the horizontal tab viewport.
                bx, by, bw, bh = tabs["bounds"]
                tab_labels = [n for n in value["nodes"] if n["kind"] == "Text"
                              and n.get("text") and len(n.get("bounds", [])) == 4
                              and by <= n["bounds"][1] < by + bh
                              and n["bounds"][0] < bx + bw
                              and n["bounds"][0] + n["bounds"][2] > bx]
                tab_labels.sort(key=lambda n: n["bounds"][0])
                if len(tab_labels) < 2:
                    raise RuntimeError("Source drawer has no second visible source tab")
                tab = tab_labels[1]
                x, y, width, height = tab["bounds"]
                point((max(bx, x) + min(bx + bw, x + width)) / 2, y + height / 2)
                xdo("click", 1)
                changed = wait(lambda s: any(n.get("richTextRunCount", 0) > 1
                                             and n.get("text") != original_text
                                             for n in s.get("nodes", [])),
                               "source tab replaces native styled content")
                save("source-second-tab-state.json", changed)
                close = next(n for n in changed["nodes"] if n.get("text") == "×"
                             and cx <= n["bounds"][0] < cx + cw
                             and 0 <= n["bounds"][1] < cy)
                x, y, width, height = close["bounds"]
                point(x + width / 2, y + height / 2)
                xdo("click", 1)
                wait(lambda s: not ({n["id"] for n in s.get("nodes", [])}
                                   & {n["id"] for n in added}),
                     "source drawer retires its nested scrollers")
                save("source-interactions.json", {
                    "nativeSelection": True, "nativeSelectionCopy": True,
                    "nativeSelectAllCopy": True, "verticalWheel": True, "horizontalWheel": True,
                    "outerScrollerStayedStill": True, "secondSourceTab": tab["text"],
                    "closed": True, "allSourceTabsQualified": False})
            if args.interaction_cases:
                WebsiteInteractions(state=state, wait=wait, fresh=fresh, xdo=xdo,
                                    pointer_owner=verify_pointer_owner, key=owned_key,
                                    save=save, screenshot=screenshot, window=window,
                                    outer_id=main_id, clipboard=lambda: read_owned_clipboard(app.pid)
                                    ).run(args.interaction_cases)
        app.terminate()
        if app.wait(timeout=30) != 0:
            raise RuntimeError("Website did not close normally")
        log.flush()
        if "gl_refs=0 gl_bytes=0" not in (out / "app.log").read_text():
            raise RuntimeError("Owned GL publications were not observed retired")
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
                failure = (failure or "") + "; forced cleanup after normal close timed out"
        log.close()
        save("startup.json", {**milestones,
                              "clockAnchor": {"monotonicNs": started_ns,
                                              "unixNs": started_unix_ns,
                                              "monotonicReadEndNs": anchor_read_end_ns},
                              "observationPollSeconds": .1,
                              "hostReportPeriodSeconds": .25, "coldFilesystemCache": False,
                              "caveat": "First committed report is an observed upper bound; not GPU presentation or full-site readiness."})
        save("samples.json", samples)
        save("phases.json", {"phases": phases, "displayFPSQualified": False,
                             "requestedTraversals": args.traversals,
                             "requestedStationarySeconds": args.stationary_seconds or args.seconds,
                             "stationaryOnly": args.stationary_only,
                             "cacheEvictionObservation": eviction_observation,
                             "screenshotsEnabled": args.screenshots,
                             "memoryCaveat": "Opt-in screenshots occur outside timing but can change later RSS high-water marks.",
                             "percentiles": "Host report histograms are startup-inclusive; phase means use cumulative differences."})
        save("checks.json", {"completed": completed, "error": failure,
                             "probeSHA256": probe_hash,
                             "interactionHelperSHA256": hashlib.sha256(
                                 Path(__file__).with_name("website_interactions.py").read_bytes()).hexdigest(),
                             "requestedInteractionCases": args.interaction_cases,
                             "interactionsOnly": args.interactions_only,
                             "exitCode": app.poll() if app else None,
                             "fullWebsiteAcceptance": False})
    if not completed:
        raise SystemExit(failure)


if __name__ == "__main__":
    main()
