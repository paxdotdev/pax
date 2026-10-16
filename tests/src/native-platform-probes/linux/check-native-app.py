#!/usr/bin/env python3
"""Drive the real desktop fixture through X11 input and opt-in host diagnostics.

Run inside the authenticated NVIDIA Xorg session. Screenshots are explicit
one-shot test downloads; continuous presentation does not read back pixels.
"""
import argparse
import json
import os
from pathlib import Path
import signal
import struct
import zlib
import subprocess
import time



def png_pixels(path):
    """Decode the explicit GTK test capture; production never calls this path."""
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("Expected a PNG fixture capture")
    position, compressed = 8, bytearray()
    while position < len(data):
        size = struct.unpack_from(">I", data, position)[0]
        kind = data[position+4:position+8]
        chunk = data[position+8:position+8+size]
        if kind == b"IHDR":
            width, height, depth, color, compression, filtering, interlace = struct.unpack(">IIBBBBB", chunk)
            if depth != 8 or color not in (2, 6) or compression or filtering or interlace:
                raise ValueError("Expected a noninterlaced RGB/RGBA8 GTK capture")
            channels = 4 if color == 6 else 3
        elif kind == b"IDAT":
            compressed.extend(chunk)
        position += size + 12
    raw = zlib.decompress(compressed)
    stride = width * channels
    if len(raw) != height * (stride + 1):
        raise ValueError("Unexpected PNG scanline length")
    rows, previous = [], bytearray(stride)
    for y in range(height):
        offset = y * (stride + 1)
        filter_type, row = raw[offset], bytearray(raw[offset+1:offset+1+stride])
        for x in range(stride):
            left = row[x-channels] if x >= channels else 0
            up, upper_left = previous[x], previous[x-channels] if x >= channels else 0
            if filter_type == 0:
                prediction = 0
            elif filter_type == 1:
                prediction = left
            elif filter_type == 2:
                prediction = up
            elif filter_type == 3:
                prediction = (left + up) // 2
            elif filter_type == 4:
                p = left + up - upper_left
                a, b, c = abs(p-left), abs(p-up), abs(p-upper_left)
                prediction = left if a <= b and a <= c else up if b <= c else upper_left
            else:
                raise ValueError("Unknown PNG filter")
            row[x] = (row[x] + prediction) & 255
        rows.append(row)
        previous = row
    return width, height, channels, rows


def visible_text_pixels(pixels, bounds):
    width, height, channels, rows = pixels
    x, y, w, h = bounds
    return sum(max(rows[j][i*channels:i*channels+3]) < 130
               for j in range(max(0, round(y)+3), min(height, round(y+h)-3))
               for i in range(max(0, round(x)+3), min(width, round(x+w)-3)))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("app", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--seconds", type=float, default=60)
    parser.add_argument("--hold-ms", type=int, default=0, choices=range(0, 5001), metavar="0..5000")
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = output / "state.json"
    screenshot = output / "native.png"
    environment = dict(os.environ, GSK_RENDERER="ngl", PAX_NATIVE_TEST_REPORT=str(report),
                       PAX_NATIVE_TEST_SCREENSHOT=str(screenshot))
    if args.hold_ms:
        environment["PAX_NATIVE_TEST_RETAIN_MS"] = str(args.hold_ms)
    checks, samples, observations = [], [], []
    log = (output / "app.log").open("w")
    app = subprocess.Popen([str(args.app.resolve())], env=environment, stdout=log, stderr=log)

    def check(condition, name):
        checks.append({"check": name, "passed": bool(condition)})
        if not condition:
            raise RuntimeError(name)
        print("PASS", name, flush=True)

    def state():
        if app.poll() is not None:
            raise RuntimeError(f"Native app exited {app.returncode}; inspect {output / 'app.log'}")
        try:
            value = json.loads(report.read_text())
            if "ticks" in value and len(observations) < 4096:
                observations.append({k: value[k] for k in
                                     ["ticks", "frames", "pendingCommit", "backpressure", "glLiveImages", "glLiveBytes"]})
            return value
        except (FileNotFoundError, json.JSONDecodeError):
            return {}

    def wait(predicate, name, seconds=15):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            value = state()
            if predicate(value):
                return value
            time.sleep(.05)
        raise RuntimeError("Timeout: " + name)

    def node(value, kind, text=None):
        return next((n for n in value.get("nodes", [])
                     if n["kind"] == kind and (text is None or n.get("text") == text)), None)

    def xdo(*arguments):
        return subprocess.check_output(["xdotool", *map(str, arguments)], text=True).strip()

    def click(control, x=.5, y=.5):
        left, top, width, height = control["bounds"]
        xdo("mousemove", "--window", window, round(left + width*x), round(top + height*y))
        xdo("click", 1)

    def capture(name):
        Path(str(screenshot) + ".request").touch()
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline and Path(str(screenshot) + ".request").exists():
            state()
            time.sleep(.05)
        check(screenshot.exists() and not Path(str(screenshot) + ".request").exists(), name)
        destination = output / (name + ".png")
        screenshot.rename(destination)
        return destination

    try:
        value = wait(lambda s: node(s, "Text", "Initialized by user-owned main"), "startup initializer")
        check(True, "user-owned main startup data reached actual Pax Text")
        window = xdo("search", "--pid", app.pid, "--name", "Pax native desktop acceptance").splitlines()[-1]
        xdo("windowfocus", "--sync", window)
        check(node(value, "Text", "Bundled Manrope: WWW iii 012345").get("resolvedFont") == "Manrope",
              "native Pango layout resolves the packaged Manrope font")
        initial = capture("initial-unscrolled")
        pixels = png_pixels(initial)
        outer_label = node(value, "Text", "Outer native scroller")
        nested_editor = max((n for n in value["nodes"] if n["kind"] == "Textbox"),
                            key=lambda n: n["bounds"][1])
        check(visible_text_pixels(pixels, outer_label["bounds"]) > 20,
              "outer native scroller label is visibly composited above its GPU background")
        check(visible_text_pixels(pixels, nested_editor["bounds"]) > 20,
              "nested native editor text is visibly composited above its GPU background")
        click(node(value, "Button", "Increment"))
        value = wait(lambda s: node(s, "Text", "Count: 1"), "Rust increment")
        check(True, "actual native button invokes Rust handler")
        editor = min((n for n in value["nodes"] if n["kind"] == "Textbox"), key=lambda n: n["bounds"][1])
        editor_id = editor["id"]
        click(editor, .25)
        xdo("key", "ctrl+a")
        xdo("type", "--clearmodifiers", "--delay", 2, "Linux native input retained")
        xdo("key", "ctrl+a")
        value = wait(lambda s: node(s, "Text", "Rust received: Linux native input retained"), "textbox Rust echo")
        check(True, "native editable text reaches Rust and two-way binding")
        start_frame = value["frames"]
        value = wait(lambda s: s.get("frames", 0) >= start_frame + 75, "retained selection during animation")
        editor = next(n for n in value["nodes"] if n["id"] == editor_id)
        check(editor["focused"] and sorted([editor["selectionStart"], editor["selectionEnd"]]) == [0, len(editor["text"])],
              "full native selection survives transformed frame delivery")
        check(all(n.get("text") == "Linux native input retained" for n in value["nodes"] if n["kind"] == "Textbox"),
              "nested editor observes shared Rust property")
        click(node(value, "Button", "Increment"))
        value = wait(lambda s: node(s, "Text", "Count: 2"), "second Rust increment")
        stable_after = value["frames"] + 75
        value = wait(lambda s: s.get("frames", 0) >= stable_after, "native frame animation settles")
        editor = next(n for n in value["nodes"] if n["id"] == editor_id)
        opaque, fractional = editor.get("opaquePoint"), editor.get("fractionalPoint")
        check(opaque is not None and fractional is not None and opaque[2] < .01 and 0 < fractional[2] < 1,
              "runtime mask provides interior opaque and fractional native-editor regions")
        presses = value["scenePointerPresses"]
        xdo("mousemove", "--window", window, round(opaque[0]), round(opaque[1]))
        xdo("click", 1)
        value = wait(lambda s: s.get("scenePointerPresses", 0) > presses, "opaque mask routes actual click to Pax scene")
        check(not next(n for n in value["nodes"] if n["id"] == editor_id)["focused"],
              "fully occluded editor region rejects native focus")
        xdo("mousemove", "--window", window, round(fractional[0]), round(fractional[1]))
        xdo("click", 1)
        value = wait(lambda s: next(n for n in s.get("nodes", []) if n["id"] == editor_id)["focused"],
                     "fractional mask retains actual native focus")
        check(True, "fractional editor region remains interactive")
        click(node(value, "Checkbox"))
        value = wait(lambda s: node(s, "Text", "Checked: yes"), "checkbox binding")
        check(node(value, "Checkbox")["checked"], "native checkbox updates Pax binding")
        scrollers = sorted((n for n in value["nodes"] if n["kind"] == "Scroller"), key=lambda n: n["bounds"][2])
        inner, outer = scrollers
        x, y, w, h = inner["bounds"]
        xdo("mousemove", "--window", window, round(x+w*.8), round(y+h*.75))
        xdo("click", "--repeat", 6, "--delay", 60, 5)
        value = wait(lambda s: next(n for n in s.get("nodes", []) if n["id"] == inner["id"])["scrollY"] > 50,
                     "inner native scroller")
        check(next(n for n in value["nodes"] if n["id"] == outer["id"])["scrollY"] == 0,
              "nested native scrolling preserves outer offset")
        capture("interactions")
        deadline = time.monotonic() + args.seconds
        count = 2
        next_resize = time.monotonic() + 10
        wide = False
        while time.monotonic() < deadline:
            value = state()
            click(node(value, "Button", "Increment"))
            count += 1
            value = wait(lambda s: node(s, "Text", f"Count: {count}"), "continuous Rust animation")
            if time.monotonic() >= next_resize:
                wide = not wide
                xdo("windowsize", window, 960 if wide else 800, 700 if wide else 640)
                next_resize = time.monotonic() + 10
            gpu = value.get("runtime", {}).get("gpu", {})
            check(gpu.get("liveAllocations", 49) <= 48 and gpu.get("bytes", 2**30) <= 256*1024*1024,
                  "Vulkan live and retiring allocations stay within global budget")
            check(value["glLiveImages"] <= 48 and value["glLiveBytes"] <= 256*1024*1024,
                  "immutable GL live and copy-pending publications stay within budget")
            samples.append({k: value[k] for k in ["frames", "backpressure", "glLiveImages", "glLiveBytes"]} | {"gpu": gpu})
            time.sleep(1)
        check(samples[-1]["gpu"].get("reused", 0) > 50, "continuous frames reuse Vulkan exports with completion evidence")
        if args.hold_ms:
            blocked = [o for o in observations if o["pendingCommit"] and o["backpressure"] > 0]
            check(bool(blocked), "retained GDK consumers cause actual publication backpressure")
            check(all(o["ticks"] == o["frames"] + 1 for o in blocked),
                  "backpressure retains at most one uncommitted native/GPU tick")
            check(value["frames"] > blocked[0]["frames"], "frame delivery resumes after retained consumers release")
        capture("continuous-final")
        app.send_signal(signal.SIGTERM)
        check(app.wait(timeout=20) == 0, "close drains the native application successfully")
        log.flush()
        contents = (output / "app.log").read_text()
        check("PAX_LINUX_CLOSED" in contents and "gl_refs=0 gl_bytes=0" in contents,
              "standalone close retires all owned GL publications")
    except Exception:
        if app.poll() is None:
            try:
                capture("failure")
            except Exception as capture_error:
                print("Failure screenshot unavailable:", capture_error, flush=True)
        raise
    finally:
        if app.poll() is None:
            app.terminate()
            try:
                app.wait(timeout=5)
            except subprocess.TimeoutExpired:
                app.kill()
                app.wait()
        log.close()
        (output / "checks.json").write_text(json.dumps({"checks": checks, "samples": samples, "observations": observations}, indent=2) + "\n")


if __name__ == "__main__":
    main()
